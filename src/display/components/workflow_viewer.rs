//! Gemini-style TUI Workflow Trace Viewer (Task 233).
//!
//! Renders a full `WorkflowTrace` with:
//! - Timeline / step list (left pane)
//! - Expandable detail panel for the selected step (right pane)
//! - Color coding: success=green, fail=red, decision=highlight
//! - Special rendering for LLM-generated code and tool outputs
//! - Keyboard navigation (↑/↓, j/k, Enter/Space to toggle detail, q/Esc to quit)
//!
//! Usage:
//! ```ignore
//! use crate::workflow::WorkflowTrace;
//! use crate::display::components::workflow_viewer::run_workflow_viewer;
//!
//! run_workflow_viewer(trace).await?;
//! ```

use crate::display::terminal::{Tui, init_tui, restore_tui};
use crate::workflow::{WorkflowStep, WorkflowTrace};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::collections::HashSet;
use std::time::Duration;

/// Entry point to display a workflow trace in a rich TUI viewer.
/// Restores terminal on exit.
pub async fn run_workflow_viewer(trace: WorkflowTrace) -> Result<()> {
    if trace.steps.is_empty() {
        println!("No workflow steps to display.");
        return Ok(());
    }

    let mut terminal = init_tui()?;
    let mut app = WorkflowViewerApp::new(trace);
    let res = run_app(&mut terminal, &mut app).await;
    restore_tui()?;
    res
}

struct WorkflowViewerApp {
    trace: WorkflowTrace,
    state: ListState,
    expanded: HashSet<usize>, // indices of expanded steps (for future collapsible use)
    should_quit: bool,
    show_full_detail: bool, // toggle full vs summary in detail pane
}

