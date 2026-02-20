//! Audit logging for external file access
//!
//! This module provides comprehensive audit logging for all external file access attempts.
//! Logs are stored in JSONL format (JSON Lines) for easy parsing and analysis.

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use tracing::{debug, error, info};

/// A single external file access log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAccessLog {
    /// Timestamp of the access attempt
    pub timestamp: DateTime<Utc>,

    /// Path that was accessed (or attempted to access)
    pub path: String,

    /// Type of operation (e.g., "read", "list", "search")
    pub operation: String,

    /// Decision made: "allowed", "approved_once", "approved_always", "denied", "error"
    pub decision: String,

    /// User who made the decision (system username)
    pub user: String,

    /// Session identifier
    pub session_id: String,

    /// Optional reason for denial
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denial_reason: Option<String>,

    /// Configuration source (e.g., ".grok/.env", "environment variable")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_source: Option<String>,
}

/// Audit logger for external file access
pub struct AuditLogger {
    log_file_path: PathBuf,
    enabled: bool,
}

impl AuditLogger {
    /// Create a new audit logger
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether audit logging is enabled
    ///
    /// # Example
    ///
    /// ```no_run
    /// use grok_cli::security::audit::AuditLogger;
    ///
    /// let logger = AuditLogger::new(true).expect("Failed to create audit logger");
    /// ```
    pub fn new(enabled: bool) -> Result<Self> {
        let log_dir = Self::get_audit_log_dir()?;

        // Create audit directory if it doesn't exist
        if !log_dir.exists() {
            fs::create_dir_all(&log_dir)
                .map_err(|e| anyhow!("Failed to create audit log directory: {}", e))?;
            info!("Created audit log directory: {:?}", log_dir);
        }

        let log_file_path = log_dir.join("external_access.jsonl");

        Ok(Self {
            log_file_path,
            enabled,
        })
    }

    /// Get the audit log directory path
    ///
    /// Returns ~/.grok/audit on Unix-like systems
    /// Returns %LOCALAPPDATA%\.grok\audit on Windows
    fn get_audit_log_dir() -> Result<PathBuf> {
        let base_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow!("Could not determine local data directory"))?;

