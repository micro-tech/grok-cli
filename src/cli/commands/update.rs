//! `grok update` command — entry point for the self-update system.
//!
//! This command either:
//! - Runs a quick version check
//! - Or activates the `self-updater` skill with the user's request
//!
//! The real intelligence lives in the `self-updater` skill.

use anyhow::Result;
use colored::Colorize;
use crate::utils::version::check_for_update;
use crate::config::Config;

/// Handle the `grok update` subcommand.
pub async fn handle_update_command(
    check_only: bool,
    force: bool,
    config: &Config,
) -> Result<()> {
    // Respect global disable flag unless --force is used
    if config.general.disable_auto_update && !force {
        println!(
            "{} Auto-updates are disabled in your configuration.",
            "ℹ".bright_blue()
        );
        println!(
            "   You can still run {} or set {} to false.",
            "grok update --force".bright_cyan(),
            "general.disable_auto_update".bright_yellow()
        );
        return Ok(());
    }

    println!("{}", "Checking for updates...".bright_cyan());

    match check_for_update().await {
        Ok(result) => {
            println!();
            println!("  Current version: {}", result.current.bright_white());
            println!("  Latest version:  {}", result.latest.bright_white());

            if result.is_newer_available {
                println!();
                println!(
                    "{} New version available!",
                    "✨".bright_green()
                );

                if let Some(name) = &result.release.name {
                    println!("  Release: {}", name.bright_yellow());
                }
                println!("  {}", result.release.html_url.bright_blue());

                if let Some(body) = &result.release.body {
                    let summary: String = body.lines().take(6).collect::<Vec<_>>().join("\n");
                    if !summary.trim().is_empty() {
                        println!();
                        println!("{}", "Release notes (first few lines):".dimmed());
                        println!("{}", summary.dimmed());
                    }
                }

                println!();

                if check_only {
                    println!(
                        "Run {} to proceed with the update.",
                        "grok update".bright_cyan()
                    );
                    return Ok(());
                }

                // For a full update, we now hand off to the skill system.
                // The best experience is to tell the model (or the interactive loop)
                // to use the self-updater skill.
                println!(
                    "{} The recommended way to update is to activate the {} skill.",
                    "→".bright_cyan(),
                    "self-updater".bright_yellow().bold()
                );
                println!();
                println!("You can do this in two ways:");
                println!("  1. In an interactive session:   {}", "/activate self-updater".bright_cyan());
                println!("  2. Or run:                      {}", "grok skills show self-updater".bright_cyan());
                println!();
                println!(
                    "The skill will walk you through download, verification, and safe replacement."
                );
                println!();
                println!(
                    "{} This command can also be used with {} for non-interactive flows in the future.",
                    "Note:".bright_yellow(),
                    "--apply".bright_cyan()
                );

            } else {
                println!();
                println!(
                    "{} You are running the latest version ({}).",
                    "✓".bright_green(),
                    result.current.bright_green()
                );
            }
        }
        Err(e) => {
            eprintln!(
                "{} Failed to check for updates: {}",
                "✗".bright_red(),
                e
            );
            eprintln!("   You can still visit https://github.com/micro-tech/grok-cli/releases");
        }
    }

    Ok(())
}

/// Quick version check used by banner / health etc.
pub async fn quick_version_check() -> Option<(String, String, bool)> {
    match check_for_update().await {
        Ok(r) if r.is_newer_available => Some((r.current, r.latest, true)),
        _ => None,
    }
}