impl WorkflowViewerApp {
    fn new(trace: WorkflowTrace) -> Self {
        let mut state = ListState::default();
        if !trace.steps.is_empty() {
            state.select(Some(0));
        }
        Self {
            trace,
            state,
            expanded: HashSet::new(),
            should_quit: false,
            show_full_detail: true,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.trace.steps.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.trace.steps.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn toggle_expand(&mut self) {
        if let Some(i) = self.state.selected() {
            if self.expanded.contains(&i) {
                self.expanded.remove(&i);
            } else {
                self.expanded.insert(i);
            }
            self.show_full_detail = !self.show_full_detail;
        }
    }

    fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    fn selected_step(&self) -> Option<&WorkflowStep> {
        self.selected_index()
            .and_then(|i| self.trace.steps.get(i))
    }
}

async fn run_app(terminal: &mut Tui, app: &mut WorkflowViewerApp) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                KeyCode::Enter | KeyCode::Char(' ') => app.toggle_expand(),
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    // Toggle full vs truncated detail
                    app.show_full_detail = !app.show_full_detail;
                }
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(frame: &mut ratatui::Frame, app: &mut WorkflowViewerApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main area (timeline + detail)
            Constraint::Length(3),  // Footer / legend
        ])
        .split(frame.area());

    // Header
    let header = Paragraph::new(" Workflow Trace Viewer (Gemini-style) ")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Split main area: left = timeline, right = detail
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    // === Timeline (left) ===
    let items: Vec<ListItem> = app
        .trace
        .steps
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            let (icon, color, summary) = step_summary(step);
            let selected = app.selected_index() == Some(idx);

            let style = if selected {
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(color)
            };

            let prefix = if app.expanded.contains(&idx) { "▾" } else { "▸" };

            let content = Line::from(vec![
                Span::styled(format!("{:02} ", idx), Style::default().fg(Color::Gray)),
                Span::raw(prefix),
                Span::raw(" "),
                Span::styled(icon, style),
                Span::raw(" "),
                Span::styled(summary, style),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Timeline "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, main_chunks[0], &mut app.state);

    // === Detail pane (right) ===
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .title(" Detail (Enter/Space to toggle, 'f' for full) ");

    let detail_content = if let Some(step) = app.selected_step() {
        render_step_detail(step, app.show_full_detail)
    } else {
        Paragraph::new("No step selected").style(Style::default().fg(Color::Gray))
    };

    let detail_widget = detail_content
        .block(detail_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(detail_widget, main_chunks[1]);

    // Footer / legend
    let legend = Paragraph::new(
        "↑/↓ or j/k: navigate  •  Enter/Space: toggle expand  •  f: full/truncated  •  q/Esc: quit",
    )
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(legend, chunks[2]);
}

/// Returns (icon, color, short summary) for a step in the timeline.
fn step_summary(step: &WorkflowStep) -> (String, Color, String) {
    match step {
        WorkflowStep::UserPrompt(p) => {
            ("👤".to_string(), Color::Blue, truncate(p, 45))
        }
        WorkflowStep::LlmGeneratedCode(code) => {
            (
                "📝".to_string(),
                Color::Yellow,
                format!("code ({} bytes)", code.len()),
            )
        }
        WorkflowStep::ToolRun { tool, success, .. } => {
            let icon = if *success { "✓" } else { "✗" };
            let color = if *success { Color::Green } else { Color::Red };
            let summary = format!("{} {}", icon, tool);
            (summary, color, truncate(tool, 40))
        }
        WorkflowStep::Decision { passed } => {
            let status = if *passed { "PASS" } else { "FAIL" };
            let color = if *passed { Color::Green } else { Color::Red };
            ("⚖".to_string(), color, format!("Decision: {}", status))
        }
        WorkflowStep::ReturnedToLlm(reason) => {
            ("🔄".to_string(), Color::Magenta, truncate(reason, 40))
        }
        WorkflowStep::ReturnedToUser(msg) => {
            ("✅".to_string(), Color::Cyan, truncate(msg, 40))
        }
    }
}

/// Renders rich detail for the selected step.
fn render_step_detail(step: &WorkflowStep, full: bool) -> Paragraph<'static> {
    match step {
        WorkflowStep::UserPrompt(prompt) => Paragraph::new(vec![
            Line::from(Span::styled("User Prompt", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(if full { prompt.clone() } else { truncate(prompt, 500) }),
        ]),

        WorkflowStep::LlmGeneratedCode(code) => {
            let header = Span::styled(
                "LLM Generated Code",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            );
            let body = if full {
                code.clone()
            } else {
                truncate(code, 800)
            };

            Paragraph::new(vec![
                Line::from(header),
                Line::from(""),
                Line::from(Span::styled(
                    body,
                    Style::default().fg(Color::White).bg(Color::Black),
                )),
            ])
        }

        WorkflowStep::ToolRun { tool, output, success } => {
            let status = if *success { "✓ SUCCESS" } else { "✗ FAILED" };
            let color = if *success { Color::Green } else { Color::Red };

            let header = Span::styled(
                format!("Tool: {}  [{}]", tool, status),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            );

            let body = if full {
                output.clone()
            } else {
                truncate(output, 1200)
            };

            Paragraph::new(vec![
                Line::from(header),
                Line::from(""),
                Line::from(body),
            ])
        }

        WorkflowStep::Decision { passed } => {
            let status = if *passed { "PASS ✓" } else { "FAIL ✗" };
            let color = if *passed { Color::Green } else { Color::Red };

            Paragraph::new(vec![
                Line::from(Span::styled(
                    format!("Decision Gate: {}", status),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(if *passed {
                    "All validation steps succeeded. Code ready for user."
                } else {
                    "Validation failed. Trace will be returned to LLM for fixes."
                }),
            ])
        }

        WorkflowStep::ReturnedToLlm(reason) => Paragraph::new(vec![
            Line::from(Span::styled(
                "Returned to LLM for Fix",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(reason.clone()),
        ]),

        WorkflowStep::ReturnedToUser(msg) => Paragraph::new(vec![
            Line::from(Span::styled(
                "Returned to User / Editor",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(msg.clone()),
        ]),
    }
}

/// Simple truncation helper (adds … when needed).
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.min(s.len())])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::WorkflowTrace;

    #[test]
    fn workflow_viewer_app_basic_construction() {
        let mut trace = WorkflowTrace::new();
        trace.push(WorkflowStep::UserPrompt("test prompt".into()));
        trace.push(WorkflowStep::Decision { passed: true });

        let app = WorkflowViewerApp::new(trace);
        assert_eq!(app.trace.steps.len(), 2);
        assert!(app.selected_index().is_some());
    }

    #[test]
    fn step_summary_produces_colors() {
        let step = WorkflowStep::ToolRun {
            tool: "cargo check".into(),
            output: "ok".into(),
            success: true,
        };
        let (_, color, _) = step_summary(&step);
        assert_eq!(color, Color::Green);
    }
}
