//! General and output configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    #[serde(default)]
    pub preview_features: bool,
    #[serde(default)]
    pub preferred_editor: String,
    #[serde(default)]
    pub vim_mode: bool,
    #[serde(default)]
    pub disable_auto_update: bool,
    #[serde(default)]
    pub disable_update_nag: bool,
    #[serde(default)]
    pub enable_prompt_completion: bool,
    #[serde(default)]
    pub retry_fetch_errors: bool,
    #[serde(default)]
    pub debug_keystroke_logging: bool,
    #[serde(default)]
    pub session_retention: SessionRetentionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRetentionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_age: u64, // in hours
    #[serde(default)]
    pub max_count: u32,
    #[serde(default)]
    pub min_retention: u64, // in hours
}

impl Default for SessionRetentionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_age: 168, // 7 days
            max_count: 50,
            min_retention: 24, // 1 day
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_format")]
    pub format: String,
}

fn default_output_format() -> String {
    "text".to_string()
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_output_format(),
        }
    }
}
