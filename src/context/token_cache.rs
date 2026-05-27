//! Token caching for prompt components to reduce repeated token usage.
//!
//! This module provides stable hashing for system prompts, tool schemas,
//! and compressed context so unchanged fragments can be reused across turns.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Cache key and metadata for prompt token optimization.
#[derive(Debug, Clone, Default)]
pub struct TokenCache {
    /// Hash of the current system prompt.
    system_prompt_hash: Option<u64>,
    /// Fingerprint of the tool schema (name + parameters shape).
    tool_schema_fingerprint: Option<u64>,
    /// Hash of the compressed context layer summary.
    compressed_context_hash: Option<u64>,
}

impl TokenCache {
    /// Create a new empty token cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute a stable hash for a system prompt string.
    pub fn hash_system_prompt(&mut self, prompt: &str) -> u64 {
        let hash = Self::stable_hash(prompt);
        self.system_prompt_hash = Some(hash);
        hash
    }

    /// Compute a fingerprint for a tool schema (name + JSON schema shape).
    pub fn fingerprint_tool_schema(&mut self, tool_name: &str, schema: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        schema.hash(&mut hasher);
        let fp = hasher.finish();
        self.tool_schema_fingerprint = Some(fp);
        fp
    }

    /// Compute a hash for compressed context.
    pub fn hash_compressed_context(&mut self, context: &str) -> u64 {
        let hash = Self::stable_hash(context);
        self.compressed_context_hash = Some(hash);
        hash
    }

    /// Returns true if the system prompt has changed since last hash.
    pub fn system_prompt_changed(&self, prompt: &str) -> bool {
        match self.system_prompt_hash {
            Some(h) => h != Self::stable_hash(prompt),
            None => true,
        }
    }

    /// Returns true if we have a cached system prompt hash.
    pub fn has_system_prompt(&self) -> bool {
        self.system_prompt_hash.is_some()
    }

    /// Returns true if we have a cached tool schema fingerprint.
    pub fn has_tool_schema(&self) -> bool {
        self.tool_schema_fingerprint.is_some()
    }

    /// Returns true if we have a cached compressed context hash.
    pub fn has_compressed_context(&self) -> bool {
        self.compressed_context_hash.is_some()
    }

    fn stable_hash(input: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_hash_stability() {
        let mut cache = TokenCache::new();
        let h1 = cache.hash_system_prompt("You are a helpful assistant.");
        let h2 = cache.hash_system_prompt("You are a helpful assistant.");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_tool_schema_fingerprint() {
        let mut cache = TokenCache::new();
        let fp = cache.fingerprint_tool_schema("read_file", r#"{"type":"object"}"#);
        assert!(fp > 0);
    }

    #[test]
    fn test_system_prompt_changed_detection() {
        let mut cache = TokenCache::new();
        cache.hash_system_prompt("old prompt");
        assert!(cache.system_prompt_changed("new prompt"));
        assert!(!cache.system_prompt_changed("old prompt"));
    }
}
