//! HOH Task Reward Model (Task 327.22)
//!
//! Defines what "good" task completion looks like and scores completed tasks.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSignal {
    pub task_id: u64,
    /// 0.0–1.0 composite reward.
    pub reward: f32,
    /// Breakdown of reward components.
    pub quality_score: f32,
    pub speed_score: f32,
    pub learning_score: f32,
    pub alignment_score: f32,
    pub rationale: String,
}

/// Score a completed task.
///
/// - `completion_days`: how many days the task took (lower is better for speed)
/// - `test_passed`: whether tests passed after completion
/// - `helix_score`: external quality signal (0.0–1.0)
/// - `goal_keywords`: words from the current HOH goals to check alignment
pub fn compute_reward(
    task: &Task,
    completion_days: f32,
    test_passed: bool,
    helix_score: Option<f32>,
    goal_keywords: &[String],
) -> RewardSignal {
    // Quality: test pass + helix
    let quality_score = if test_passed { 0.5 } else { 0.1 }
        + helix_score.unwrap_or(0.5) * 0.5;

    // Speed: normalize to 0–1 where 1 day = 1.0, 30 days = 0.0
    let speed_score = (1.0 - (completion_days / 30.0).min(1.0)).max(0.0);

    // Learning: tasks with test strategy and details produce more learning
    let learning_score = {
        let has_test = !task.test_strategy.is_empty();
        let has_details = task.details.split_whitespace().count() > 10;
        (if has_test { 0.5 } else { 0.0 }) + (if has_details { 0.5 } else { 0.0 })
    };

    // Alignment: how many goal keywords appear in the task
    let text = format!("{} {}", task.title, task.description).to_lowercase();
    let aligned = goal_keywords.iter()
        .filter(|kw| text.contains(kw.to_lowercase().as_str()))
        .count();
    let alignment_score = if goal_keywords.is_empty() {
        0.5
    } else {
        (aligned as f32 / goal_keywords.len() as f32).min(1.0)
    };

    let reward = quality_score * 0.4
        + speed_score * 0.2
        + learning_score * 0.2
        + alignment_score * 0.2;

    RewardSignal {
        task_id: task.id,
        reward,
        quality_score,
        speed_score,
        learning_score,
        alignment_score,
        rationale: format!(
            "quality={:.2} speed={:.2} learning={:.2} alignment={:.2}",
            quality_score, speed_score, learning_score, alignment_score
        ),
    }
}

/// Rank a batch of completed tasks by reward.
pub fn rank_by_reward(signals: &mut Vec<RewardSignal>) {
    signals.sort_by(|a, b| b.reward.partial_cmp(&a.reward).unwrap_or(std::cmp::Ordering::Equal));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> Task {
        Task {
            id: 1,
            title: "Implement caching".to_string(),
            test_strategy: "Run cache tests".to_string(),
            details: "Create a cache module with expiry".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_fast_passing_task_has_high_reward() {
        let s = compute_reward(&task(), 1.0, true, Some(0.9), &[]);
        assert!(s.reward > 0.5);
    }

    #[test]
    fn test_slow_failing_task_has_low_reward() {
        let s = compute_reward(&task(), 28.0, false, Some(0.2), &[]);
        assert!(s.reward < 0.4);
    }

    #[test]
    fn test_rank_sorts_descending() {
        let mut signals = vec![
            RewardSignal { task_id: 1, reward: 0.3, quality_score: 0.0,
                speed_score: 0.0, learning_score: 0.0, alignment_score: 0.0, rationale: String::new() },
            RewardSignal { task_id: 2, reward: 0.8, quality_score: 0.0,
                speed_score: 0.0, learning_score: 0.0, alignment_score: 0.0, rationale: String::new() },
        ];
        rank_by_reward(&mut signals);
        assert_eq!(signals[0].task_id, 2);
    }
}
