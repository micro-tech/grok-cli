//! Chat command handler for grok-cli
//!
//! Handles interactive and non-interactive chat sessions with Grok AI

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::{Result, anyhow};
use colored::*;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::acp::security::SecurityPolicy;
use crate::acp::tools;
use crate::cli::{create_spinner, format_grok_response, print_error, print_info, print_success};
use crate::config::RateLimitConfig;
use crate::{GrokClient, ToolCall, content_to_string, extract_text_content};

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
}

pub async fn handle_chat(options: ChatOptions<'_>) -> Result<()> {
    let client =
        GrokClient::with_settings(options.api_key, options.timeout_secs, options.max_retries)?
            .with_rate_limits(options.rate_limit_config);

    if options.interactive {
        handle_interactive_chat(
            client,
            options.system,
            options.temperature,
            options.max_tokens,
            options.model,
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
    client: GrokClient,
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
            if let Some(tool_calls) = &response.tool_calls {
                if !tool_calls.is_empty() {
                    print_info("Executing requested operations...");
                    let mut security = SecurityPolicy::new();
                    security.add_trusted_directory(&env::current_dir()?);

                    for tool_call in tool_calls {
                        execute_tool_call(tool_call, &security).await?;
                    }
                    print_success("All operations completed!");
                    return Ok(());
                }
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

async fn execute_tool_call(tool_call: &ToolCall, security: &SecurityPolicy) -> Result<()> {
    let name = &tool_call.function.name;
    let args: Value = serde_json::from_str(&tool_call.function.arguments)?;

    match name.as_str() {
        "write_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing content"))?;
            let result = tools::write_file(path, content, security)?;
            println!("  {} {}", "✓".green(), result);
        }
        "read_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let content = tools::read_file(path, security)?;
            println!(
                "  {} Read {} bytes from {}",
                "✓".green(),
                content.len(),
                path
            );
        }
        "replace" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let old = args["old_string"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing old_string"))?;
            let new = args["new_string"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing new_string"))?;
            let expected = args
                .get("expected_replacements")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let result = tools::replace(path, old, new, expected, security)?;
            println!("  {} {}", "✓".green(), result);
        }
        "list_directory" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing path"))?;
            let result = tools::list_directory(path, security)?;
            println!("  {} Directory contents of {}:", "✓".green(), path);
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        "glob_search" => {
            let pattern = args["pattern"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing pattern"))?;
            let result = tools::glob_search(pattern, security)?;
            println!("  {} Files matching '{}':", "✓".green(), pattern);
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        "save_memory" => {
            let fact = args["fact"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing fact"))?;
            let result = tools::save_memory(fact)?;
            println!("  {} {}", "✓".green(), result);
        }
        "run_shell_command" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing command"))?;
            println!("  {} Executing: {}", "⚙".cyan(), command);
            let result = tools::run_shell_command(command, security)?;
            println!("  {} Command output:", "✓".green());
            for line in result.lines() {
                println!("    {}", line);
            }
        }
        "web_search" => {
            let query = args["query"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing query"))?;
            println!("  {} Searching for: {}", "🔍".cyan(), query);
            match tools::web_search(query).await {
                Ok(results) => {
                    println!("  {} Search results:", "✓".green());
                    println!("{}", results);
                }
                Err(e) => {
                    println!("  {} Search failed: {}", "✗".red(), e);
                }
            }
        }
        "web_fetch" => {
            let url = args["url"].as_str().ok_or_else(|| anyhow!("Missing url"))?;
            println!("  {} Fetching: {}", "🔍".cyan(), url);
            match tools::web_fetch(url).await {
                Ok(content) => {
                    println!("  {} Fetched {} bytes", "✓".green(), content.len());
                }
                Err(e) => {
                    println!("  {} Fetch failed: {}", "✗".red(), e);
                }
            }
        }
        _ => {
            println!("  {} Unsupported tool: {}", "⚠".yellow(), name);
        }
    }

    Ok(())
}

async fn handle_interactive_chat(
    client: GrokClient,
    system: Option<String>,
    temperature: f32,
    max_tokens: u32,
    model: &str,
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

                // Handle special commands
                if lower_input == "exit" || lower_input == "quit" || lower_input == "q" {
                    println!("{}", "Goodbye!".cyan());
                    break;
                } else if lower_input == "help" || lower_input == "h" {
                    print_help();
                    continue;
                } else if lower_input == "clear" || lower_input == "cls" {
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
                    continue;
                } else if lower_input == "history" {
                    print_conversation_history(&conversation_history);
                    continue;
                } else if lower_input == "ls" || lower_input == "dir" {
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
                    continue;
                } else if lower_input.starts_with("cd ") {
                    let path = input[3..].trim();
                    if let Err(e) = env::set_current_dir(path) {
                        print_error(&format!("Failed to change directory to '{}': {}", path, e));
                    } else {
                        print_success(&format!(
                            "Changed directory to {}",
                            env::current_dir().unwrap_or_default().display()
                        ));
                    }
                    continue;
                } else if let Some(stripped) = input.strip_prefix('!') {
                    let command_line = stripped.trim();
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
                    continue;
                } else if input.is_empty() {
                    continue;
                }

                // Add user message to history
                conversation_history.push(json!({
                    "role": "user",
                    "content": input
                }));

                // Show spinner while waiting for response
                let spinner = create_spinner("Grok is thinking...");

                // Get response with timeout and retries (including tool definitions)
                let response_with_finish = client
                    .chat_completion_with_history(
                        &conversation_history,
                        temperature,
                        max_tokens,
                        model,
                        Some(tools.clone()),
                    )
                    .await?;

                let response_msg = response_with_finish.message;

                spinner.finish_and_clear();

                // Handle tool calls if present
                if let Some(tool_calls) = &response_msg.tool_calls {
                    if !tool_calls.is_empty() {
                        println!("{}", "Grok is executing operations...".blue().dimmed());

                        for tool_call in tool_calls {
                            if let Err(e) = execute_tool_call(tool_call, &security).await {
                                print_error(&format!("Tool execution failed: {}", e));
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

fn print_help() {
    println!();
    println!("{}", "Available commands:".cyan().bold());
    println!("  {} - Exit the chat session", "exit, quit, q".yellow());
    println!("  {} - Show this help message", "help, h".yellow());
    println!("  {} - Clear conversation history", "clear, cls".yellow());
    println!("  {} - Show conversation history", "history".yellow());
    println!("  {} - List files in current directory", "ls, dir".yellow());
    println!("  {} - Change current directory", "cd <path>".yellow());
    println!("  {} - Execute shell command", "!<command>".yellow());
    println!();
    println!("{}", "Tool Support:".cyan().bold());
    println!("  Grok can now automatically create files and directories!");
    println!("  Just ask naturally, e.g.:");
    println!("    {} 'Create a new Rust project structure'", "•".green());
    println!(
        "    {} 'Write a hello world program to main.rs'",
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
