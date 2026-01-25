//! CLI command handlers and utilities for grok-cli
//!
//! This module contains all the command-line interface logic, including
//! argument parsing, command dispatch, and user interaction utilities.

pub mod app;
pub mod commands;

use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

/// Create a progress spinner with the given message
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Print a success message with green color
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

/// Print an error message with red color
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

/// Print a warning message with yellow color
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

/// Print an info message with blue color
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

/// Prompt user for confirmation
pub fn confirm(message: &str) -> Result<bool> {
    print!("{} {} [y/N]: ", "?".cyan(), message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

/// Format code with syntax highlighting (simplified version)
pub fn format_code(code: &str, language: Option<&str>) -> String {
    // For now, just return the code as-is with some basic formatting
    // In a full implementation, you'd use a syntax highlighter like syntect
    let header = match language {
        Some(lang) => format!("```{}", lang),
        None => "```".to_string(),
    };

    format!("{}\n{}\n```", header, code)
}

/// Format a response from Grok with nice styling
pub fn format_grok_response(response: &str, show_thinking: bool) -> String {
    let mut formatted = String::new();

    if show_thinking {
        formatted.push_str(&format!("{}\n", "🤔 Grok's response:".cyan()));
    }

    formatted.push_str("┌─────────────────────────────────────────────────────┐\n");

    // Split response into lines and format each one
    for line in response.lines() {
        formatted.push_str(&format!("│ {:<51} │\n", line));
    }

    formatted.push_str("└─────────────────────────────────────────────────────┘\n");
    formatted
}

/// Truncate text to fit in terminal width
pub fn truncate_text(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        text.to_string()
    } else {
        format!("{}...", &text[..max_width.saturating_sub(3)])
    }
}

/// Get terminal width, defaulting to 80 if unable to determine
pub fn get_terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .unwrap_or(80)
}

/// Format a table with headers and rows
pub fn format_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut table = String::new();
    let terminal_width = get_terminal_width();
    let col_width = (terminal_width - headers.len() - 1) / headers.len();

    // Header
    table.push('┌');
    for (i, _) in headers.iter().enumerate() {
        if i > 0 {
            table.push('┬');
        }
        table.push_str(&"─".repeat(col_width));
    }
    table.push_str("┐\n");

    // Header content
    table.push('│');
    for header in headers {
        let formatted_header = format!(" {:<width$} ", header, width = col_width - 2);
        table.push_str(&truncate_text(&formatted_header, col_width));
        table.push('│');
    }
    table.push('\n');

    // Separator
    table.push('├');
    for (i, _) in headers.iter().enumerate() {
        if i > 0 {
            table.push('┼');
        }
        table.push_str(&"─".repeat(col_width));
    }
    table.push_str("┤\n");

    // Rows
    for row in rows {
        table.push('│');
        for (i, cell) in row.iter().enumerate() {
            if i >= headers.len() {
                break;
            }
            let formatted_cell = format!(" {:<width$} ", cell, width = col_width - 2);
            table.push_str(&truncate_text(&formatted_cell, col_width));
            table.push('│');
        }
        table.push('\n');
    }

    // Bottom
    table.push('└');
    for (i, _) in headers.iter().enumerate() {
        if i > 0 {
            table.push('┴');
        }
        table.push_str(&"─".repeat(col_width));
    }
    table.push_str("┘\n");

    table
}
