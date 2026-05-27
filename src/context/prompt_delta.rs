//! Delta prompting support.
//!
//! Allows sending only the changed parts of the prompt between turns,
//! significantly reducing token usage when most of the context is stable.

use serde::{Deserialize, Serialize};

/// Represents a delta (change) in the prompt between turns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptDelta {
    /// No changes — the prompt is identical to the previous turn.
    Unchanged,
    /// Only the user message changed (most common case).
    UserMessage { content: String },
    /// System prompt or tool schema changed.
    SystemOrToolsChanged,
    /// Full prompt must be sent (first turn, or after major context reset).
    Full { content: String },
    /// Compressed context layer changed.
    CompressedContextChanged { summary: String },
}

impl Default for PromptDelta {
    fn default() -> Self {
        Self::Full {
            content: String::new(),
        }
    }
}

impl PromptDelta {
    /// Returns true if this delta requires sending the full prompt.
    pub fn requires_full_prompt(&self) -> bool {
        matches!(self, PromptDelta::Full { .. } | PromptDelta::SystemOrToolsChanged)
    }

    /// Returns a short description of the delta for logging/debugging.
    pub fn description(&self) -> &'static str {
        match self {
            PromptDelta::Unchanged => "unchanged",
            PromptDelta::UserMessage { .. } => "user_message",
            PromptDelta::SystemOrToolsChanged => "system_or_tools",
            PromptDelta::Full { .. } => "full",
            PromptDelta::CompressedContextChanged { .. } => "compressed_context",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_unchanged() {
        let d = PromptDelta::Unchanged;
        assert!(!d.requires_full_prompt());
        assert_eq!(d.description(), "unchanged");
    }

    #[test]
    fn test_delta_full_requires_prompt() {
        let d = PromptDelta::Full {
            content: "test".into(),
        };
        assert!(d.requires_full_prompt());
    }
}
