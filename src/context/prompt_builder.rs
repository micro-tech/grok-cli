//! Prompt builder integration for delta prompting and schema optimization.

use crate::context::prompt_delta::PromptDelta;
use crate::context::prompt_diff::should_use_delta;
use crate::context::tool_optimizer::{compress_schema, prune_unused_tools, schema_hash};

/// Build a (possibly delta) prompt, optionally pruning tools.
pub fn build_prompt_with_delta(
    previous_prompt: Option<&str>,
    current_prompt: &str,
    system_changed: bool,
    tools: Vec<serde_json::Value>,
    allowed_tools: &[&str],
) -> (PromptDelta, Vec<serde_json::Value>) {
    let delta = should_use_delta(previous_prompt, current_prompt, system_changed)
        .unwrap_or_else(|_| PromptDelta::Full { content: current_prompt.to_string() });

    let pruned_tools = prune_unused_tools(tools, allowed_tools);
    let mut optimized = pruned_tools;
    for t in &mut optimized {
        let _ = compress_schema(t);
    }

    (delta, optimized)
}

/// Returns a cache key based on schema hashes.
pub fn prompt_cache_key(prompt: &str, tools: &[serde_json::Value]) -> String {
    let tool_hashes: Vec<_> = tools.iter().map(schema_hash).collect();
    format!("{}-{:?}", prompt.len(), tool_hashes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_delta() {
        let (delta, tools) = build_prompt_with_delta(
            None,
            "hello",
            false,
            vec![],
            &[],
        );
        assert!(matches!(delta, PromptDelta::Full { .. }));
        assert!(tools.is_empty());
    }

    #[test]
    fn test_prompt_cache_key() {
        let key = prompt_cache_key("test", &[]);
        assert!(key.contains("4-"));
    }
}
