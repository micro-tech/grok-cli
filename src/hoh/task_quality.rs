//! HOH Task Quality Scoring (Task 327.28)

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub task_id: u64,
    /// 0.0–1.0
    pub score: f32,
    pub clarity: f32,
    pub actionability: f32,
    pub testability: f32,
    pub completeness: f32,
    pub suggestions: Vec<String>,
}

pub fn score_quality(task: &Task) -> QualityScore {
    let mut suggestions = Vec::new();

    // Clarity: title + description length and specificity
    let title_words = task.title.split_whitespace().count();
    let desc_words  = task.description.split_whitespace().count();
    let clarity = {
        let t = (title_words.min(12) as f32 / 12.0) * 0.5
              + (desc_words.min(30) as f32 / 30.0).min(1.0) * 0.5;
        if title_words < 3 { suggestions.push("Title is too short — be more descriptive".to_string()); }
        if desc_words < 5  { suggestions.push("Add a more detailed description".to_string()); }
        t
    };

    // Actionability: details field richness
    let detail_words = task.details.split_whitespace().count();
    let actionability = (detail_words.min(100) as f32 / 100.0).min(1.0);
    if detail_words < 20 {
        suggestions.push("Expand the details field with implementation steps".to_string());
    }

    // Testability: test strategy presence and length
    let ts_words = task.test_strategy.split_whitespace().count();
    let testability = (ts_words.min(30) as f32 / 30.0).min(1.0);
    if ts_words < 5 {
        suggestions.push("Define a concrete testStrategy with specific success criteria".to_string());
    }

    // Completeness: required fields present
    let has_priority = !task.priority.is_empty() && task.priority != "medium";
    let completeness = {
        let mut c = 0.0f32;
        if !task.title.is_empty()       { c += 0.25; }
        if !task.description.is_empty() { c += 0.25; }
        if !task.details.is_empty()     { c += 0.25; }
        if !task.test_strategy.is_empty() { c += 0.25; }
        if !has_priority { suggestions.push("Set an explicit priority level".to_string()); }
        c
    };

    let score = clarity * 0.25 + actionability * 0.3 + testability * 0.25 + completeness * 0.2;

    QualityScore { task_id: task.id, score, clarity, actionability, testability, completeness, suggestions }
}

pub fn score_all(tasks: &[Task]) -> Vec<QualityScore> {
    tasks.iter().map(score_quality).collect()
}

/// Return tasks with quality score below threshold, sorted worst first.
pub fn low_quality_tasks(tasks: &[Task], threshold: f32) -> Vec<QualityScore> {
    let mut scores: Vec<QualityScore> = tasks.iter()
        .map(score_quality)
        .filter(|s| s.score < threshold)
        .collect();
    scores.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_task_low_score() {
        let t = Task { id: 1, title: "Do".to_string(), ..Default::default() };
        let s = score_quality(&t);
        assert!(s.score < 0.5);
        assert!(!s.suggestions.is_empty());
    }

    #[test]
    fn test_complete_task_high_score() {
        let t = Task {
            id: 2,
            title: "Implement async caching layer for LLM responses".to_string(),
            description: "Add a performant in-memory cache with TTL expiry to reduce redundant LLM calls".to_string(),
            details: "Create src/cache/mod.rs with CacheEntry struct. Implement get/set/expire methods. Wire into send_to_grok. Add LRU eviction. Handle concurrent access with tokio Mutex.".to_string(),
            test_strategy: "cargo test cache:: passes. Verify cache hit rate > 80% in integration test. Check concurrent access is safe.".to_string(),
            priority: "high".to_string(),
            ..Default::default()
        };
        let s = score_quality(&t);
        assert!(s.score > 0.5, "expected > 0.5, got {}", s.score);
    }
}
