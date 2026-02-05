//! Chat history viewer command
//!
//! This module provides commands to view, search, and manage chat session logs.

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::{Context, Result};
use colored::*;
use std::path::PathBuf;

use crate::cli::{print_error, print_info, print_success};
use crate::utils::chat_logger::{ChatLogger, ChatLoggerConfig, ChatSession};

/// Handle history-related commands
pub async fn handle_history_action(action: crate::HistoryAction) -> Result<()> {
    match action {
        crate::HistoryAction::List => list_sessions().await,
        crate::HistoryAction::View { session_id } => view_session(&session_id).await,
        crate::HistoryAction::Search { query } => search_sessions(&query).await,
        crate::HistoryAction::Clear { confirm } => clear_history(confirm).await,
    }
}

/// List all available chat sessions
async fn list_sessions() -> Result<()> {
    let config = get_logger_config();
    let logger =
        ChatLogger::new(config).context("Failed to initialize chat logger for listing sessions")?;

    let sessions = logger
        .list_sessions()
        .context("Failed to list chat sessions")?;

    if sessions.is_empty() {
        print_info("No chat sessions found.");
        return Ok(());
    }

    println!("\n{}", "=".repeat(80).bright_cyan());
    println!(
        "{}",
        format!("  CHAT SESSIONS ({} total)", sessions.len())
            .bright_cyan()
            .bold()
    );
    println!("{}\n", "=".repeat(80).bright_cyan());

    for (i, session_id) in sessions.iter().enumerate() {
        // Try to load session metadata
        match logger.load_session(session_id) {
            Ok(session) => {
                let start_time = session.start_time.format("%Y-%m-%d %H:%M:%S UTC");
                let msg_count = session.messages.len();
                let status = if session.end_time.is_some() {
                    "Completed".green()
                } else {
                    "Active".yellow()
                };

                println!(
                    "{}. {} {}",
                    format!("{:3}", i + 1).bright_black(),
                    session_id.bright_white().bold(),
                    status
                );
                println!(
                    "   {} {} | {} messages",
                    "Started:".bright_black(),
                    start_time.to_string().bright_white(),
                    msg_count.to_string().bright_cyan()
                );

                if let Some(end_time) = session.end_time {
                    let duration = end_time
                        .signed_duration_since(session.start_time)
                        .num_seconds();
                    println!(
                        "   {} {}",
                        "Duration:".bright_black(),
                        format!("{} seconds", duration).bright_white()
                    );
                }

                // Show first user message preview if available
                if let Some(first_msg) = session.messages.iter().find(|m| m.role == "user") {
                    let preview = first_msg
                        .content
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(60)
                        .collect::<String>();
                    if !preview.is_empty() {
                        println!(
                            "   {} {}{}",
                            "Preview:".bright_black(),
                            preview.bright_white(),
                            if first_msg.content.len() > 60 {
                                "..."
                            } else {
                                ""
                            }
                        );
                    }
                }
                println!();
            }
            Err(e) => {
                println!(
                    "{}. {} {}",
                    format!("{:3}", i + 1).bright_black(),
                    session_id.bright_white().bold(),
                    "(error loading)".red()
                );
                println!("   Error: {}", e.to_string().red());
                println!();
            }
        }
    }

    println!("{}", "=".repeat(80).bright_cyan());
    println!(
        "\n{} {}",
        "Tip:".bright_cyan().bold(),
        "View a session with: grok history view <session-id>".bright_white()
    );

    Ok(())
}

/// View a specific chat session
async fn view_session(session_id: &str) -> Result<()> {
    let config = get_logger_config();
    let logger =
        ChatLogger::new(config).context("Failed to initialize chat logger for viewing session")?;

    let session = logger
        .load_session(session_id)
        .with_context(|| format!("Failed to load session: {}", session_id))?;

    display_session(&session)?;

    Ok(())
}

/// Display a chat session in formatted output
fn display_session(session: &ChatSession) -> Result<()> {
    println!("\n{}", "=".repeat(80).bright_cyan());
    println!(
        "{}",
        format!("  CHAT SESSION: {}", session.session_id)
            .bright_cyan()
            .bold()
    );
    println!("{}\n", "=".repeat(80).bright_cyan());

    // Display metadata
    println!(
        "{} {}",
        "Start Time:".bright_white().bold(),
        session.start_time.format("%Y-%m-%d %H:%M:%S UTC")
    );
    if let Some(end_time) = session.end_time {
        println!(
            "{} {}",
            "End Time:  ".bright_white().bold(),
            end_time.format("%Y-%m-%d %H:%M:%S UTC")
        );
        let duration = end_time
            .signed_duration_since(session.start_time)
            .num_seconds();
        println!(
            "{} {} seconds",
            "Duration:  ".bright_white().bold(),
            duration
        );
    } else {
        println!(
            "{} {}",
            "Status:    ".bright_white().bold(),
            "Active".yellow()
        );
    }
    println!(
        "{} {}\n",
        "Messages:  ".bright_white().bold(),
        session.messages.len()
    );

    println!("{}", "-".repeat(80).bright_black());
    println!();

    // Display messages
    for (i, msg) in session.messages.iter().enumerate() {
        let role_display = match msg.role.as_str() {
            "user" => "USER".bright_green().bold(),
            "assistant" => "ASSISTANT".bright_blue().bold(),
            "system" => "SYSTEM".bright_yellow().bold(),
            _ => msg.role.bright_white().bold(),
        };

        let time = msg.timestamp.format("%H:%M:%S");

        println!(
            "{} {} {}",
            format!("[{}]", i + 1).bright_black(),
            role_display,
            format!("({})", time).bright_black()
        );
        println!("{}", "-".repeat(80).bright_black());

        // Print message content with proper wrapping
        for line in msg.content.lines() {
            println!("{}", line);
        }

        // Display metadata if present
        if let Some(metadata) = &msg.metadata {
            println!();
            println!("{}", "Metadata:".bright_black());
            match serde_json::to_string_pretty(metadata) {
                Ok(json) => {
                    for line in json.lines() {
                        println!("  {}", line.bright_black());
                    }
                }
                Err(_) => println!("  {}", "(error displaying metadata)".red()),
            }
        }

        println!();
    }

    println!("{}", "=".repeat(80).bright_cyan());
    println!(
        "{} {} messages in session {}\n",
        "Total:".bright_cyan().bold(),
        session.messages.len(),
        session.session_id.bright_white()
    );

    Ok(())
}

