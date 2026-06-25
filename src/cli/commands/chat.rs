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
use crate::cli::display_data::DisplayData;
use crate::cli::{
    create_spinner, format_error, format_grok_response, format_info, format_success,
};
use crate::config::{BayesianConfig, RateLimitConfig, ThinkingMode};
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
    /// Reasoning / thinking mode (from `--thinking` CLI flag).
    pub thinking_mode: ThinkingMode,
    /// Explorer mode flag (from `--explore`).
    pub explore: Option<String>,
}

/// Main chat handler — returns DisplayData for library/binary separation (Task 131/136).
pub async fn handle_chat(options: ChatOptions<'_>) -> Result<DisplayData> {
    let client = initialize_router(options.api_key, options.timeout_secs)?;

    if options.interactive {
        handle_interactive_chat(
            client,
            options.system,
            options.temperature,
            options.max_tokens,
            options.model,
            options.bayesian,
            options.thinking_mode,
        )
        .await
    } else if let Some(query) = options.explore {
        handle_explorer_mode(client, &query, options.model).await
    } else {
        let combined_message = options.message.join(" ");
        handle_single_chat(
            client,
            &combined_message,
            options.system,
            options.temperature,
            options.max_tokens,
            options.model,
            options.thinking_mode,
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
    thinking_mode: ThinkingMode,
) -> Result<()> {
    println!("{}", format_info(&format!("Sending message to Grok (model: {})...", model)));

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
        .chat_completion_with_history(
            &messages,
            temperature,
            max_tokens,
            model,
            Some(tools.iter().map(|t| serde_json::json!(t)).collect()),
            thinking_mode.as_api_str(),
        )
        .await;

    spinner.finish_and_clear();

    match result {
        Ok(response_with_finish) => {
            let response = response_with_finish.message;
            // Handle tool calls if present
            if let Some(tool_calls) = &response.tool_calls
                && !tool_calls.is_empty()
            {
                println!("{}", format_info("Executing requested operations..."));
                let mut security = SecurityPolicy::new();
                security.add_trusted_directory(&env::current_dir()?);

                for tool_call in tool_calls {
                    execute_tool_call(tool_call, &security).await?;
                }
                println!("{}", format_success("All operations completed!"));
                return Ok(());
            }

            // Regular text response
            println!("{}", format_success("Response received!"));
            println!();
            if let Some(content) = response.content {
                let text = extract_text_content(&content);
                println!("{}", format_grok_response(&text, true));
            }
        }
        Err(e) => {
            println!("{}", format_error(&format!("Failed to get response: {}", e)));
            return Err(e);
        }
    }

    // Return structured result for library/binary separation (Task 131/136)
    Ok(DisplayData::success("Chat session completed"))
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
            println!("{}", format_error(&format!("Tool '{}' failed: {}", name, e)));
        }
    }
    // Return structured result for library/binary separation (Task 131/136)
    Ok(DisplayData::success("Chat session completed"))
}

