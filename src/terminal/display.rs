//! Terminal display functions for grok-cli binary
//!
//! This module contains all functions that print to stdout/stderr.
//! These are binary-only functions and should NOT be in the library.

use colored::*;
use std::io::{self, Write};

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

/// Clear the terminal screen
pub fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap_or(());
}

/// Print a separator line
pub fn print_separator(width: u16, color: Option<Color>) {
    let line = "─".repeat(width as usize);
    if let Some(c) = color {
        println!("{}", line.color(c));
    } else {
        println!("{}", line.dimmed());
    }
}

/// Print centered text
pub fn print_centered(text: &str, width: u16, color: Option<Color>) {
    let text_len = text.len();
    let padding = if width as usize > text_len {
        (width as usize - text_len) / 2
    } else {
        0
    };

    let centered = format!("{}{}", " ".repeat(padding), text);
    if let Some(c) = color {
        println!("{}", centered.color(c));
    } else {
        println!("{}", centered);
    }
}