        Ok(base_dir.join(".grok").join("audit"))
    }

    /// Log an external file access attempt
    ///
    /// # Arguments
    ///
    /// * `log` - The log entry to record
    ///
    /// # Example
    ///
    /// ```no_run
    /// use grok_cli::security::audit::{AuditLogger, ExternalAccessLog};
    /// use chrono::Utc;
    ///
    /// let logger = AuditLogger::new(true).unwrap();
    /// let log = ExternalAccessLog {
    ///     timestamp: Utc::now(),
    ///     path: "C:\\external\\file.txt".to_string(),
    ///     operation: "read".to_string(),
    ///     decision: "approved_once".to_string(),
    ///     user: "john".to_string(),
    ///     session_id: "abc123".to_string(),
    ///     denial_reason: None,
    ///     config_source: Some(".grok/.env".to_string()),
    /// };
    ///
    /// logger.log_access(log).unwrap();
    /// ```
    pub fn log_access(&self, log: ExternalAccessLog) -> Result<()> {
        if !self.enabled {
            debug!("Audit logging disabled, skipping log entry");
            return Ok(());
        }

        // Serialize to JSON
        let json = serde_json::to_string(&log)
            .map_err(|e| anyhow!("Failed to serialize log entry: {}", e))?;

        // Append to log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
            .map_err(|e| anyhow!("Failed to open audit log file: {}", e))?;

        writeln!(file, "{}", json).map_err(|e| anyhow!("Failed to write to audit log: {}", e))?;

        debug!("Logged external access: {} - {}", log.path, log.decision);
        Ok(())
    }

    /// Get the most recent log entries
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of entries to return
    ///
    /// # Returns
    ///
    /// Vector of log entries, most recent first
    pub fn get_recent_logs(&self, count: usize) -> Result<Vec<ExternalAccessLog>> {
        if !self.log_file_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_file_path)
            .map_err(|e| anyhow!("Failed to open audit log file: {}", e))?;

        let reader = BufReader::new(file);

        // Read all lines and parse
        let mut logs: Vec<ExternalAccessLog> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| {
                serde_json::from_str(&line)
                    .map_err(|e| {
                        error!("Failed to parse log line: {}", e);
                        e
                    })
                    .ok()
            })
            .collect();

        // Sort by timestamp (most recent first)
        logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Take only the requested count
        logs.truncate(count);

        Ok(logs)
    }

    /// Get all log entries
    ///
    /// # Returns
    ///
    /// Vector of all log entries, most recent first
    pub fn get_all_logs(&self) -> Result<Vec<ExternalAccessLog>> {
        if !self.log_file_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_file_path)
            .map_err(|e| anyhow!("Failed to open audit log file: {}", e))?;

        let reader = BufReader::new(file);

        let mut logs: Vec<ExternalAccessLog> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(logs)
    }

    /// Get log entries within a date range
    ///
    /// # Arguments
    ///
    /// * `start` - Start of date range (inclusive)
    /// * `end` - End of date range (inclusive)
    ///
    /// # Returns
    ///
    /// Vector of log entries within the date range, most recent first
    pub fn get_logs_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ExternalAccessLog>> {
        let all_logs = self.get_all_logs()?;

        let filtered: Vec<ExternalAccessLog> = all_logs
            .into_iter()
            .filter(|log| log.timestamp >= start && log.timestamp <= end)
            .collect();

        Ok(filtered)
    }

    /// Get log entries for a specific path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to filter by
    ///
    /// # Returns
    ///
    /// Vector of log entries for the specified path, most recent first
    pub fn get_logs_for_path(&self, path: &str) -> Result<Vec<ExternalAccessLog>> {
        let all_logs = self.get_all_logs()?;

        let filtered: Vec<ExternalAccessLog> = all_logs
            .into_iter()
            .filter(|log| log.path == path)
            .collect();

        Ok(filtered)
    }

    /// Get statistics about logged access attempts
    ///
    /// # Returns
    ///
    /// Tuple of (total, allowed, denied)
    pub fn get_statistics(&self) -> Result<(usize, usize, usize)> {
        let all_logs = self.get_all_logs()?;

        let total = all_logs.len();
        let allowed = all_logs
            .iter()
            .filter(|log| {
                log.decision == "allowed"
                    || log.decision == "approved_once"
                    || log.decision == "approved_always"
            })
            .count();
        let denied = all_logs
            .iter()
            .filter(|log| log.decision == "denied")
            .count();

        Ok((total, allowed, denied))
    }

    /// Get the most frequently accessed paths
    ///
    /// # Arguments
    ///
    /// * `count` - Number of top paths to return
    ///
    /// # Returns
    ///
    /// Vector of (path, access_count) tuples, sorted by count descending
    pub fn get_top_accessed_paths(&self, count: usize) -> Result<Vec<(String, usize)>> {
        let all_logs = self.get_all_logs()?;

        // Count accesses per path
        let mut path_counts = std::collections::HashMap::new();
        for log in all_logs {
            *path_counts.entry(log.path.clone()).or_insert(0) += 1;
        }

        // Convert to vector and sort
        let mut sorted: Vec<(String, usize)> = path_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(count);

        Ok(sorted)
    }

    /// Clear all audit logs
    ///
    /// # Warning
    ///
    /// This permanently deletes all audit log entries.
    pub fn clear_logs(&self) -> Result<()> {
        if self.log_file_path.exists() {
            fs::remove_file(&self.log_file_path)
                .map_err(|e| anyhow!("Failed to delete audit log file: {}", e))?;
            info!("Cleared audit log file");
        }
        Ok(())
    }

    /// Get the path to the audit log file
    pub fn get_log_file_path(&self) -> &PathBuf {
        &self.log_file_path
    }

    /// Check if audit logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Create a log entry for external file access
