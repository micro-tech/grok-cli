//! Centralized magic numbers and defaults for the entire crate.
//!
//! This module exists to eliminate duplicated literals scattered across
//! config, acp, tools, context, agent, etc. (Task 283).
//!
//! Philosophy:
//! - Use descriptive, UPPER_SNAKE_CASE names.
//! - Group by domain (context, tokens, timeouts, thresholds, etc.).
//! - Provide model-aware helpers where appropriate (grok-4 vs legacy).
//! - Keep the values here as the single source of truth.
//!
//! When adding a new magic number, add it here first, then update call sites.

/// ---------------------------------------------------------------------------
/// Context windows and token budgets (model-aware)
/// ---------------------------------------------------------------------------

/// Full context window for Grok-4 family models.
pub const GROK4_CONTEXT_WINDOW: usize = 1_048_576;

/// Legacy context window for Grok-3 / Grok-2 family (and unknown models).
pub const LEGACY_CONTEXT_WINDOW: usize = 131_072;

/// Soft token budget for legacy models (grok-3, grok-2, etc.).
/// Leaves headroom for response + tool definitions.
pub const LEGACY_CONTEXT_BUDGET: usize = 220_000;

/// Soft token budget for Grok-4.x models.
/// Leaves ~50k headroom for response + tools.
pub const GROK4_CONTEXT_BUDGET: usize = 950_000;

/// ---------------------------------------------------------------------------
/// Default maximum output tokens (per model family)
/// ---------------------------------------------------------------------------

/// Default max output tokens for Grok-4 / Grok-4.x models.
pub const DEFAULT_MAX_OUTPUT_TOKENS_GROK4: u32 = 16_384;

/// Default max output tokens for standard non-mini models (grok-3, grok-2, etc.).
pub const DEFAULT_MAX_OUTPUT_TOKENS_STANDARD: u32 = 8_192;

/// Default max output tokens for "mini" models.
pub const DEFAULT_MAX_OUTPUT_TOKENS_MINI: u32 = 4_096;

/// ---------------------------------------------------------------------------
/// Tool result & history truncation
/// ---------------------------------------------------------------------------

/// Maximum characters to keep in a tool result before truncation.
/// Large file reads are the most common cause of context bloat.
pub const MAX_TOOL_RESULT_CHARS: usize = 30_000;

/// Maximum number of conversation turns kept in session context before trimming.
pub const MAX_HISTORY_MESSAGES: usize = 40;

/// ---------------------------------------------------------------------------
/// Tool output truncation defaults (used by config + file tools)
/// ---------------------------------------------------------------------------

/// Default character threshold before truncating tool output in the UI / logs.
pub const DEFAULT_TRUNCATE_TOOL_OUTPUT_THRESHOLD: u32 = 10_000;

/// Default number of lines to keep when truncating tool output.
pub const DEFAULT_TRUNCATE_TOOL_OUTPUT_LINES: u32 = 100;

/// ---------------------------------------------------------------------------
/// Timeouts (in seconds)
/// ---------------------------------------------------------------------------

/// Default shell command timeout (used by SecurityPolicy and tools).
/// 300s is enough for `cargo build`, `git` operations, etc.
pub const DEFAULT_SHELL_TIMEOUT_SECS: u64 = 300;

/// Default permission prompt timeout (how long to wait for user Allow/Deny).
pub const DEFAULT_PERMISSION_TIMEOUT_SECS: u64 = 300;

/// Default network / HTTP read timeout.
pub const DEFAULT_NETWORK_TIMEOUT_SECS: u64 = 300;

/// Hard cap for the `sleep` tool (prevents accidental very long sleeps).
pub const MAX_SLEEP_SECS: u64 = 300;

/// Default overall request timeout used in several places.
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// ---------------------------------------------------------------------------
/// Compression & context management thresholds
/// ---------------------------------------------------------------------------

/// Fraction of context budget at which auto-compression is triggered.
/// (e.g. 0.75 = fire when we reach 75% of the budget)
pub const COMPRESSION_THRESHOLD: f32 = 0.75;

