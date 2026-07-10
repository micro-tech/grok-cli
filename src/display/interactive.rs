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

use crate::acp::security::SecurityPolicy;
use crate::acp::slash_commands;
use crate::acp::tools;
use crate::config::Config;
use crate::content_to_string;
use crate::display::{
    BannerConfig, clear_current_line, format_directory_recommendation, format_grok_logo,
    format_welcome_banner,
};
use crate::router::AppRouter;
use crate::skills::{AutoActivationEngine, list_skills};
use crate::tools::registry as tool_registry;
use crate::tools::tool_context::ToolContext;
use crate::utils::context::{
    format_context_for_prompt, get_context_file_path, load_project_context,
};
use crate::utils::session::{list_sessions, load_session, save_session};
use crate::utils::shell_permissions::{ApprovalMode, ShellPermissions};
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
    /// List of currently active skill names
    #[serde(default)]
    pub active_skills: Vec<String>,
    /// Whether the auto-activation engine should check each user message
    /// and suggest/activate relevant skills automatically.
    /// Toggled at runtime with `/auto-skills on|off`.
    #[serde(default = "default_auto_skills_enabled")]
    pub auto_skills_enabled: bool,
    /// When true, all user messages are routed through the dry-run simulation
    /// engine instead of being executed for real.
    /// Toggled at runtime with `/simulate on|off`.
    #[serde(default)]
    pub simulate_mode: bool,

    /// Per-session shell permissions (ApprovalMode + always-allow list).
    /// Persisted with the session so the user does not have to re-approve
    /// the same commands every time the session is reloaded.
    #[serde(default)]
    pub permissions: ShellPermissions,
}

