//! Chat command handler for grok-cli
//!
//! Handles interactive and non-interactive chat sessions with Grok AI

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::Result;
use colored::*;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::Command;

use crate::acp::security::SecurityPolicy;
use crate::acp::slash_commands;
use crate::acp::tools;
use crate::agent::router::{Router, RouterAction};
use crate::cli::{create_spinner, format_grok_response, print_error, print_info, print_success};
use crate::config::{BayesianConfig, RateLimitConfig};
use crate::router::AppRouter;
use crate::tools::registry as tool_registry;
use crate::tools::tool_context::ToolContext;
use crate::utils::client::initialize_router;
use crate::{ToolCall, content_to_string, extract_text_content};

pub struct ChatOptions<'a> {
    pub message: Vec<String>,
    pub interactive: bool,
    pub system: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub api_key: &'a str,
    pub model: &'a str,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub rate_limit_config: RateLimitConfig,
    pub bayesian: BayesianConfig,
}

pub async fn handle_chat(options: ChatOptions<'_>) -> Result<()> {
    let client = initialize_router(options.api_key, options.timeout_secs)?;

    if options.interactive {
        handle_interactive_chat(
            client,
            options.system,
            options.temperature,
            options.max_tokens,
            options.model,
            options.bayesian,
        )
        .await
    } else {
        let combined_message = options.message.join(" ");
        handle_single_chat(
            client,
            &combined_message,
            options.system,
            options.temperature,
            options.max_tokens,
            options.model,
        )
        .await
    }
}

