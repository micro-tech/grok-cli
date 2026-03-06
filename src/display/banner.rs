//! Banner display module for Grok CLI
//!
//! Handles startup banners, update notifications, and warning messages

use colored::*;
use std::io::{self, Write};

/// Banner types for different contexts
#[derive(Debug, Clone, PartialEq)]
pub enum BannerType {
    /// Welcome/startup banner
    Welcome,
    /// Update notification
    Update,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Information message
    Info,
}

/// Banner configuration
#[derive(Debug, Clone)]
pub struct BannerConfig {
    pub show_banner: bool,
    pub show_tips: bool,
    pub show_updates: bool,
    pub width: Option<u16>,
}

impl Default for BannerConfig {
    fn default() -> Self {
        Self {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: None,
        }
    }
}

/// Format a bordered banner with content (pure function)
pub fn format_banner(
    title: &str,
    content: &[&str],
    banner_type: BannerType,
    width: Option<u16>,
) -> String {
    let term_width = width.unwrap_or(crate::display::get_terminal_size().0);
    let banner_width = std::cmp::min(term_width - 4, 80); // Leave margin and cap at 80
    let mut output = String::new();

    let (border_color, title_color, content_color) = match banner_type {
        BannerType::Welcome => (Color::Cyan, Color::BrightCyan, Color::White),
        BannerType::Update => (Color::Green, Color::BrightGreen, Color::White),
        BannerType::Warning => (Color::Yellow, Color::BrightYellow, Color::White),
        BannerType::Error => (Color::Red, Color::BrightRed, Color::White),
        BannerType::Info => (Color::Blue, Color::BrightBlue, Color::White),
    };

    // Top border
    output.push_str(&format!(
        "{}\n",
        format!("┌{}┐", "─".repeat(banner_width as usize - 2)).color(border_color)
    ));

    // Title
    if !title.is_empty() {
        let title_line = format_banner_line(title, banner_width, true);
        output.push_str(&format!(
            "{}{}{}\n",
            "│".color(border_color),
            title_line.color(title_color).bold(),
            "│".color(border_color)
        ));

        // Separator after title
        output.push_str(&format!(
            "{}\n",
            format!("├{}┤", "─".repeat(banner_width as usize - 2)).color(border_color)
        ));
    }

    // Content lines
    for line in content {
        if line.trim().is_empty() {
            // Empty line
            output.push_str(&format!(
                "{}{}{}\n",
                "│".color(border_color),
                " ".repeat(banner_width as usize - 2),
                "│".color(border_color)
            ));
        } else {
            let content_line = format_banner_line(line, banner_width, false);
            output.push_str(&format!(
                "{}{}{}\n",
                "│".color(border_color),
                content_line.color(content_color),
                "│".color(border_color)
            ));
        }
    }

    // Bottom border
    output.push_str(&format!(
        "{}\n",
        format!("└{}┘", "─".repeat(banner_width as usize - 2)).color(border_color)
    ));

    output
}

/// Print a bordered banner with content
#[deprecated(note = "Use format_banner and println! instead")]
pub fn print_banner(title: &str, content: &[&str], banner_type: BannerType, width: Option<u16>) {
    println!("{}", format_banner(title, content, banner_type, width));
}

/// Format a line to fit within banner width with proper padding
fn format_banner_line(text: &str, width: u16, center: bool) -> String {
    let content_width = width as usize - 4; // Account for borders and padding
    let text = text.trim();

    if text.len() > content_width {
        // Truncate if too long
        format!(
            " {:<width$} ",
            &text[..content_width],
            width = content_width
        )
    } else if center {
        // Center the text
        let padding = (content_width - text.len()) / 2;
        let right_padding = content_width - text.len() - padding;
        format!(
            " {}{}{} ",
            " ".repeat(padding),
            text,
            " ".repeat(right_padding)
        )
    } else {
        // Left align with padding
        format!(" {:<width$} ", text, width = content_width)
    }
}

/// Format welcome banner with logo and tips (pure function)
pub fn format_welcome_banner(config: &BannerConfig) -> String {
    if !config.show_banner {
        return String::new();
    }

    let (width, _) = crate::display::get_terminal_size();
    let mut output = String::new();

    // Logo
    output.push_str(&crate::display::ascii_art::format_grok_logo(width));

    // Version
    let version = env!("CARGO_PKG_VERSION");
    let version_text = format!("v{}", version);
    let padding = (width as usize - version_text.len()) / 2;
    output.push_str(&format!(
        "{}{}\n\n",
        " ".repeat(padding),
        version_text.dimmed()
    ));

    // Welcome message
    if config.show_tips {
        let content = vec![
            "Tips for getting started:",
            "1. Ask questions, edit files, or run commands.",
            "2. Be specific for the best results.",
            "3. /help for more information.",
        ];

        output.push_str(&format_banner(
            "",
            &content,
            BannerType::Welcome,
            config.width,
        ));
    }

    output
}

