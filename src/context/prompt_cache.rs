//! Simple in-memory prompt cache keyed by content + tool schema hashes.

use std::collections::HashMap;

use crate::context::prompt_builder::prompt_cache_key;

pub struct PromptCache {
    store: HashMap<String, String>,
    max_entries: usize,
}

impl PromptCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            store: HashMap::new(),
            max_entries,
        }
    }

    pub fn get(&self, prompt: &str, tools: &[serde_json::Value]) -> Option<&String> {
        let key = prompt_cache_key(prompt, tools);
        self.store.get(&key)
    }

    pub fn insert(&mut self, prompt: &str, tools: &[serde_json::Value], response: String) {
        if self.store.len() >= self.max_entries {
            // Simple eviction: clear oldest (in real impl use LRU)
            self.store.clear();
        }
        let key = prompt_cache_key(prompt, tools);
        self.store.insert(key, response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_miss_then_hit() {
        let mut cache = PromptCache::new(10);
        let tools: Vec<serde_json::Value> = vec![];
        assert!(cache.get("hello", &tools).is_none());

        cache.insert("hello", &tools, "world".into());
        assert_eq!(cache.get("hello", &tools).unwrap(), "world");
    }
}
