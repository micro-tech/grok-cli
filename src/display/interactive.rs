//! Interactive mode for Grok CLI
//!
//! Provides a Gemini CLI-like interactive experience with persistent sessions,
//! input prompts, and real-time status display

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::Result;
use colored::*;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::time::{Duration, sleep};

use crate::GrokClient;
use crate::acp::security::SecurityPolicy;
use crate::acp::tools;
use crate::config::Config;
use crate::display::{
    BannerConfig, clear_current_line, print_directory_recommendation, print_grok_logo,
    print_welcome_banner,
};
use crate::utils::context::{
    format_context_for_prompt, get_all_context_file_paths, load_and_merge_project_context,
};
use crate::utils::session::{list_sessions, load_session, save_session};
use crate::utils::shell_permissions::{ApprovalMode, ShellPermissions};
use crate::{content_to_string, extract_text_content, text_content};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Interactive session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveSession {
    pub session_id: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
    pub conversation_history: Vec<ConversationItem>,
    pub current_directory: PathBuf,
    pub show_context_usage: bool,
    pub total_tokens_used: u32,
}

/// Conversation item in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationItem {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tokens_used: Option<u32>,
}

/// Interactive mode configuration
#[derive(Debug, Clone)]
pub struct InteractiveConfig {
    pub show_banner: bool,
    pub show_tips: bool,
    pub show_status: bool,
    pub auto_save_session: bool,
    pub prompt_style: PromptStyle,
    pub check_directory: bool,
}

/// Different prompt styles
#[derive(Debug, Clone, PartialEq)]
pub enum PromptStyle {
    Simple,
    Rich,
    Minimal,
}

impl Default for InteractiveConfig {
    fn default() -> Self {
        Self {
            show_banner: true,
            show_tips: true,
            show_status: true,
            auto_save_session: false,
            prompt_style: PromptStyle::Rich,
            check_directory: true,
        }
    }
}

impl InteractiveSession {
    /// Create a new interactive session
    pub fn new(model: String, system_prompt: Option<String>) -> Self {
        let session_id = generate_session_id();
        let current_directory = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            session_id,
            model,
            temperature: 0.7,
            max_tokens: 4096,
            system_prompt,
            conversation_history: Vec::new(),
            current_directory,
            show_context_usage: true,
            total_tokens_used: 0,
        }
    }

    /// Add a conversation item to the history
    pub fn add_conversation_item(&mut self, role: &str, content: &str, tokens_used: Option<u32>) {
        let item = ConversationItem {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            tokens_used,
        };

        if let Some(tokens) = tokens_used {
            self.total_tokens_used += tokens;
        }

        self.conversation_history.push(item);
    }

    /// Get context usage information
    pub fn get_context_info(&self) -> String {
        let conversation_count = self.conversation_history.len();
        let context_percentage = if self.total_tokens_used > 0 {
            let estimated_max = 8192; // Rough estimate for context window
            ((self.total_tokens_used as f32 / estimated_max as f32) * 100.0) as u8
        } else {
            0
        };

        format!(
            "{}% context left | {} messages",
            100 - context_percentage,
            conversation_count
        )
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("grok-{}", timestamp)
}

/// Start the interactive mode
pub async fn start_interactive_mode(
    api_key: &str,
    model: &str,
    config: &Config,
    interactive_config: InteractiveConfig,
) -> Result<()> {
    // Load project context if available
    let mut project_context = load_project_context_for_session(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );

    // Load skills context
    if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
        if let Ok(skills_context) = crate::skills::get_skills_context(&skills_dir) {
            if !skills_context.is_empty() {
                let ctx = project_context.get_or_insert_with(String::new);
                ctx.push_str(&skills_context);
            }
        }
    }

    let mut session = InteractiveSession::new(model.to_string(), project_context);
    let client = GrokClient::new(api_key)?;

    // Display startup elements
    if interactive_config.show_banner {
        display_startup_screen(&interactive_config, &session, config).await?;
    }

    // Check if running in home directory
    if interactive_config.check_directory && is_home_directory(&session.current_directory) {
        let banner_config = BannerConfig::default();
        print_directory_recommendation(
            &session.current_directory.display().to_string(),
            &banner_config,
        );
    }

    // Main interactive loop
    loop {
        match run_interactive_loop(&mut session, &client, &interactive_config, config).await {
            Ok(should_continue) => {
                if !should_continue {
                    break;
                }
            }
            Err(e) => {
                eprintln!("{} {}", "Error:".red(), e);
                continue;
            }
        }
    }

    // Goodbye message
    println!("{}", "\n👋 Thanks for using Grok CLI!".bright_cyan());

    if interactive_config.auto_save_session && !session.conversation_history.is_empty() {
        println!("{}", "Session saved for future reference.".dimmed());
    }

    Ok(())
}

