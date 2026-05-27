//! Prompt diff computation for delta prompting.

use crate::context::prompt_delta::PromptDelta;

/// Compute the delta between the previous prompt and the current one.
pub fn compute_delta(previous: &str, current: &str, system_changed: bool) -> PromptDelta {
    if system_changed {
        return PromptDelta::SystemOrToolsChanged;
    }

    if previous == current {
        return PromptDelta::Unchanged;
    }

    // Simple heuristic: if only the last user message differs, send just that.
    if current.len() > previous.len() && current.starts_with(previous) {
        let delta_content = current[previous.len()..].trim().to_string();
        if !delta_content.is_empty() {
            return PromptDelta::UserMessage {
                content: delta_content,
            };
        }
    }

    // Fallback: send full prompt
    PromptDelta::Full {
        content: current.to_string(),
    }
}

/// High-level helper to decide whether to use delta prompting.
pub fn should_use_delta(previous: Option<&str>, current: &str, system_changed: bool) -> PromptDelta {
    match previous {
        Some(prev) => compute_delta(prev, current, system_changed),
        None => PromptDelta::Full {
            content: current.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unchanged() {
        let d = compute_delta("hello", "hello", false);
        assert_eq!(d, PromptDelta::Unchanged);
    }

    #[test]
    fn test_system_changed() {
        let d = compute_delta("old", "new", true);
        assert_eq!(d, PromptDelta::SystemOrToolsChanged);
    }

    #[test]
    fn test_user_message_delta() {
        let prev = "system\nuser: hi";
        let curr = "system\nuser: hi\nuser: how are you?";
        let d = compute_delta(prev, curr, false);
        assert!(matches!(d, PromptDelta::UserMessage { .. }));
    }
}
