use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import directly from the library crate to avoid duplicate compilation
use grok_cli::cli;
use grok_cli::utils::chat_logger;

// Binary-only modules for terminal I/O
mod terminal;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "grok_cli=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize chat logger
    let chat_logger_enabled = std::env::var("GROK_CHAT_LOGGING_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let chat_log_dir = std::env::var("GROK_CHAT_LOG_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".grok")
                .join("logs")
                .join("chat_sessions")
        });

    let config = chat_logger::ChatLoggerConfig {
        enabled: chat_logger_enabled,
        log_dir: chat_log_dir,
        json_format: true,
        text_format: true,
        max_file_size_mb: std::env::var("GROK_CHAT_LOG_MAX_SIZE_MB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10),
        rotation_count: std::env::var("GROK_CHAT_LOG_ROTATION_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5),
        include_system: std::env::var("GROK_CHAT_LOG_INCLUDE_SYSTEM")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true),
    };

    if let Err(e) = chat_logger::init(config) {
        eprintln!("Warning: Failed to initialize chat logger: {}", e);
    }

    // Run the application
    cli::app::run().await
}
