//! Audit logging for external file access
//!
//! This module provides comprehensive audit logging for all external file access attempts.
//! Logs are stored in JSONL format (JSON Lines) for easy parsing and analysis.

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{atomic::{AtomicUsize, Ordering}, Mutex};

#[cfg(test)]
static DISK_READ_COUNT: AtomicUsize = AtomicUsize::new(0);
use tracing::{debug, info};

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
    /// Lazily initialized buffered writer. Opened on first enabled write.
    /// This eliminates per-entry open/write/flush/close.
    writer: Mutex<Option<BufWriter<File>>>,
    /// In-memory cache of log entries (oldest first).
    /// Populated on first query or maintained on writes.
    cache: Mutex<Vec<ExternalAccessLog>>,
    /// Flag indicating the cache may be stale (needs reload from disk).
    cache_dirty: Mutex<bool>,
}

/// Maximum number of entries to keep in the in-memory stats cache.
/// Prevents unbounded memory growth for very long sessions.
const MAX_CACHE_ENTRIES: usize = 10_000;

impl AuditLogger {
    /// Create a new audit logger using the standard audit directory.
    ///
    /// Directory creation is deferred until the first `log_access` call
    /// (only when `enabled == true`). This avoids side-effects for
    /// `new(false)` and for test scenarios.
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
        let log_file_path = log_dir.join("external_access.jsonl");

