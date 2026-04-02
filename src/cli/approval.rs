//! Approval UI for external file access
//!
//! This module provides interactive prompts for user approval when accessing
//! files outside project boundaries.

use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use std::path::Path;

/// User's decision on external file access request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Allow access this one time only
    AllowOnce,
    /// Add to session-trusted paths (allow for rest of session)
    TrustAlways,
    /// Deny access to this file
    Deny,
}

/// Prompt user for approval to access an external file
///
/// Displays a styled terminal UI with the file path and options.
/// Returns the user's decision.
///
/// # Arguments
///
/// * `path` - The external file path being requested
/// * `config_source` - Where the external access config came from (e.g., ".grok/.env")
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use grok_cli::cli::approval::{prompt_external_access_approval, ApprovalDecision};
///
/// let path = Path::new("H:\\GitHub\\shared\\config.toml");
/// match prompt_external_access_approval(path, ".grok/.env") {
///     Ok(ApprovalDecision::AllowOnce) => println!("Allowed once"),
///     Ok(ApprovalDecision::TrustAlways) => println!("Trusted for session"),
///     Ok(ApprovalDecision::Deny) => println!("Denied"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn prompt_external_access_approval<P: AsRef<Path>>(
    path: P,
    config_source: &str,
) -> Result<ApprovalDecision> {
    let path = path.as_ref();

    // Don't clear screen, just add some spacing
    println!();

    // Draw the approval prompt box
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!(
        "│ {} External File Access Request                             │",
        "🔒".yellow()
    );
    println!("├─────────────────────────────────────────────────────────────┤");

    // Display path (truncate if too long)
    let path_str = path.display().to_string();
    let path_display = if path_str.len() > 55 {
        format!("...{}", &path_str[path_str.len() - 52..])
    } else {
        format!("{:<55}", path_str)
    };
    println!("│ Path: {}│", path_display.cyan());

    println!("│ Type: {:<53}│", "Read".green());
    println!("│ Reason: {:<49}│", "Requested by AI assistant".dimmed());
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ This path is OUTSIDE your project directory.                │");
    println!(
        "│ External access is configured in: {:<24}│",
        config_source.cyan()
    );
    println!("├─────────────────────────────────────────────────────────────┤");
    println!(
        "│ [{}]llow Once  [{}]rust Always  [{}]eny  [{}]iew Path           │",
        "A".green().bold(),
        "T".green().bold(),
        "D".red().bold(),
        "V".blue().bold()
    );
    println!("└─────────────────────────────────────────────────────────────┘");

    // Get user input
    loop {
        print!("\n{} ", "Your choice:".bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "a" | "allow" | "allow once" => {
                println!("  {} Access allowed for this request", "✓".green());
                return Ok(ApprovalDecision::AllowOnce);
            }
            "t" | "trust" | "trust always" => {
                println!("  {} Path trusted for this session", "✓".green());
                return Ok(ApprovalDecision::TrustAlways);
            }
            "d" | "deny" => {
                println!("  {} Access denied", "✗".red());
                return Ok(ApprovalDecision::Deny);
            }
            "v" | "view" | "view path" => {
                println!("\n{}", "Canonical Path:".bold().underline());
                match path.canonicalize() {
                    Ok(canonical) => println!("  {}", format!("{}", canonical.display()).cyan()),
                    Err(e) => println!("  {} Cannot canonicalize: {}", "⚠".yellow(), e),
                }
                if let Some(parent) = path.parent() {
                    println!("\n{}", "Parent Directory:".bold().underline());
                    println!("  {}", format!("{}", parent.display()).dimmed());
                }
                println!("\n{}", "File Info:".bold().underline());
                match path.metadata() {
                    Ok(metadata) => {
                        println!("  Exists: {}", "Yes".green());
                        println!(
                            "  Is file: {}",
                            if metadata.is_file() {
                                "Yes".green()
                            } else {
                                "No".red()
                            }
                        );
                        println!(
                            "  Is directory: {}",
                            if metadata.is_dir() {
                                "Yes".green()
                            } else {
                                "No".red()
                            }
                        );
                        let size = metadata.len();
                        println!("  Size: {} bytes", format_size(size).cyan());
                    }
                    Err(_) => {
                        println!("  Exists: {}", "No".yellow());
                    }
                }
                println!("\n{}", "Press Enter to continue...".dimmed());
                let mut _temp = String::new();
                let _ = io::stdin().read_line(&mut _temp)?;

                // Redraw the prompt
                println!("\n┌─────────────────────────────────────────────────────────────┐");
                println!(
                    "│ {} External File Access Request                             │",
                    "🔒".yellow()
                );
                println!("├─────────────────────────────────────────────────────────────┤");
                println!("│ Path: {}│", path_display.clone().cyan());
                println!(
                    "│ [{}]llow Once  [{}]rust Always  [{}]eny  [{}]iew Path           │",
                    "A".green().bold(),
                    "T".green().bold(),
                    "D".red().bold(),
                    "V".blue().bold()
                );
                println!("└─────────────────────────────────────────────────────────────┘");
                continue;
            }
            "" => {
                println!("  {} Please enter a choice (A/T/D/V)", "⚠".yellow());
                continue;
            }
            _ => {
                println!(
                    "  {} Invalid choice. Please enter A, T, D, or V.",
                    "✗".red()
                );
                continue;
            }
        }
    }
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Prompt for batch approval of multiple files
///
/// Used when multiple external files need access at once.
/// Returns a tuple of (allow_all, deny_all, individual_decisions)
pub fn prompt_batch_approval<P: AsRef<Path>>(
    paths: &[P],
    config_source: &str,
) -> Result<Vec<(String, ApprovalDecision)>> {
    let mut decisions = Vec::new();

    println!();
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!(
        "│ {} Multiple External File Access Requests                   │",
        "🔒".yellow()
    );
    println!("├─────────────────────────────────────────────────────────────┤");
    println!(
        "│ {} files need external access                              │",
        format!("{}", paths.len()).cyan().bold()
    );
    println!("│ Configured in: {:<44}│", config_source.cyan());
    println!("└─────────────────────────────────────────────────────────────┘");

    for path in paths {
        let path_str = path.as_ref().display().to_string();
        println!("\n{} {}", "→".blue(), path_str.cyan());

        let decision = prompt_external_access_approval(path, config_source)?;
        decisions.push((path_str, decision.clone()));

        // If user denies, ask if they want to deny all remaining
        if decision == ApprovalDecision::Deny && decisions.len() < paths.len() {
            print!("\n{} Deny all remaining files? [y/N]: ", "?".yellow());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                // Deny all remaining
                for remaining_path in &paths[decisions.len()..] {
                    let remaining_path_str = remaining_path.as_ref().display().to_string();
                    decisions.push((remaining_path_str, ApprovalDecision::Deny));
                }
                break;
            }
        }
    }

    Ok(decisions)
}

