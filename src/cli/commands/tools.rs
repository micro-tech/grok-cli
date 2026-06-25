//! Tools command handler
//!
//! Refactored to return structured `DisplayData` (Task 131).

use anyhow::Result;
use colored::Colorize;

use crate::cli::display_data::DisplayData;

pub async fn handle_tools_command(command: crate::ToolsAction) -> Result<DisplayData> {
    match command {
        crate::ToolsAction::List => {
            let mut lines = vec![
                "Available Tools".bright_cyan().bold().to_string(),
                format!(
                    "  {} — tools for interacting with the system",
                    format!(
                        "{} tool(s)",
                        crate::tools::registry::get_tool_definitions().len()
                    )
                    .dimmed()
                ),
            ];

            for tool_json in crate::tools::registry::get_tool_definitions() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(tool_json) {
                    if let (Some(name), Some(desc)) = (
                        v.get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str()),
                        v.get("function")
                            .and_then(|f| f.get("description"))
                            .and_then(|d| d.as_str()),
                    ) {
                        lines.push(format!(
                            "  {} {}  [{}]",
                            "•".bright_white(),
                            name.bright_yellow().bold(),
                            desc.dimmed()
                        ));
                    }
                }
            }

            lines.push(String::new());
            lines.push(format!(
                "  Tip: use {} or {} to see more details",
                "grok tools describe <name>".bright_cyan(),
                "grok tools examples <name>".bright_cyan()
            ));

            Ok(DisplayData::Text(lines.join("\n")))
        }

        crate::ToolsAction::Describe { name } => {
            match crate::tools::discovery_tools::describe_tool(&name) {
                Ok(schema) => Ok(DisplayData::Text(format!(
                    "{}\n\n{}",
                    format!("Tool Schema: {}", name).bright_cyan().bold(),
                    schema
                ))),
                Err(e) => Ok(DisplayData::Error(format!(
                    "{} {}\n  Run {} to see available tools.",
                    "✗".bright_red(),
                    e,
                    "grok tools list".bright_cyan()
                ))),
            }
        }

        crate::ToolsAction::Examples { name } => {
            match crate::tools::discovery_tools::tool_examples(&name) {
                Ok(examples) => Ok(DisplayData::Text(format!(
                    "{}\n\n{}",
                    format!("Tool Examples: {}", name).bright_cyan().bold(),
                    examples
                ))),
                Err(e) => Ok(DisplayData::Error(format!(
                    "{} {}\n  Run {} to see available tools.",
                    "✗".bright_red(),
                    e,
                    "grok tools list".bright_cyan()
                ))),
            }
        }
    }
}