/// Search through chat sessions for a query string
async fn search_sessions(query: &str) -> Result<()> {
    let config = get_logger_config();
    let logger = ChatLogger::new(config)
        .context("Failed to initialize chat logger for searching sessions")?;

    let sessions = logger
        .list_sessions()
        .context("Failed to list chat sessions")?;

    if sessions.is_empty() {
        print_info("No chat sessions found.");
        return Ok(());
    }

    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    for session_id in sessions {
        if let Ok(session) = logger.load_session(&session_id) {
            // Search in messages
            for (msg_idx, msg) in session.messages.iter().enumerate() {
                if msg.content.to_lowercase().contains(&query_lower) {
                    matches.push((session.clone(), msg_idx, msg.clone()));
                }
            }
        }
    }

    if matches.is_empty() {
        print_info(&format!("No matches found for query: '{}'", query));
        return Ok(());
    }

    println!("\n{}", "=".repeat(80).bright_cyan());
    println!(
        "{}",
        format!(
            "  SEARCH RESULTS: {} matches for '{}'",
            matches.len(),
            query
        )
        .bright_cyan()
        .bold()
    );
    println!("{}\n", "=".repeat(80).bright_cyan());

    for (i, (session, msg_idx, msg)) in matches.iter().enumerate() {
        println!(
            "{}. {} {}",
            format!("{:3}", i + 1).bright_black(),
            "Session:".bright_white().bold(),
            session.session_id.bright_cyan()
        );
        println!(
            "   {} Message {} by {}",
            "Match:".bright_black(),
            msg_idx + 1,
            msg.role.bright_white()
        );

        // Show context around the match
        let content_lines: Vec<&str> = msg.content.lines().collect();
        let matching_lines: Vec<(usize, &str)> = content_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&query_lower))
            .map(|(idx, line)| (idx, *line))
            .collect();

        for (line_idx, line) in matching_lines.iter().take(3) {
            // Show up to 3 matching lines
            // Highlight the query in the line
            let highlighted = highlight_query(line, query);
            println!(
                "   {}: {}",
                format!("L{}", line_idx + 1).bright_black(),
                highlighted
            );
        }

        println!();
    }

    println!("{}", "=".repeat(80).bright_cyan());
    println!(
        "\n{} {}",
        "Tip:".bright_cyan().bold(),
        "View full session with: grok history view <session-id>".bright_white()
    );

    Ok(())
}

/// Highlight query string in text
fn highlight_query(text: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    if let Some(pos) = text_lower.find(&query_lower) {
        let before = &text[..pos];
        let matched = &text[pos..pos + query.len()];
        let after = &text[pos + query.len()..];

        format!("{}{}{}", before, matched.bright_yellow().bold(), after)
    } else {
        text.to_string()
    }
}

/// Clear chat history
async fn clear_history(confirm: bool) -> Result<()> {
    if !confirm {
        print_error("This will delete all chat session logs!");
        println!(
            "{} {}",
            "To confirm, run:".bright_white(),
            "grok history clear --confirm".bright_cyan()
        );
        return Ok(());
    }

    let config = get_logger_config();

    if !config.log_dir.exists() {
        print_info("No chat history to clear.");
        return Ok(());
    }

    // Delete all log files
    let entries = std::fs::read_dir(&config.log_dir).context("Failed to read log directory")?;

    let mut deleted_count = 0;
    for entry in entries.flatten() {
        if entry.path().is_file() {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                print_error(&format!(
                    "Failed to delete {}: {}",
                    entry.path().display(),
                    e
                ));
            } else {
                deleted_count += 1;
            }
        }
    }

    print_success(&format!(
        "Cleared chat history: {} files deleted",
        deleted_count
    ));

    Ok(())
}

/// Get chat logger configuration from environment
fn get_logger_config() -> ChatLoggerConfig {
    let log_dir = std::env::var("GROK_CHAT_LOG_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".grok")
                .join("logs")
                .join("chat_sessions")
        });

    ChatLoggerConfig {
        enabled: true,
        log_dir,
        json_format: true,
        text_format: true,
        max_file_size_mb: 10,
        rotation_count: 5,
        include_system: true,
    }
}
