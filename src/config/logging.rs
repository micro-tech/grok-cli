//! Logging and telemetry configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable file logging
    #[serde(default)]
    pub file_logging: bool,

    /// Log file path (None = default location)
    #[serde(default)]
    pub log_file: Option<PathBuf>,

    /// Maximum log file size in MB
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,

    /// Number of log files to rotate
    #[serde(default = "default_rotation_count")]
    pub rotation_count: u32,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_file_size_mb() -> u64 {
    10
}

fn default_rotation_count() -> u32 {
    5
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_logging: false,
            log_file: None,
            max_file_size_mb: default_max_file_size_mb(),
            rotation_count: default_rotation_count(),
        }
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryConfig {
    /// Enable telemetry
    pub enabled: bool,

    /// Path to telemetry log file
    pub log_file: Option<PathBuf>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_tokens_per_minute: u32,
    pub max_requests_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_minute: 100000,
            max_requests_per_minute: 60,
        }
    }
}
