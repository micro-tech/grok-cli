//! Token Cache — stores hashes and tokenized segments to avoid repeated work.
//!
//! Caches:
//! - System prompt hashes
//! - Tool schema fingerprints
//! - Compressed context
//! - Tokenized segments

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Debug, Default)]
pub struct TokenCache {
    /// Maps content hash → token count
    prompt_tokens: HashMap<u64, usize>,
    /// Maps schema fingerprint → token count
    schema_tokens: HashMap<u64, usize>,
    /// Maps context hash → compressed token count
    context_tokens: HashMap<u64, usize>,
}

impl TokenCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn hash(content: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns cached token count if available, otherwise None.
    pub fn get_prompt_tokens(&self, content: &str) -> Option<usize> {
        self.prompt_tokens.get(&Self::hash(content)).copied()
    }

    pub fn store_prompt_tokens(&mut self, content: &str, tokens: usize) {
        self.prompt_tokens.insert(Self::hash(content), tokens);
    }

    pub fn get_schema_tokens(&self, schema_json: &str) -> Option<usize> {
        self.schema_tokens.get(&Self::hash(schema_json)).copied()
    }

    pub fn store_schema_tokens(&mut self, schema_json: &str, tokens: usize) {
        self.schema_tokens.insert(Self::hash(schema_json), tokens);
    }

    pub fn get_context_tokens(&self, context: &str) -> Option<usize> {
        self.context_tokens.get(&Self::hash(context)).copied()
    }

    pub fn store_context_tokens(&mut self, context: &str, tokens: usize) {
        self.context_tokens.insert(Self::hash(context), tokens);
    }

    // === Subtask implementations ===

    /// 118.1 — Cache system prompt tokens
    pub fn cache_system_prompt(&mut self, prompt: &str, tokens: usize) {
        self.store_prompt_tokens(prompt, tokens);
    }

    /// 118.2 — Cache tool schema tokens
    pub fn cache_tool_schema(&mut self, schema_json: &str, tokens: usize) {
        self.store_schema_tokens(schema_json, tokens);
    }

    /// 118.3 — Cache compressed context tokens
    pub fn cache_compressed_context(&mut self, context: &str, tokens: usize) {
        self.store_context_tokens(context, tokens);
    }

    pub fn clear(&mut self) {
        self.prompt_tokens.clear();
        self.schema_tokens.clear();
        self.context_tokens.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::TokenCache;

    #[test]
    fn test_prompt_token_caching() {
        let mut cache = TokenCache::new();
        let content = "system prompt here";

        assert!(cache.get_prompt_tokens(content).is_none());
        cache.store_prompt_tokens(content, 42);
        assert_eq!(cache.get_prompt_tokens(content), Some(42));
    }

    #[test]
    fn test_token_level_caching_subtasks() {
        let mut cache = TokenCache::new();

        // 118.1
        cache.cache_system_prompt("sys", 50);
        assert_eq!(cache.get_prompt_tokens("sys"), Some(50));

        // 118.2
        cache.cache_tool_schema(r#"{"name":"read"}"#, 30);
        assert_eq!(cache.get_schema_tokens(r#"{"name":"read"}"#), Some(30));

        // 118.3
        cache.cache_compressed_context("compressed ctx", 120);
        assert_eq!(cache.get_context_tokens("compressed ctx"), Some(120));
    }
}