        Ok(Self {
            log_file_path,
            enabled,
            writer: Mutex::new(None),
            cache: Mutex::new(Vec::new()),
            cache_dirty: Mutex::new(true),
        })
    }

    /// Create an audit logger that writes to an explicit file path.
    ///
    /// This is primarily intended for tests so they can use a
    /// `tempfile::TempDir` and avoid contaminating the real audit
    /// directory on the developer's machine.
    ///
    /// No directory is created at construction time.
    pub fn new_with_path(enabled: bool, log_file_path: PathBuf) -> Self {
        Self {
            log_file_path,
            enabled,
            writer: Mutex::new(None),
            cache: Mutex::new(Vec::new()),
            cache_dirty: Mutex::new(true),
        }
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

        // Lazy directory creation: only create when we are actually going to write.
        // This ensures new(false) and test constructions have zero FS side-effects.
        if let Some(parent) = self.log_file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| anyhow!("Failed to create audit log directory: {}", e))?;
                info!("Created audit log directory: {:?}", parent);
            }
        }

        // Serialize to JSON
        let json = serde_json::to_string(&log)
            .map_err(|e| anyhow!("Failed to serialize log entry: {}", e))?;

        // Use the persistent buffered writer (lazily opened)
        {
            let mut writer_guard = self.writer.lock().unwrap();
            if writer_guard.is_none() {
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.log_file_path)
                    .map_err(|e| anyhow!("Failed to open audit log file: {}", e))?;
                *writer_guard = Some(BufWriter::new(file));
            }

            if let Some(writer) = writer_guard.as_mut() {
                writeln!(writer, "{}", json)
                    .map_err(|e| anyhow!("Failed to write to audit log: {}", e))?;
                // Flush on every write for audit durability (cheap with BufWriter)
                // The main win is avoiding repeated open/close per entry.
                writer
                    .flush()
                    .map_err(|e| anyhow!("Failed to flush audit log: {}", e))?;
            }
        }

        // Update in-memory cache (append + enforce size limit)
        {
            let mut cache = self.cache.lock().unwrap();
            cache.push(log.clone());
            if cache.len() > MAX_CACHE_ENTRIES {
                cache.remove(0);
            }
            *self.cache_dirty.lock().unwrap() = false;
        }

        debug!("Logged external access: {} - {}", log.path, log.decision);
        Ok(())
    }

    /// Get the most recent log entries
    ///
    /// Uses the in-memory cache (populated lazily) for performance.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of entries to return
    ///
    /// # Returns
    ///
    /// Vector of log entries, most recent first
    pub fn get_recent_logs(&self, count: usize) -> Result<Vec<ExternalAccessLog>> {
        self.ensure_cache_loaded()?;

        let cache = self.cache.lock().unwrap();
        // Cache stores oldest-first; take last N then reverse for most-recent-first
        let start = if cache.len() > count { cache.len() - count } else { 0 };
        let mut recent: Vec<_> = cache[start..].to_vec();
        recent.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        recent.truncate(count);
        Ok(recent)
    }

    /// Get all log entries
    ///
    /// Uses the in-memory cache when available and not dirty for performance.
    /// Falls back to disk read and populates cache.
    ///
    /// # Returns
    ///
    /// Vector of all log entries, most recent first
    pub fn get_all_logs(&self) -> Result<Vec<ExternalAccessLog>> {
        self.ensure_cache_loaded()?;

        let cache = self.cache.lock().unwrap();
        // Return a reversed copy so most-recent-first (cache stores oldest-first)
        let mut logs = cache.clone();
        logs.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(logs)
    }

    /// Ensure the in-memory cache is populated from disk if dirty or empty.
    fn ensure_cache_loaded(&self) -> Result<()> {
        let mut dirty = self.cache_dirty.lock().unwrap();
        if !*dirty {
            return Ok(());
        }

        let mut cache = self.cache.lock().unwrap();
        cache.clear();

        if !self.log_file_path.exists() {
            *dirty = false;
            return Ok(());
        }

        #[cfg(test)]
        DISK_READ_COUNT.fetch_add(1, Ordering::SeqCst);

        let file = File::open(&self.log_file_path)
            .map_err(|e| anyhow!("Failed to open audit log file: {}", e))?;

        let reader = BufReader::new(file);

        let mut loaded: Vec<ExternalAccessLog> = reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        // Keep only the most recent MAX_CACHE_ENTRIES
        if loaded.len() > MAX_CACHE_ENTRIES {
            loaded.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
            loaded.truncate(MAX_CACHE_ENTRIES);
            loaded.sort_by_key(|b| b.timestamp); // restore chronological (oldest first)
        }

        *cache = loaded;
        *dirty = false;
        Ok(())
    }

    /// Get log entries within a date range
    ///
    /// Serves from the in-memory cache (avoids full disk re-read on cache hits).
    pub fn get_logs_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ExternalAccessLog>> {
        self.ensure_cache_loaded()?;

        let cache = self.cache.lock().unwrap();
        let mut filtered: Vec<_> = cache
            .iter()
            .filter(|log| log.timestamp >= start && log.timestamp <= end)
            .cloned()
            .collect();

        filtered.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(filtered)
    }

    /// Get log entries for a specific path
    ///
    /// Serves from the in-memory cache.
    pub fn get_logs_for_path(&self, path: &str) -> Result<Vec<ExternalAccessLog>> {
        self.ensure_cache_loaded()?;

        let cache = self.cache.lock().unwrap();
        let mut filtered: Vec<_> = cache
            .iter()
            .filter(|log| log.path == path)
            .cloned()
            .collect();

        filtered.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(filtered)
    }

    /// Get statistics about logged access attempts
    ///
    /// Uses the in-memory cache (no disk re-read on repeated calls after first load).
    pub fn get_statistics(&self) -> Result<(usize, usize, usize)> {
        self.ensure_cache_loaded()?;

        let cache = self.cache.lock().unwrap();
        let total = cache.len();
        let allowed = cache
            .iter()
            .filter(|log| {
                log.decision == "allowed"
                    || log.decision == "approved_once"
                    || log.decision == "approved_always"
            })
            .count();
        let denied = cache
            .iter()
            .filter(|log| log.decision == "denied")
            .count();

        Ok((total, allowed, denied))
    }

    /// Get the most frequently accessed paths
    ///
    /// Uses the in-memory cache.
    pub fn get_top_accessed_paths(&self, count: usize) -> Result<Vec<(String, usize)>> {
        self.ensure_cache_loaded()?;

        let cache = self.cache.lock().unwrap();

        // Count accesses per path (from cache)
        let mut path_counts = std::collections::HashMap::new();
        for log in cache.iter() {
            *path_counts.entry(log.path.clone()).or_insert(0) += 1;
        }

        // Convert to vector and sort
        let mut sorted: Vec<(String, usize)> = path_counts.into_iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
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
        // Invalidate cache so next reads start fresh
        {
            let mut cache = self.cache.lock().unwrap();
            cache.clear();
            *self.cache_dirty.lock().unwrap() = true;
        }
        // Also drop any open writer handle
        {
            let mut writer = self.writer.lock().unwrap();
            *writer = None;
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

impl Drop for AuditLogger {
    fn drop(&mut self) {
        // Best-effort flush of any buffered data on drop.
        if let Ok(mut guard) = self.writer.lock() {
            if let Some(writer) = guard.as_mut() {
                let _ = writer.flush();
            }
        }
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

    fn temp_logger(enabled: bool) -> (AuditLogger, TempDir) {
        let temp = TempDir::new().unwrap();
        let log_path = temp.path().join("external_access.jsonl");
        let logger = AuditLogger::new_with_path(enabled, log_path);
        (logger, temp)
    }

    #[test]
    fn test_create_audit_logger() {
        let (logger, _temp) = temp_logger(true);
        assert!(logger.is_enabled());
    }

    #[test]
    fn test_log_access() {
        let (logger, _temp) = temp_logger(true);

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
        let (logger, _temp) = temp_logger(true);

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
        let (logger, _temp) = temp_logger(true);

        // Log some entries (no need to clear; temp dir is fresh)
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
        assert_eq!(total, 2);
        assert_eq!(allowed, 1);
        assert_eq!(denied, 1);
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
        let (logger, _temp) = temp_logger(false);
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

    #[test]
    fn test_new_false_creates_no_directory() {
        let temp = TempDir::new().unwrap();
        let log_path = temp.path().join("audit").join("external_access.jsonl");

        // Before construction
        assert!(!log_path.parent().unwrap().exists());

        let _logger = AuditLogger::new_with_path(false, log_path.clone());

        // Construction must not create anything
        assert!(!log_path.parent().unwrap().exists());
    }

    #[test]
    fn test_new_true_defers_directory_until_log() {
        let temp = TempDir::new().unwrap();
        let log_path = temp.path().join("audit").join("external_access.jsonl");
        let parent = log_path.parent().unwrap();

        assert!(!parent.exists());

        let logger = AuditLogger::new_with_path(true, log_path.clone());

        // new(true) must still not create the directory
        assert!(!parent.exists());

        // Only after writing does the directory appear
        let log = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "test".to_string(),
            operation: "read".to_string(),
            decision: "allowed".to_string(),
            user: "test".to_string(),
            session_id: "s1".to_string(),
            denial_reason: None,
            config_source: None,
        };
        logger.log_access(log).unwrap();

        assert!(parent.exists());
        assert!(log_path.exists());
    }

    #[test]
    fn test_stats_served_from_cache_after_first_load() {
        let (logger, _temp) = temp_logger(true);

        // Seed a couple of entries
        let log1 = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "C:\\test\\a.txt".to_string(),
            operation: "read".to_string(),
            decision: "allowed".to_string(),
            user: "t".to_string(),
            session_id: "s".to_string(),
            denial_reason: None,
            config_source: None,
        };
        let log2 = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "C:\\test\\b.txt".to_string(),
            operation: "read".to_string(),
            decision: "denied".to_string(),
            user: "t".to_string(),
            session_id: "s".to_string(),
            denial_reason: None,
            config_source: None,
        };
        logger.log_access(log1).unwrap();
        logger.log_access(log2).unwrap();

        // Force a cache load
        let _ = logger.get_statistics().unwrap();

        // Reset the test-only disk read counter
        #[cfg(test)]
        DISK_READ_COUNT.store(0, Ordering::SeqCst);

        // Second call must NOT read from disk again
        let (total, _, _) = logger.get_statistics().unwrap();
        assert_eq!(total, 2);

        #[cfg(test)]
        {
            let reads = DISK_READ_COUNT.load(Ordering::SeqCst);
            assert_eq!(reads, 0, "expected no additional disk read on cache hit");
        }
    }

    #[test]
    fn test_cache_invalidated_on_clear_logs() {
        let (logger, _temp) = temp_logger(true);

        let log = ExternalAccessLog {
            timestamp: Utc::now(),
            path: "C:\\test\\x.txt".to_string(),
            operation: "read".to_string(),
            decision: "allowed".to_string(),
            user: "t".to_string(),
            session_id: "s".to_string(),
            denial_reason: None,
            config_source: None,
        };
        logger.log_access(log).unwrap();

        // Populate cache
        let _ = logger.get_statistics().unwrap();

        // Clear should invalidate cache
        logger.clear_logs().unwrap();

        // After clear, stats should be zero (cache was invalidated)
        let (total, _, _) = logger.get_statistics().unwrap();
        assert_eq!(total, 0);
    }
}
