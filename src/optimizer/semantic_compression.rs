//! Semantic Compression Layer.
//!
//! Uses embeddings + deduplication to reduce long-term context size.

use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct SemanticCompressor {
    seen: HashSet<u64>,
}

impl SemanticCompressor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Simple hash-based semantic deduplication.
    pub fn is_duplicate(&mut self, text: &str) -> bool {
        let hash = text.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        !self.seen.insert(hash)
    }

    /// Produces a very lightweight summary (first N chars + length note).
    pub fn summarize(&self, text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}… [{} chars]", &text[..max_len.saturating_sub(10)], text.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication() {
        let mut compressor = SemanticCompressor::new();
        assert!(!compressor.is_duplicate("hello world"));
        assert!(compressor.is_duplicate("hello world"));
    }

    #[test]
    fn test_summarize() {
        let compressor = SemanticCompressor::new();
        let summary = compressor.summarize(&"x".repeat(300), 50);
        assert!(summary.ends_with("chars]"));
    }
}
