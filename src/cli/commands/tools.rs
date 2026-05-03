use anyhow::Result;
use colored::Colorize;

pub async fn handle_tools_command(command: crate::ToolsAction) -> Result<()> {
    match command {
        // ── List ─────────────────────────────────────────────────────────────
        crate::ToolsAction::List => {
            println!("{}", "Available Tools".bright_cyan().bold());
            println!(
                "  {} — tools for interacting with the system",
                format!(
                    "{} tool(s)",
                    crate::tools::registry::get_tool_definitions().len()
                )
                .dimmed()
            );
            println!();

            for tool in crate::tools::registry::get_tool_definitions() {
                if let Some(name) = tool
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    && let Some(desc) = tool
                        .get("function")
                        .and_then(|f| f.get("description"))
                        .and_then(|d| d.as_str())
                {
                    println!(
                        "  {} {}  [{}]",
                        "•".bright_white(),
                        name.bright_yellow().bold(),
                        desc.dimmed()
                    );
                }
            }

            println!();
            println!(
                "  Tip: use {} or {} to see more details",
                "grok tools describe <name>".bright_cyan(),
                "grok tools examples <name>".bright_cyan()
            );
        }

        // ── Describe ─────────────────────────────────────────────────────────
        crate::ToolsAction::Describe { name } => {
            match crate::tools::discovery_tools::describe_tool(&name) {
                Ok(schema) => {
                    println!("{}", format!("Tool Schema: {}", name).bright_cyan().bold());
                    println!();
                    println!("{}", schema);
                }
                Err(e) => {
                    eprintln!("{} {}", "✗".bright_red(), e);
                    println!(
                        "  Run {} to see available tools.",
                        "grok tools list".bright_cyan()
                    );
                }
            }
        }

        // ── Examples ─────────────────────────────────────────────────────────
        crate::ToolsAction::Examples { name } => {
            match crate::tools::discovery_tools::tool_examples(&name) {
                Ok(examples) => {
                    println!(
                        "{}",
                        format!("Tool Examples: {}", name).bright_cyan().bold()
                    );
                    println!();
                    println!("{}", examples);
                }
                Err(e) => {
                    eprintln!("{} {}", "✗".bright_red(), e);
                    println!(
                        "  Run {} to see available tools.",
                        "grok tools list".bright_cyan()
                    );
                }
            }
        }
    }

    Ok(())
}
