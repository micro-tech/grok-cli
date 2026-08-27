//! Network configuration (Starlink / satellite optimizations).
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.
//!
//! This now also feeds the unified RetryPolicy (Task 287).

use serde::{Deserialize, Serialize};

/// Network configuration optimized for satellite connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable Starlink-specific optimizations (longer backoff tail, etc.)
    pub starlink_optimizations: bool,

    /// Base retry delay in seconds
    pub base_retry_delay: u64,

    /// Maximum retry delay in seconds
    pub max_retry_delay: u64,

    /// Maximum number of retries for transient network errors
    /// (global default; individual callers may override)
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Jitter in milliseconds added to backoff delays (prevents thundering herd)
    #[serde(default = "default_jitter_ms")]
    pub jitter_ms: u64,

    /// Enable network health monitoring
    pub health_monitoring: bool,

    /// Connection timeout in seconds
    pub connect_timeout: u64,

    /// Read timeout in seconds
    pub read_timeout: u64,
}

fn default_max_retries() -> u32 { 5 }
fn default_jitter_ms() -> u64 { 500 }

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            starlink_optimizations: true,
            base_retry_delay: 2,
            max_retry_delay: 60,
            max_retries: default_max_retries(),
            jitter_ms: default_jitter_ms(),
            health_monitoring: true,
            connect_timeout: 15,
            read_timeout: 300,
        }
    }
}
