//! HOH TaskList Telemetry (Task 327.29)

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskListMetrics {
    pub snapshot_at: u64,
    pub total_tasks: usize,
    pub by_status: HashMap<String, usize>,
    pub by_priority: HashMap<String, usize>,
    pub completion_velocity: f32,   // tasks completed per day (rolling 7d)
    pub mutation_rate: f32,         // mutations per iteration (estimate)
    pub avg_quality_score: f32,
    pub stalled_count: usize,
    pub high_risk_count: usize,
}

pub fn compute_metrics(tasks: &[Task]) -> TaskListMetrics {
    let mut by_status: HashMap<String, usize> = HashMap::new();
    let mut by_priority: HashMap<String, usize> = HashMap::new();
    let mut stalled = 0usize;

    for t in tasks {
        *by_status.entry(t.status.clone()).or_insert(0) += 1;
        *by_priority.entry(t.priority.clone()).or_insert(0) += 1;
        if t.status == "in_progress" && t.age_days() > 7.0 { stalled += 1; }
    }

    // Approximate quality using title/description length heuristic
    let avg_quality = if tasks.is_empty() { 0.0 } else {
        tasks.iter().map(|t| {
            let title_ok = if t.title.len() > 10 { 0.5 } else { 0.0 };
            let desc_ok  = if t.description.len() > 20 { 0.5 } else { 0.0 };
            title_ok + desc_ok
        }).sum::<f32>() / tasks.len() as f32
    };

    // High risk: tasks with security/breaking keywords
    let high_risk = tasks.iter().filter(|t| {
        let text = format!("{} {}", t.title, t.description).to_lowercase();
        text.contains("security") || text.contains("breaking") || text.contains("unsafe")
    }).count();

    TaskListMetrics {
        snapshot_at: now_secs(),
        total_tasks: tasks.len(),
        by_status,
        by_priority,
        completion_velocity: 0.0, // would need historical data
        mutation_rate: 0.0,
        avg_quality_score: avg_quality,
        stalled_count: stalled,
        high_risk_count: high_risk,
    }
}

/// Append a metrics snapshot to `.grok/hoh/logs/tasklist_telemetry.jsonl`
pub async fn emit_telemetry(tasks: &[Task], project_root: &Path) -> std::io::Result<()> {
    let metrics = compute_metrics(tasks);
    let line = serde_json::to_string(&metrics)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let log_dir = project_root.join(".grok/hoh/logs");
    tokio::fs::create_dir_all(&log_dir).await?;
    let path = log_dir.join("tasklist_telemetry.jsonl");

    use tokio::io::AsyncWriteExt;
    let mut f = tokio::fs::OpenOptions::new().create(true).append(true).open(&path).await?;
    f.write_all(format!("{}\n", line).as_bytes()).await?;

    tracing::debug!(
        total = metrics.total_tasks, stalled = metrics.stalled_count,
        "[HOH Telemetry] emitted metrics snapshot"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, status: &str, priority: &str) -> Task {
        Task { id, status: status.to_string(), priority: priority.to_string(), title: format!("Task {}", id), ..Default::default() }
    }

    #[test]
    fn test_compute_metrics_counts_correctly() {
        let tasks = vec![t(1,"done","high"), t(2,"pending","low"), t(3,"in_progress","high")];
        let m = compute_metrics(&tasks);
        assert_eq!(m.total_tasks, 3);
        assert_eq!(m.by_status["done"], 1);
        assert_eq!(m.by_priority["high"], 2);
    }
}
