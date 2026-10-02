//! HOH Knowledge Rewriting (Task 361.26)

use crate::hoh::knowledge_injection::KnowledgeBundle;
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

/// Rewrite a knowledge bundle to be more concise, clear, or adapted to a new context.
pub fn rewrite_for_context(bundle: &KnowledgeBundle, context: &str) -> KnowledgeBundle {
    let content = &bundle.content;

    // Remove sentences irrelevant to context
    let context_words: Vec<&str> = context.split_whitespace().collect();
    let filtered: Vec<&str> = content.split('.')
        .filter(|sentence| {
            let lower = sentence.to_lowercase();
            context_words.iter().any(|w| lower.contains(&w.to_lowercase()))
                || sentence.len() < 80 // keep short sentences
        })
        .collect();

    let rewritten = if filtered.is_empty() {
        format!("[Adapted for {}] {}", context, content.chars().take(200).collect::<String>())
    } else {
        format!("[{}] {}", context, filtered.join(". ").trim().to_string())
    };

    KnowledgeBundle {
        source: format!("rewritten:{}:{}", context.split_whitespace().next().unwrap_or("ctx"), bundle.source),
        content: rewritten,
        relevance_score: (bundle.relevance_score * 0.95).min(1.0),
        timestamp: now(),
    }
}

/// Simplify technical jargon in a bundle.
pub fn simplify(bundle: &KnowledgeBundle) -> KnowledgeBundle {
    let simplified = bundle.content
        .replace("asynchronous", "async")
        .replace("implementation", "impl")
        .replace("functionality", "feature")
        .replace("utilize", "use")
        .replace("initialization", "init")
        .replace("parameterization", "config");

    KnowledgeBundle {
        source: format!("simplified:{}", bundle.source),
        content: simplified,
        relevance_score: bundle.relevance_score,
        timestamp: now(),
    }
}

/// Rewrite a set of bundles and return the rewritten versions.
pub fn rewrite_all(bundles: &[KnowledgeBundle], context: &str) -> Vec<KnowledgeBundle> {
    bundles.iter().map(|b| rewrite_for_context(b, context)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(content: &str) -> KnowledgeBundle {
        KnowledgeBundle { source: "test".to_string(), content: content.to_string(), relevance_score: 0.8, timestamp: 0 }
    }

    #[test]
    fn test_rewrite_produces_different_content() {
        let original = b("This is about caching. Security is important. Performance matters.");
        let rewritten = rewrite_for_context(&original, "caching performance");
        assert_ne!(original.content, rewritten.content);
        assert!(rewritten.source.contains("rewritten"));
    }

    #[test]
    fn test_simplify_replaces_jargon() {
        let bundle = b("The initialization of asynchronous functionality utilizes parameterization.");
        let simplified = simplify(&bundle);
        assert!(simplified.content.contains("async"));
        assert!(!simplified.content.contains("asynchronous"));
    }
}