/// Show a summary of external access requests
pub fn show_access_summary(decisions: &[(String, ApprovalDecision)]) {
    println!();
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!(
        "│ {} External Access Summary                                   │",
        "📊".cyan()
    );
    println!("├─────────────────────────────────────────────────────────────┤");

    let allowed_once = decisions
        .iter()
        .filter(|(_, d)| *d == ApprovalDecision::AllowOnce)
        .count();
    let trusted = decisions
        .iter()
        .filter(|(_, d)| *d == ApprovalDecision::TrustAlways)
        .count();
    let denied = decisions
        .iter()
        .filter(|(_, d)| *d == ApprovalDecision::Deny)
        .count();

    println!(
        "│ Allowed once:   {:<43}│",
        format!("{} files", allowed_once).green()
    );
    println!(
        "│ Trusted:        {:<43}│",
        format!("{} files", trusted).green()
    );
    println!(
        "│ Denied:         {:<43}│",
        format!("{} files", denied).red()
    );
    println!("└─────────────────────────────────────────────────────────────┘");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_approval_decision_equality() {
        assert_eq!(ApprovalDecision::AllowOnce, ApprovalDecision::AllowOnce);
        assert_ne!(ApprovalDecision::AllowOnce, ApprovalDecision::Deny);
    }
}
