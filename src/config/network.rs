//! Network configuration (Starlink / satellite optimizations).
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

/// Network configuration optimized for satellite connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable Starlink-specific optimizations
    pub starlink_optimizations: bool,

    /// Base retry delay in seconds
    pub base_retry_delay: u64,

    /// Maximum retry delay in seconds
    pub max_retry_delay: u64,

    /// Enable network health monitoring
    pub health_monitoring: bool,

    /// Connection timeout in seconds
    pub connect_timeout: u64,

    /// Read timeout in seconds
    pub read_timeout: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            starlink_optimizations: true,
            base_retry_delay: 1,
            max_retry_delay: 60,
            health_monitoring: true,
            connect_timeout: 10,
            read_timeout: 300,
        }
    }
}
