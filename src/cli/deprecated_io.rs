//! Deprecated I/O functions (Task 139).
//!
//! These functions perform terminal I/O and should only be used from the
//! binary crate (`src/bin/grok.rs`). They are kept here for backwards
//! compatibility during the library/binary separation.
//!
//! All new code should use the pure formatting functions in `crate::cli`
//! (format_success, format_error, etc.) and let the binary handle printing.

#![allow(deprecated)]

use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

/// Create a progress spinner with the given message
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
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

/// Print an error message with red color
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

/// Print a warning message with yellow color
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

/// Print an info message with blue color
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

/// Prompt user for confirmation
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn confirm(message: &str) -> Result<bool> {
    print!("{} {} [y/N]: ", "?".cyan(), message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

/// Get terminal width, defaulting to 80 if unable to determine
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn get_terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .unwrap_or(80)
}