/// Display the startup screen
async fn display_startup_screen(
    config: &InteractiveConfig,
    session: &InteractiveSession,
    app_config: &Config,
) -> Result<()> {
    let (width, _) = crate::display::get_terminal_size();

    // Clear screen and show logo with animation
    crate::display::clear_screen();

    if config.show_banner && !config.show_tips {
        print_grok_logo(width);
        sleep(Duration::from_millis(500)).await;
    }

    if config.show_tips {
        let banner_config = BannerConfig {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: Some(width),
        };
        print_welcome_banner(&banner_config);
    }

    // Show current session info
    if config.show_status {
        print_session_info(session, app_config);
    }

    Ok(())
}

/// Print current session information
fn print_session_info(session: &InteractiveSession, config: &Config) {
    println!("{}", "Current session:".bright_white());
    println!("  Model: {}", session.model.bright_cyan());
    println!(
        "  Directory: {}",
        session
            .current_directory
            .display()
            .to_string()
            .bright_yellow()
    );

    // Show config source
    if let Some(source) = &config.config_source {
        println!("  Configuration: {}", source.display().bright_magenta());
    }

    // Show context files info if loaded
    let context_paths = get_all_context_file_paths(&session.current_directory);
    if !context_paths.is_empty() {
        if context_paths.len() == 1 {
            println!(
                "  Context loaded: {}",
                context_paths[0]
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .bright_green()
            );
        } else {
            println!(
                "  Context loaded: {} files",
                context_paths.len().to_string().bright_green()
            );
            for path in &context_paths {
                println!(
                    "    - {}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .dimmed()
                );
            }
        }
    }

    // Show loaded skills
    if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
        if let Ok(skills) = crate::skills::list_skills(&skills_dir) {
            if !skills.is_empty() {
                println!(
                    "  Skills: {}",
                    format!("{} loaded", skills.len()).bright_green()
                );
                for skill in skills {
                    println!("    - {}", skill.config.name.dimmed());
                }
            }
        }
    }

    if let Some(system) = &session.system_prompt {
        let preview = if system.len() > 60 {
            format!("{}...", &system[..60])
        } else {
            system.clone()
        };
        println!("  System prompt: {}", preview.bright_green());
    }
    println!();
}

/// Load project context for a new session
fn load_project_context_for_session(project_root: &PathBuf) -> Option<String> {
    match load_and_merge_project_context(project_root) {
        Ok(Some(context)) => {
            let formatted = format_context_for_prompt(&context);
            let context_paths = get_all_context_file_paths(project_root);

            if context_paths.is_empty() {
                // Shouldn't happen but handle gracefully
                return Some(formatted);
            }

            if context_paths.len() == 1 {
                let context_file_name = context_paths[0]
                    .file_name()
                    .and_then(|n| n.to_os_string().into_string().ok())
                    .unwrap_or_else(|| "context file".to_string());

                println!(
                    "{} {}",
                    "✓".bright_green(),
                    format!("Loaded project context from {}", context_file_name).dimmed()
                );
            } else {
                println!(
                    "{} {}",
                    "✓".bright_green(),
                    format!("Loaded and merged {} context files", context_paths.len()).dimmed()
                );
                for path in &context_paths {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        println!("  {} {}", "•".dimmed(), name.dimmed());
                    }
                }
            }

            Some(formatted)
        }
        Ok(None) => {
            // No context file found - this is normal
            None
        }
        Err(e) => {
            eprintln!(
                "{} Failed to load project context: {}",
                "⚠".yellow(),
                e.to_string().dimmed()
            );
            None
        }
    }
}

