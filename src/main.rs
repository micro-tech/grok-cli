//! Grok CLI entry point
//!
//! Initialises structured logging (tracing) before handing off to the
//! library's `cli::app::run()` function.
//!
//! ## Logging
//!
//! Two sinks are configured at startup:
//!
//! 1. **stderr** — compact, coloured output.  Level is controlled by the
//!    `RUST_LOG` environment variable (defaults to `warn` when unset).
//!    Example: `RUST_LOG=grok_cli=debug grok`
//!
//! 2. **~/.grok/logs/grok-errors.log** — append-only JSON lines, always
//!    captures `warn` and `error` events regardless of `RUST_LOG`.  Safe to
//!    tail or ingest into a log aggregator.

use std::sync::Mutex;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Set up the global `tracing` subscriber.
///
/// Returns `()` — any failure to open the log file is handled gracefully
/// (stderr-only logging is used as the fallback).
fn setup_logging() {
    // ── Log file path: ~/.grok/logs/grok-errors.log ──────────────────────────
    let log_file_path = dirs::home_dir()
        .map(|h| h.join(".grok").join("logs").join("grok-errors.log"))
        .unwrap_or_else(|| std::path::PathBuf::from("grok-errors.log"));

    if let Some(parent) = log_file_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // ── Single global filter ──────────────────────────────────────────────────
    //
    // Respects RUST_LOG; falls back to "warn" so users aren't flooded with
    // noise by default.  Both the stderr and file sinks share this filter.
    //
    // Examples:
    //   RUST_LOG=warn          (default)
    //   RUST_LOG=grok_cli=debug
    //   RUST_LOG=info
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    // ── Stderr layer (coloured, compact) ─────────────────────────────────────
    let stderr_layer = fmt::layer()
        .compact()
        .with_target(false)
        .with_thread_ids(false)
        .with_writer(std::io::stderr);

    // ── File layer (JSON, no ANSI) ────────────────────────────────────────────
    //
    // Open the log file for appending.  If this fails (e.g. read-only FS) we
    // fall back to stderr-only logging — the CLI must still work.
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(file) => {
            let file_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_ansi(false)
                .with_writer(Mutex::new(file));

            tracing_subscriber::registry()
                .with(env_filter)
                .with(stderr_layer)
                .with(file_layer)
                .init();
        }
        Err(err) => {
            // Can't open the log file — stderr only.
            tracing_subscriber::registry()
                .with(env_filter)
                .with(stderr_layer)
                .init();

            // Emit a warning now that the subscriber IS initialised.
            tracing::warn!(
                log_path = %log_file_path.display(),
                error = %err,
                "Could not open error log file — logging to stderr only"
            );
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();
    grok_cli::cli::app::run().await
}
