//! Binary entry point for grok-cli (Task 137).
//!
//! This file lives in src/bin/ so it becomes a separate binary crate.
//! All heavy lifting is done by the library (`grok_cli::cli::app::run`).
//! The library itself contains only pure functions (no terminal I/O).

use std::sync::Mutex;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn setup_logging() {
    let log_file_path = dirs::home_dir()
        .map(|h| h.join(".grok-cli").join("logs").join("grok-errors.log"))
        .unwrap_or_else(|| std::path::PathBuf::from("grok-errors.log"));

    if let Some(parent) = log_file_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    let stderr_layer = fmt::layer()
        .compact()
        .with_target(false)
        .with_thread_ids(false)
        .with_writer(std::io::stderr);

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
            tracing_subscriber::registry()
                .with(env_filter)
                .with(stderr_layer)
                .init();

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