use crate::display::components::input::{Suggestion, read_input_with_suggestions};

/// Main interactive loop
async fn run_interactive_loop(
    session: &mut InteractiveSession,
    client: &GrokClient,
    interactive_config: &InteractiveConfig,
    app_config: &Config,
) -> Result<bool> {
    // Prepare prompt
    let prompt = match interactive_config.prompt_style {
        PromptStyle::Simple => format!("{} ", ">".bright_cyan()),
        PromptStyle::Rich => {
            let context_info = if session.show_context_usage {
                format!(" | {}", session.get_context_info())
            } else {
                String::new()
            };

            format!(
                "{} {} ",
                format!("Grok ({})", session.model).bright_cyan(),
                format!(
                    "[{}{}]",
                    session
                        .current_directory
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?"),
                    context_info
                )
                .dimmed()
            )
        }
        PromptStyle::Minimal => "» ".to_string(),
    };

    // Prepare suggestions
    let suggestions = vec![
        Suggestion {
            text: "/clear".to_string(),
            description: "Clear screen".to_string(),
        },
        Suggestion {
            text: "/help".to_string(),
            description: "Show help message".to_string(),
        },
        Suggestion {
            text: "/history".to_string(),
            description: "Show history".to_string(),
        },
        Suggestion {
            text: "/list".to_string(),
            description: "List saved sessions".to_string(),
        },
        Suggestion {
            text: "/load".to_string(),
            description: "Load a session".to_string(),
        },
        Suggestion {
            text: "/model".to_string(),
            description: "Change model".to_string(),
        },
        Suggestion {
            text: "/quit".to_string(),
            description: "Exit interactive mode".to_string(),
        },
        Suggestion {
            text: "/reset".to_string(),
            description: "Reset session".to_string(),
        },
        Suggestion {
            text: "/save".to_string(),
            description: "Save current session".to_string(),
        },
        Suggestion {
            text: "/settings".to_string(),
            description: "Open settings".to_string(),
        },
        Suggestion {
            text: "/status".to_string(),
            description: "Show status".to_string(),
        },
        Suggestion {
            text: "/system".to_string(),
            description: "Set system prompt".to_string(),
        },
        Suggestion {
            text: "/tools".to_string(),
            description: "List coding tools".to_string(),
        },
        Suggestion {
            text: "/version".to_string(),
            description: "Show version info".to_string(),
        },
        Suggestion {
            text: "/config".to_string(),
            description: "Show configuration info".to_string(),
        },
        Suggestion {
            text: "!ls".to_string(),
            description: "List files (shell command)".to_string(),
        },
        Suggestion {
            text: "!dir".to_string(),
            description: "List files on Windows (shell command)".to_string(),
        },
        Suggestion {
            text: "!git status".to_string(),
            description: "Check git status (shell command)".to_string(),
        },
        Suggestion {
            text: "!pwd".to_string(),
            description: "Print working directory (shell command)".to_string(),
        },
    ];

    // Read user input
    // Note: We're running blocking TUI code in an async context, which is generally bad,
    // but for a CLI it's acceptable as we're awaiting user input anyway.
    let input =
        tokio::task::spawn_blocking(move || read_input_with_suggestions(&prompt, &suggestions))
            .await??;

    let input = input.trim();

    // Handle empty input
    if input.is_empty() {
        return Ok(true);
    }

    // Handle shell commands (starting with !)
    if input.starts_with('!') {
        return handle_shell_command(input).await;
    }

    // Handle special commands
    if let Some(command_result) =
        handle_special_commands(input, session, interactive_config, app_config).await?
    {
        return Ok(command_result);
    }

    // Send to Grok API
    match send_to_grok(client, session, input).await {
        Ok(_) => Ok(true),
        Err(e) => {
            eprintln!("{} Failed to get response: {}", "Error:".red(), e);
            Ok(true)
        }
    }
}

