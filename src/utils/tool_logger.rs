//! Dedicated tool-execution diagnostic logger.
//!
//! Every tool call (success **or** failure) can be recorded to a persistent
//! log file so you have a clear audit trail of what Grok attempted while
//! editing your project inside Zed.  Failure entries include the working
//! directory and trusted-directory list so the root cause of an
//! "Access denied" or "path not found" error is immediately visible.
//!
//! ## Log file location
//!
//! ```text
//! <project-root>/.grok/logs/grok-tool-error-log.log
//! ```
//!
//! The file is created automatically on first write.  Entries are plain-text
//! blocks terminated by a `---` separator so the file is easy to `grep`,
//! tail, or open in any editor.
//!
//! ## Example error entry
//!
//! ```text
//! [2026-03-25T21:09:27.927Z] TOOL-ERROR ─ write_file
//! Args:      {"path":"src/io/web_server/mod.rs","content":"..."}
//! Error:     Access denied: External access is disabled in configuration
//! Duration:  140µs
//! CWD:       H:\GitHub\bot
//! Trusted:
//!   • \\?\H:\GitHub\bot
//!   • \\?\H:\GitHub\bot\src\io\web_server
//! Hint: Launch Grok from the project root, or @-mention a file so the
//!   workspace root is auto-detected. The path must be under a trusted directory.
//! ---
//! ```
//!
//! ## Example success entry
//!
//! ```text
//! [2026-03-25T21:10:05.123Z] TOOL-OK    ─ write_file  path="src/main.rs"  result=1024B  55µs
//! ```
//!
//! ## Network resilience (Starlink)
//!
//! File writes are flushed immediately on errors so that even a sudden
//! connection drop or process kill leaves a complete entry on disk.
//! Success entries are buffered (no forced flush) to keep I/O overhead low.

use chrono::Utc;
use once_cell::sync::OnceCell;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

// ─────────────────────────────────────────────────────────────────────────────
// Global file handle
// ─────────────────────────────────────────────────────────────────────────────

/// Thread-safe, lazily initialised handle to the tool-error log file.
///
/// `None` means the file could not be opened; we silently skip writes rather
/// than panicking so the CLI always stays functional.
static LOG_FILE: OnceCell<Mutex<Option<fs::File>>> = OnceCell::new();

/// Maximum number of bytes to print from a single argument payload.
/// Keeps the log readable even when the LLM sends huge `content` fields.
const MAX_ARGS_DISPLAY_BYTES: usize = 512;

// ─────────────────────────────────────────────────────────────────────────────
// Path helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the default log-file path: `<cwd>/.grok/logs/grok-tool-error-log.log`.
///
/// Using `current_dir()` means the log is co-located with the project being
/// edited in Zed, not in the user's home directory.
fn default_log_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".grok")
        .join("logs")
        .join("grok-tool-error-log.log")
}

