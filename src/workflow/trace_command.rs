//! /trace slash command implementation (Task 235).
//!
//! Supports:
//!   /trace            → show last trace (TUI if possible, else text)
//!   /trace last
//!   /trace list
//!   /trace json
//!   /trace <partial-timestamp>   (e.g. 20250405-1430)
//!
//! The command is usable from both:
//! - ACP sessions (returns nice markdown/text)
//! - Pure CLI interactive (can launch the rich ratatui viewer)

use crate::workflow::persistence::{
    list_traces, load_latest_trace, load_trace, trace_file_summary,
};
use crate::workflow::WorkflowTrace;

/// Handle the `/trace` command. Returns a human-readable response string.
///
/// In pure TTY CLI mode this may launch the interactive TUI viewer
/// (blocking until the user quits). In ACP / non-TTY it always returns text.
pub async fn handle_trace_command(subcommand: &str) -> String {
    let sub = subcommand.trim().to_lowercase();

    match sub.as_str() {
        "" | "last" => handle_last().await,
        "list" => handle_list(),
        "json" => handle_json().await,
        other => handle_specific(other).await,
    }
}

async fn handle_last() -> String {
    match load_latest_trace() {
        Ok(Some(trace)) => {
            // Try to launch the rich TUI viewer when we are in an interactive terminal.
            if is_interactive_terminal() {
                match crate::display::run_workflow_viewer(trace.clone()).await {
                    Ok(()) => {
                        return "✅ Opened workflow trace viewer. Press q or Esc to return.".to_string();
                    }
                    Err(e) => {
                        // Fall through to text rendering
                        tracing::warn!("TUI viewer failed: {}. Falling back to text.", e);
                    }
                }
            }

            // Text fallback (works everywhere)
            format!(
                "## Latest Workflow Trace\n\n{}",
                format_trace_text(&trace)
            )
        }
        Ok(None) => {
            "No workflow traces found yet.\n\n\
             Traces are automatically saved when you run validation workflows \
             (e.g. code generation followed by cargo check/clippy/test)."
                .to_string()
        }
        Err(e) => format!("❌ Failed to load latest trace: {}", e),
    }
}

fn handle_list() -> String {
    match list_traces() {
        Ok(traces) if traces.is_empty() => {
            "No saved workflow traces yet.".to_string()
        }
        Ok(traces) => {
            let mut out = String::from("## Saved Workflow Traces (newest first)\n\n");
            for (i, path) in traces.iter().take(25).enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, trace_file_summary(path)));
            }
            if traces.len() > 25 {
                out.push_str(&format!("... and {} more\n", traces.len() - 25));
            }
            out.push_str("\nUse `/trace <timestamp>` or `/trace last` to view one.");
            out
        }
        Err(e) => format!("❌ Failed to list traces: {}", e),
    }
}

async fn handle_json() -> String {
    match load_latest_trace() {
        Ok(Some(trace)) => {
            match serde_json::to_string_pretty(&trace) {
                Ok(json) => format!("```json\n{}\n```", json),
                Err(e) => format!("❌ Failed to serialize trace: {}", e),
            }
        }
        Ok(None) => "No trace available for JSON dump.".to_string(),
        Err(e) => format!("❌ Error: {}", e),
    }
}

async fn handle_specific(id_or_prefix: &str) -> String {
    let traces = match list_traces() {
        Ok(t) => t,
        Err(e) => return format!("❌ Could not list traces: {}", e),
    };

    // Try exact filename match or prefix match
    let found = traces.iter().find(|p| {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        name == id_or_prefix || name.contains(id_or_prefix)
    });

    match found {
        Some(path) => {
            match load_trace(path) {
                Ok(trace) => {
                    if is_interactive_terminal() {
                        // Launch viewer for the specific trace
                        match crate::display::run_workflow_viewer(trace.clone()).await {
                            Ok(()) => {
                                return format!(
                                    "✅ Opened trace {} in viewer.",
                                    trace_file_summary(path)
                                );
                            }
                            Err(_) => {}
                        }
                    }
                    format!(
                        "## Workflow Trace: {}\n\n{}",
                        trace_file_summary(path),
                        format_trace_text(&trace)
                    )
                }
                Err(e) => format!("❌ Failed to load {}: {}", trace_file_summary(path), e),
            }
        }
        None => {
            format!(
                "No trace matching '{}' found.\n\nTry `/trace list` to see available traces.",
                id_or_prefix
            )
        }
    }
}

/// Simple heuristic: are we in an interactive terminal where a TUI makes sense?
fn is_interactive_terminal() -> bool {
    // In ACP stdio mode we usually don't want to steal the terminal.
    // A good heuristic is whether stdout is a TTY and we are not inside a known agent context.
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
        && std::env::var("GROK_ACP").is_err()
        && std::env::var("AGENT_CLIENT_PROTOCOL").is_err()
}

/// Pretty-print a trace as markdown/text (used when TUI is not available).
fn format_trace_text(trace: &WorkflowTrace) -> String {
    let mut out = String::new();

    for (i, step) in trace.steps.iter().enumerate() {
        match step {
            crate::workflow::WorkflowStep::UserPrompt(p) => {
                out.push_str(&format!("**{}.** 👤 User: {}\n\n", i + 1, truncate(p, 120)));
            }
            crate::workflow::WorkflowStep::LlmGeneratedCode(code) => {
                out.push_str(&format!(
                    "**{}.** 📝 LLM generated code ({} bytes)\n\n```rust\n{}\n```\n\n",
                    i + 1,
                    code.len(),
                    truncate(code, 400)
                ));
            }
            crate::workflow::WorkflowStep::ToolRun { tool, output, success } => {
                let icon = if *success { "✅" } else { "❌" };
                out.push_str(&format!(
                    "**{}.** {} Tool: `{}`\n\n```\n{}\n```\n\n",
                    i + 1,
                    icon,
                    tool,
                    truncate(output, 600)
                ));
            }
            crate::workflow::WorkflowStep::Decision { passed } => {
                let status = if *passed { "PASS ✅" } else { "FAIL ❌" };
                out.push_str(&format!("**{}.** ⚖️ Decision: **{}**\n\n", i + 1, status));
            }
            crate::workflow::WorkflowStep::ReturnedToLlm(reason) => {
                out.push_str(&format!(
                    "**{}.** 🔄 Returned to LLM: {}\n\n",
                    i + 1,
                    truncate(reason, 200)
                ));
            }
            crate::workflow::WorkflowStep::ReturnedToUser(msg) => {
                out.push_str(&format!(
                    "**{}.** ✅ Returned to user: {}\n\n",
                    i + 1,
                    truncate(msg, 200)
                ));
            }
        }
    }

    if let Some(passed) = trace.last_decision_passed() {
        out.push_str(&format!(
            "---\n**Final result:** {}\n",
            if passed { "All steps passed" } else { "Some steps failed" }
        ));
    }

    out
}

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

    #[test]
    fn trace_command_list_empty_is_graceful() {
        // We can't easily mock the filesystem here, but the function should not panic.
        let _ = handle_list();
    }
}
