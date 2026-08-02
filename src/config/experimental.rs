//! Experimental and extension configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperimentalConfig {
    #[serde(default)]
    pub enable_agents: bool,
    #[serde(default)]
    pub extension_management: bool,
    #[serde(default)]
    pub extension_reloading: bool,
    #[serde(default)]
    pub jit_context: bool,
    #[serde(default)]
    pub codebase_investigator_settings: CodebaseInvestigatorConfig,
    #[serde(default)]
    pub extensions: ExtensionsConfig,
}

/// Extensions configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionsConfig {
    /// Enable extensions system
    #[serde(default)]
    pub enabled: bool,

    /// Directory to load extensions from
    #[serde(default)]
    pub extension_dir: Option<PathBuf>,

    /// List of enabled extensions
    #[serde(default)]
    pub enabled_extensions: Vec<String>,

    /// Allow loading extensions from config
    #[serde(default)]
    pub allow_config_extensions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseInvestigatorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub max_num_turns: u32,
    #[serde(default)]
    pub max_time_minutes: u32,
    #[serde(default)]
    pub thinking_budget: u32,
    #[serde(default)]
    pub model: String,
}

impl Default for CodebaseInvestigatorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_num_turns: 10,
            max_time_minutes: 15,
            thinking_budget: 1000,
            model: "auto".to_string(),
        }
    }
}
