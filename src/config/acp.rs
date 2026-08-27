//! ACP (Agent Client Protocol) configuration.

use serde::{Deserialize, Serialize};

use super::ThinkingMode;
use crate::constants::{
    COMPRESSION_CHUNK_RATIO, COMPRESSION_THRESHOLD, DEFAULT_MAX_TOOL_LOOP_ITERATIONS,
    DEFAULT_PERMISSION_TIMEOUT_SECS, DEFAULT_SHELL_TIMEOUT_SECS, GROK4_CONTEXT_BUDGET,
    LEGACY_CONTEXT_BUDGET, MAX_HISTORY_MESSAGES, MAX_TOOL_RESULT_CHARS,
};

/// ACP-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpConfig {
    /// Enable ACP server functionality
    pub enabled: bool,

    /// Default port for ACP server
    pub default_port: Option<u16>,

    /// Host to bind ACP server to
    pub bind_host: String,

    /// ACP protocol version to use (1.2.0 for latest agent-client-protocol)
    pub protocol_version: String,

    /// Enable development features
    pub dev_mode: bool,

    /// Maximum number of tool loop iterations before timeout
    /// This prevents infinite loops when the AI repeatedly calls tools
    /// Default: 25 (increase for complex multi-step tasks)
    #[serde(default = "default_max_tool_loop_iterations")]
    pub max_tool_loop_iterations: u32,

    /// Require explicit user permission before executing each tool call.
    ///
    /// Zed and other ACP-compliant clients support `session/request_permission`
    /// and will present a three-button dialog (Always Allow / Allow / Reject).
    /// Set to `false` to skip the prompt and allow all tool calls automatically.
    ///
    /// Default: true
    #[serde(default = "default_true_permission")]
    pub require_permission: bool,

    /// Timeout in seconds to wait for user permission response
    /// Default: 60
    #[serde(default = "default_permission_timeout_secs")]
    pub permission_timeout_secs: u64,

    /// Maximum number of conversation turns kept in session context.
    /// Older messages beyond this limit are trimmed before each API call
    /// to prevent unbounded context growth.
    /// Default: 40
    #[serde(default = "default_max_history_messages")]
    pub max_history_messages: usize,

    /// Soft token budget for the outgoing request (prompt + history) when
    /// using grok-3 or older models.
    /// Messages are trimmed from the oldest end until the estimated token
    /// count fits within this limit.  Leave ~36 k headroom for the model
    /// response and tool definitions.
    /// Grok-3 / grok-beta context window = 256 k tokens.
    /// Default: LEGACY_CONTEXT_BUDGET (220_000)  (for grok-3 / grok-2 — see grok4_max_context_tokens for grok-4.x)
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: usize,

    /// Soft token budget for grok-4.x models (grok-4 and later).
    /// Certain grok-4.x variants expose a 1_048_576-token context window.
    /// This budget leaves ~50 k headroom for model response + tool definitions.
    /// Default: GROK4_CONTEXT_BUDGET (950_000)
    #[serde(default = "default_grok4_max_context_tokens")]
    pub grok4_max_context_tokens: usize,

    /// Maximum number of characters kept per tool-result message before
    /// it is truncated.  Large file reads are the most common cause of
    /// context-window overflow; truncating them here keeps the history
    /// manageable without losing the conversation structure.
    /// 0 = no per-message truncation.
    /// Default: MAX_TOOL_RESULT_CHARS (30_000 ~7 500 tokens)
    #[serde(default = "default_max_tool_result_chars")]
    pub max_tool_result_chars: usize,

    /// Enable automatic context summarization when the active context approaches
    /// the token limit.  When triggered, the oldest messages are summarized by
    /// the AI and archived to disk; a compact notice replaces them in the active
    /// window so the model knows the history exists and can recall it.
    /// Set to `false` to revert to the old drop-only behaviour.
    /// Default: true
    #[serde(default = "default_auto_compress")]
    pub auto_compress: bool,

    /// Fraction of `max_context_tokens` at which auto-compression fires (0.0–1.0).
    /// When estimated prompt tokens exceed `max_context_tokens * compression_threshold`,
    /// the oldest chunk is summarized and archived.
    /// Default: COMPRESSION_THRESHOLD (0.75)  (fires at 75 % of the token budget)
    #[serde(default = "default_compression_threshold")]
    pub compression_threshold: f32,

    /// Fraction of current non-system messages to compress per compression event.
    /// E.g. 0.40 compresses the oldest 40 % of messages each time.
    /// Minimum of 4 messages is always enforced.
    /// Default: COMPRESSION_CHUNK_RATIO (0.40)
    #[serde(default = "default_compression_chunk_ratio")]
    pub compression_chunk_ratio: f32,

    /// Default reasoning / thinking mode for new sessions.
    ///
    /// - `off`  — no chain-of-thought reasoning (standard response).
    /// - `low`  — light reasoning effort.
    /// - `high` — deep reasoning effort (slower, more tokens).
    ///
    /// Only grok-4 / grok-4.x and grok-3-mini support this field.
    /// Default: off
    #[serde(default)]
    pub thinking_mode: ThinkingMode,

    /// Whether to emit `context_usage_update` notifications to ACP clients
    /// (Zed, etc.) after each turn.  Useful for showing a context meter in the editor.
    /// Default: true
    #[serde(default = "default_true")]
    pub show_context_usage: bool,

    /// Whether to emit `thinking_update` notifications containing the model's
    /// chain-of-thought / reasoning trace.
    /// Default: true
    #[serde(default = "default_true")]
    pub stream_thinking: bool,

    /// Custom instructions appended to every commit-message generation prompt
    /// (used by `/commit` and the `generate_commit_message` tool).
    /// Example: "Use Conventional Commits with scope and breaking-change footer."
    #[serde(default)]
    pub commit_message_instructions: String,
}