/// Fraction of current non-system messages to compress in one event.
/// (e.g. 0.40 = compress the oldest 40%)
pub const COMPRESSION_CHUNK_RATIO: f32 = 0.40;

/// ---------------------------------------------------------------------------
/// Tool loop & iteration limits
/// ---------------------------------------------------------------------------

/// Maximum number of tool-loop iterations before the turn is forcibly stopped.
pub const MAX_TOOL_LOOP_ITERATIONS: u32 = 25;

/// ---------------------------------------------------------------------------
/// Model-aware helper functions
/// (These replace scattered if/else logic in context_trim, acp/mod, etc.)
/// ---------------------------------------------------------------------------

/// Returns the full context window for a model.
pub fn get_context_window(model: &str) -> usize {
    let m = model.to_ascii_lowercase();
    if m.starts_with("grok-4") {
        GROK4_CONTEXT_WINDOW
    } else {
        LEGACY_CONTEXT_WINDOW
    }
}

/// Returns the appropriate soft token budget for the model.
pub fn get_context_budget(model: &str) -> usize {
    let m = model.to_ascii_lowercase();
    if m.starts_with("grok-4") {
        GROK4_CONTEXT_BUDGET
    } else {
        LEGACY_CONTEXT_BUDGET
    }
}

/// Returns the default maximum output tokens for a model.
pub fn get_default_max_output_tokens(model: &str) -> u32 {
    let m = model.to_ascii_lowercase();
    if m.starts_with("grok-4") {
        DEFAULT_MAX_OUTPUT_TOKENS_GROK4
    } else if m.contains("mini") {
        DEFAULT_MAX_OUTPUT_TOKENS_MINI
    } else {
        DEFAULT_MAX_OUTPUT_TOKENS_STANDARD
    }
}

/// ---------------------------------------------------------------------------
/// Other common constants (add more groups as needed)
/// ---------------------------------------------------------------------------

/// Default maximum tool loop iterations (exposed for config).
pub const DEFAULT_MAX_TOOL_LOOP_ITERATIONS: u32 = MAX_TOOL_LOOP_ITERATIONS;

/// Default temperature.
pub const DEFAULT_TEMPERATURE: f32 = 0.7;

/// Default max retries for network operations.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_aware_helpers_work() {
        assert_eq!(get_context_window("grok-4"), GROK4_CONTEXT_WINDOW);
        assert_eq!(get_context_window("grok-4.3"), GROK4_CONTEXT_WINDOW);
        assert_eq!(get_context_window("grok-3"), LEGACY_CONTEXT_WINDOW);
        assert_eq!(get_context_window("unknown"), LEGACY_CONTEXT_WINDOW);

        assert_eq!(get_context_budget("grok-4"), GROK4_CONTEXT_BUDGET);
        assert_eq!(get_context_budget("grok-3"), LEGACY_CONTEXT_BUDGET);

        assert_eq!(get_default_max_output_tokens("grok-4.20"), DEFAULT_MAX_OUTPUT_TOKENS_GROK4);
        assert_eq!(get_default_max_output_tokens("grok-4.3"), DEFAULT_MAX_OUTPUT_TOKENS_GROK4);
        assert_eq!(get_default_max_output_tokens("grok-4-latest"), DEFAULT_MAX_OUTPUT_TOKENS_GROK4);
        assert_eq!(get_default_max_output_tokens("grok-3-mini"), DEFAULT_MAX_OUTPUT_TOKENS_MINI);
        assert_eq!(get_default_max_output_tokens("grok-3"), DEFAULT_MAX_OUTPUT_TOKENS_STANDARD);
        assert_eq!(get_default_max_output_tokens("grok-coder"), DEFAULT_MAX_OUTPUT_TOKENS_STANDARD);
        assert_eq!(get_default_max_output_tokens("Grok-Coder-v2"), DEFAULT_MAX_OUTPUT_TOKENS_STANDARD);
        assert_eq!(get_default_max_output_tokens("unknown-model"), DEFAULT_MAX_OUTPUT_TOKENS_STANDARD);
        assert_eq!(get_default_max_output_tokens("grok-2"), DEFAULT_MAX_OUTPUT_TOKENS_STANDARD);
    }
}