async fn handle_single_chat(
    client: AppRouter,
    message: &str,
    system: Option<String>,
    temperature: f32,
    max_tokens: u32,
    model: &str,
) -> Result<()> {
    print_info(&format!("Sending message to Grok (model: {})...", model));

    let spinner = create_spinner("Thinking...");

    // Prepare messages
    let mut messages = Vec::new();
    if let Some(sys) = system {
        messages.push(json!({
            "role": "system",
            "content": sys
        }));
    }
    messages.push(json!({
        "role": "user",
        "content": message
    }));

    // Add tool definitions
    let tools = tools::get_available_tool_definitions();

    let result = client
        .chat_completion_with_history(&messages, temperature, max_tokens, model, Some(tools))
        .await;

    spinner.finish_and_clear();

    match result {
        Ok(response_with_finish) => {
            let response = response_with_finish.message;
            // Handle tool calls if present
            if let Some(tool_calls) = &response.tool_calls
                && !tool_calls.is_empty()
            {
                print_info("Executing requested operations...");
                let mut security = SecurityPolicy::new();
                security.add_trusted_directory(&env::current_dir()?);

                for tool_call in tool_calls {
                    execute_tool_call(tool_call, &security).await?;
                }
                print_success("All operations completed!");
                return Ok(());
            }

            // Regular text response
            print_success("Response received!");
            println!();
            if let Some(content) = response.content {
                let text = extract_text_content(&content);
                println!("{}", format_grok_response(&text, true));
            }
        }
        Err(e) => {
            print_error(&format!("Failed to get response: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Execute a tool call from the AI using the full tool registry (all 31 tools).
/// Previously only ~9 tools were handled here; now every tool defined in
/// `tools::registry::get_tool_definitions` is dispatched correctly.
async fn execute_tool_call(tool_call: &ToolCall, security: &SecurityPolicy) -> Result<()> {
    let name = &tool_call.function.name;
    let args: Value = serde_json::from_str(&tool_call.function.arguments)?;
    let ctx = ToolContext::new(security.clone());

    println!("  {} Running: {}", "⚙".cyan(), name);
    match tool_registry::execute_tool(name, &args, &ctx).await {
        Ok(output) => {
            println!("  {} {}", "✓".green(), name);
            let lines: Vec<&str> = output.lines().collect();
            for line in lines.iter().take(20) {
                println!("    {}", line);
            }
            if lines.len() > 20 {
                println!("    {} ({} more lines)", "...".dimmed(), lines.len() - 20);
            }
        }
        Err(e) => {
            print_error(&format!("Tool '{}' failed: {}", name, e));
        }
    }
    Ok(())
}

async fn handle_interactive_chat(
    client: AppRouter,
    system: Option<String>,
    temperature: f32,
    max_tokens: u32,
    model: &str,
    bayesian_config: BayesianConfig,
) -> Result<()> {
    println!("{}", "🤖 Interactive Grok Chat Session".cyan().bold());
    println!("{}", format!("Model: {}", model).dimmed());

    if let Some(ref sys) = system {
        println!("{}", format!("System: {}", sys).dimmed());
    }

    println!(
        "{}",
        "Type 'exit', 'quit', or press Ctrl+C to end the session".dimmed()
    );
    println!("{}", "Type 'help' for available commands".dimmed());
    println!();

    let mut conversation_history = Vec::new();

    // Add system message if provided
    if let Some(sys) = system {
        conversation_history.push(json!({
            "role": "system",
            "content": sys
        }));
    }

    // Set up security policy with current directory as trusted
    let mut security = SecurityPolicy::new();
    security.add_trusted_directory(&env::current_dir()?);

    // Get tool definitions for function calling
    let tools = tools::get_available_tool_definitions();

    let enable_bayesian_router = bayesian_config.enabled;
    let mut show_belief_graph = bayesian_config.show_belief_graph;
    let mut router = Router::new_with_config(&bayesian_config);

    loop {
        // Prompt for input
        let cwd = env::current_dir().unwrap_or_default();
        print!(
            "{} {} ",
            format!("[{}]", cwd.display()).dimmed(),
            "You:".green().bold()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // EOF reached (Ctrl+D)
                println!("\n{}", "Goodbye!".cyan());
                break;
            }
            Ok(_) => {
                let input = input.trim();
                let lower_input = input.to_lowercase();

                // Handle special commands using a command registry
                if let Some(result) =
                    handle_interactive_command(&lower_input, input, &mut conversation_history)?
                {
                    match result {
                        CommandResult::Exit => {
                            println!("{}", "Goodbye!".cyan());
                            break;
                        }
                        CommandResult::Continue => continue,
                    }
                }

                if lower_input == "/bayes" || lower_input == "/beliefs" {
                    show_belief_graph = !show_belief_graph;
                    if show_belief_graph {
                        println!("{} Belief graph visualization enabled.", "✓".green());
                        if enable_bayesian_router {
                            println!("{}", router.visualize_beliefs());
                        } else {
                            println!("{}", "⚠ Bayesian router is disabled in config.".yellow());
                        }
                    } else {
                        println!("{} Belief graph visualization disabled.", "✓".green());
                    }
                    continue;
                }

                let mut actual_input = input.to_string();

                if enable_bayesian_router {
                    let action = router.route(&actual_input).await;
                    match action {
                        RouterAction::AskClarification(msg) => {
                            println!("{} {}", "🤖 Grok Router:".cyan().bold(), msg.yellow());
                            if show_belief_graph {
                                println!("\n{}", router.visualize_beliefs());
                            }
                            continue;
                        }
                        RouterAction::UseSkill(skill) => {
                            actual_input = format!(
                                "{}\n[System: High probability of needing skill '{}'. Please use it if appropriate.]",
                                actual_input, skill
                            );
                        }
                        RouterAction::UseTool(tool) => {
                            actual_input = format!(
                                "{}\n[System: High probability of needing tool '{}'. Please use it if appropriate.]",
                                actual_input, tool
                            );
                        }
                        RouterAction::NormalChat => {}
                    }

                    if show_belief_graph {
                        println!("\n{}", router.visualize_beliefs());
                    }

                    if router.is_low_confidence() {
                        actual_input = format!(
                            "{}\n[System Alert: The intent probability is below threshold (low_confidence). Do NOT call any tools yet. Output a brief 3-step Markdown plan and ask the user if it looks correct before proceeding.]",
                            actual_input
                        );
                    } else if let Some(persona) = router.get_adaptive_system_prompt() {
                        actual_input = format!("{}\n[{}]", actual_input, persona);
                    }
                }

                // Add user message to history
                conversation_history.push(json!({
                    "role": "user",
                    "content": actual_input
                }));

                // Show spinner while waiting for response
                let spinner = create_spinner("Grok is thinking...");

                // Get response with timeout and retries (including tool definitions)
                let active_tools = if enable_bayesian_router {
                    router.get_contextual_tools(tools.clone())
                } else {
                    tools.clone()
                };

                let response_with_finish = client
                    .chat_completion_with_history(
                        &conversation_history,
                        temperature,
                        max_tokens,
                        model,
                        Some(active_tools),
                    )
                    .await?;

                let response_msg = response_with_finish.message;

                spinner.finish_and_clear();

                // Handle tool calls if present
                if let Some(tool_calls) = &response_msg.tool_calls
                    && !tool_calls.is_empty()
                {
                    println!("{}", "Grok is executing operations...".blue().dimmed());

                    for tool_call in tool_calls {
                        if let Err(e) = execute_tool_call(tool_call, &security).await {
                            print_error(&format!("Tool execution failed: {}", e));
                        } else {
                            if enable_bayesian_router {
                                router.learn_from_tool(&tool_call.function.name);
                            }
                        }
                    }

                    // Add assistant's tool call response to history
                    conversation_history.push(json!({
                        "role": "assistant",
                        "content": response_msg.content.clone(),
                        "tool_calls": tool_calls
                    }));

                    continue;
                }

                let response = content_to_string(response_msg.content.as_ref());

                // Add assistant response to history
                conversation_history.push(json!({
                    "role": "assistant",
                    "content": response.clone()
                }));

                // Display response
                println!("{} {}", "Grok:".blue().bold(), response);
            }
            Err(e) => {
                print_error(&format!("Failed to read input: {}", e));
                break;
            }
        }
    }

    Ok(())
}

/// Enum to represent the result of processing a command
enum CommandResult {
    Exit,
    Continue,
}

/// Handle interactive mode commands
fn handle_interactive_command(
    lower_input: &str,
    input: &str,
    conversation_history: &mut Vec<Value>,
) -> Result<Option<CommandResult>> {
    match lower_input {
        "exit" | "quit" | "q" => Ok(Some(CommandResult::Exit)),
        "help" | "h" => {
            print_help();
            Ok(Some(CommandResult::Continue))
        }
        "clear" | "cls" => {
            // Clear conversation history but keep system message
            if conversation_history
                .first()
                .and_then(|msg| msg.get("role"))
                .and_then(|role| role.as_str())
                == Some("system")
            {
                let system_msg = conversation_history[0].clone();
                conversation_history.clear();
                conversation_history.push(system_msg);
            } else {
                conversation_history.clear();
            }
            print_success("Conversation history cleared!");
            Ok(Some(CommandResult::Continue))
        }
        "history" => {
            print_conversation_history(conversation_history);
            Ok(Some(CommandResult::Continue))
        }
        // ── Slash commands (work in both ACP and CLI modes) ──────────────────
        "/tools" | "tools" => {
            println!("{}", slash_commands::format_tools_text());
            Ok(Some(CommandResult::Continue))
        }
        "/help" => {
            print_help();
            Ok(Some(CommandResult::Continue))
        }
        "/clear" => {
            if conversation_history
                .first()
                .and_then(|msg| msg.get("role"))
                .and_then(|role| role.as_str())
                == Some("system")
            {
                let system_msg = conversation_history[0].clone();
                conversation_history.clear();
                conversation_history.push(system_msg);
            } else {
                conversation_history.clear();
            }
            print_success("Conversation history cleared!");
            Ok(Some(CommandResult::Continue))
        }
        "ls" | "dir" => {
            match fs::read_dir(".") {
                Ok(entries) => {
                    println!("{}", "Current Directory Entries:".cyan().bold());
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        if path.is_dir() {
                            println!("  {} {}", name.blue().bold(), "(DIR)".dimmed());
                        } else {
                            println!("  {}", name);
                        }
                    }
                }
                Err(e) => print_error(&format!("Failed to list directory: {}", e)),
            }
            Ok(Some(CommandResult::Continue))
        }
        _ if lower_input.starts_with("cd ") => {
            let path = input[3..].trim();
            if let Err(e) = env::set_current_dir(path) {
                print_error(&format!("Failed to change directory to '{}': {}", path, e));
            } else {
                print_success(&format!(
                    "Changed directory to {}",
                    env::current_dir().unwrap_or_default().display()
                ));
            }
            Ok(Some(CommandResult::Continue))
        }
        _ if input.starts_with('!') => {
            let command_line = input[1..].trim();
            if !command_line.is_empty() {
                let result = if cfg!(target_os = "windows") {
                    Command::new("cmd").arg("/C").arg(command_line).status()
                } else {
                    Command::new("sh").arg("-c").arg(command_line).status()
                };

                match result {
                    Ok(status) => {
                        if !status.success() {
                            print_error(&format!("Command exited with status: {}", status));
                        }
                    }
                    Err(e) => print_error(&format!("Failed to execute command: {}", e)),
                }
            }
            Ok(Some(CommandResult::Continue))
        }
        _ if input.is_empty() => Ok(Some(CommandResult::Continue)),
        _ => Ok(None),
    }
}

fn print_help() {
    println!();
    println!("{}", "Available commands:".cyan().bold());
    println!("  {} - Exit the chat session", "exit, quit, q".yellow());
    println!("  {} - Show this help message", "help, h, /help".yellow());
    println!(
        "  {} - Clear conversation history",
        "clear, cls, /clear".yellow()
    );
    println!("  {} - Show conversation history", "history".yellow());
    println!("  {} - List all available AI tools", "/tools".yellow());
    println!("  {} - List files in current directory", "ls, dir".yellow());
    println!("  {} - Change current directory", "cd <path>".yellow());
    println!("  {} - Execute shell command", "!<command>".yellow());
    println!();
    println!("{}", "Tool Support:".cyan().bold());
    println!("  Grok has access to all 31 tools (file, shell, web, memory, tasks, LSP, MCP…).");
    println!("  Just ask naturally, e.g.:");
    println!("    {} 'Create a new Rust project structure'", "•".green());
    println!(
        "    {} 'Write a hello world program to main.rs'",
        "•".green()
    );
    println!(
        "    {} 'Search the web for Rust async examples'",
        "•".green()
    );
    println!(
        "    {} 'Run cargo check and show me the errors'",
        "•".green()
    );
    println!();
}

fn print_conversation_history(history: &[Value]) {
    if history.is_empty() {
        print_info("No conversation history yet.");
        return;
    }

    println!();
    println!("{}", "Conversation History:".cyan().bold());
    println!("{}", "─".repeat(50));

    for (i, message) in history.iter().enumerate() {
        if let (Some(role), Some(content)) = (
            message.get("role").and_then(|r| r.as_str()),
            message.get("content").and_then(|c| c.as_str()),
        ) {
            let role_display = match role {
                "system" => "System".magenta().bold(),
                "user" => "You".green().bold(),
                "assistant" => "Grok".blue().bold(),
                _ => role.white().bold(),
            };

            println!("{}: {}", role_display, content);

            if i < history.len() - 1 {
                println!();
            }
        }
    }

    println!("{}", "─".repeat(50));
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_chat_command_structure() {
        // Test that the chat command structure is properly defined
        // This is a placeholder test - in a real implementation you'd mock the API
        // The test passes as long as the module compiles correctly
    }
}