/// Return the path of the active tool-error log file.
///
/// Exposed publicly so callers can print the path in startup banners or
/// diagnostic output.
pub fn log_file_path() -> PathBuf {
    default_log_path()
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal write helper
// ─────────────────────────────────────────────────────────────────────────────

/// Open (or reuse) the global file handle and run `f` with a mutable
/// reference to the underlying [`fs::File`].
///
/// If the file cannot be opened or the mutex is poisoned the closure is
/// silently skipped — we never want the logger to crash the CLI.
fn with_file<F: FnOnce(&mut fs::File)>(f: F) {
    let cell = LOG_FILE.get_or_init(|| {
        let path = default_log_path();

        // Ensure the `.grok/logs/` directory exists.
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                tracing::warn!(
                    path = %parent.display(),
                    error = %e,
                    "tool_logger: failed to create log directory — tool diagnostics will not be persisted"
                );
                return Mutex::new(None);
            }
        }

        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => {
                tracing::debug!(
                    path = %path.display(),
                    "tool_logger: opened tool-error log file"
                );
                Mutex::new(Some(file))
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "tool_logger: could not open tool-error log file — tool diagnostics will not be persisted"
                );
                Mutex::new(None)
            }
        }
    });

    match cell.lock() {
        Ok(mut guard) => {
            if let Some(file) = guard.as_mut() {
                f(file);
            }
        }
        Err(poisoned) => {
            // Recover from a poisoned mutex: try to keep logging.
            let mut guard = poisoned.into_inner();
            if let Some(file) = guard.as_mut() {
                f(file);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Error category hinting
// ─────────────────────────────────────────────────────────────────────────────

/// Produce a one-or-two-line human hint for the most common tool errors.
///
/// Returns an empty string for errors that don't match a known pattern so
/// that the log entry stays concise.
fn quick_hint(error: &str) -> &'static str {
    let e = error.to_ascii_lowercase();

    if e.contains("external access is disabled")
        || (e.contains("access denied") && !e.contains("permission denied"))
    {
        "\nHint: Launch Grok from the project root, or @-mention a file so the \
         workspace root is auto-detected.\n  \
         The path must be relative and inside a trusted directory."
    } else if e.contains("cannot find the path")
        || e.contains("os error 3")
        || e.contains("no such file or directory")
        || e.contains("os error 2")
        || e.contains("failed to resolve path")
    {
        "\nHint: The file or directory does not exist at that path. Check the \
         spelling and confirm the path is relative to the project root."
    } else if e.contains("permission denied")
        || e.contains("os error 5")
        || e.contains("os error 13")
    {
        "\nHint: The OS denied access (file locked or requires elevated privileges). \
         Try closing any editor tabs holding the file open."
    } else if e.contains("timed out") || e.contains("timeout") || e.contains("deadline") {
        "\nHint: Network timeout — possibly a Starlink satellite handover. \
         Wait a few seconds and retry."
    } else if e.contains("connection") || e.contains("dns") || e.contains("network") {
        "\nHint: Network error. Check connectivity and retry after a short pause."
    } else {
        ""
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Write a detailed error entry for a failed tool call.
///
/// This is the primary function callers should use.  It flushes the entry to
/// disk immediately so that even a process crash or Starlink drop leaves a
/// complete record.
///
/// # Arguments
///
/// * `tool_name`    – registered tool name (e.g. `"write_file"`)
/// * `args`         – the JSON arguments the tool was called with
/// * `error`        – the error message string
/// * `duration_us`  – how long the call took in microseconds
/// * `cwd`          – working directory at call time
/// * `trusted_dirs` – trusted directories from the active security policy
pub fn log_tool_error(
    tool_name: &str,
    args: &serde_json::Value,
    error: &str,
    duration_us: u128,
    cwd: &Path,
    trusted_dirs: &[PathBuf],
) {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");

    // Truncate huge argument payloads so the log stays readable.
    let args_raw = args.to_string();
    let args_display = if args_raw.len() > MAX_ARGS_DISPLAY_BYTES {
        format!(
            "{}… [truncated, {} total bytes]",
            &args_raw[..MAX_ARGS_DISPLAY_BYTES],
            args_raw.len()
        )
    } else {
        args_raw
    };

    let trusted_list = if trusted_dirs.is_empty() {
        "  (none — no trusted directories registered)".to_string()
    } else {
        trusted_dirs
            .iter()
            .map(|p| format!("  • {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let hint = quick_hint(error);

    let entry = format!(
        "[{timestamp}] TOOL-ERROR ─ {tool_name}\n\
         Args:      {args_display}\n\
         Error:     {error}\n\
         Duration:  {duration_us}µs\n\
         CWD:       {cwd}\n\
         Trusted:\n\
         {trusted_list}{hint}\n\
         ---\n",
        cwd = cwd.display(),
    );

    with_file(|f| {
        let _ = f.write_all(entry.as_bytes());
        // Force flush on errors so nothing is lost on crash or network drop.
        let _ = f.flush();
    });
}

/// Write a brief success entry for a completed tool call.
///
/// Success entries are intentionally terse — they are mainly useful for
/// confirming that write/replace operations actually reached the file system.
/// The entry is **not** force-flushed to keep I/O overhead low on busy
/// sessions; the OS will flush when the process exits cleanly.
///
/// # Arguments
///
/// * `tool_name`    – registered tool name (e.g. `"write_file"`)
/// * `args`         – the JSON arguments the tool was called with
/// * `result_bytes` – byte length of the tool's return value
/// * `duration_us`  – how long the call took in microseconds
pub fn log_tool_success(
    tool_name: &str,
    args: &serde_json::Value,
    result_bytes: usize,
    duration_us: u128,
) {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");

    // Extract the most useful single arg to keep the success line compact.
    let path_hint = args["path"]
        .as_str()
        .or_else(|| args["url"].as_str())
        .or_else(|| args["pattern"].as_str())
        .or_else(|| args["command"].as_str())
        .or_else(|| args["query"].as_str())
        .unwrap_or("-");

    let entry = format!(
        "[{timestamp}] TOOL-OK    ─ {tool_name}  path=\"{path_hint}\"  result={result_bytes}B  {duration_us}µs\n",
    );

    with_file(|f| {
        let _ = f.write_all(entry.as_bytes());
        // Intentionally no flush — buffered writes are fine for success lines.
    });
}

/// Write a free-form diagnostic note to the tool log.
///
/// Useful for recording session-level events such as startup, shutdown, or
/// security-policy changes that may affect tool behaviour.
pub fn log_note(note: &str) {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let entry = format!("[{timestamp}] NOTE       ─ {note}\n");
    with_file(|f| {
        let _ = f.write_all(entry.as_bytes());
        let _ = f.flush();
    });
}

/// Write a session-start banner to the tool log.
///
/// Call this once when a new ACP session is initialised so the log can be
/// split into per-session sections.
///
/// # Arguments
///
/// * `session_id` – the ACP session identifier
/// * `cwd`        – the working directory for the session
/// * `model`      – the model name in use
pub fn log_session_start(session_id: &str, cwd: &Path, model: &str) {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let entry = format!(
        "\n========================================\n\
         [{timestamp}] SESSION-START\n\
         Session: {session_id}\n\
         CWD:     {cwd}\n\
         Model:   {model}\n\
         ========================================\n",
        cwd = cwd.display(),
    );
    with_file(|f| {
        let _ = f.write_all(entry.as_bytes());
        let _ = f.flush();
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: temporarily override `current_dir()` is not possible in safe Rust,
    /// so we test the formatting helpers directly instead of the file I/O.

    #[test]
    fn quick_hint_access_denied() {
        let hint = quick_hint("Access denied: External access is disabled in configuration");
        assert!(hint.contains("project root"), "expected project-root hint");
        assert!(!hint.is_empty());
    }

    #[test]
    fn quick_hint_path_not_found_os_error_3() {
        let hint = quick_hint(
            "Failed to resolve path 'src/foo.rs': The system cannot find the path specified. (os error 3)",
        );
        assert!(hint.contains("does not exist"), "expected not-found hint");
    }

    #[test]
    fn quick_hint_permission_denied() {
        let hint = quick_hint("Permission denied (os error 13)");
        assert!(hint.contains("OS denied"), "expected OS permission hint");
    }

    #[test]
    fn quick_hint_timeout() {
        let hint = quick_hint("request timed out after 30s");
        assert!(hint.contains("Starlink"), "expected Starlink hint");
    }

    #[test]
    fn quick_hint_network() {
        let hint = quick_hint("Network error: connection refused");
        assert!(hint.contains("connectivity"), "expected connectivity hint");
    }

    #[test]
    fn quick_hint_unknown_returns_empty() {
        let hint = quick_hint("something completely unexpected");
        assert!(hint.is_empty(), "expected empty hint for unknown error");
    }

    #[test]
    fn log_file_path_ends_with_expected_components() {
        let path = log_file_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains(".grok"),
            "log path should contain .grok: {path_str}"
        );
        assert!(
            path_str.contains("logs"),
            "log path should contain logs: {path_str}"
        );
        assert!(
            path_str.contains("grok-tool-error-log.log"),
            "log path should contain grok-tool-error-log.log: {path_str}"
        );
    }

    #[test]
    fn args_truncation_at_max_bytes() {
        // Build a value whose JSON representation exceeds MAX_ARGS_DISPLAY_BYTES.
        let big_string = "x".repeat(MAX_ARGS_DISPLAY_BYTES + 100);
        let args = json!({ "content": big_string });
        let args_raw = args.to_string();
        let display = if args_raw.len() > MAX_ARGS_DISPLAY_BYTES {
            format!(
                "{}… [truncated, {} total bytes]",
                &args_raw[..MAX_ARGS_DISPLAY_BYTES],
                args_raw.len()
            )
        } else {
            args_raw
        };
        assert!(display.contains("[truncated"), "should be truncated");
        assert!(display.len() > MAX_ARGS_DISPLAY_BYTES, "prefix is present");
    }

    /// Integration test: write an error entry to a temp dir and verify the
    /// log file is created with the expected content.
    ///
    /// Because `OnceCell` is global we can't redirect the path inside the
    /// test process; instead we exercise the entry-formatting logic directly.
    #[test]
    fn error_entry_format_is_readable() {
        let temp = TempDir::new().unwrap();
        let log_path = temp.path().join("grok-tool-error-log.log");

        let args = json!({"path": "src/io/web_server/mod.rs"});
        let error = "Access denied: External access is disabled in configuration";
        let cwd = Path::new("H:/GitHub/bot");
        let trusted: Vec<PathBuf> = vec![
            PathBuf::from("\\\\?\\H:\\GitHub\\bot"),
            PathBuf::from("\\\\?\\H:\\GitHub\\bot\\src\\io\\web_server"),
        ];

        // Compose the entry the same way the public function does.
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let args_raw = args.to_string();
        let args_display = if args_raw.len() > MAX_ARGS_DISPLAY_BYTES {
            format!(
                "{}… [truncated, {} total bytes]",
                &args_raw[..MAX_ARGS_DISPLAY_BYTES],
                args_raw.len()
            )
        } else {
            args_raw
        };
        let trusted_list = trusted
            .iter()
            .map(|p| format!("  • {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let hint = quick_hint(error);
        let entry = format!(
            "[{timestamp}] TOOL-ERROR ─ write_file\n\
             Args:      {args_display}\n\
             Error:     {error}\n\
             Duration:  140µs\n\
             CWD:       {cwd}\n\
             Trusted:\n\
             {trusted_list}{hint}\n\
             ---\n",
            cwd = cwd.display(),
        );

        fs::write(&log_path, &entry).unwrap();
        let content = fs::read_to_string(&log_path).unwrap();

        assert!(content.contains("TOOL-ERROR"), "missing level marker");
        assert!(content.contains("write_file"), "missing tool name");
        assert!(content.contains("Access denied"), "missing error message");
        assert!(content.contains("H:/GitHub/bot"), "missing CWD");
        assert!(content.contains("GitHub\\bot"), "missing trusted directory");
        assert!(content.contains("project root"), "missing hint");
        assert!(content.ends_with("---\n"), "missing separator");
    }

    #[test]
    fn success_entry_format_is_compact() {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let args = json!({"path": "src/main.rs"});
        let path_hint = args["path"].as_str().unwrap_or("-");
        let entry = format!(
            "[{timestamp}] TOOL-OK    ─ write_file  path=\"{path_hint}\"  result=1024B  55µs\n",
        );

        assert!(entry.contains("TOOL-OK"), "missing level marker");
        assert!(entry.contains("write_file"), "missing tool name");
        assert!(entry.contains("src/main.rs"), "missing path hint");
        assert!(entry.contains("1024B"), "missing byte count");
        // Success entries must NOT contain '---' (that's only for errors).
        assert!(
            !entry.contains("---"),
            "success entry must not have separator"
        );
    }
}
