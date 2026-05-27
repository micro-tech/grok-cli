//! Prompt diff computation for delta prompting.

use crate::context::error::{ContextError, ContextResult};
use crate::context::prompt_delta::PromptDelta;

/// Compute the delta between the previous prompt and the current one.
pub fn compute_delta(previous: &str, current: &str, system_changed: bool) -> ContextResult<PromptDelta> {
    if previous.len() > 1_000_000 || current.len() > 1_000_000 {
        return Err(ContextError::PromptTooLarge);
    }

    if system_changed {
        return Ok(PromptDelta::SystemOrToolsChanged);
    }

    if previous == current {
        return Ok(PromptDelta::Unchanged);
    }

    // Simple heuristic: if only the last user message differs, send just that.
    if current.len() > previous.len() && current.starts_with(previous) {
        let delta_content = current[previous.len()..].trim().to_string();
        if !delta_content.is_empty() {
            return Ok(PromptDelta::UserMessage {
                content: delta_content,
            });
        }
    }

    // Fallback: send full prompt
    Ok(PromptDelta::Full {
        content: current.to_string(),
    })
}

/// High-level helper to decide whether to use delta prompting.
pub fn should_use_delta(previous: Option<&str>, current: &str, system_changed: bool) -> ContextResult<PromptDelta> {
    match previous {
        Some(prev) => compute_delta(prev, current, system_changed),
        None => Ok(PromptDelta::Full {
            content: current.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unchanged() {
        let d = compute_delta("hello", "hello", false).unwrap();
        assert_eq!(d, PromptDelta::Unchanged);
    }

    #[test]
    fn test_system_changed() {
        let d = compute_delta("old", "new", true).unwrap();
        assert_eq!(d, PromptDelta::SystemOrToolsChanged);
    }

    #[test]
    fn test_user_message_delta() {
        let prev = "system\nuser: hi";
        let curr = "system\nuser: hi\nuser: how are you?";
        let d = compute_delta(prev, curr, false).unwrap();
        assert!(matches!(d, PromptDelta::UserMessage { .. }));
    }

    #[test]
    fn test_too_large_prompt() {
        let big = "x".repeat(2_000_000);
        assert!(compute_delta(&big, "small", false).is_err());
    }

    #[test]
    fn test_should_use_delta_none_previous() {
        let d = should_use_delta(None, "first turn", false).unwrap();
        assert!(matches!(d, PromptDelta::Full { .. }));
    }
}
