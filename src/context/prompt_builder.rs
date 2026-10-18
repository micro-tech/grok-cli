//! Prompt builder integration for delta prompting and schema optimization.

use crate::context::prompt_delta::PromptDelta;
use crate::context::prompt_diff::should_use_delta;
use crate::context::tool_optimizer::{compress_schema, prune_unused_tools, schema_hash};
use crate::optimizer::token_cache::TokenCache;

/// Build a (possibly delta) prompt, optionally pruning tools.
pub fn build_prompt_with_delta(
    previous_prompt: Option<&str>,
    current_prompt: &str,
    system_changed: bool,
    tools: Vec<serde_json::Value>,
    allowed_tools: &[&str],
) -> (PromptDelta, Vec<serde_json::Value>) {
    let delta =
        should_use_delta(previous_prompt, current_prompt, system_changed).unwrap_or_else(|_| {
            PromptDelta::Full {
                content: current_prompt.to_string(),
            }
        });

    let pruned_tools = prune_unused_tools(tools, allowed_tools);
    let mut optimized = pruned_tools;
    for t in &mut optimized {
        let _ = compress_schema(t);
    }

    (delta, optimized)
}

/// Returns a cache key based on the prompt *content* hash and schema hashes.
///
/// Task 471.3: the old key used `prompt.len()`, so two different same-length
/// prompts with identical tools collided and could return the wrong cached
/// response.  (Currently only re-exported via the prelude — a landmine, not
/// a live bug — but fixed anyway.)
pub fn prompt_cache_key(prompt: &str, tools: &[serde_json::Value]) -> String {
    let tool_hashes: Vec<_> = tools.iter().map(schema_hash).collect();
    let prompt_hash = schema_hash(&serde_json::Value::String(prompt.to_string()));
    format!("{:x}-{:?}", prompt_hash, tool_hashes)
}

/// Estimates or retrieves cached token count for a prompt.
pub fn estimate_or_cached_tokens(cache: &TokenCache, prompt: &str) -> usize {
    if let Some(tokens) = cache.get_prompt_tokens(prompt) {
        tokens
    } else {
        // Very rough estimate: ~4 chars per token
        let est = (prompt.len() / 4).max(1);
        // In real usage we would call the tokenizer here
        est
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_delta() {
        let (delta, tools) = build_prompt_with_delta(None, "hello", false, vec![], &[]);
        assert!(matches!(delta, PromptDelta::Full { .. }));
        assert!(tools.is_empty());
    }

    #[test]
    fn test_prompt_cache_key_hashes_content_not_length() {
        // Task 471.3: two different same-length prompts must not collide.
        let tools = vec![];
        let k1 = prompt_cache_key("aaaa", &tools);
        let k2 = prompt_cache_key("bbbb", &tools);
        assert_ne!(k1, k2, "same-length different prompts must not collide");
        assert_eq!(k1, prompt_cache_key("aaaa", &tools), "key must be stable");
    }

    #[test]
    fn test_prompt_cache_key() {
        // Key is derived from content hash + tool hashes — never the prompt length.
        let key = prompt_cache_key("test", &[]);
        assert!(key.contains('-'), "key must separate prompt and tool hashes");
        assert!(!key.starts_with("4-"), "key must not leak prompt length");
        assert_eq!(key, prompt_cache_key("test", &[]), "key must be stable");
    }

    #[test]
    fn test_estimate_or_cached_tokens() {
        let mut cache = TokenCache::new();
        let prompt = "system prompt";
        cache.store_prompt_tokens(prompt, 100);

        assert_eq!(estimate_or_cached_tokens(&cache, prompt), 100);
        assert!(estimate_or_cached_tokens(&cache, "new prompt") > 0);
    }
}
