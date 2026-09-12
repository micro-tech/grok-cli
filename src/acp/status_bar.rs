//! Dynamic Status Bar + Native Thinking support for ACP (v0.15+)
//!
//! This module builds and emits rich `session/update` events that Zed
//! renders as a beautiful bottom status bar + native thinking blocks.

use serde_json::{Value, json};
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

    /// Visual context graph, e.g. "[.....|      ]"
    /// - dots/spaces before `|` = used context
    /// - `|` = compression / archive point
    /// - after `|` = free space
    /// - `]` = max context
    pub context_graph: String,

    /// Small icons for currently active sub-agents.
    /// Populated from session.active_agents.
    pub agent_icons: Vec<String>,
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
            context_graph: "[............]".to_string(),
            agent_icons: vec![],
        }
    }
}

/// Maps a sub-agent role name to a cute little icon.
/// These are designed to look like small "shoulder + head" figures.
pub fn icon_for_agent_role(role: &str) -> &'static str {
    match role.to_lowercase().as_str() {
        "planner" | "plan" | "architect" => "🗺️",   // map / planning
        "coder" | "code" | "dev" | "programmer" => "💻", // coding
        "researcher" | "research" | "explorer" | "search" => "🔎", // research
        "verifier" | "tester" | "reviewer" => "✅",
        "writer" | "docs" => "📝",
        "debugger" | "fixer" => "🐛",
        _ => "🧑", // generic person (shoulder + head feel)
    }
}

/// Creates a compact visual context meter.
///
/// Style requested by user:
///   .....|..]
///
/// - `.` for used context (before compress point)
/// - `|` marks the compression / archive point
/// - remaining characters = free headroom
/// - `]` closes the bar (max context)
pub fn format_context_graph(current: usize, max: usize, compress_at: usize) -> String {
    const WIDTH: usize = 12;

    if max == 0 {
        return "[............]".to_string();
    }

    let used_ratio = (current as f64 / max as f64).clamp(0.0, 1.0);
    let comp_ratio = (compress_at as f64 / max as f64).clamp(0.0, 1.0);

    let used_chars = (used_ratio * WIDTH as f64).round() as usize;
    let comp_pos = (comp_ratio * WIDTH as f64).round() as usize;

    let mut bar = String::with_capacity(WIDTH + 3);
    bar.push('[');

    for i in 0..WIDTH {
        if i < used_chars {
            bar.push('█');           // used (solid block)
        } else if i == comp_pos {
            bar.push('|');           // compress point
        } else {
            bar.push('░');           // free
        }
    }

    bar.push(']');
    bar
}

/// Solid block version (higher contrast).
pub fn format_context_graph_blocks(current: usize, max: usize, compress_at: usize) -> String {
    const WIDTH: usize = 12;

    if max == 0 {
        return "[............]".to_string();
    }

    let used_ratio = (current as f64 / max as f64).clamp(0.0, 1.0);
    let comp_ratio = (compress_at as f64 / max as f64).clamp(0.0, 1.0);

    let used_chars = (used_ratio * WIDTH as f64).round() as usize;
    let comp_pos = (comp_ratio * WIDTH as f64).round() as usize;

    let mut bar = String::with_capacity(WIDTH + 3);
    bar.push('[');

    for i in 0..WIDTH {
        if i < used_chars {
            bar.push('█');
        } else if i == comp_pos {
            bar.push('|');
        } else {
            bar.push('░');
        }
    }

    bar.push(']');
    bar
}

/// Build the compact status line payload (structured for future native support).
pub fn build_status_line(state: &StatusBarState) -> Value {
    let status_icon = if state.is_generating { "⏳" } else { "●" };

    let agents_part = if state.agent_icons.is_empty() {
        String::new()
    } else {
        format!(" {}", state.agent_icons.join(""))
    };

    json!({
        "sessionUpdate": "status_update",
        "status": {
            "kind": "compact",
            "text": format!(
                "{} {}  {}  🧠 {}{}",
                status_icon,
                state.model,
                state.context_graph,
                state.thinking_mode,
                agents_part
            ),
            "timestamp": current_timestamp(),
        }
    })
}

/// Build the expanded action bar.
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

/// Structured thinking block.
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