//! HOH Task Difficulty Estimator (Task 327.16)

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DifficultyLevel { Easy, Medium, Hard, VeryHard }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyEstimate {
    pub task_id: u64,
    pub level: DifficultyLevel,
    /// 0.0 (trivial) to 1.0 (extremely hard).
    pub score: f32,
    pub rationale: String,
}

/// Estimate difficulty from static signals: description length, subtask count,
/// dependency count, details length, and known complexity keywords.
pub fn estimate_difficulty(task: &Task) -> DifficultyEstimate {
    let mut score: f32 = 0.0;

    // Subtask count: more subtasks = harder
    score += (task.subtasks.len() as f32 * 0.05).min(0.3);

    // Dependency count
    score += (task.dependencies.len() as f32 * 0.04).min(0.2);

    // Details length: longer details = more complex
    let detail_words = task.details.split_whitespace().count();
    score += (detail_words as f32 / 200.0).min(0.25);

    // Complexity keywords
    let text = format!("{} {} {}", task.title, task.description, task.details).to_lowercase();
    for kw in &["async", "unsafe", "concurren", "distributed", "security", "crypto",
                "migration", "refactor", "architecture", "redesign", "integrate"] {
        if text.contains(kw) { score += 0.04; }
    }
    score = score.clamp(0.0, 1.0);

    let level = match score {
        s if s < 0.25 => DifficultyLevel::Easy,
        s if s < 0.5  => DifficultyLevel::Medium,
        s if s < 0.75 => DifficultyLevel::Hard,
        _             => DifficultyLevel::VeryHard,
    };

    DifficultyEstimate {
        task_id: task.id,
        level,
        score,
        rationale: format!(
            "subtasks={}, deps={}, detail_words={}, score={:.2}",
            task.subtasks.len(), task.dependencies.len(), detail_words, score
        ),
    }
}

pub fn estimate_all(tasks: &[Task]) -> Vec<DifficultyEstimate> {
    tasks.iter().map(estimate_difficulty).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_task_is_easy() {
        let t = Task { id: 1, title: "Do X".into(), ..Default::default() };
        let e = estimate_difficulty(&t);
        assert_eq!(e.level, DifficultyLevel::Easy);
    }

    #[test]
    fn test_complex_task_harder() {
        let mut t = Task { id: 2, title: "Redesign async distributed architecture".into(), ..Default::default() };
        t.details = "Very long details ".repeat(20);
        t.dependencies = vec![1, 2, 3, 4];
        let e = estimate_difficulty(&t);
        assert!(e.score > 0.3);
    }
}
