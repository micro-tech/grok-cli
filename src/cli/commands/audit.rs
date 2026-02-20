// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

//! Audit command handler for grok-cli
//!
//! Handles audit log management operations including viewing, filtering,
//! exporting, and clearing external file access logs.

use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use colored::*;
use std::fs::File;
use std::io::Write;

use crate::AuditAction;
use crate::cli::{confirm, print_error, print_info, print_success};
use crate::security::audit::AuditLogger;

/// Handle audit-related commands
pub async fn handle_audit_action(action: AuditAction) -> Result<()> {
    match action {
        AuditAction::ExternalAccess {
            summary,
            count,
            from,
            to,
            path,
            export,
        } => handle_external_access_audit(summary, count, from, to, path, export).await,
        AuditAction::Clear { confirm: confirmed } => clear_audit_logs(confirmed).await,
    }
}

/// Handle external access audit log viewing
async fn handle_external_access_audit(
    show_summary: bool,
    count: usize,
    from_date: Option<String>,
    to_date: Option<String>,
    filter_path: Option<String>,
    export_file: Option<String>,
) -> Result<()> {
    let logger = AuditLogger::new(true)?;

    if !logger.is_enabled() {
        print_info("Audit logging is disabled");
        return Ok(());
    }

    // Parse date filters if provided
    let from_datetime = if let Some(date_str) = from_date {
        Some(parse_date(&date_str)?)
    } else {
        None
    };

    let to_datetime = if let Some(date_str) = to_date {
        Some(parse_date(&date_str)?)
    } else {
        None
    };

    // Get logs based on filters
    let logs = if let (Some(from), Some(to)) = (from_datetime, to_datetime) {
        logger.get_logs_in_range(from, to)?
    } else if let Some(path) = &filter_path {
        logger.get_logs_for_path(path)?
    } else {
        logger.get_recent_logs(count)?
    };

    // Export if requested
    if let Some(export_path) = export_file {
        export_to_csv(&logs, &export_path)?;
        print_success(&format!(
            "Exported {} entries to {}",
            logs.len(),
            export_path
        ));
        return Ok(());
    }

    // Show summary statistics
    if show_summary {
        show_audit_summary(&logger)?;
    }

    // Display log entries
    if logs.is_empty() {
        println!("{}", "No audit log entries found".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "External Access Audit Log".cyan().bold());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".cyan()
    );
    println!();

    for (i, log) in logs.iter().enumerate() {
        if i > 0 {
            println!(
                "{}",
                "───────────────────────────────────────────────────────────────".dimmed()
            );
        }

        // Timestamp
        let timestamp = log.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        println!("{} {}", "Timestamp:".bold(), timestamp.dimmed());

        // Path
        println!("{} {}", "Path:".bold(), log.path.cyan());

        // Operation
        println!("{} {}", "Operation:".bold(), log.operation);

        // Decision (colored based on outcome)
        let decision_colored = match log.decision.as_str() {
            "allowed" | "approved_once" | "approved_always" => log.decision.green(),
            "denied" => log.decision.red(),
            "error" => log.decision.yellow(),
            _ => log.decision.normal(),
        };
        println!("{} {}", "Decision:".bold(), decision_colored);

        // User
        println!("{} {}", "User:".bold(), log.user);

        // Session ID
        println!("{} {}", "Session:".bold(), log.session_id.dimmed());

        // Denial reason (if present)
        if let Some(reason) = &log.denial_reason {
            println!("{} {}", "Reason:".bold(), reason.red());
        }

        // Config source (if present)
        if let Some(source) = &log.config_source {
            println!("{} {}", "Config:".bold(), source.dimmed());
        }

        println!();
    }

    println!("{} entries displayed", logs.len());

    Ok(())
}

