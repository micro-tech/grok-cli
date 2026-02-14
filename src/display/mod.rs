//! Display module for Grok CLI
//!
//! Handles ASCII art, banners, tips, and other visual elements.
//! This module provides formatting functions and display components.
//!
//! # Architecture Note
//! Some functions in this module perform direct I/O (printing to stdout/stderr).
//! According to Rust best practices, these should be moved to the binary crate.
//! They are marked as deprecated and will be refactored in a future release.

pub mod ascii_art;
pub mod banner;
pub mod components;
pub mod interactive;
pub mod terminal;
pub mod tips;

// Re-export commonly used items
pub use ascii_art::{get_logo_for_width, print_grok_logo};
pub use banner::{
    BannerConfig, BannerType, clear_current_line, print_banner, print_directory_recommendation,
    print_welcome_banner,
};
pub use tips::{get_random_tip, get_random_tips};

use colored::*;
use std::io::{self, Write};
use terminal_size::{Height, Width, terminal_size};

/// Get terminal dimensions
///
/// This function attempts to detect the actual terminal size.
/// Falls back to (80, 24) if detection fails.
pub fn get_terminal_size() -> (u16, u16) {
    if let Some((Width(w), Height(h))) = terminal_size() {
        (w, h)
    } else {
        (80, 24) // Default fallback
    }
}

/// Clear the terminal screen
///
/// # Deprecated
/// This function performs I/O and should not be in the library.
/// Use the terminal module in the binary crate instead.
#[deprecated(note = "Move to binary crate - performs I/O")]
pub fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap_or(());
}

/// Format a separator line (pure function, returns String)
pub fn format_separator(width: u16) -> String {
    "─".repeat(width as usize)
}

/// Format centered text (pure function, returns String)
pub fn format_centered(text: &str, width: u16) -> String {
    let text_len = text.len();
    let padding = if width as usize > text_len {
        (width as usize - text_len) / 2
    } else {
        0
    };
    format!("{}{}", " ".repeat(padding), text)
}
