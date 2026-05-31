//! Delta Prompting Engine.
//!
//! Sends only changed messages, updated tool schemas, and new context segments
//! instead of rebuilding full prompts every turn.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptDelta {
    Full { content: String },
    Delta { added: String, removed: Option<String> },
    None,
}

/// Computes a simple diff between previous and current prompt.
pub fn compute_delta(previous: Option<&str>, current: &str) -> PromptDelta {
    match previous {
        Some(prev) if prev == current => PromptDelta::None,
        Some(prev) => {
            // Very naive delta: just send the new part
            let added = if current.len() > prev.len() {
                current[prev.len()..].to_string()
            } else {
                current.to_string()
            };
            PromptDelta::Delta {
                added,
                removed: None,
            }
        }
        None => PromptDelta::Full {
            content: current.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_none() {
        let delta = compute_delta(Some("hello"), "hello");
        assert_eq!(delta, PromptDelta::None);
    }

    #[test]
    fn test_delta_full() {
        let delta = compute_delta(None, "first message");
        assert!(matches!(delta, PromptDelta::Full { .. }));
    }
}
