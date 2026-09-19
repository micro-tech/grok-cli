//! Structured display data returned by command handlers.
//!
//! This is the first step toward making all command handlers pure
//! (returning data instead of performing I/O directly).

use serde::{Deserialize, Serialize};

/// Structured result that command handlers can return instead of printing directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisplayData {
    /// Plain text to be printed (success/info level).
    Text(String),

    /// Error message.
    Error(String),

    /// Success message with optional data.
    Success { message: String, data: Option<serde_json::Value> },

    /// Multiple lines of output.
    Lines(Vec<String>),

    /// No output needed.
    None,

    /// Multiple DisplayData items.
    Multiple(Vec<DisplayData>),
}

impl DisplayData {
    pub fn text(s: impl Into<String>) -> Self {
        DisplayData::Text(s.into())
    }

    pub fn success(msg: impl Into<String>) -> Self {
        DisplayData::Success {
            message: msg.into(),
            data: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        DisplayData::Error(msg.into())
    }

    /// Convenience constructor for simple success with optional JSON payload.
    pub fn ok(msg: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        DisplayData::Success {
            message: msg.into(),
            data,
        }
    }
}