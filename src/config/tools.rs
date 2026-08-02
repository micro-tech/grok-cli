//! Tools configuration (shell, auto-accept, etc.).
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub auto_accept: bool,
    #[serde(default)]
    pub core: Vec<String>,
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub discovery_command: String,
    #[serde(default)]
    pub call_command: String,
    #[serde(default = "default_true")]
    pub use_ripgrep: bool,
    #[serde(default = "default_true")]
    pub enable_tool_output_truncation: bool,
    #[serde(default)]
    pub truncate_tool_output_threshold: u32,
    #[serde(default)]
    pub truncate_tool_output_lines: u32,
    #[serde(default = "default_true")]
    pub enable_message_bus_integration: bool,
    #[serde(default)]
    pub enable_hooks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    #[serde(default = "default_true")]
    pub enable_interactive_shell: bool,
    #[serde(default)]
    pub pager: String,
    #[serde(default)]
    pub show_color: bool,
    #[serde(default)]
    pub inactivity_timeout: u32,
    /// Hard timeout (seconds) applied to every `run_shell_command` call.
    /// 300 s is the default — enough for `cargo build` and `git` operations
    /// on slow connections.  Set to 0 to use the built-in default.
    /// Env override: `GROK_SHELL_TIMEOUT`
    #[serde(default = "default_shell_command_timeout")]
    pub command_timeout_secs: u64,
}

fn default_true() -> bool {
    true
}

fn default_shell_command_timeout() -> u64 {
    300
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            shell: ShellConfig::default(),
            auto_accept: false,
            core: Vec::new(),
            allowed: Vec::new(),
            exclude: Vec::new(),
            discovery_command: String::new(),
            call_command: String::new(),
            use_ripgrep: true,
            enable_tool_output_truncation: true,
            truncate_tool_output_threshold: 10000,
            truncate_tool_output_lines: 100,
            enable_message_bus_integration: true,
            enable_hooks: false,
        }
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            enable_interactive_shell: true,
            pager: String::new(),
            show_color: false,
            inactivity_timeout: 0,
            command_timeout_secs: default_shell_command_timeout(),
        }
    }
}
