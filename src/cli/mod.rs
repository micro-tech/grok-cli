//! CLI utilities for grok-cli
//!
//! This module contains formatting functions and legacy I/O helpers.
//!
//! # Architecture Note
//! The I/O functions below are deprecated and should be moved to the binary crate.
//! They are kept here temporarily for backwards compatibility.
//! TODO: Refactor command handlers to use the terminal module in src/main.rs

// Allow deprecated warnings in this module since we know these functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users.
#![allow(deprecated)]

pub mod app;
pub mod commands;

use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

// ============================================================================
// DEPRECATED I/O FUNCTIONS - TODO: Move to binary crate
// ============================================================================

/// Create a progress spinner with the given message
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
/// Use the terminal module in the binary crate instead.
#[deprecated(note = "Move to binary crate - performs I/O")]
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
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

/// Print an error message with red color
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

/// Print a warning message with yellow color
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

/// Print an info message with blue color
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

/// Prompt user for confirmation
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn confirm(message: &str) -> Result<bool> {
    print!("{} {} [y/N]: ", "?".cyan(), message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

/// Get terminal width, defaulting to 80 if unable to determine
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn get_terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .unwrap_or(80)
}

// ============================================================================
// PURE FORMATTING FUNCTIONS - These are OK in library
// ============================================================================

/// Format code with syntax highlighting markers (simplified version)
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
        formatted.push_str("🤔 Grok's response:\n");
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

/// Format a table with headers and rows (returns String, does not print)
/// Uses provided terminal width or defaults to 80
pub fn format_table_with_width(
    headers: &[&str],
    rows: &[Vec<String>],
    terminal_width: usize,
) -> String {
    let mut table = String::new();
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

/// Format a table with headers and rows (auto-detects terminal width)
///
/// # Deprecated
/// This function calls get_terminal_width() which performs I/O.
#[deprecated(note = "Use format_table_with_width instead")]
pub fn format_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let terminal_width = get_terminal_width();
    format_table_with_width(headers, rows, terminal_width)
}

/// Format a list with bullet points
pub fn format_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("  • {}", item))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a key-value pair
pub fn format_key_value(key: &str, value: &str) -> String {
    format!("{}: {}", key, value)
}

/// Format multiple key-value pairs as a list
pub fn format_key_value_list(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("  {}: {}", key, value))
        .collect::<Vec<_>>()
        .join("\n")
}
