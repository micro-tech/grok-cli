//! Dynamic Status Bar + Native Thinking support for ACP (v0.15+)
//!
//! This module builds and emits rich `session/update` events that Zed
//! renders as a beautiful bottom status bar + native thinking blocks.

use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Compact status line shown at the bottom of the ACP session.
#[derive(Debug, Clone)]
pub struct StatusBarState {
    pub model: String,
    pub thinking_mode: String, // "Off" | "Low" | "High"
    pub current_tokens: usize,
    pub max_tokens: usize,
    pub context_percent: f32,
    pub is_generating: bool,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            model: "grok-4".to_string(),
            thinking_mode: "Off".to_string(),
            current_tokens: 0,
            max_tokens: 950_000,
            context_percent: 0.0,
            is_generating: false,
        }
    }
}

/// Build the compact status line payload.
pub fn build_status_line(state: &StatusBarState) -> Value {
    let token_str = if state.max_tokens > 0 {
        format!(
            "{}/{} ({:.0}%)",
            state.current_tokens,
            state.max_tokens,
            state.context_percent * 100.0
        )
    } else {
        state.current_tokens.to_string()
    };

    let status_icon = if state.is_generating { "⏳" } else { "●" };

    json!({
        "sessionUpdate": "status_update",
        "status": {
            "kind": "compact",
            "text": format!(
                "{} {}  |  🧠 {}  |  📊 {}",
                status_icon, state.model, state.thinking_mode, token_str
            ),
            "timestamp": current_timestamp(),
        }
    })
}

/// Build the expanded action bar (shown when the status line is focused).
pub fn build_action_bar(state: &StatusBarState) -> Value {
    json!({
        "sessionUpdate": "action_bar_update",
        "actions": [
            { "id": "stop", "label": "■ Stop", "enabled": state.is_generating },
            { "id": "new_chat", "label": "＋ New Chat", "enabled": true },
            { "id": "think_high", "label": "🧠 High", "enabled": true, "active": state.thinking_mode == "High" },
            { "id": "think_low", "label": "🧠 Low", "enabled": true, "active": state.thinking_mode == "Low" },
            { "id": "think_off", "label": "🧠 Off", "enabled": true, "active": state.thinking_mode == "Off" },
            { "id": "clear_context", "label": "🗑 Clear", "enabled": true },
        ]
    })
}

/// Structured thinking block (native in Zed instead of markdown).
pub fn build_thinking_block(content: &str, is_partial: bool) -> Value {
    json!({
        "sessionUpdate": "thinking_block",
        "thinking": {
            "content": content,
            "is_partial": is_partial,
            "timestamp": current_timestamp(),
        }
    })
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
