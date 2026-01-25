use crate::cli::commands::settings::{SettingDefinition, SettingType};
use crate::config::Config;
use crate::display::terminal::{init_tui, restore_tui, Tui};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::time::Duration;

pub async fn run_settings_tui(config: &Config, settings: Vec<SettingDefinition>) -> Result<()> {
    let mut terminal = init_tui()?;
    let mut app = SettingsApp::new(config.clone(), settings);
    let res = run_app(&mut terminal, &mut app).await;
    restore_tui()?;
    res
}

#[derive(PartialEq)]
enum EditMode {
    View,
    Input,
}

struct SettingsApp {
    config: Config,
    settings: Vec<SettingDefinition>,
    state: ListState,
    should_quit: bool,
    mode: EditMode,
    input_buffer: String,
    status_message: Option<(String, Color)>,
}

impl SettingsApp {
    fn new(config: Config, settings: Vec<SettingDefinition>) -> Self {
        let mut state = ListState::default();
        if !settings.is_empty() {
            state.select(Some(0));
        }
        Self {
            config,
            settings,
            state,
            should_quit: false,
            mode: EditMode::View,
            input_buffer: String::new(),
            status_message: None,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.settings.len() - 1 {
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
                    self.settings.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    async fn handle_selection(&mut self) -> Result<()> {
        if let Some(index) = self.state.selected() {
            let setting = self.settings[index].clone();

            match setting.setting_type {
                SettingType::Boolean => {
                    let current_bool = setting.current_value == "true";
                    let new_value = (!current_bool).to_string();
                    self.update_setting(&setting.key, &new_value).await?;
                }
                SettingType::String | SettingType::Number => {
                    self.mode = EditMode::Input;
                    self.input_buffer = setting.current_value.clone();
                }
                _ => {
                    self.set_status("Editing this type is not supported in TUI", Color::Red);
                }
            }
        }
        Ok(())
    }

    async fn update_setting(&mut self, key: &str, value: &str) -> Result<()> {
        let mut updated_config = self.config.clone();
        if let Err(e) = updated_config.set_value(key, value) {
            self.set_status(&format!("Error: {}", e), Color::Red);
            return Ok(());
        }

        if let Err(e) = updated_config.validate() {
            self.set_status(&format!("Validation Error: {}", e), Color::Red);
            return Ok(());
        }

        if let Err(e) = updated_config.save(None).await {
            self.set_status(&format!("Save Error: {}", e), Color::Red);
            return Ok(());
        }

        self.config = updated_config;
        self.settings = crate::cli::commands::settings::get_all_settings(&self.config);
        self.set_status("Setting updated successfully", Color::Green);
        Ok(())
    }

    fn set_status(&mut self, msg: &str, color: Color) {
        self.status_message = Some((msg.to_string(), color));
    }

    async fn submit_input(&mut self) -> Result<()> {
        if let Some(index) = self.state.selected() {
            let setting = self.settings[index].clone();

            // Validate number if needed
            if let SettingType::Number = setting.setting_type
                && self.input_buffer.parse::<f64>().is_err() {
                    self.set_status("Invalid number format", Color::Red);
                    return Ok(());
                }

            let value = self.input_buffer.clone();
            self.update_setting(&setting.key, &value).await?;
        }
        self.mode = EditMode::View;
        Ok(())
    }
}

async fn run_app(terminal: &mut Tui, app: &mut SettingsApp) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press {
                    match app.mode {
                        EditMode::View => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            KeyCode::Enter | KeyCode::Char(' ') => app.handle_selection().await?,
                            _ => {}
                        },
                        EditMode::Input => match key.code {
                            KeyCode::Enter => app.submit_input().await?,
                            KeyCode::Esc => {
                                app.mode = EditMode::View;
                                app.set_status("Cancelled", Color::Yellow);
                            }
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                app.input_buffer.push(c);
                            }
                            _ => {}
                        },
                    }
                }
        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(frame: &mut ratatui::Frame, app: &mut SettingsApp) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // List
            Constraint::Length(3), // Description
            Constraint::Length(1), // Status/Footer
        ])
        .split(frame.area());

    // Header
    let header = Paragraph::new("Grok CLI Settings (TUI)")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, vertical[0]);

    // List
    let items: Vec<ListItem> = app
        .settings
        .iter()
        .map(|setting| {
            let value_style = if setting.current_value == "true" {
                Style::default().fg(Color::Green)
            } else if setting.current_value == "false" {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("{:<30}", setting.label),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(" │ "),
                Span::styled(setting.current_value.to_string(), value_style),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Settings"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, vertical[1], &mut app.state);

    // Description
    let description = if let Some(index) = app.state.selected() {
        &app.settings[index].description
    } else {
        "Select a setting to edit"
    };

    let desc_widget = Paragraph::new(description.to_string())
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Description"));
    frame.render_widget(desc_widget, vertical[2]);

    // Status/Help
    let status_text = if let Some((msg, color)) = &app.status_message {
        Span::styled(msg, Style::default().fg(*color))
    } else {
        Span::styled(
            "Press 'q' to quit, Enter to edit/toggle",
            Style::default().fg(Color::DarkGray),
        )
    };
    frame.render_widget(Paragraph::new(status_text), vertical[3]);

    // Input Popup
    if app.mode == EditMode::Input {
        let block = Block::default()
            .title("Edit Value")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Blue));

        let area = centered_rect(60, 20, frame.area());
        frame.render_widget(Clear, area); // Clear background

        let input = Paragraph::new(app.input_buffer.clone())
            .style(Style::default().fg(Color::White))
            .block(block);

        frame.render_widget(input, area);
    }
}

/// Helper function to center a rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