/// Display the input prompt
fn display_prompt(session: &InteractiveSession, config: &InteractiveConfig) -> Result<()> {
    match config.prompt_style {
        PromptStyle::Simple => {
            print!("{} ", ">".bright_cyan());
        }
        PromptStyle::Rich => {
            let context_info = if session.show_context_usage {
                format!(" | {}", session.get_context_info())
            } else {
                String::new()
            };

            print!(
                "{} {} ",
                format!("Grok ({})", session.model).bright_cyan(),
                format!(
                    "[{}{}]",
                    session
                        .current_directory
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?"),
                    context_info
                )
                .dimmed()
            );
        }
        PromptStyle::Minimal => {
            print!("» ");
        }
    }

    io::stdout().flush()?;
    Ok(())
}

/// Read user input from stdin
fn read_user_input() -> Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input)
}

/// Handle shell commands (those starting with !)
async fn handle_shell_command(input: &str) -> Result<bool> {
    let command = input.trim_start_matches('!').trim();

    if command.is_empty() {
        println!("{}", "Error: No command specified".red());
        return Ok(true);
    }

    // Create permissions manager (TODO: pass from session state)
    let mut permissions = ShellPermissions::new(ApprovalMode::Default);

    // Check if command should be executed
    match permissions.should_execute(command) {
        Ok(true) => {
            // Permission granted, execute command
            println!();
            println!("{} {}", "Executing:".bright_cyan(), command.bright_yellow());
            println!();

            // Determine shell based on OS
            #[cfg(target_os = "windows")]
            let shell = "cmd";
            #[cfg(target_os = "windows")]
            let shell_arg = "/C";

            #[cfg(not(target_os = "windows"))]
            let shell = "sh";
            #[cfg(not(target_os = "windows"))]
            let shell_arg = "-c";

            // Execute the command
            match std::process::Command::new(shell)
                .arg(shell_arg)
                .arg(command)
                .output()
            {
                Ok(output) => {
                    // Print stdout
                    if !output.stdout.is_empty() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        print!("{}", stdout);
                    }

                    // Print stderr in red
                    if !output.stderr.is_empty() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprint!("{}", stderr.red());
                    }

                    // Show exit code if non-zero
                    if !output.status.success() {
                        println!();
                        println!(
                            "{} Command exited with code: {}",
                            "⚠".yellow(),
                            output.status.code().unwrap_or(-1)
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to execute command: {}", "Error:".red(), e);
                }
            }

            println!();
        }
        Ok(false) => {
            // Permission denied
            println!();
            println!("{}", "Command execution cancelled".yellow());
            println!();
        }
        Err(e) => {
            eprintln!("{} Permission check failed: {}", "Error:".red(), e);
        }
    }

    Ok(true)
}

