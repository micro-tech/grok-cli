//! HOH Task Semantic Analyzer (Task 327.19)
//!
//! Keyword + TF-IDF–style semantic understanding of tasks.
//! Finds related tasks and clusters by topic without an embedding model.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticProfile {
    pub task_id: u64,
    /// Top keywords extracted from the task text.
    pub keywords: Vec<String>,
    /// Topic cluster name (derived from dominant keywords).
    pub cluster: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSimilarity {
    pub task_a: u64,
    pub task_b: u64,
    /// Jaccard similarity of keyword sets (0.0–1.0).
    pub score: f32,
}

static STOP_WORDS: &[&str] = &[
    "the","a","an","and","or","in","of","to","for","with","is","are","be",
    "it","this","that","from","by","as","at","on","if","add","hoh","task",
    "implement","create","build","support","use","via","new","will","can",
];

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.len() > 3 && !STOP_WORDS.contains(w))
        .map(String::from)
        .collect()
}

fn task_text(t: &Task) -> String {
    format!("{} {} {} {}", t.title, t.description, t.details, t.test_strategy)
}

/// Build a semantic profile for every task.
pub fn build_profiles(tasks: &[Task]) -> Vec<SemanticProfile> {
    // Compute IDF: how rare is each word?
    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    let n = tasks.len().max(1);
    for t in tasks {
        let words: HashSet<String> = tokenize(&task_text(t)).into_iter().collect();
        for w in words { *doc_freq.entry(w).or_insert(0) += 1; }
    }

    tasks.iter().map(|t| {
        let tokens = tokenize(&task_text(t));
        let mut tf: HashMap<String, usize> = HashMap::new();
        for tok in &tokens { *tf.entry(tok.clone()).or_insert(0) += 1; }

        // Score by TF * (1 / doc_freq) — rare words score higher
        let mut scored: Vec<(String, f32)> = tf.iter().map(|(w, &count)| {
            let idf = (n as f32 / (*doc_freq.get(w).unwrap_or(&1) as f32)).ln().max(0.1);
            (w.clone(), count as f32 * idf)
        }).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let keywords: Vec<String> = scored.into_iter().take(8).map(|(w, _)| w).collect();
        let cluster = keywords.first().cloned().unwrap_or_else(|| "general".to_string());

        SemanticProfile { task_id: t.id, keywords, cluster }
    }).collect()
}

/// Compute pairwise Jaccard similarity for all task pairs above threshold.
pub fn find_similar(tasks: &[Task], threshold: f32) -> Vec<SemanticSimilarity> {
    let profiles = build_profiles(tasks);
    let mut results = Vec::new();

    for i in 0..profiles.len() {
        for j in (i + 1)..profiles.len() {
            let a: HashSet<&str> = profiles[i].keywords.iter().map(|s| s.as_str()).collect();
            let b: HashSet<&str> = profiles[j].keywords.iter().map(|s| s.as_str()).collect();
            let inter = a.intersection(&b).count();
            let union = a.union(&b).count();
            if union == 0 { continue; }
            let score = inter as f32 / union as f32;
            if score >= threshold {
                results.push(SemanticSimilarity {
                    task_a: profiles[i].task_id,
                    task_b: profiles[j].task_id,
                    score,
                });
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, title: &str, desc: &str) -> Task {
        Task { id, title: title.to_string(), description: desc.to_string(), ..Default::default() }
    }

    #[test]
    fn test_profiles_non_empty() {
        let tasks = vec![
            t(1, "Implement caching layer", "Cache repeated expensive computations"),
            t(2, "Add test coverage", "Increase unit test coverage for modules"),
        ];
        let profiles = build_profiles(&tasks);
        assert_eq!(profiles.len(), 2);
        assert!(!profiles[0].keywords.is_empty());
    }

    #[test]
    fn test_similar_tasks_detected() {
        let tasks = vec![
            t(1, "Implement memory cache for LLM responses", "Use in-memory cache"),
            t(2, "Add LLM response caching", "Cache LLM responses in memory"),
            t(3, "Write database schema migration", "SQL migration scripts"),
        ];
        let similar = find_similar(&tasks, 0.2);
        // Tasks 1 and 2 should be more similar to each other than to 3
        assert!(similar.iter().any(|s| {
            (s.task_a == 1 && s.task_b == 2) || (s.task_a == 2 && s.task_b == 1)
        }));
    }
}