pub fn create_access_log(
    path: &str,
    operation: &str,
    decision: &str,
    session_id: &str,
    denial_reason: Option<String>,
    config_source: Option<String>,
) -> ExternalAccessLog {
    let user = whoami::username().unwrap_or_else(|_| "unknown".to_string());

    ExternalAccessLog {
        timestamp: Utc::now(),
        path: path.to_string(),
        operation: operation.to_string(),
        decision: decision.to_string(),
        user,
        session_id: session_id.to_string(),
        denial_reason,
        config_source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_audit_logger() {
        let logger = AuditLogger::new(true);
        assert!(logger.is_ok());

        let logger = logger.unwrap();
        assert!(logger.is_enabled());
    }

    #[test]
    fn test_log_access() {
        let logger = AuditLogger::new(true).unwrap();

        let log = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "C:\\test\\file.txt".to_string(),
            operation: "read".to_string(),
            decision: "approved_once".to_string(),
            user: "test_user".to_string(),
            session_id: "test_session".to_string(),
            denial_reason: None,
            config_source: Some(".grok/.env".to_string()),
        };

        let result = logger.log_access(log);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_recent_logs() {
        let logger = AuditLogger::new(true).unwrap();

        // Log multiple entries
        for i in 0..5 {
            let log = ExternalAccessLog {
                timestamp: Utc::now(),
                path: format!("C:\\test\\file{}.txt", i),
                operation: "read".to_string(),
                decision: "approved_once".to_string(),
                user: "test_user".to_string(),
                session_id: "test_session".to_string(),
                denial_reason: None,
                config_source: None,
            };
            logger.log_access(log).unwrap();
        }

        let recent = logger.get_recent_logs(3).unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_get_statistics() {
        let logger = AuditLogger::new(true).unwrap();

        // Clear existing logs
        let _ = logger.clear_logs();

        // Log some entries
        let log_allowed = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "C:\\test\\allowed.txt".to_string(),
            operation: "read".to_string(),
            decision: "approved_once".to_string(),
            user: "test_user".to_string(),
            session_id: "test_session".to_string(),
            denial_reason: None,
            config_source: None,
        };

        let log_denied = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "C:\\test\\denied.txt".to_string(),
            operation: "read".to_string(),
            decision: "denied".to_string(),
            user: "test_user".to_string(),
            session_id: "test_session".to_string(),
            denial_reason: Some("User denied".to_string()),
            config_source: None,
        };

        logger.log_access(log_allowed).unwrap();
        logger.log_access(log_denied).unwrap();

        let (total, allowed, denied) = logger.get_statistics().unwrap();
        assert!(total >= 2);
        assert!(allowed >= 1);
        assert!(denied >= 1);
    }

    #[test]
    fn test_create_access_log() {
        let log = create_access_log(
            "C:\\test\\file.txt",
            "read",
            "approved_once",
            "session123",
            None,
            Some(".grok/.env".to_string()),
        );

        assert_eq!(log.path, "C:\\test\\file.txt");
        assert_eq!(log.operation, "read");
        assert_eq!(log.decision, "approved_once");
        assert_eq!(log.session_id, "session123");
    }

    #[test]
    fn test_disabled_logger() {
        let logger = AuditLogger::new(false).unwrap();
        assert!(!logger.is_enabled());

        let log = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "C:\\test\\file.txt".to_string(),
            operation: "read".to_string(),
            decision: "approved_once".to_string(),
            user: "test_user".to_string(),
            session_id: "test_session".to_string(),
            denial_reason: None,
            config_source: None,
        };

        // Should succeed but not actually log
        let result = logger.log_access(log);
        assert!(result.is_ok());
    }
}
