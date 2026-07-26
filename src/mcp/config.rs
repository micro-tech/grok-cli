use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    pub servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerConfig {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        /// Whether to perform the legacy `initialize` + `notifications/initialized` handshake.
        /// 
        /// Set to `false` to use the stateless 2026-07-28+ model where client info
        /// travels in `_meta` on every request instead of a one-time handshake.
        /// 
        /// Default: `true` (for backward compatibility with 2024-11-05 and 2025 servers).
        #[serde(default = "default_true")]
        use_legacy_handshake: bool,
    },
    #[serde(rename = "sse")]
    Sse { url: String },
}

fn default_true() -> bool {
    true
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self::Stdio {
            command: String::new(),
            args: vec![],
            env: HashMap::new(),
            use_legacy_handshake: true,
        }
    }
}
