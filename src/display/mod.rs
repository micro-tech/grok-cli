//! Display module for Grok CLI
//!
//! Handles ASCII art, banners, tips, and other visual elements

pub mod ascii_art;
pub mod banner;
pub mod components;
pub mod interactive;
pub mod terminal;
pub mod tips;

pub use ascii_art::print_grok_logo;
pub use banner::{
    clear_current_line, print_directory_recommendation, print_welcome_banner,
    BannerConfig,
};

use colored::*;
use std::io::{self, Write};
use terminal_size::{terminal_size, Height, Width};

/// Get terminal dimensions
pub fn get_terminal_size() -> (u16, u16) {
    if let Some((Width(w), Height(h))) = terminal_size() {
        (w, h)
    } else {
        (80, 24) // Default fallback
    }
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
