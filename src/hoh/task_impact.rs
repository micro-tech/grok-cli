//! HOH Task Impact Analysis (Task 327.15)
//!
//! Estimates downstream impact of completing (or skipping) a task
//! by walking the dependency graph and scoring transitive unblocking.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub task_id: u64,
    /// Number of tasks directly unblocked by completing this task.
    pub direct_unblocked: usize,
    /// Total transitively unblocked tasks (downstream chain).
    pub transitive_unblocked: usize,
    /// 0.0–1.0 composite impact score.
    pub impact_score: f32,
    pub rationale: String,
}

/// Compute impact for every task in the list.
pub fn analyze_impact(tasks: &[Task]) -> Vec<ImpactReport> {
    // Build reverse-dep map: task_id -> set of tasks that depend on it
    let mut rdeps: HashMap<u64, HashSet<u64>> = HashMap::new();
    for t in tasks {
        rdeps.entry(t.id).or_default();
        for &dep in &t.dependencies {
            rdeps.entry(dep).or_default().insert(t.id);
        }
    }

    let all_ids: HashSet<u64> = tasks.iter().map(|t| t.id).collect();

    tasks
        .iter()
        .map(|t| {
            let direct = rdeps.get(&t.id).map(|s| s.len()).unwrap_or(0);
            let transitive = transitive_count(t.id, &rdeps, &all_ids);
            let score = (direct as f32 * 0.4 + transitive as f32 * 0.6)
                .min(50.0)
                / 50.0;
            let rationale = format!(
                "Completing task {} directly unblocks {} task(s) and transitively unblocks {}.",
                t.id, direct, transitive
            );
            ImpactReport {
                task_id: t.id,
                direct_unblocked: direct,
                transitive_unblocked: transitive,
                impact_score: score,
                rationale,
            }
        })
        .collect()
}

/// BFS to count all transitively unblocked nodes.
fn transitive_count(
    start: u64,
    rdeps: &HashMap<u64, HashSet<u64>>,
    all_ids: &HashSet<u64>,
) -> usize {
    let mut visited = HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    if let Some(direct) = rdeps.get(&start) {
        for &d in direct {
            if all_ids.contains(&d) {
                queue.push_back(d);
            }
        }
    }
    while let Some(id) = queue.pop_front() {
        if visited.insert(id) {
            if let Some(next) = rdeps.get(&id) {
                for &n in next {
                    if !visited.contains(&n) {
                        queue.push_back(n);
                    }
                }
            }
        }
    }
    visited.len()
}

/// Return top-N highest-impact tasks.
pub fn top_impact(tasks: &[Task], n: usize) -> Vec<ImpactReport> {
    let mut reports = analyze_impact(tasks);
    reports.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap_or(std::cmp::Ordering::Equal));
    reports.truncate(n);
    reports
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, deps: Vec<u64>) -> Task {
        Task { id, dependencies: deps, title: format!("Task {}", id),
               status: "pending".to_string(), ..Default::default() }
    }

    #[test]
    fn test_blocking_task_has_high_impact() {
        let tasks = vec![t(1, vec![]), t(2, vec![1]), t(3, vec![1])];
        let reports = analyze_impact(&tasks);
        let r1 = reports.iter().find(|r| r.task_id == 1).unwrap();
        assert!(r1.direct_unblocked >= 2);
        assert!(r1.impact_score > 0.0);
    }

    #[test]
    fn test_leaf_task_has_zero_impact() {
        let tasks = vec![t(1, vec![]), t(2, vec![1])];
        let reports = analyze_impact(&tasks);
        let r2 = reports.iter().find(|r| r.task_id == 2).unwrap();
        assert_eq!(r2.direct_unblocked, 0);
    }
}
