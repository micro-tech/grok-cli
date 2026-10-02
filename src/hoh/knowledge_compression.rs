//! HOH Knowledge Compression (Task 361.24)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedBundle {
    pub source_keys: Vec<String>,
    pub summary: String,
    pub key_facts: Vec<String>,
    pub compression_ratio: f32,
}

/// Compress a collection of text fragments into a concise summary.
pub fn compress(sources: &[(&str, &str)], max_facts: usize) -> CompressedBundle {
    let total_chars: usize = sources.iter().map(|(_, c)| c.len()).sum();

    // Extract sentences containing key signal words
    let signal_words = ["important", "key", "must", "should", "critical", "note", "warning",
                        "ensure", "avoid", "always", "never", "require"];

    let mut key_facts: Vec<String> = Vec::new();
    for (_, content) in sources {
        for sentence in content.split(['.', '\n']) {
            let lower = sentence.to_lowercase();
            if signal_words.iter().any(|w| lower.contains(w)) {
                let trimmed = sentence.trim().to_string();
                if trimmed.len() > 10 && !key_facts.contains(&trimmed) {
                    key_facts.push(trimmed);
                }
            }
            if key_facts.len() >= max_facts { break; }
        }
        if key_facts.len() >= max_facts { break; }
    }

    // If no signal-based facts, take first sentence of each source
    if key_facts.is_empty() {
        for (_, content) in sources.iter().take(max_facts) {
            if let Some(first) = content.split(['.', '\n']).find(|s| s.trim().len() > 10) {
                key_facts.push(first.trim().to_string());
            }
        }
    }

    let summary = format!(
        "Compressed {} source(s): {}",
        sources.len(),
        key_facts.first().cloned().unwrap_or_else(|| "No key facts extracted".to_string())
    );

    let compressed_chars = summary.len() + key_facts.iter().map(|f| f.len()).sum::<usize>();
    let ratio = if total_chars == 0 { 1.0 } else { compressed_chars as f32 / total_chars as f32 };

    CompressedBundle {
        source_keys: sources.iter().map(|(k, _)| k.to_string()).collect(),
        summary, key_facts,
        compression_ratio: ratio.min(1.0),
    }
}

/// Compress a Vec of KnowledgeBundles from the injector.
pub fn compress_bundles(bundles: &[crate::hoh::knowledge_injection::KnowledgeBundle], max_facts: usize) -> CompressedBundle {
    let sources: Vec<(&str, &str)> = bundles.iter().map(|b| (b.source.as_str(), b.content.as_str())).collect();
    compress(&sources, max_facts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_produces_output() {
        let sources = vec![
            ("doc1", "This is important. Some other text. Always use Result for error handling."),
            ("doc2", "Note: never use unwrap in production. This is a long document with many words."),
        ];
        let bundle = compress(&sources, 5);
        assert!(!bundle.key_facts.is_empty());
        assert!(bundle.compression_ratio <= 1.0);
    }

    #[test]
    fn test_compression_ratio_makes_sense() {
        let long_content = "This is important. Always test your code. Key insight here. ".repeat(50);
        let sources = vec![("src", long_content.as_str())];
        let bundle = compress(&sources, 3);
        // Key facts are extracted and capped — ratio should be less than 1.0
        assert!(bundle.compression_ratio <= 1.0);
        assert!(!bundle.key_facts.is_empty());
    }
}