/// Show audit summary statistics
fn show_audit_summary(logger: &AuditLogger) -> Result<()> {
    let (total, allowed, denied) = logger.get_statistics()?;

    println!();
    println!("{}", "Audit Summary Statistics".cyan().bold());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".cyan()
    );
    println!();

    // Overall statistics
    println!("{}", "Overall Statistics:".green().bold());
    println!("  Total Requests:     {}", total.to_string().cyan());
    println!(
        "  Allowed:            {} ({:.1}%)",
        allowed.to_string().green(),
        if total > 0 {
            (allowed as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  Denied:             {} ({:.1}%)",
        denied.to_string().red(),
        if total > 0 {
            (denied as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    );
    println!();

    // Top accessed paths
    println!("{}", "Most Accessed Paths:".green().bold());
    let top_paths = logger.get_top_accessed_paths(10)?;

    if top_paths.is_empty() {
        println!("  {}", "No data available".dimmed());
    } else {
        for (i, (path, count)) in top_paths.iter().enumerate() {
            println!(
                "  {}. {} ({})",
                i + 1,
                truncate_path(path, 50).cyan(),
                format!("{} times", count).dimmed()
            );
        }
    }
    println!();

    // Recent denials
    let all_logs = logger.get_all_logs()?;
    let recent_denials: Vec<_> = all_logs
        .iter()
        .filter(|log| log.decision == "denied")
        .take(5)
        .collect();

    if !recent_denials.is_empty() {
        println!("{}", "Recent Denials:".yellow().bold());
        for (i, log) in recent_denials.iter().enumerate() {
            let reason = log.denial_reason.as_deref().unwrap_or("No reason");
            println!(
                "  {}. {} - {}",
                i + 1,
                truncate_path(&log.path, 40).red(),
                reason.dimmed()
            );
        }
        println!();
    }

    // Log file location
    println!("{}", "Log File:".green().bold());
    println!(
        "  {}",
        logger.get_log_file_path().display().to_string().dimmed()
    );
    println!();

    Ok(())
}

/// Export audit logs to CSV
fn export_to_csv(logs: &[crate::security::audit::ExternalAccessLog], path: &str) -> Result<()> {
    let mut file = File::create(path).map_err(|e| anyhow!("Failed to create CSV file: {}", e))?;

    // Write CSV header
    writeln!(
        file,
        "Timestamp,Path,Operation,Decision,User,SessionID,DenialReason,ConfigSource"
    )?;

    // Write data rows
    for log in logs {
        let denial_reason = log.denial_reason.as_deref().unwrap_or("");
        let config_source = log.config_source.as_deref().unwrap_or("");

        writeln!(
            file,
            "{},{},{},{},{},{},{},{}",
            log.timestamp.to_rfc3339(),
            escape_csv(&log.path),
            escape_csv(&log.operation),
            escape_csv(&log.decision),
            escape_csv(&log.user),
            escape_csv(&log.session_id),
            escape_csv(denial_reason),
            escape_csv(config_source)
        )?;
    }

    Ok(())
}

/// Escape CSV field (add quotes if needed)
fn escape_csv(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Parse date string (YYYY-MM-DD) to DateTime<Utc>
fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| anyhow!("Invalid date format '{}': {}. Use YYYY-MM-DD", date_str, e))?;

    let naive_datetime = naive_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Failed to create datetime"))?;

    Ok(DateTime::from_naive_utc_and_offset(naive_datetime, Utc))
}

/// Truncate path to fit in display width
fn truncate_path(path: &str, max_width: usize) -> String {
    if path.len() <= max_width {
        path.to_string()
    } else {
        let start_len = max_width / 2 - 2;
        let end_len = max_width / 2 - 2;
        format!("{}...{}", &path[..start_len], &path[path.len() - end_len..])
    }
}

/// Clear all audit logs
async fn clear_audit_logs(confirmed: bool) -> Result<()> {
    let logger = AuditLogger::new(true)?;

    if !confirmed {
        println!(
            "{}",
            "⚠️  This will permanently delete all audit logs"
                .yellow()
                .bold()
        );
        println!();

        if !confirm("Are you sure you want to clear all audit logs?")? {
            println!("{}", "Operation cancelled".dimmed());
            return Ok(());
        }
    }

    logger.clear_logs()?;
    print_success("All audit logs have been cleared");

    Ok(())
}
