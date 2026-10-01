//! HOH Task Completion Tracker (Task 327.6)
//!
//! Tracks completion metrics, time-to-complete, and quality signals for tasks.
//! Feeds data back into prioritization and evolution.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskCompletionRecord {
    pub task_id: u64,
    pub title: String,
    pub completed_at: u64,
    pub started_at: Option<u64>,
    pub duration_secs: Option<u64>,
    pub quality_score: Option<f32>,      // 0.0 - 1.0
    pub test_pass_rate: Option<f32>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionStats {
    pub total_completed: usize,
    pub avg_duration_secs: Option<f64>,
    pub high_quality_count: usize,
    pub completion_rate_by_priority: HashMap<String, f32>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskCompletionTracker {
    records: Vec<TaskCompletionRecord>,
    /// In-memory map of when tasks entered "in_progress"
    start_times: HashMap<u64, u64>,
}

impl TaskCompletionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a task has started being worked on.
    pub fn mark_started(&mut self, task_id: u64) {
        let now = current_timestamp();
        self.start_times.insert(task_id, now);
    }

    /// Record a task as completed with optional quality signals.
    pub fn record_completion(
        &mut self,
        task: &Task,
        quality_score: Option<f32>,
        test_pass_rate: Option<f32>,
        notes: impl Into<String>,
    ) {
        let now = current_timestamp();
        let started_at = self.start_times.remove(&task.id);

        let duration_secs = started_at.map(|start| now.saturating_sub(start));

        let record = TaskCompletionRecord {
            task_id: task.id,
            title: task.title.clone(),
            completed_at: now,
            started_at,
            duration_secs,
            quality_score,
            test_pass_rate,
            notes: notes.into(),
        };

        self.records.push(record);
    }

    /// Get all completion records.
    pub fn get_records(&self) -> &[TaskCompletionRecord] {
        &self.records
    }

    /// Compute aggregate statistics.
    pub fn get_stats(&self) -> CompletionStats {
        let total = self.records.len();
        if total == 0 {
            return CompletionStats::default();
        }

        let mut total_duration: u64 = 0;
        let mut duration_count = 0;
        let mut high_quality = 0;

        for r in &self.records {
            if let Some(d) = r.duration_secs {
                total_duration += d;
                duration_count += 1;
            }
            if r.quality_score.unwrap_or(0.0) >= 0.8 {
                high_quality += 1;
            }
        }

        let avg_duration = if duration_count > 0 {
            Some(total_duration as f64 / duration_count as f64)
        } else {
            None
        };

        // Simple priority breakdown (placeholder - would be richer with more data)
        // let _by_prio: HashMap<String, (usize, usize)> = HashMap::new();
        // For now we just count total per priority from records (no original priority stored)
        // In real use we'd attach priority at completion time.

        CompletionStats {
            total_completed: total,
            avg_duration_secs: avg_duration,
            high_quality_count: high_quality,
            completion_rate_by_priority: HashMap::new(), // TODO: populate with real data
        }
    }

    /// Get the N most recent completions.
    pub fn recent_completions(&self, n: usize) -> Vec<&TaskCompletionRecord> {
        let mut recs: Vec<_> = self.records.iter().collect();
        recs.sort_by_key(|r| std::cmp::Reverse(r.completed_at));
        recs.into_iter().take(n).collect()
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::tasklist_adapter::Task;

    #[test]
    fn test_completion_tracking() {
        let mut tracker = TaskCompletionTracker::new();

        let task = Task {
            id: 42,
            title: "Do the thing".to_string(),
            priority: "high".to_string(),
            ..Default::default()
        };

        tracker.mark_started(42);
        tracker.record_completion(&task, Some(0.92), Some(1.0), "All tests passed");

        let stats = tracker.get_stats();
        assert_eq!(stats.total_completed, 1);
        assert!(stats.high_quality_count >= 1);
        assert_eq!(tracker.get_records().len(), 1);
    }
}
