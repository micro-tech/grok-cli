//! HOH Knowledge Expansion (Task 361.25)

use crate::hoh::knowledge_injection::KnowledgeBundle;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandedKnowledge {
    pub original_key: String,
    pub new_bundles: Vec<KnowledgeBundle>,
    pub expansion_type: ExpansionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpansionType { RelatedTopics, Examples, Counterexamples, DeepDive }

/// Expand a knowledge fragment by generating related knowledge.
pub fn expand(bundle: &KnowledgeBundle, expansion_type: ExpansionType) -> ExpandedKnowledge {
    let words: Vec<&str> = bundle.content.split_whitespace().take(5).collect();
    let topic = words.join(" ");

    let new_bundles = match expansion_type {
        ExpansionType::RelatedTopics => {
            let related = derive_related_topics(&bundle.content);
            related.into_iter().map(|topic| KnowledgeBundle {
                source: format!("expanded:related:{}", bundle.source),
                content: format!("Related topic to '{}': {}", bundle.source, topic),
                relevance_score: bundle.relevance_score * 0.8,
                timestamp: now(),
            }).collect()
        }
        ExpansionType::Examples => vec![KnowledgeBundle {
            source: format!("expanded:examples:{}", bundle.source),
            content: format!("Example application of '{}': use in context of autonomous HOH iteration planning", topic),
            relevance_score: bundle.relevance_score * 0.9,
            timestamp: now(),
        }],
        ExpansionType::Counterexamples => vec![KnowledgeBundle {
            source: format!("expanded:counter:{}", bundle.source),
            content: format!("Counterexample for '{}': do not apply when resources are constrained or risks are high", topic),
            relevance_score: bundle.relevance_score * 0.7,
            timestamp: now(),
        }],
        ExpansionType::DeepDive => vec![KnowledgeBundle {
            source: format!("expanded:deepdive:{}", bundle.source),
            content: format!("Deep dive on '{}': {}\n\nFurther considerations: test thoroughly, monitor performance, iterate based on feedback.", topic, bundle.content),
            relevance_score: bundle.relevance_score,
            timestamp: now(),
        }],
    };

    ExpandedKnowledge {
        original_key: bundle.source.clone(),
        new_bundles,
        expansion_type,
    }
}

fn derive_related_topics(content: &str) -> Vec<String> {
    let topic_map: &[(&str, &[&str])] = &[
        ("cache", &["eviction policy", "TTL", "LRU", "memory pressure"]),
        ("test", &["coverage", "mocking", "integration testing", "property-based testing"]),
        ("async", &["tokio runtime", "backpressure", "cancellation", "timeout"]),
        ("agent", &["specialization", "collaboration", "skill evolution", "role assignment"]),
        ("hoh", &["outer loop", "iteration state", "planning phase", "evaluation"]),
    ];

    let lower = content.to_lowercase();
    let mut related = Vec::new();
    for (key, topics) in topic_map {
        if lower.contains(key) {
            related.extend(topics.iter().map(|t| t.to_string()));
        }
    }
    related.truncate(3);
    if related.is_empty() { related.push("best practices for the topic".to_string()); }
    related
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(source: &str, content: &str) -> KnowledgeBundle {
        KnowledgeBundle { source: source.to_string(), content: content.to_string(), relevance_score: 0.8, timestamp: 0 }
    }

    #[test]
    fn test_expand_related_topics() {
        let b = bundle("doc", "async Rust with tokio runtime for concurrent operations");
        let expanded = expand(&b, ExpansionType::RelatedTopics);
        assert!(!expanded.new_bundles.is_empty());
    }

    #[test]
    fn test_expand_deep_dive() {
        let b = bundle("okf", "caching strategy for LLM responses");
        let expanded = expand(&b, ExpansionType::DeepDive);
        assert!(expanded.new_bundles[0].content.len() > b.content.len());
    }
}
