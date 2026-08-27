//! Default value providers for the top-level `Config` struct.
//!
//! These functions are used both by `#[serde(default = "...")]` attributes
//! and by the `Default` implementation.
//!
//! Extracted from the monolithic `mod.rs` to keep configuration defaults
//! centralized and easy to maintain.

use crate::constants::{
    DEFAULT_MAX_OUTPUT_TOKENS_GROK4, DEFAULT_MAX_RETRIES, DEFAULT_TIMEOUT_SECS,
    DEFAULT_TEMPERATURE,
};

/// Current recommended default model. Update when xAI releases a new flagship.
pub(crate) fn default_model() -> String {
    "grok-4".to_string()
}

pub(crate) fn default_temperature() -> f32 {
    DEFAULT_TEMPERATURE
}

pub(crate) fn default_max_tokens() -> u32 {
    // Output token budget (not context window size).
    // grok-4 / grok-4.x supports large output budgets; 16 384 is a safe default
    // that avoids accidental large responses while still allowing detailed answers.
    DEFAULT_MAX_OUTPUT_TOKENS_GROK4
}

pub(crate) fn default_timeout_secs() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

pub(crate) fn default_max_retries() -> u32 {
    DEFAULT_MAX_RETRIES
}
