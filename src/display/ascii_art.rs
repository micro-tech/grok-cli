//! ASCII art definitions for Grok CLI
//!
//! Four size tiers are defined so the banner degrades gracefully on narrow
//! terminals.  All tiers now spell out **GROK-CLI** (replacing the old
//! GROK-only art) and are intentionally compact — the largest variant is
//! only 3 lines tall.

use colored::*;

// ─── Logo constants ───────────────────────────────────────────────────────────

/// Large logo — Calvin-S box-drawing style, 3 rows tall, ~30 chars wide.
/// Shown on terminals ≥ 50 columns.
pub const GROK_LOGO_LARGE: &str = r#"
  ┌─┐┬─┐┌─┐┬┌─  ┌─┐┬  ┬
  │ ┬├┬┘│ │├┴┐  │  │  │
  └─┘┴└─└─┘┴ ┴  └─┘┴─┘┴
"#;

/// Medium logo — same art, tighter indent, shown at 36–49 columns.
pub const GROK_LOGO_MEDIUM: &str = r#"
 ┌─┐┬─┐┌─┐┬┌─ ┌─┐┬  ┬
 │ ┬├┬┘│ │├┴┐ │  │  │
 └─┘┴└─└─┘┴ ┴ └─┘┴─┘┴
"#;

/// Small logo — single-line bracketed text, shown at 22–35 columns.
pub const GROK_LOGO_SMALL: &str = r#"
  ╓── GROK-CLI ──╖
"#;

/// Tiny logo — plain text fallback for very narrow terminals (< 22 columns).
pub const GROK_LOGO_TINY: &str = r#"GROK-CLI"#;

// ─── X.ai branding ───────────────────────────────────────────────────────────

/// X.ai branding ASCII art
pub const X_AI_BRANDING: &str = r#"
    ▄▀█   █   █▄▄ █▄█
    █▄█ ▄ █ ▄ █▄█  █
"#;

// ─── Width-based selection ───────────────────────────────────────────────────

/// Return the appropriate logo constant for the given terminal width.
pub fn get_logo_for_width(width: u16) -> &'static str {
    if width >= 50 {
        GROK_LOGO_LARGE
    } else if width >= 36 {
        GROK_LOGO_MEDIUM
    } else if width >= 22 {
        GROK_LOGO_SMALL
    } else {
        GROK_LOGO_TINY
    }
}

/// Return the width (in columns) of the widest line in a logo string.
pub fn get_logo_width(logo: &str) -> usize {
    logo.lines()
        .map(|line| line.trim_end().len())
        .max()
        .unwrap_or(0)
}

// ─── Formatting ──────────────────────────────────────────────────────────────

/// Format the Grok logo with a two-tone blue/cyan gradient (pure function).
pub fn format_grok_logo(width: u16) -> String {
    let logo = get_logo_for_width(width);
    let mut output = String::new();

    for (i, line) in logo.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        // Three-line logos: top & bottom bright-blue, middle bright-cyan.
        let colored_line = match i % 2 {
            0 => line.bright_blue(),
            _ => line.bright_cyan(),
        };

        output.push_str(&format!("{}\n", colored_line));
    }

    output
}

/// Print the Grok logo with gradient colors.
#[deprecated(note = "Use format_grok_logo and println! instead")]
pub fn print_grok_logo(width: u16) {
    print!("{}", format_grok_logo(width));
}

/// Print X.ai branding.
pub fn print_x_ai_branding() {
    for line in X_AI_BRANDING.lines() {
        if !line.trim().is_empty() {
            println!("{}", line.bright_black());
        }
    }
}

/// Animated logo display (prints each line with a small delay).
pub fn print_animated_logo(width: u16) {
    use std::{thread, time::Duration};

    let logo = get_logo_for_width(width);

    for (i, line) in logo.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let colored_line = match i % 2 {
            0 => line.bright_blue(),
            _ => line.bright_cyan(),
        };

        println!("{}", colored_line);
        thread::sleep(Duration::from_millis(80));
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_logo_for_width() {
        assert_eq!(get_logo_for_width(80), GROK_LOGO_LARGE);
        assert_eq!(get_logo_for_width(50), GROK_LOGO_LARGE);
        assert_eq!(get_logo_for_width(40), GROK_LOGO_MEDIUM);
        assert_eq!(get_logo_for_width(36), GROK_LOGO_MEDIUM);
        assert_eq!(get_logo_for_width(25), GROK_LOGO_SMALL);
        assert_eq!(get_logo_for_width(10), GROK_LOGO_TINY);
    }

    #[test]
    fn test_get_logo_width() {
        assert!(get_logo_width(GROK_LOGO_LARGE) > 0);
        assert!(get_logo_width(GROK_LOGO_MEDIUM) > 0);
        assert!(get_logo_width(GROK_LOGO_SMALL) > 0);
    }

    #[test]
    fn test_logos_contain_grok_cli() {
        // Every logo (except tiny which is just the text) should render "GROK" and "CLI"
        // in some form. We verify they're non-empty.
        assert!(!GROK_LOGO_LARGE.trim().is_empty());
        assert!(!GROK_LOGO_MEDIUM.trim().is_empty());
        assert!(!GROK_LOGO_SMALL.trim().is_empty());
        assert_eq!(GROK_LOGO_TINY, "GROK-CLI");
    }
}