/// Handle special commands (those starting with /)
async fn handle_special_commands(
    input: &str,
    session: &mut InteractiveSession,
    interactive_config: &InteractiveConfig,
    app_config: &Config,
) -> Result<Option<bool>> {
    if !input.starts_with('/') {
        return Ok(None);
    }

    let command = input.trim_start_matches('/').trim();
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(Some(true));
    }

    match parts[0] {
        "help" | "h" => {
            print_interactive_help();
            Ok(Some(true))
        }
        "quit" | "exit" | "q" => Ok(Some(false)),
        "clear" | "cls" => {
            crate::display::clear_screen();
            if interactive_config.show_banner {
                let (width, _) = crate::display::get_terminal_size();
                print_grok_logo(width);
            }
            Ok(Some(true))
        }
        "model" | "models" => {
            if parts.len() > 1 {
                session.model = parts[1].to_string();
                println!(
                    "{} Model changed to: {}",
                    "✓".bright_green(),
                    session.model.bright_cyan()
                );
            } else {
                println!(
                    "{} Current model: {}",
                    "ℹ".bright_blue(),
                    session.model.bright_cyan()
                );
            }
            Ok(Some(true))
        }
        "system" => {
            if parts.len() > 1 {
                let system_prompt = parts[1..].join(" ");
                session.system_prompt = Some(system_prompt.clone());
                println!(
                    "{} System prompt set: {}",
                    "✓".bright_green(),
                    system_prompt.bright_yellow()
                );
            } else {
                match &session.system_prompt {
                    Some(prompt) => println!(
                        "{} Current system prompt: {}",
                        "ℹ".bright_blue(),
                        prompt.bright_yellow()
                    ),
                    None => println!("{} No system prompt set", "ℹ".bright_blue()),
                }
            }
            Ok(Some(true))
        }
        "settings" => {
            crate::cli::commands::settings::handle_settings_action(
                crate::SettingsAction::Show,
                app_config,
            )
            .await?;
            Ok(Some(true))
        }
        "tools" => {
            print_available_tools();
            Ok(Some(true))
        }
        "history" => {
            print_conversation_history(session);
            Ok(Some(true))
        }
        "status" => {
            print_session_status(session);
            Ok(Some(true))
        }
        "version" => {
            println!(
                "{} Grok CLI v{}",
                "ℹ".bright_blue(),
                env!("CARGO_PKG_VERSION")
            );
            Ok(Some(true))
        }
        "reset" => {
            session.conversation_history.clear();
            session.total_tokens_used = 0;
            println!("{} Conversation history cleared", "✓".bright_green());
            Ok(Some(true))
        }
        "save" => {
            if parts.len() < 2 {
                println!("{} Usage: /save <name>", "⚠".bright_yellow());
            } else {
                let name = parts[1];
                match save_session(session, name) {
                    Ok(path) => {
                        println!("{} Session saved to {}", "✓".bright_green(), path.display())
                    }
                    Err(e) => println!("{} Failed to save session: {}", "✗".bright_red(), e),
                }
            }
            Ok(Some(true))
        }
        "load" => {
            if parts.len() < 2 {
                println!("{} Usage: /load <name>", "⚠".bright_yellow());
            } else {
                let name = parts[1];
                match load_session(name) {
                    Ok(loaded_session) => {
                        *session = loaded_session;
                        println!("{} Session '{}' loaded", "✓".bright_green(), name);
                        // Note: Can't show config here as we don't have access to it in this scope
                        println!("  Model: {}", session.model.bright_cyan());
                        println!(
                            "  Directory: {}",
                            session
                                .current_directory
                                .display()
                                .to_string()
                                .bright_yellow()
                        );
                    }
                    Err(e) => println!("{} Failed to load session: {}", "✗".bright_red(), e),
                }
            }
            Ok(Some(true))
        }
        "list" | "sessions" => {
            match list_sessions() {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        println!("{} No saved sessions found", "ℹ".bright_blue());
                    } else {
                        println!("{}", "Saved Sessions:".bright_cyan().bold());
                        for s in sessions {
                            println!("  • {}", s);
                        }
                    }
                }
                Err(e) => println!("{} Failed to list sessions: {}", "✗".bright_red(), e),
            }
            Ok(Some(true))
        }
        _ => {
            println!("{} Unknown command: /{}", "⚠".bright_yellow(), parts[0]);
            println!("Type /help for available commands");
            Ok(Some(true))
        }
    }
}

/// Print interactive mode help
fn print_interactive_help() {
    println!("{}", "Interactive mode commands:".bright_cyan().bold());
    println!();

    let commands = vec![
        ("/help, /h", "Show this help message"),
        ("/quit, /exit, /q", "Exit interactive mode"),
        ("/clear, /cls", "Clear screen and show logo"),
        ("/model [name]", "Show or change the current model"),
        ("/system [prompt]", "Show or set system prompt"),
        ("/tools", "List available coding tools"),
        (
            "!<command>",
            "Execute shell command locally (e.g., !dir, !ls -la)",
        ),
        ("/settings", "Open settings menu"),
        ("/history", "Show conversation history"),
        ("/status", "Show session status"),
        ("/version", "Show version info"),
        ("/config", "Show configuration info"),
        ("/reset", "Clear conversation history"),
        ("/save [name]", "Save current session"),
        ("/load [name]", "Load a saved session"),
        ("/list", "List saved sessions"),
    ];

    for (command, description) in commands {
        println!("  {:<20} {}", command.bright_white(), description);
    }
    println!();
    println!("{}", "Just type your message to chat with Grok!".dimmed());
    println!();
}

