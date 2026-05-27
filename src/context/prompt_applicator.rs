//! Applies PromptDelta values to construct the final prompt sent to the model.

use crate::context::prompt_delta::PromptDelta;

/// Apply a delta to a base prompt and return the final text.
pub fn apply_delta(base: &str, delta: &PromptDelta) -> String {
    match delta {
        PromptDelta::Unchanged => base.to_string(),
        PromptDelta::UserMessage { content } => {
            format!("{}\n{}", base, content)
        }
        PromptDelta::Full { content } => content.clone(),
        PromptDelta::SystemOrToolsChanged => base.to_string(), // caller should rebuild
        PromptDelta::CompressedContextChanged { summary } => {
            format!("{}\n[compressed] {}", base, summary)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_user_delta() {
        let base = "system prompt";
        let delta = PromptDelta::UserMessage {
            content: "user: hi".into(),
        };
        let result = apply_delta(base, &delta);
        assert!(result.contains("user: hi"));
    }
}
