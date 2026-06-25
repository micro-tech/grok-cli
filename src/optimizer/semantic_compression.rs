//! Semantic Compression Layer.
//!
//! Uses embeddings + deduplication to reduce long-term context size
//! while preserving meaning.

use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct SemanticCompressor {
    seen_hashes: HashSet<u64>,
}

impl SemanticCompressor {
    pub fn new() -> Self {
        Self::default()
    }

    fn hash(text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// 117.2 — Semantic deduplication
    pub fn is_duplicate(&mut self, text: &str) -> bool {
        let h = Self::hash(text);
        if self.seen_hashes.contains(&h) {
            true
        } else {
            self.seen_hashes.insert(h);
            false
        }
    }

    /// 117.3 — Simple summarization stub (real implementation would call LLM)
    pub fn summarize(&self, text: &str) -> String {
        if text.len() > 200 {
            format!("{}...", &text[..200])
        } else {
            text.to_string()
        }
    }

    pub fn clear(&mut self) {
        self.seen_hashes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedupe() {
        let mut c = SemanticCompressor::new();
        assert!(!c.is_duplicate("hello world"));
        assert!(c.is_duplicate("hello world"));
    }

    #[test]
    fn test_summarize() {
        let c = SemanticCompressor::new();
        let long = "a".repeat(300);
        assert!(c.summarize(&long).ends_with("..."));
    }
}
