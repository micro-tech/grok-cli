//! HOH Task Progress Estimator (Task 327.24)

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEstimate {
    pub task_id: u64,
    /// 0–100 percent complete.
    pub percent_complete: u8,
    pub remaining_effort_days: f32,
    pub confidence: f32,
    pub basis: String,
}

pub fn estimate_progress(task: &Task) -> ProgressEstimate {
    let total_subtasks = task.subtasks.len();

    let (percent, basis) = if total_subtasks > 0 {
        let done = task.subtasks.iter().filter(|s| s.status == "done").count();
        let pct = ((done as f32 / total_subtasks as f32) * 100.0) as u8;
        (pct, format!("{}/{} subtasks done", done, total_subtasks))
    } else {
        match task.status.as_str() {
            "done"        => (100, "task marked done".to_string()),
            "in_progress" => (40,  "task in_progress, no subtasks".to_string()),
            "pending"     => (0,   "task not started".to_string()),
            _             => (0,   format!("unknown status: {}", task.status)),
        }
    };

    // Rough effort estimate: assume each subtask = 0.5 days, else 2 days flat
    let remaining = if total_subtasks > 0 {
        let remaining_subtasks = total_subtasks.saturating_sub(
            task.subtasks.iter().filter(|s| s.status == "done").count()
        );
        remaining_subtasks as f32 * 0.5
    } else if task.status == "pending" { 2.0 } else { 1.0 };

    ProgressEstimate {
        task_id: task.id,
        percent_complete: percent,
        remaining_effort_days: remaining,
        confidence: if total_subtasks > 0 { 0.85 } else { 0.4 },
        basis,
    }
}

pub fn estimate_all(tasks: &[Task]) -> Vec<ProgressEstimate> {
    tasks.iter().map(estimate_progress).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subtask(status: &str) -> Task {
        Task { id: 0, status: status.to_string(), title: String::new(), ..Default::default() }
    }

    #[test]
    fn test_half_subtasks_done_is_50_percent() {
        let mut t = Task { id: 1, status: "in_progress".to_string(), title: String::new(), ..Default::default() };
        t.subtasks = vec![subtask("done"), subtask("done"), subtask("pending"), subtask("pending")];
        let e = estimate_progress(&t);
        assert_eq!(e.percent_complete, 50);
    }

    #[test]
    fn test_pending_no_subtasks_is_zero() {
        let t = Task { id: 2, status: "pending".to_string(), title: String::new(), ..Default::default() };
        let e = estimate_progress(&t);
        assert_eq!(e.percent_complete, 0);
    }
}
