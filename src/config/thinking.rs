//! Reasoning / thinking mode configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

/// Reasoning / thinking mode for models that support extended chain-of-thought.
///
/// Sent as the `reasoning_effort` field in the API request.
/// - `Off`  — no reasoning trace (`reasoning_effort` omitted from request).
/// - `Low`  — light reasoning; faster and cheaper.
/// - `High` — deep reasoning; highest quality, slower, higher cost.
///
/// Only grok-4, grok-4.x, grok-3-mini, and similar reasoning-capable models honour
/// this field.  Sending it to other models will result in an API error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingMode {
    /// No reasoning trace — standard response (default).
    #[default]
    Off,
    /// Light reasoning effort: faster, lower token cost.
    Low,
    /// High reasoning effort: most thorough, slower, higher token cost.
    High,
}

impl ThinkingMode {
    /// Convert to the `reasoning_effort` string expected by the xAI API.
    /// Returns `None` when `Off` so callers can skip the field entirely.
    pub fn as_api_str(&self) -> Option<&'static str> {
        match self {
            ThinkingMode::Off => None,
            ThinkingMode::Low => Some("low"),
            ThinkingMode::High => Some("high"),
        }
    }

    /// Parse from a human-readable string (case-insensitive).
    /// Accepts `"off"`, `"low"`, `"high"`.
    pub fn from_str_ci(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "off" | "none" => Some(ThinkingMode::Off),
            "low" => Some(ThinkingMode::Low),
            "high" | "on" => Some(ThinkingMode::High),
            _ => None,
        }
    }
}