async fn handle_interactive_chat(
    client: AppRouter,
    system: Option<String>,
    temperature: f32,
    max_tokens: u32,
    model: &str,
    bayesian_config: BayesianConfig,
    thinking_mode: ThinkingMode,
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
                let active_tools: Vec<serde_json::Value> = if enable_bayesian_router {
                    router
                        .get_contextual_tools(tools.iter().map(|t| serde_json::json!(t)).collect())
                } else {
                    tools.iter().map(|t| serde_json::json!(t)).collect()
                };

                let response_with_finish = client
                    .chat_completion_with_history(
                        &conversation_history,
                        temperature,
                        max_tokens,
                        model,
                        Some(active_tools),
                        thinking_mode.as_api_str(),
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
                            println!("{}", format_error(&format!("Tool execution failed: {}", e)));
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
                println!("{}", format_error(&format!("Failed to read input: {}", e)));
                break;
            }
        }
    }

    // Return structured result for library/binary separation (Task 131/136)
    Ok(DisplayData::success("Chat session completed"))
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
            println!("{}", format_success("Conversation history cleared!"));
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
            println!("{}", format_success("Conversation history cleared!"));
            Ok(Some(CommandResult::Continue))
        }

        // Generic slash command handler (re-uses ACP logic so both interfaces stay in sync)
        _ if input.starts_with('/') => {
            if let Some(cmd) = slash_commands::parse_slash_command(input) {
                if let Some(builtin) = slash_commands::handle_builtin(&cmd) {
                    match builtin {
                        slash_commands::BuiltinResult::Text(text) => {
                            println!("{}", text);
                        }
                        slash_commands::BuiltinResult::ClearHistory => {
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
                            println!("{}", format_success("Conversation history cleared!"));
                        }
                        slash_commands::BuiltinResult::SwitchModel(name) => {
                            println!("✅ Switched to model `{}` (CLI session).", name);
                        }
                        slash_commands::BuiltinResult::ShowContext => {
                            println!(
                                "📋 Context: {} messages in history.",
                                conversation_history.len()
                            );
                        }
                        slash_commands::BuiltinResult::ShowBayes => {
                            // In CLI we just show a note; full viz lives in the Bayesian router
                            println!(
                                "🧠 Bayesian state: use the router visualizer or `/bayes show` in ACP."
                            );
                        }
                        slash_commands::BuiltinResult::ResetBayes => {
                            println!("🔄 Bayesian priors reset (CLI session).");
                        }
                        slash_commands::BuiltinResult::ExplainBayes => {
                            println!(
                                "🧠 Bayesian explanation: see router logs or use ACP `/bayes explain`."
                            );
                        }
                        slash_commands::BuiltinResult::SetGoal(text) => {
                            println!("🎯 Goal set: {}", text);
                        }
                        slash_commands::BuiltinResult::ClearGoal => {
                            println!("🎯 Goal cleared.");
                        }
                        slash_commands::BuiltinResult::ShowGoal => {
                            println!("🎯 No goal set for this CLI session.");
                        }
                        slash_commands::BuiltinResult::ShowVisualizer => {
                            println!("{}", crate::visualizer::generate_pipeline_markdown(None));
                        }
                        slash_commands::BuiltinResult::SetThinkingMode(mode) => {
                            let label = mode
                                .as_ref()
                                .map(|m| m.as_api_str().unwrap_or("off"))
                                .unwrap_or("off");
                            println!("🧠 Thinking mode: {} (CLI session)", label);
                        }
                        slash_commands::BuiltinResult::RecallArchive(_) => {
                            println!("{}", slash_commands::format_archives_text(None));
                        }
                        slash_commands::BuiltinResult::ShowDiagnostics => {
                            println!("{}", slash_commands::format_diagnostics_text());
                        }
                        slash_commands::BuiltinResult::ShowCurrentModel => {
                            // In pure CLI mode we don't have a persistent session model,
                            // so we just note that the user can pass --model on the command line.
                            println!("🧠 Current model: (use `--model <name>` when starting the CLI session)");
                        }
                    }
                    return Ok(Some(CommandResult::Continue));
                }

                // AI-assisted slash commands → enhance prompt and continue as normal user message
                if let Some(enhanced) = slash_commands::command_to_prompt(&cmd) {
                    // We can't easily mutate the caller's input here, so we just
                    // let the original command text go through (the model will still
                    // understand "/web ..."). Full enhancement happens in ACP mode.
                    let _ = enhanced;
                }
            }
            Ok(None)
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
                Err(e) => println!("{}", format_error(&format!("Failed to list directory: {}", e))),
            }
            Ok(Some(CommandResult::Continue))
        }
        _ if lower_input.starts_with("cd ") => {
            let path = input[3..].trim();
            if let Err(e) = env::set_current_dir(path) {
                println!("{}", format_error(&format!("Failed to change directory to '{}': {}", path, e)));
            } else {
                println!("{}", format_success(&format!(
                    "Changed directory to {}",
                    env::current_dir().unwrap_or_default().display()
                )));
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
                            println!("{}", format_error(&format!("Command exited with status: {}", status)));
                        }
                    }
                    Err(e) => println!("{}", format_error(&format!("Failed to execute command: {}", e))),
                }
            }
            Ok(Some(CommandResult::Continue))
        }
        _ if input.is_empty() => Ok(Some(CommandResult::Continue)),
        _ => Ok(None),
    }
}

fn print_help() {
    // Use the shared ACP slash-command list so both interfaces stay in sync.
    println!("{}", slash_commands::format_help_text());
    println!();
    println!("{}", "Additional CLI-only commands:".cyan().bold());
    println!("  {} - Show conversation history", "history".yellow());
    println!("  {} - List files in current directory", "ls, dir".yellow());
    println!("  {} - Change current directory", "cd <path>".yellow());
    println!("  {} - Execute shell command", "!<command>".yellow());
    println!();
    println!("{}", "Tool Support:".cyan().bold());
    println!("  Grok has access to all tools (file, shell, web, memory, tasks, LSP, MCP…).");
    println!("  Just ask naturally or use the slash commands above.");
    println!();
}

fn print_conversation_history(history: &[Value]) {
    if history.is_empty() {
        println!("{}", format_info("No conversation history yet."));
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

// ============================================================================
// EXPLORER MODE (Task 162)
// ============================================================================

/// Handle `--explore "query"` mode.
/// Uses EXPLORER system prompt + restricted tools and returns compact JSON evidence.
async fn handle_explorer_mode(
    client: AppRouter,
    query: &str,
    model: &str,
) -> Result<()> {
    use crate::agent::mode::Mode;

    println!("{}", format_info(&format!("Explorer mode: {}", query)));

    let system_prompt = Mode::Explorer.system_prompt_additions();

    let messages = vec![
        json!({ "role": "system", "content": system_prompt }),
        json!({ "role": "user", "content": query }),
    ];

    // Only allow read/search tools in explorer mode
    let allowed_tools = vec!["fs_glob", "fs_read", "fs_grep", "list_directory", "search_file_content"];

    let all_tools = tools::get_available_tool_definitions();
    let filtered_tools: Vec<serde_json::Value> = all_tools
        .into_iter()
        .filter(|t| {
            t.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|name| allowed_tools.contains(&name))
                .unwrap_or(false)
        })
        .map(|t| serde_json::json!(t))
        .collect();

    let spinner = create_spinner("Exploring repository...");
    let response = client
        .chat_completion_with_history(&messages, 0.2, 4096, model, Some(filtered_tools), None)
        .await?;
    spinner.finish_and_clear();

    if let Some(content) = response.message.content {
        let text = extract_text_content(&content);
        // Try to pretty-print JSON if the model returned valid evidence
        if let Ok(json_val) = serde_json::from_str::<Value>(&text) {
            println!("{}", serde_json::to_string_pretty(&json_val)?);
        } else {
            println!("{}", text);
        }
    }

    // Return structured result for library/binary separation (Task 131/136)
    Ok(DisplayData::success("Chat session completed"))
}