/// Print available coding tools
fn print_available_tools() {
    use colored::*;

    println!("{}", "Available Coding Tools:".bright_cyan().bold());
    println!();
    println!("{}", "These tools are available when using the ACP server or when Grok needs to perform file operations:".dimmed());
    println!();

    let tools = vec![
        (
            "read_file",
            "Read the content of a file",
            "read_file(path: string)",
        ),
        (
            "write_file",
            "Write content to a file",
            "write_file(path: string, content: string)",
        ),
        (
            "replace",
            "Replace text in a file",
            "replace(path: string, old_string: string, new_string: string)",
        ),
        (
            "list_directory",
            "List files and directories",
            "list_directory(path: string)",
        ),
        (
            "glob_search",
            "Find files matching a pattern",
            "glob_search(pattern: string)",
        ),
        (
            "search_file_content",
            "Search for text in files",
            "search_file_content(path: string, pattern: string)",
        ),
        (
            "run_shell_command",
            "Execute a shell command",
            "run_shell_command(command: string)",
        ),
        ("web_search", "Search the web", "web_search(query: string)"),
        (
            "web_fetch",
            "Fetch content from a URL",
            "web_fetch(url: string)",
        ),
        (
            "save_memory",
            "Save a fact to memory",
            "save_memory(fact: string)",
        ),
    ];

    println!("{}", "File Operations:".bright_yellow().bold());
    for (name, desc, sig) in &tools[0..3] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "File Search & Discovery:".bright_yellow().bold());
    for (name, desc, sig) in &tools[3..6] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Execution & Web:".bright_yellow().bold());
    for (name, desc, sig) in &tools[6..9] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Memory:".bright_yellow().bold());
    for (name, desc, sig) in &tools[9..10] {
        println!("  {} {}", name.bright_white().bold(), "-".dimmed());
        println!("    {}", desc.dimmed());
        println!("    {}", sig.bright_blue());
        println!();
    }

    println!("{}", "Note:".bright_cyan());
    println!(
        "  {}",
        "• Tools are automatically used by Grok when needed".dimmed()
    );
    println!(
        "  {}",
        "• For ACP server mode, use: grok acp stdio".dimmed()
    );
    println!(
        "  {}",
        "• All file operations respect security permissions".dimmed()
    );
    println!();
}

/// Print conversation history
fn print_conversation_history(session: &InteractiveSession) {
    if session.conversation_history.is_empty() {
        println!("{} No conversation history yet", "ℹ".bright_blue());
        return;
    }

    println!("{}", "Conversation History:".bright_cyan().bold());
    println!();

    for (i, item) in session.conversation_history.iter().enumerate() {
        let role_color = if item.role == "user" {
            Color::BrightGreen
        } else {
            Color::BrightBlue
        };

        let role_symbol = if item.role == "user" { "👤" } else { "🤖" };

        println!(
            "{} {} {}",
            format!("{}.", i + 1).dimmed(),
            role_symbol,
            item.role.color(role_color).bold()
        );

        // Show first 100 chars of content
        let content_preview = if item.content.len() > 100 {
            format!("{}...", &item.content[..97])
        } else {
            item.content.clone()
        };

        println!("   {}", content_preview);

        if let Some(tokens) = item.tokens_used {
            println!("   {} tokens used", tokens.to_string().dimmed());
        }
        println!();
    }
}

/// Print session status
fn print_session_status(session: &InteractiveSession) {
    println!("{}", "Session Status:".bright_cyan().bold());
    println!("  Session ID: {}", session.session_id.bright_white());
    println!("  Model: {}", session.model.bright_cyan());
    println!(
        "  Temperature: {}",
        session.temperature.to_string().bright_yellow()
    );
    println!(
        "  Max tokens: {}",
        session.max_tokens.to_string().bright_yellow()
    );
    println!(
        "  Messages: {}",
        session
            .conversation_history
            .len()
            .to_string()
            .bright_green()
    );
    println!(
        "  Total tokens used: {}",
        session.total_tokens_used.to_string().bright_red()
    );
    println!(
        "  Directory: {}",
        session
            .current_directory
            .display()
            .to_string()
            .bright_magenta()
    );

    if let Some(system) = &session.system_prompt {
        println!("  System prompt: {}", system.bright_green());
    }
    println!();
}