fn default_true() -> bool {
    true
}

fn default_true_permission() -> bool {
    true
}

fn default_max_tool_loop_iterations() -> u32 {
    DEFAULT_MAX_TOOL_LOOP_ITERATIONS
}

fn default_max_history_messages() -> usize {
    MAX_HISTORY_MESSAGES
}

fn default_max_context_tokens() -> usize {
    // grok-3 / grok-2 budget (256 k window, ~36 k headroom)
    LEGACY_CONTEXT_BUDGET
}

fn default_grok4_max_context_tokens() -> usize {
    // grok-4.x budget: 1_048_576-token window, ~50 k headroom for response + tools
    GROK4_CONTEXT_BUDGET
}

fn default_auto_compress() -> bool {
    true
}

fn default_compression_threshold() -> f32 {
    COMPRESSION_THRESHOLD
}

fn default_compression_chunk_ratio() -> f32 {
    COMPRESSION_CHUNK_RATIO
}

fn default_max_tool_result_chars() -> usize {
    MAX_TOOL_RESULT_CHARS
}

fn default_permission_timeout_secs() -> u64 {
    DEFAULT_PERMISSION_TIMEOUT_SECS
}

impl Default for AcpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_port: None, // Auto-assign
            bind_host: "127.0.0.1".to_string(),
            protocol_version: "1.2.0".to_string(),
            dev_mode: false,
            max_tool_loop_iterations: default_max_tool_loop_iterations(),
            require_permission: true,
            permission_timeout_secs: default_permission_timeout_secs(),
            max_history_messages: default_max_history_messages(),
            max_context_tokens: default_max_context_tokens(),
            grok4_max_context_tokens: default_grok4_max_context_tokens(),
            max_tool_result_chars: default_max_tool_result_chars(),
            auto_compress: default_auto_compress(),
            compression_threshold: default_compression_threshold(),
            compression_chunk_ratio: default_compression_chunk_ratio(),
            thinking_mode: ThinkingMode::default(),
            show_context_usage: true,
            stream_thinking: true,
            commit_message_instructions: String::new(),
        }
    }
}
