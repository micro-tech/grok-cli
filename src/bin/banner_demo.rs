//! Simple banner demo to showcase Grok CLI visual features
//!
//! This binary demonstrates the ASCII art, banners, and visual elements
//! without requiring API keys or interactive input.

use colored::*;
use std::env;

// Import the display modules from the main grok-cli crate
use grok_cli::display::*;

fn main() {
    println!(
        "{}",
        "🎪 Grok CLI Visual Features Demo".bright_cyan().bold()
    );
    println!("{}", "─".repeat(60).dimmed());
    println!();

    // Get terminal size
    let (width, height) = get_terminal_size();
    println!(
        "{}",
        format!("Terminal: {}×{} characters", width, height).dimmed()
    );
    println!();

    // Demo 1: ASCII Art Logo
    println!("{}", "1. ASCII Art Logo (Adaptive)".bright_blue().bold());
    println!("{}", format_grok_logo(width));
    println!();

    // Demo 2: Welcome Banner
    println!("{}", "2. Welcome Banner".bright_green().bold());
    let banner_config = BannerConfig {
        show_banner: true,
        show_tips: true,
        show_updates: true,
        width: Some(width),
    };
    println!("{}", format_welcome_banner(&banner_config));

    // Demo 3: Different Banner Types
    println!("{}", "3. Banner Variations".bright_magenta().bold());

    // Info banner
    let info_content = vec![
        "This is an information banner",
        "It shows helpful system messages",
        "Like configuration status or tips",
    ];
    println!(
        "{}",
        format_banner(
            "System Information",
            &info_content,
            BannerType::Info,
            Some(width),
        )
    );

    // Directory recommendation (simulated)
    if env::current_dir()
        .map(|d| {
            d.file_name().and_then(|n| n.to_str()).unwrap_or("") == "Users"
                || d.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .contains("home")
        })
        .unwrap_or(false)
    {
        println!(
            "{}",
            format_directory_recommendation(
                &env::current_dir().unwrap().display().to_string(),
                &banner_config,
            )
        );
    }

    // Demo 4: Color Scheme
    println!("{}", "4. Color Scheme".bright_yellow().bold());
    println!();

    let color_examples = vec![
        (
            "🔵 Primary (Blue)",
            "Grok branding and main headers",
            Color::BrightBlue,
        ),
        (
            "🟢 Success (Green)",
            "Successful operations and confirmations",
            Color::BrightGreen,
        ),
        (
            "🟡 Warning (Yellow)",
            "Warnings and important notices",
            Color::BrightYellow,
        ),
        (
            "🔴 Error (Red)",
            "Errors and critical issues",
            Color::BrightRed,
        ),
        (
            "🟣 Accent (Magenta)",
            "Interactive elements and highlights",
            Color::BrightMagenta,
        ),
        ("🔄 Info (Cyan)", "Information and tips", Color::BrightCyan),
    ];

    for (label, description, color) in color_examples {
        println!("  {} - {}", label.color(color), description);
    }
    println!();

    // Demo 5: Logo Size Variations
    println!("{}", "5. Logo Adaptability".bright_cyan().bold());
    println!();

    let sizes = vec![
        (80, "Large (80+ columns)"),
        (50, "Medium (45+ columns)"),
        (35, "Small (30+ columns)"),
        (20, "Tiny (<30 columns)"),
    ];

    for (test_width, description) in sizes {
        println!("{}: {}", "Testing".dimmed(), description);
        println!("{}", format_grok_logo(test_width));
        println!("{}", "─".repeat(40).dimmed());
        println!();
    }

    // Demo 6: Status Information
    println!("{}", "6. Status Display".bright_white().bold());
    println!();

    println!(
        "  {} API Key: {}",
        "🔑".bright_yellow(),
        "✓ Configured".bright_green()
    );
    println!(
        "  {} Model: {}",
        "🤖".bright_blue(),
        "grok-4".bright_cyan()
    );
    println!(
        "  {} Network: {}",
        "🌐".bright_green(),
        "✓ Connected".bright_green()
    );
    println!(
        "  {} Features: {}",
        "⚡".bright_magenta(),
        "All systems ready".bright_white()
    );
    println!();

    // Demo 7: Tips Display
    println!("{}", "7. Helpful Tips".bright_green().bold());
    let tips_content = vec![
        "💡 Use 'grok chat \"your question\"' for quick answers",
        "💻 Try 'grok code explain file.rs' for code analysis",
        "🔧 Run 'grok health --all' for system diagnostics",
        "⚙️  Configure with 'grok config show' and 'grok config set'",
        "🎭 Enable Zed integration with 'grok acp server'",
    ];

    for tip in tips_content {
        println!("  {}", tip);
    }
    println!();

    // Demo 8: Feature Summary
    let features = [
        "✨ Beautiful ASCII art with adaptive sizing",
        "🎨 Professional color scheme throughout",
        "📱 Responsive design for any terminal width",
        "🔔 Contextual banners and notifications",
        "💬 Rich interactive prompts (when working)",
        "📊 Comprehensive health monitoring",
        "⚙️  Advanced configuration management",
        "🚀 Starlink-optimized networking",
    ];

    let feature_banner_content: Vec<&str> = features.to_vec();
    println!(
        "{}",
        format_banner(
            "Enhanced Features",
            &feature_banner_content,
            BannerType::Welcome,
            Some(width),
        )
    );

    // Conclusion
    println!("{}", "🎉 Demo Complete!".bright_green().bold());
    println!();
    println!("{}", "The Grok CLI now features:".bright_white());
    println!(
        "• {} that rivals modern AI CLI tools",
        "Visual polish".bright_cyan()
    );
    println!(
        "• {} for any terminal environment",
        "Adaptive interface".bright_blue()
    );
    println!(
        "• {} with helpful guidance",
        "User-friendly experience".bright_green()
    );
    println!(
        "• {} for professional use",
        "Production-ready code".bright_magenta()
    );
    println!();
    println!(
        "{}",
        "Ready for interactive mode once the input loop is fixed! 🚀".bright_yellow()
    );
}
