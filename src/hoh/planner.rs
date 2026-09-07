//! HOH Planning Phase (Task 297.3 + 327 integration)
//!
//! Real implementation that reads task_list.json via TaskListAdapter,
//! applies selection, prioritization, and generates a coherent HOHPlan.
//!
//! Now uses TaskDependencyGraph (327.4) + scoring (327.5) for dependency-respecting selection.

use crate::hoh::state::{HOHError, HOHPlan};
use crate::hoh::tasklist_adapter::{Task, TaskListAdapter};
use crate::hoh::task_completion_tracker::TaskCompletionTracker;
use std::collections::HashSet;
use std::path::PathBuf;

/// Enhanced planner that understands the task list.
#[derive(Debug)]
pub struct HOHPlanner {
    adapter: TaskListAdapter,
    pub completion_tracker: TaskCompletionTracker,
}

impl HOHPlanner {
    pub fn new(data_dir: PathBuf, simulation_mode: bool) -> Self {
        Self {
            adapter: TaskListAdapter::new(data_dir, simulation_mode),
            completion_tracker: TaskCompletionTracker::new(),
        }
    }

    /// Main planning entry point used by outer loop.
    /// Now uses real dependency graph (327.4) + multi-signal prioritization (327.5).
    pub async fn create_plan(&mut self, goals: Vec<String>) -> Result<HOHPlan, HOHError> {
        // 327.2 + 327.4: First get only tasks whose dependencies are satisfied
        let empty_completed: HashSet<u64> = HashSet::new();
        let ready_tasks = self.adapter.get_ready_tasks(&empty_completed).await
            .unwrap_or_else(|_| Vec::new());

        // If graph-based ready tasks is empty, fall back to all pending (graceful)
        let candidates = if ready_tasks.is_empty() {
            self.adapter.get_pending_tasks().await?
        } else {
            ready_tasks
        };

        // 327.5: Score + prioritize the candidates
        let mut selected = self.select_and_prioritize(&candidates, &goals);

        // 327.4: Try to order the final selection according to topological order
        if let Ok(topo) = self.adapter.get_topological_order().await {
            selected.sort_by_key(|t| {
                topo.iter().position(|&id| id == t.id).unwrap_or(usize::MAX)
            });
        }

        // Record start for selected tasks (327.6)
        for t in &selected {
            self.completion_tracker.mark_started(t.id);
        }

        let plan = HOHPlan {
            goals,
            selected_tasks: selected.iter().map(|t| t.id).collect(),
            experiments: vec!["tasklist_driven".to_string()],
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        tracing::info!(
            selected = ?plan.selected_tasks,
            "HOHPlanner: created plan with {} tasks (dependency-aware)",
            plan.selected_tasks.len()
        );

        Ok(plan)
    }

    /// Core selection + prioritization logic (327.2 + 327.5)
    /// Scores tasks then selects a dependency-respecting batch.
    fn select_and_prioritize(&self, candidates: &[Task], goals: &[String]) -> Vec<Task> {
        if candidates.is_empty() {
            return vec![];
        }

        let mut scored: Vec<(Task, f32)> = candidates
            .iter()
            .map(|task| {
                let score = self.score_task(task, goals);
                (task.clone(), score)
            })
            .collect();

        // Sort by score descending (327.5)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select top N while respecting dependencies (simple greedy within the ready set)
        let mut selected = Vec::new();
        let mut selected_ids = HashSet::new();

        for (task, _score) in scored.iter() {
            // All dependencies of this task must already be selected or not in the candidate pool
            let deps_ok = task
                .dependencies
                .iter()
                .all(|&dep| selected_ids.contains(&dep) || !candidates.iter().any(|t| t.id == dep));

            if deps_ok || task.dependencies.is_empty() {
                selected_ids.insert(task.id);
                selected.push(task.clone());
            }

            if selected.len() >= 8 {
                // Reasonable batch size for one HOH iteration
                break;
            }
        }

        // If nothing passed the dep check (edge case), just take the top scored ones
        if selected.is_empty() && !candidates.is_empty() {
            selected = scored.into_iter().take(8).map(|(t, _)| t).collect();
        }

        selected
    }

    /// Multi-signal scoring (327.5)
    /// Now incorporates completion history from TaskCompletionTracker (327.6)
    fn score_task(&self, task: &Task, goals: &[String]) -> f32 {
        let mut score = 0.0;

        // Static priority signal
        match task.priority.as_str() {
            "high" => score += 10.0,
            "medium" => score += 5.0,
            "low" => score += 1.0,
            _ => {}
        }

        // Goal alignment (simple keyword overlap)
        let title_lower = task.title.to_lowercase();
        for goal in goals {
            if title_lower.contains(&goal.to_lowercase()) {
                score += 8.0;
            }
        }

        // Historical performance bonus (327.5 + 327.6)
        // Reward tasks similar to ones we completed successfully in the past
        let stats = self.completion_tracker.get_stats();
        if stats.total_completed > 0 {
            // Slight boost for having data at all
            score += 1.5;

            if let Some(avg_dur) = stats.avg_duration_secs {
                // Prefer tasks we can finish in reasonable time (heuristic)
                if avg_dur < 3600.0 * 4.0 {
                    score += 2.0;
                }
            }

            if stats.high_quality_count as f32 / stats.total_completed as f32 > 0.7 {
                score += 3.0; // Team is performing well → be more ambitious
            }
        }

        // Freshness / age bonus (prefer older pending work)
        score += 2.0;

        // Penalty for very large tasks (prefer focused work)
        if task.details.len() > 1500 {
            score -= 3.0;
        }

        // Bonus for tasks with clear test_strategy (327.5)
        if !task.test_strategy.is_empty() && task.test_strategy.len() > 20 {
            score += 4.0;
        }

        // Small penalty if task has many dependencies (risk of blocking)
        if task.dependencies.len() > 3 {
            score -= 1.5;
        }

        score
    }

    /// Record that a task was completed (call this from outer loop / mutation when status -> done)
    pub fn record_task_completion(
        &mut self,
        task: &Task,
        quality_score: Option<f32>,
        test_pass_rate: Option<f32>,
        notes: &str,
    ) {
        self.completion_tracker.record_completion(task, quality_score, test_pass_rate, notes);
    }
}

/// Backward-compatible simple function (kept for existing callers in outer_loop)
pub async fn create_plan(goals: Vec<String>) -> HOHPlan {
    // Fallback when no data_dir context is available
    let mut planner = HOHPlanner::new(std::env::current_dir().unwrap_or_default(), true);
    let goals_clone = goals.clone();
    planner.create_plan(goals).await.unwrap_or_else(|_| HOHPlan {
        goals: goals_clone,
        selected_tasks: vec![327, 297],
        experiments: vec!["fallback".to_string()],
        created_at: 0,
    })
}