fn default_auto_skills_enabled() -> bool {
    true
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
            active_skills: Vec::new(),
            auto_skills_enabled: true,
            simulate_mode: false,
            system_prompt,
            conversation_history: Vec::new(),
            current_directory,
            show_context_usage: true,
            total_tokens_used: 0,
            permissions: ShellPermissions::new(ApprovalMode::Default),
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
    let mut app_config = config.clone();

    // Load project context if available
    let project_context = load_project_context_for_session(
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );

    // Note: Skills are now loaded on-demand based on active_skills list
    // rather than loading all skills at startup

    let mut session = InteractiveSession::new(model.to_string(), project_context);
    let client = AppRouter::new(api_key, 30)?;

    // Display startup elements
    if interactive_config.show_banner {
        display_startup_screen(&interactive_config, &session, &app_config).await?;
    }

    // Check if running in home directory
    if interactive_config.check_directory && is_home_directory(&session.current_directory) {
        let banner_config = BannerConfig::default();
        println!(
            "{}",
            format_directory_recommendation(
                &session.current_directory.display().to_string(),
                &banner_config,
            )
        );
    }

    // Main interactive loop
    loop {
        match run_interactive_loop(&mut session, &client, &interactive_config, &mut app_config)
            .await
        {
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
        println!("{}", format_grok_logo(width));
        sleep(Duration::from_millis(500)).await;
    }

    if config.show_tips {
        let banner_config = BannerConfig {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: Some(width),
        };
        println!("{}", format_welcome_banner(&banner_config));
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
    if let Some(context_path) = get_context_file_path(&session.current_directory) {
        println!(
            "  Context loaded: {}",
            context_path.display().to_string().bright_green()
        );
        // When hide_context_summary is false, show a short preview of the file
        if !config.ui.hide_context_summary
            && let Ok(content) = std::fs::read_to_string(&context_path)
        {
            let preview: Vec<&str> = content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(3)
                .collect();
            for line in preview {
                let truncated = if line.len() > 80 {
                    format!("{}...", &line[..80])
                } else {
                    line.to_string()
                };
                println!("      {}", truncated.dimmed());
            }
        }
    }

    // Show available and active skills
    if let Some(skills_dir) = crate::skills::get_default_skills_dir()
        && let Ok(skills) = crate::skills::list_skills(&skills_dir)
    {
        let total = skills.len();
        let active = session.active_skills.len();
        if total > 0 {
            println!(
                "  Skills: {} available, {} active",
                format!("{}", total).bright_blue(),
                format!("{}", active).bright_green()
            );
            if active > 0 {
                let skill_names: Vec<String> = session
                    .active_skills
                    .iter()
                    .map(|s| s.bright_yellow().to_string())
                    .collect();
                println!("    Active: {}", skill_names.join(", "));
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
    match load_project_context(project_root) {
        Ok(Some(context)) => {
            let formatted = format_context_for_prompt(&context);
            // Show the single file that was loaded (project first, system fallback)
            if let Some(path) = get_context_file_path(project_root) {
                println!(
                    "{} {}",
                    "✓".bright_green(),
                    format!("Loaded context from {}", path.display()).dimmed()
                );
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
    client: &AppRouter,
    interactive_config: &InteractiveConfig,
    app_config: &mut Config,
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
            text: "/skills".to_string(),
            description: "List available skills".to_string(),
        },
        Suggestion {
            text: "/activate".to_string(),
            description: "Activate a skill".to_string(),
        },
        Suggestion {
            text: "/deactivate".to_string(),
            description: "Deactivate a skill".to_string(),
        },
        Suggestion {
            text: "/auto-skills".to_string(),
            description: "Toggle skill auto-activation (on/off)".to_string(),
        },
        Suggestion {
            text: "/simulate".to_string(),
            description: "Dry-run simulation mode (on/off or status)".to_string(),
        },
        Suggestion {
            text: "/image".to_string(),
            description: "Attach an image for vision analysis".to_string(),
        },
        Suggestion {
            text: "/init".to_string(),
            description: "Initialize .grok/ project config".to_string(),
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
        return handle_shell_command(input, &mut session.permissions).await;
    }

    // Handle special commands
    if let Some(command_result) =
        handle_special_commands(input, session, interactive_config, app_config).await?
    {
        return Ok(command_result);
    }

    // Auto-activate skills based on the user's message context.
    if session.auto_skills_enabled
        && let Some(skills_dir) = crate::skills::get_default_skills_dir()
        && let Ok(available) = list_skills(&skills_dir)
    {
        let engine = AutoActivationEngine::new();
        let suggestions = engine.check(
            input,
            &session.current_directory,
            &available,
            &session.active_skills,
        );
        for m in suggestions {
            println!(
                "{} Auto-activating skill {} (confidence: {}%)",
                "🔧".bright_cyan(),
                m.skill_name.bright_yellow(),
                m.confidence
            );
            for reason in &m.reasons {
                println!("     {}", reason.dimmed());
            }
            // Activate via the existing helper so security validation runs.
            let _ = activate_skill(session, &m.skill_name);
        }
    }

    // Route to simulation engine or real API depending on mode
    if session.simulate_mode {
        match run_simulation(client, session, input).await {
            Ok(_) => Ok(true),
            Err(e) => {
                eprintln!("{} Simulation failed: {}", "Error:".red(), e);
                Ok(true)
            }
        }
    } else {
        match send_to_grok(client, session, input).await {
            Ok(_) => Ok(true),
            Err(e) => {
                eprintln!("{} Failed to get response: {}", "Error:".red(), e);
                Ok(true)
            }
        }
    }
}

/// Display the input prompt with proper cursor positioning
#[allow(dead_code)]
fn display_prompt(session: &InteractiveSession, config: &InteractiveConfig) -> Result<()> {
    let prompt = match config.prompt_style {
        PromptStyle::Simple => "> ".to_string(),
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

    print!("{}", prompt);
    io::stdout().flush()?;
    Ok(())
}

/// Read user input from stdin with cursor cleanup
#[allow(dead_code)]
fn read_user_input() -> Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // Clean up any ANSI escape sequences that might affect cursor position
    let cleaned = input
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string();
    Ok(cleaned)
}

/// Handle shell commands (those starting with !)
async fn handle_shell_command(input: &str, permissions: &mut ShellPermissions) -> Result<bool> {
    let command = input.trim_start_matches('!').trim();

    if command.is_empty() {
        println!("{}", "Error: No command specified".red());
        return Ok(true);
    }

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
    app_config: &mut Config,
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
                println!("{}", format_grok_logo(width));
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

            // Reload config after modifying settings
            if let Ok(new_config) = Config::load_hierarchical().await {
                *app_config = new_config;
                println!("{} Configuration reloaded successfully", "✓".bright_green());
            }

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
        "skills" => {
            print_available_skills(session);
            Ok(Some(true))
        }
        "activate" => {
            if parts.len() < 2 {
                println!("{} Usage: /activate <skill-name>", "⚠".bright_yellow());
            } else {
                let skill_name = parts[1];
                activate_skill(session, skill_name)?;
            }
            Ok(Some(true))
        }
        "deactivate" => {
            if parts.len() < 2 {
                println!("{} Usage: /deactivate <skill-name>", "⚠".bright_yellow());
            } else {
                let skill_name = parts[1];
                deactivate_skill(session, skill_name)?;
            }
            Ok(Some(true))
        }
        "auto-skills" => {
            match parts.get(1).copied() {
                Some("on") => {
                    session.auto_skills_enabled = true;
                    println!(
                        "{} Skill auto-activation {}",
                        "✓".bright_green(),
                        "enabled".bright_green()
                    );
                    println!(
                        "  {}",
                        "Skills will be activated automatically based on your messages.".dimmed()
                    );
                }
                Some("off") => {
                    session.auto_skills_enabled = false;
                    println!(
                        "{} Skill auto-activation {}",
                        "✓".bright_green(),
                        "disabled".bright_yellow()
                    );
                    println!(
                        "  {}",
                        "Use /activate <skill-name> to enable skills manually.".dimmed()
                    );
                }
                _ => {
                    let state = if session.auto_skills_enabled {
                        "on".bright_green()
                    } else {
                        "off".bright_yellow()
                    };
                    println!(
                        "{} Skill auto-activation is currently: {}",
                        "ℹ".bright_blue(),
                        state
                    );
                    println!(
                        "  Usage: {} or {}",
                        "/auto-skills on".bright_cyan(),
                        "/auto-skills off".bright_cyan()
                    );
                    println!(
                        "  {}",
                        "Skills with 'auto-activate' triggers in their SKILL.md will be".dimmed()
                    );
                    println!(
                        "  {}",
                        "suggested automatically based on keywords, patterns, and project files."
                            .dimmed()
                    );
                }
            }
            Ok(Some(true))
        }
        "hooks" => {
            print_hooks_info(app_config);
            Ok(Some(true))
        }
        "init" => {
            // Mirror ACP `/init` and `grok init [--force]`.
            // Usage: /init | /init --force | /init force
            let force = parts.iter().any(|p| *p == "--force" || *p == "force");
            match crate::tools::run_init(force) {
                Ok(msg) => println!("{}", msg),
                Err(e) => println!("{} Failed to initialize: {}", "❌".bright_red(), e),
            }
            Ok(Some(true))
        }
        "image" => {
            if parts.len() < 2 {
                println!("{} Usage: /image <path> [prompt]", "⚠".bright_yellow());
            } else {
                let path = parts[1];
                let prompt = if parts.len() > 2 {
                    parts[2..].join(" ")
                } else {
                    String::new()
                };
                match crate::tools::image::prepare_image_content(path) {
                    Ok(_) => {
                        crate::tools::image::print_image_attached_feedback(path);
                        let msg = if prompt.is_empty() {
                            format!("[Attached image: {}] Please analyze this image.", path)
                        } else {
                            format!("[Attached image: {}] {}", path, prompt)
                        };
                        // Store the image reference in the next user message
                        session.add_conversation_item("user", &msg, None);
                        println!("📎 Image attached. The next message you send will include the image.");
                    }
                    Err(e) => println!("❌ {}", e),
                }
            }
            Ok(Some(true))
        }
        "simulate" => {
            match parts.get(1).copied() {
                Some("on") => {
                    session.simulate_mode = true;
                    println!(
                        "{} Simulation mode {}",
                        "✓".bright_green(),
                        "enabled".bright_green()
                    );
                    println!(
                        "  {}",
                        "All messages will be dry-run simulated — nothing will be executed."
                            .dimmed()
                    );
                    println!(
                        "  {}",
                        "Use /simulate off to return to normal mode.".dimmed()
                    );
                }
                Some("off") => {
                    session.simulate_mode = false;
                    println!(
                        "{} Simulation mode {}",
                        "✓".bright_green(),
                        "disabled".bright_yellow()
                    );
                    println!("  {}", "Returning to normal execution mode.".dimmed());
                }
                Some(message) => {
                    // /simulate <message> — one-shot simulation without entering persistent mode
                    // Re-join remaining parts in case the message had spaces
                    let full_message = parts[1..].join(" ");
                    // We need the client here — signal the caller via a special return that
                    // this command needs async execution. Instead we print a hint and let the
                    // user know how to use it.
                    // NOTE: one-shot simulate is handled in the main loop via a leading prefix.
                    // For now guide the user to use persistent mode or prefix their message.
                    println!(
                        "{} To simulate a one-shot message, enable simulation mode first:",
                        "ℹ".bright_blue()
                    );
                    println!("  {}", "/simulate on".bright_cyan());
                    println!("  Then type your message: {}", full_message.bright_white());
                    println!("  Then turn it off: {}", "/simulate off".bright_cyan());
                    let _ = message; // suppress unused warning
                }
                None => {
                    let state = if session.simulate_mode {
                        "on".bright_green()
                    } else {
                        "off".bright_yellow()
                    };
                    println!(
                        "{} Simulation mode is currently: {}",
                        "ℹ".bright_blue(),
                        state
                    );
                    println!(
                        "  Usage: {} or {}",
                        "/simulate on".bright_cyan(),
                        "/simulate off".bright_cyan()
                    );
                    println!(
                        "  {}",
                        "When on, messages are dry-run — the model describes what it WOULD do"
                            .dimmed()
                    );
                    println!(
                        "  {}",
                        "without executing any tools or making real changes.".dimmed()
                    );
                }
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
        ("/skills", "List available skills and their status"),
        ("/activate <skill>", "Activate a skill for this session"),
        ("/deactivate <skill>", "Deactivate an active skill"),
        (
            "/auto-skills [on|off]",
            "Toggle or show skill auto-activation",
        ),
        ("/hooks", "Show hooks system status and information"),
        (
            "/init [--force]",
            "Initialize .grok/ project config from global settings",
        ),
        (
            "/simulate [on|off]",
            "Dry-run mode: predict tool calls without executing",
        ),
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
    // Use the live registry so this always reflects the real tool count,
    // identical to what /tools shows in ACP mode.
    println!("{}", slash_commands::format_tools_text());
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
    let sim_state = if session.simulate_mode {
        "on (dry-run — messages are simulated, not executed)"
            .bright_yellow()
            .to_string()
    } else {
        "off".dimmed().to_string()
    };
    println!("  Simulate mode: {}", sim_state);
    println!();
}

/// Send message to Grok and handle response
async fn send_to_grok(
    client: &AppRouter,
    session: &mut InteractiveSession,
    input: &str,
) -> Result<()> {
    // ── Vision / Image handling ─────────────────────────────────────────────
    let mut effective_model = session.model.clone();
    let mut messages = vec![];

    if let Some(image_path) = crate::tools::extract_image_from_message(input) {
        // Show nice TUI feedback
        crate::tools::print_image_attached_feedback(&image_path);

        // Switch to a vision model if current model doesn't support it
        if !crate::tools::is_vision_model(&effective_model) {
            effective_model = crate::tools::recommended_vision_model().to_string();
            println!(
                "{} Switching to vision model: {}",
                "🖼️".bright_cyan(),
                effective_model.bright_yellow()
            );
        }

        // Build a vision-capable message (text + image)
        if let Ok(vision_msg) = crate::tools::create_vision_message(input, &image_path) {
            messages.push(vision_msg);
        } else {
            // Fallback to normal text if image preparation fails
            messages.push(json!({ "role": "user", "content": input }));
        }
    } else {
        messages.push(json!({ "role": "user", "content": input }));
    }

    // Add user message to history (original text)
    session.add_conversation_item("user", input, None);

    // Show thinking indicator
    print!("{} ", "Thinking...".bright_yellow());
    io::stdout().flush()?;

    // Prepare messages for API (we already have the first user message)

    // Build system prompt with active skills context
    let mut system_content = String::new();

    if let Some(system) = &session.system_prompt {
        system_content.push_str(system);
    }

    // Add active skills context if any skills are active
    if let Some(skills_context) = get_active_skills_context(session) {
        if !system_content.is_empty() {
            system_content.push_str("\n\n");
        }
        system_content.push_str(&skills_context);
    }

    if !system_content.is_empty() {
        messages.push(json!({
            "role": "system",
            "content": system_content
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
    let tools = tools::get_available_tool_definitions();

    // Set up security policy with current directory as trusted
    let mut security = SecurityPolicy::new();
    security.add_trusted_directory(&session.current_directory);

    // Send request using the existing client method with tools
    match client
        .chat_completion_with_history(
            &messages,
            session.temperature,
            session.max_tokens,
            &effective_model,
            Some(tools.iter().map(|t| serde_json::json!(t)).collect()),
            None, // reasoning_effort: not exposed in interactive mode yet
        )
        .await
    {
        Ok(response_with_finish) => {
            let response_msg = response_with_finish.message;
            clear_current_line();

            // Handle tool calls if present
            if let Some(tool_calls) = &response_msg.tool_calls
                && !tool_calls.is_empty()
            {
                println!("{}", "Grok is executing operations...".blue().bold());
                println!();

                for tool_call in tool_calls {
                    if let Err(e) = execute_tool_call_interactive(tool_call, &security).await {
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

/// Run the dry-run simulation engine for a user message.
///
/// Sends the message to the model with a special system prompt that instructs
/// it to describe what it *would* do without actually executing anything.
/// The response is parsed and displayed as a structured simulation report.
async fn run_simulation(
    client: &AppRouter,
    session: &InteractiveSession,
    input: &str,
) -> Result<()> {
    use crate::agent::simulator::{
        SIMULATION_SYSTEM_PROMPT, display_simulation_result, parse_simulation_response,
    };
    use colored::*;

    println!(
        "🔬 {}",
        "Running simulation (dry-run)…".bright_blue().dimmed()
    );
    print!("{} ", "Thinking...".bright_yellow());
    io::stdout().flush()?;

    // Build message list: simulation system prompt first, then the user message.
    // Tools are intentionally NOT passed so the model cannot execute anything.
    let mut messages: Vec<serde_json::Value> = Vec::new();

    // Combine existing system prompt with the simulation instructions
    let mut sim_system = String::new();
    if let Some(existing) = &session.system_prompt {
        sim_system.push_str(existing);
        sim_system.push_str("\n\n");
    }
    sim_system.push_str(SIMULATION_SYSTEM_PROMPT);

    messages.push(serde_json::json!({
        "role": "system",
        "content": sim_system
    }));

    // Include recent conversation history for context (last 6 turns)
    let recent: Vec<_> = session
        .conversation_history
        .iter()
        .rev()
        .take(6)
        .rev()
        .collect();
    for item in recent {
        messages.push(serde_json::json!({
            "role": item.role,
            "content": item.content
        }));
    }

    // The user's message being simulated
    messages.push(serde_json::json!({
        "role": "user",
        "content": input
    }));

    match client
        .chat_completion_with_history(
            &messages,
            session.temperature,
            session.max_tokens,
            &session.model,
            None, // no tools — simulation must not execute
            None, // no reasoning effort for simulation
        )
        .await
    {
        Ok(response_with_finish) => {
            clear_current_line();
            let raw = content_to_string(response_with_finish.message.content.as_ref());
            let result = parse_simulation_response(&raw);
            display_simulation_result(&result, input);
        }
        Err(e) => {
            clear_current_line();
            return Err(e);
        }
    }

    Ok(())
}

/// Execute a tool call in interactive mode
async fn execute_tool_call_interactive(
    tool_call: &crate::ToolCall,
    security: &SecurityPolicy,
) -> Result<()> {
    let name = &tool_call.function.name;
    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
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
            eprintln!("  {} Tool '{}' failed: {}", "✗".red(), name, e);
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

/// Print available skills and their activation status
fn print_available_skills(session: &InteractiveSession) {
    use crate::skills::SkillRegistry;

    if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
        match SkillRegistry::load(&skills_dir) {
            Ok(registry) => {
                if registry.is_empty() {
                    println!("{} No skills available", "ℹ".bright_blue());
                    println!(
                        "  Create a skill with: {}",
                        "grok skills new <name>".bright_cyan()
                    );
                } else {
                    println!("{}", "Available Skills:".bright_cyan().bold());
                    println!(
                        "  {}",
                        format!(
                            "{} skill(s) found — sorted by arbitration score",
                            registry.len()
                        )
                        .dimmed()
                    );
                    println!();

                    for entry in registry.entries() {
                        let is_active = session.active_skills.contains(&entry.name().to_string());
                        let is_enabled = entry.is_enabled();

                        let status = if !is_enabled {
                            "✗ DISABLED".bright_red()
                        } else if is_active {
                            "✓ ACTIVE".bright_green()
                        } else {
                            "○ inactive".dimmed()
                        };

                        let score_badge = format!("[score:{}]", entry.arbitration_score());
                        let version_str = format!("v{}", entry.version());

                        println!(
                            "  [{}] {}  {}  {}",
                            status,
                            entry.name().bright_yellow(),
                            score_badge.dimmed(),
                            version_str.dimmed()
                        );
                        println!("       {}", entry.description().dimmed());

                        // Show tags if present
                        let tags = entry.tags();
                        if !tags.is_empty() {
                            println!("       {} {}", "tags:".dimmed(), tags.join(", ").dimmed());
                        }

                        // Show author if present
                        if let Some(author) = entry.author() {
                            println!("       {} {}", "author:".dimmed(), author.dimmed());
                        }

                        // Show dependencies if present
                        let deps = entry.dependencies();
                        if !deps.is_empty() {
                            println!("       {} {}", "deps:".dimmed(), deps.join(", ").dimmed());
                        }

                        println!();
                    }

                    println!(
                        "  Use {} to enable a skill  |  {} to disable",
                        "/activate <skill-name>".bright_cyan(),
                        "/deactivate <skill-name>".bright_cyan()
                    );
                    println!(
                        "  Use {} / {} to globally enable/disable a skill",
                        "grok skills enable <name>".bright_cyan(),
                        "grok skills disable <name>".bright_cyan()
                    );
                }
            }
            Err(e) => {
                println!("{} Failed to load skill registry: {}", "✗".bright_red(), e);
            }
        }
    } else {
        println!("{} Skills directory not found", "⚠".bright_yellow());
    }
}

/// Activate a skill for the current session
fn activate_skill(session: &mut InteractiveSession, skill_name: &str) -> Result<()> {
    // Check if already active
    if session.active_skills.contains(&skill_name.to_string()) {
        println!(
            "{} Skill '{}' is already active",
            "ℹ".bright_blue(),
            skill_name.bright_yellow()
        );
        return Ok(());
    }

    // Verify skill exists and check global enabled flag via registry
    if let Some(skills_dir) = crate::skills::get_default_skills_dir() {
        // Registry check: block globally-disabled skills before anything else
        if let Ok(registry) = crate::skills::SkillRegistry::load(&skills_dir) {
            match registry.find(skill_name) {
                None => {
                    println!("{} Skill '{}' not found", "✗".bright_red(), skill_name);
                    println!("  Use {} to see available skills", "/skills".bright_cyan());
                    return Ok(());
                }
                Some(entry) if !entry.is_enabled() => {
                    println!(
                        "{} Skill '{}' is globally disabled and cannot be activated",
                        "✗".bright_red(),
                        skill_name.bright_yellow()
                    );
                    println!(
                        "  Re-enable it first with: {}",
                        format!("grok skills enable {}", skill_name).bright_cyan()
                    );
                    return Ok(());
                }
                Some(_) => {} // enabled — fall through to security check
            }
        }

        if let Some(skill) = crate::skills::find_skill(skill_name, &skills_dir) {
            // Validate skill security before activating
            let validator = crate::skills::SkillSecurityValidator::new();
            match validator.validate_skill(&skill.path) {
                Ok(crate::skills::ValidationLevel::Safe) => {
                    // Safe - activate immediately
                    session.active_skills.push(skill_name.to_string());
                    println!(
                        "{} Skill '{}' activated",
                        "✓".bright_green(),
                        skill_name.bright_yellow()
                    );
                    println!("  The skill's instructions will be included in the next message");
                }
                Ok(crate::skills::ValidationLevel::Warning(warnings)) => {
                    // Warnings - activate but show warnings
                    session.active_skills.push(skill_name.to_string());
                    println!(
                        "{} Skill '{}' activated with warnings",
                        "⚠".bright_yellow(),
                        skill_name.bright_yellow()
                    );
                    for warning in warnings {
                        println!("  • {}", warning.dimmed());
                    }
                }
                Ok(crate::skills::ValidationLevel::Suspicious(issues)) => {
                    // Suspicious - require confirmation
                    println!(
                        "{} Skill '{}' has suspicious patterns:",
                        "⚠".bright_yellow(),
                        skill_name.bright_yellow()
                    );
                    for issue in &issues {
                        println!("  • {}", issue.yellow());
                    }
                    println!();
                    println!(
                        "{}",
                        "This skill may be unsafe. Review carefully before use.".yellow()
                    );
                    println!(
                        "Use {} to see full security report",
                        format!("grok skills validate {}", skill_name).bright_cyan()
                    );

                    // For now, block suspicious skills in interactive mode for safety
                    println!("{}", "Skill activation blocked for your safety.".red());
                }
                Ok(crate::skills::ValidationLevel::Dangerous(issues)) => {
                    // Dangerous - block activation
                    println!(
                        "{} Skill '{}' is DANGEROUS and has been blocked:",
                        "🛑".bright_red(),
                        skill_name.bright_red()
                    );
                    for issue in &issues {
                        println!("  • {}", issue.red());
                    }
                    println!();
                    println!(
                        "{}",
                        "DO NOT USE THIS SKILL. It contains malicious patterns."
                            .bright_red()
                            .bold()
                    );
                }
                Err(e) => {
                    println!("{} Failed to validate skill: {}", "✗".bright_red(), e);
                }
            }
        } else {
            println!("{} Skill '{}' not found", "✗".bright_red(), skill_name);
            println!("  Use {} to see available skills", "/skills".bright_cyan());
        }
    } else {
        println!("{} Skills directory not found", "⚠".bright_yellow());
    }

    Ok(())
}

/// Deactivate a skill for the current session
fn deactivate_skill(session: &mut InteractiveSession, skill_name: &str) -> Result<()> {
    if let Some(pos) = session.active_skills.iter().position(|s| s == skill_name) {
        session.active_skills.remove(pos);
        println!(
            "{} Skill '{}' deactivated",
            "✓".bright_green(),
            skill_name.bright_yellow()
        );
    } else {
        println!("{} Skill '{}' is not active", "ℹ".bright_blue(), skill_name);
    }

    Ok(())
}

/// Get the ranked context string for currently active skills.
///
/// Uses the [`SkillRegistry`] so skills are injected in descending
/// arbitration-score order, giving higher-priority skills more influence.
fn get_active_skills_context(session: &InteractiveSession) -> Option<String> {
    use crate::skills::SkillRegistry;

    if session.active_skills.is_empty() {
        return None;
    }

    let skills_dir = crate::skills::get_default_skills_dir()?;

    match SkillRegistry::load(&skills_dir) {
        Ok(registry) => registry.ranked_context(&session.active_skills),
        Err(_) => {
            // Fallback: plain unranked context using the legacy loader
            let mut context = String::from(
                "\n\n## Active Skills\n\nThe following skills are currently active:\n\n",
            );
            for skill_name in &session.active_skills {
                if let Some(skill) = crate::skills::find_skill(skill_name, &skills_dir) {
                    context.push_str(&format!("### Skill: {}\n", skill.config.name));
                    context.push_str(&format!("Description: {}\n", skill.config.description));
                    context.push_str("\nInstructions:\n");
                    context.push_str(&skill.instructions);
                    context.push_str("\n\n---\n\n");
                }
            }
            Some(context)
        }
    }
}

fn print_hooks_info(config: &Config) {
    println!("{}", "Hooks System Information".bright_cyan().bold());
    println!();

    // Check if hooks are enabled
    let hooks_enabled = config.tools.enable_hooks;
    let status_symbol = if hooks_enabled {
        "✓".bright_green()
    } else {
        "✗".bright_red()
    };
    let status_text = if hooks_enabled {
        "Enabled".bright_green()
    } else {
        "Disabled".bright_red()
    };

    println!("  {} Hooks Status: {}", status_symbol, status_text);
    println!();

    // Extensions information
    let extensions_enabled = config.experimental.extensions.enabled;
    let ext_status_symbol = if extensions_enabled {
        "✓".bright_green()
    } else {
        "✗".bright_red()
    };
    let ext_status_text = if extensions_enabled {
        "Enabled".bright_green()
    } else {
        "Disabled".bright_red()
    };

    println!(
        "  {} Extensions System: {}",
        ext_status_symbol, ext_status_text
    );

    if extensions_enabled {
        if let Some(ext_dir) = &config.experimental.extensions.extension_dir {
            println!(
                "  {} Extension Directory: {}",
                "ℹ".bright_blue(),
                ext_dir.display()
            );
        }

        if !config.experimental.extensions.enabled_extensions.is_empty() {
            println!();
            println!("  {} Enabled Extensions:", "📦".bright_cyan());
            for ext in &config.experimental.extensions.enabled_extensions {
                println!("    • {}", ext.bright_white());
            }
        }
    }

    println!();
    println!("{}", "About Hooks:".bright_yellow().bold());
    println!("  Hooks allow you to execute custom logic before and after tool calls.");
    println!("  They can be used for logging, validation, security checks, and more.");
    println!();

    if !hooks_enabled {
        println!("{}", "To enable hooks:".bright_yellow());
        println!("  1. Edit your config file (use /settings)");
        println!("  2. Set 'tools.enable_hooks = true'");
        println!("  3. Optionally enable extensions system for custom hooks");
        println!();
    }

    if hooks_enabled && !extensions_enabled {
        println!("{}", "Tip:".bright_blue().bold());
        println!("  Enable the extensions system to load custom hooks from extensions.");
        println!("  Set 'experimental.extensions.enabled = true' in your config.");
        println!();
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
