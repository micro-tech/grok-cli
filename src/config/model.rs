//! Model configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub max_session_turns: i32,
    #[serde(default)]
    pub summarize_tool_output: bool,
    #[serde(default)]
    pub compression_threshold: f64,
    #[serde(default = "default_true")]
    pub skip_next_speaker_check: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            max_session_turns: -1, // unlimited
            summarize_tool_output: false,
            compression_threshold: 0.2,
            skip_next_speaker_check: true,
        }
    }
}
