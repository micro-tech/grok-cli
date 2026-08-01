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
        /// Set to `true` only for very old MCP servers that still require the handshake.
        /// 
        /// Default: `false` — we now use the stateless 2026-07-28+ model
        /// (client info travels in `_meta` on every request).
        #[serde(default = "default_false")]
        use_legacy_handshake: bool,
    },
    #[serde(rename = "sse")]
    Sse { url: String },
}

fn default_false() -> bool {
    false
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self::Stdio {
            command: String::new(),
            args: vec![],
            env: HashMap::new(),
            use_legacy_handshake: false,
        }
    }
}
