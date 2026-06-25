//! ASCII art definitions for Grok CLI
//!
//! Contains various sizes of the Grok logo and related ASCII art

use colored::*;

/// Full Grok logo ASCII art (large version)
pub const GROK_LOGO_LARGE: &str = r#"
  ░██████╗░██████╗░░█████╗░██╗░░██╗
  ██╔════╝░██╔══██╗██╔══██╗██║░██╔╝
  ██║░░██╗░██████╔╝██║░░██║█████═╝░
  ██║░░╚██╗██╔══██╗██║░░██║██╔═██╗░
  ╚██████╔╝██║░░██║╚█████╔╝██║░╚██╗
  ░╚═════╝░╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝
"#;

/// Medium Grok logo ASCII art
pub const GROK_LOGO_MEDIUM: &str = r#"
   ██████╗ ██████╗  ██████╗ ██╗  ██╗
  ██╔════╝ ██╔══██╗██╔═══██╗██║ ██╔╝
  ██║  ███╗██████╔╝██║   ██║█████╔╝
  ██║   ██║██╔══██╗██║   ██║██╔═██╗
  ╚██████╔╝██║  ██║╚██████╔╝██║  ██╗
   ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝
"#;

/// Small Grok logo ASCII art
pub const GROK_LOGO_SMALL: &str = r#"
   ▄████  ██▀███   ▒█████   ██ ▄█▀
  ██▒ ▀█▒▓██ ▒ ██▒▒██▒  ██▒ ██▄█▒
 ▒██░▄▄▄░▓██ ░▄█ ▒▒██░  ██▒▓███▄░
 ░▓█  ██▓▒██▀▀█▄  ▒██   ██░▓██ █▄
 ░▒▓███▀▒░██▓ ▒██▒░ ████▓▒░▒██▒ █▄
  ░▒   ▒ ░ ▒▓ ░▒▓░░ ▒░▒░▒░ ▒ ▒▒ ▓▒
"#;

/// Tiny Grok logo ASCII art (single line)
pub const GROK_LOGO_TINY: &str = r#"GROK"#;

/// X.ai branding ASCII art
pub const X_AI_BRANDING: &str = r#"
    ▄▀█   █   █▄▄ █▄█
    █▄█ ▄ █ ▄ █▄█  █
"#;

/// Get the appropriate logo based on terminal width
pub fn get_logo_for_width(width: u16) -> &'static str {
    if width >= 60 {
        GROK_LOGO_LARGE
    } else if width >= 45 {
        GROK_LOGO_MEDIUM
    } else if width >= 30 {
        GROK_LOGO_SMALL
    } else {
        GROK_LOGO_TINY
    }
}

/// Get the width of the ASCII art
pub fn get_logo_width(logo: &str) -> usize {
    logo.lines()
        .map(|line| line.trim_end().len())
        .max()
        .unwrap_or(0)
}

/// Format the Grok logo with gradient colors (pure function)
pub fn format_grok_logo(width: u16) -> String {
    let logo = get_logo_for_width(width);
    let lines: Vec<&str> = logo.lines().collect();
    let mut output = String::new();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        // Apply gradient from blue to purple to pink
        let colored_line = match i % 3 {
            0 => line.bright_blue(),
            1 => line.bright_magenta(),
            _ => line.bright_cyan(),
        };

        output.push_str(&format!("{}\n", colored_line));
    }

    output
}

/// Print the Grok logo with gradient colors
#[deprecated(note = "Use format_grok_logo and println! instead")]
pub fn print_grok_logo(width: u16) {
    print!("{}", format_grok_logo(width));
}

/// Return X.ai branding as a String (pure function, Task 138)
pub fn format_x_ai_branding() -> String {
    X_AI_BRANDING
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.bright_black().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Animated logo display (still performs I/O — deprecated for Task 138)
#[deprecated(note = "Move to binary crate - performs I/O + blocking sleep")]
pub fn print_animated_logo(width: u16) {
    use std::{thread, time::Duration};

    let logo = get_logo_for_width(width);
    let lines: Vec<&str> = logo.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let colored_line = match i % 3 {
            0 => line.bright_blue(),
            1 => line.bright_magenta(),
            _ => line.bright_cyan(),
        };

        println!("{}", colored_line);
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_logo_for_width() {
        assert_eq!(get_logo_for_width(80), GROK_LOGO_LARGE);
        assert_eq!(get_logo_for_width(50), GROK_LOGO_MEDIUM);
        assert_eq!(get_logo_for_width(35), GROK_LOGO_SMALL);
        assert_eq!(get_logo_for_width(20), GROK_LOGO_TINY);
    }

    #[test]
    fn test_get_logo_width() {
        assert!(get_logo_width(GROK_LOGO_LARGE) > 0);
        assert!(get_logo_width(GROK_LOGO_MEDIUM) > 0);
        assert!(get_logo_width(GROK_LOGO_SMALL) > 0);
    }
}