/// Send message to Grok and handle response
async fn send_to_grok(
    client: &GrokClient,
    session: &mut InteractiveSession,
    input: &str,
) -> Result<()> {
    // Add user message to history
    session.add_conversation_item("user", input, None);

    // Show thinking indicator
    print!("{} ", "Thinking...".bright_yellow());
    io::stdout().flush()?;

    // Prepare messages for API
    let mut messages = vec![];

    if let Some(system) = &session.system_prompt {
        messages.push(json!({
            "role": "system",
            "content": system
        }));
    }

    // Add conversation history (keep last 10 messages to avoid context overflow)
    let recent_history = session
        .conversation_history
        .iter()
        .rev()
        .take(10)
        .rev()
        .collect::<Vec<_>>();

    for item in recent_history {
        messages.push(json!({
            "role": item.role,
            "content": item.content
        }));
    }

    // Get tool definitions for function calling
    let tools = tools::get_tool_definitions();

    // Set up security policy with current directory as trusted
    let mut security = SecurityPolicy::new();
    security.add_trusted_directory(&session.current_directory);

    // Send request using the existing client method with tools
    match client
        .chat_completion_with_history(
            &messages,
            session.temperature,
            session.max_tokens,
            &session.model,
            Some(tools),
        )
        .await
    {
        Ok(response_msg) => {
            clear_current_line();

            // Handle tool calls if present
            if let Some(tool_calls) = &response_msg.tool_calls {
                if !tool_calls.is_empty() {
                    println!("{}", "Grok is executing operations...".blue().bold());
                    println!();

                    for tool_call in tool_calls {
                        if let Err(e) = execute_tool_call_interactive(tool_call, &security) {
                            eprintln!("  {} Tool execution failed: {}", "✗".red(), e);
                        }
                    }

                    println!();
                    println!("{}", "All operations completed!".green().bold());
                    println!();

                    // Add assistant's response to history
                    let content = content_to_string(response_msg.content.as_ref());
                    let content = if content.is_empty() {
                        "Operations completed.".to_string()
                    } else {
                        content
                    };
                    session.add_conversation_item("assistant", &content, None);
                    return Ok(());
                }
            }

            let content = content_to_string(response_msg.content.as_ref());

            // Print Grok's response with nice formatting
            println!("{} {}", "🤖".bright_blue(), "Grok:".bright_blue().bold());
            println!();
            println!("{}", content);
            println!();

            // Add to conversation history
            session.add_conversation_item("assistant", &content, None);
        }
        Err(e) => {
            clear_current_line();
            return Err(e);
        }
    }

    Ok(())
}

/// Execute a tool call in interactive mode
fn execute_tool_call_interactive(
    tool_call: &crate::ToolCall,
    security: &SecurityPolicy,
) -> Result<()> {
    use anyhow::anyhow;

    let name = &tool_call.function.name;
    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;

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
        _ => {
            println!("  {} Unsupported tool: {}", "⚠".yellow(), name);
        }
    }

    Ok(())
}

/// Check if current directory is the home directory
fn is_home_directory(current_dir: &PathBuf) -> bool {
    if let Some(home) = dirs::home_dir() {
        current_dir == &home
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = InteractiveSession::new("grok-3".to_string(), None);
        assert_eq!(session.model, "grok-3");
        assert!(session.conversation_history.is_empty());
        assert_eq!(session.total_tokens_used, 0);
    }

    #[test]
    fn test_add_conversation_item() {
        let mut session = InteractiveSession::new("grok-3".to_string(), None);
        session.add_conversation_item("user", "Hello", Some(10));

        assert_eq!(session.conversation_history.len(), 1);
        assert_eq!(session.total_tokens_used, 10);
        assert_eq!(session.conversation_history[0].content, "Hello");
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert!(id1.starts_with("grok-"));
        assert!(id2.starts_with("grok-"));
        assert_ne!(id1, id2); // Should be different due to timestamp
    }
}