/// Print welcome banner with logo and tips
#[deprecated(note = "Use format_welcome_banner and println! instead")]
pub fn print_welcome_banner(config: &BannerConfig) {
    print!("{}", format_welcome_banner(config));
}

/// Format update notification banner (pure function)
pub fn format_update_banner(
    current_version: &str,
    latest_version: &str,
    config: &BannerConfig,
) -> String {
    if !config.show_updates {
        return String::new();
    }

    let update_message = format!(
        "Grok CLI update available! {} → {}",
        current_version, latest_version
    );
    let content = vec![
        &update_message,
        "Please run: cargo install --git https://github.com/microtech/grok-cli",
    ];

    format_banner(
        "Update Available",
        &content,
        BannerType::Update,
        config.width,
    )
}

/// Print update notification banner
#[deprecated(note = "Use format_update_banner and println! instead")]
pub fn print_update_banner(current_version: &str, latest_version: &str, config: &BannerConfig) {
    print!(
        "{}",
        format_update_banner(current_version, latest_version, config)
    );
}

/// Format warning banner (pure function)
pub fn format_warning_banner(title: &str, message: &str, config: &BannerConfig) -> String {
    let content = vec![message];
    format_banner(title, &content, BannerType::Warning, config.width)
}

/// Print warning banner
#[deprecated(note = "Use format_warning_banner and println! instead")]
pub fn print_warning_banner(title: &str, message: &str, config: &BannerConfig) {
    print!("{}", format_warning_banner(title, message, config));
}

/// Format directory recommendation banner (pure function)
pub fn format_directory_recommendation(current_dir: &str, config: &BannerConfig) -> String {
    let home_message = "You are running Grok CLI in your home directory.".to_string();
    let current_dir_message = format!("Current directory: {}", current_dir);
    let content = vec![
        &home_message,
        "It is recommended to run in a project-specific directory.",
        "",
        &current_dir_message,
    ];

    format_banner(
        "Directory Recommendation",
        &content,
        BannerType::Info,
        config.width,
    )
}

/// Print directory recommendation banner
#[deprecated(note = "Use format_directory_recommendation and println! instead")]
pub fn print_directory_recommendation(current_dir: &str, config: &BannerConfig) {
    print!("{}", format_directory_recommendation(current_dir, config));
}

/// Format error banner (pure function)
pub fn format_error_banner(title: &str, error: &str) -> String {
    let content = vec![error];
    format_banner(title, &content, BannerType::Error, None)
}

/// Print error banner
#[deprecated(note = "Use format_error_banner and println! instead")]
pub fn print_error_banner(title: &str, error: &str) {
    print!("{}", format_error_banner(title, error));
}

/// Print a simple status line (like the bottom status bar in Gemini CLI)
pub fn print_status_line(
    mode: &str,
    model: &str,
    context_used: Option<&str>,
    directory: Option<&str>,
) {
    let (width, _) = crate::display::get_terminal_size();

    let mut status_parts = vec![mode.to_string()];

    if let Some(dir) = directory {
        status_parts.push(format!("📁 {}", dir));
    }

    status_parts.push(format!("🤖 {}", model));

    if let Some(context) = context_used {
        status_parts.push(format!("📊 {}", context));
    }

    let status_line = status_parts.join(" | ");
    let padding = if width as usize > status_line.len() {
        width as usize - status_line.len()
    } else {
        0
    };

    print!("{}", " ".repeat(padding));
    println!("{}", status_line.dimmed());
}

/// Clear the current line and move cursor to beginning
pub fn clear_current_line() {
    print!("\r\x1b[K");
    io::stdout().flush().unwrap_or(());
}

/// Print a simple progress indicator
pub fn print_progress_dots(count: usize) {
    clear_current_line();
    print!("Thinking{}", ".".repeat((count % 4) + 1));
    io::stdout().flush().unwrap_or(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_banner_line() {
        let result = format_banner_line("Hello", 20, false);
        assert!(result.starts_with(" Hello"));
        assert_eq!(result.len(), 18); // 20 - 2 for padding chars

        let result = format_banner_line("Hello", 20, true);
        assert!(result.contains("Hello"));
        assert_eq!(result.len(), 18);
    }

    #[test]
    fn test_banner_config_default() {
        let config = BannerConfig::default();
        assert!(config.show_banner);
        assert!(config.show_tips);
        assert!(config.show_updates);
        assert!(config.width.is_none());
    }
}
