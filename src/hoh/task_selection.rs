//! HOH Task Selection Engine (Task 327.2)
//!
//! Selects the best *ready* pending tasks for the current HOH iteration.
//!
//! Scoring is now fully delegated to [`TaskPrioritizationModel`] (Task 327.5),
//! so both systems share a single, consistent set of weights and signals.
//! `SelectionConfig` maps directly to a [`PrioritizationContext`]:
//!
//! | SelectionConfig field  | PrioritizationContext field |
//! |------------------------|-----------------------------|
//! | `goal_keywords`        | `goals`                     |
//! | `helix_score`          | `helix_score`               |
//! | `okf_terms`            | `okf_terms`                 |
//!
//! # Quick start
//!
//! ```no_run
//! # use grok_cli::hoh::task_selection::{TaskSelectionEngine, SelectionConfig};
//! # use grok_cli::hoh::tasklist_adapter::TaskListAdapter;
//! # use std::path::PathBuf;
//! # tokio_test::block_on(async {
//! let adapter = TaskListAdapter::new(PathBuf::from("."), true);
//! let engine  = TaskSelectionEngine::new(adapter.clone());
//! let config  = TaskSelectionEngine::config_from_adapter(&adapter, 5, vec![]).await.unwrap();
//! let picks   = engine.select(&config).await.unwrap();
//! for p in &picks {
//!     println!("[{:.1}] #{} — {}", p.score, p.task.id, p.task.title);
//! }
//! # });
//! ```

use crate::hoh::state::HOHError;
use crate::hoh::task_completion_tracker::TaskCompletionTracker;
use crate::hoh::task_dependency_graph::TaskDependencyGraph;
use crate::hoh::task_prioritization::{
    PrioritizationContext, TaskPrioritizationModel,
};
use crate::hoh::tasklist_adapter::{Task, TaskList, TaskListAdapter};
use std::collections::HashSet;

// ─── Public types ──────────────────────────────────────────────────────────────

/// A task chosen by the selection engine, annotated with its composite score
/// and the reasoning behind it.
#[derive(Debug, Clone)]
pub struct SelectedTask {
    pub task: Task,
    /// Composite score — higher is better. Used for ordering.
    pub score: f32,
    /// Human-readable explanation of why this task was chosen.
    pub reasoning: Vec<String>,
}

/// Configuration for a single selection run.
#[derive(Debug, Clone)]
pub struct SelectionConfig {
    /// Maximum number of tasks to return.
    pub max_tasks: usize,
    /// IDs of tasks considered complete (status `"done"` or `"deferred"`).
    /// Used to decide which pending tasks have satisfied dependencies.
    pub completed_ids: HashSet<u64>,
    /// Goal keywords forwarded to the prioritization model's `goals` signal.
    pub goal_keywords: Vec<String>,
    /// Optional Helix evaluation score (0.0–1.0) for the evaluation-feedback
    /// signal in the prioritization model.
    pub helix_score: Option<f32>,
    /// OKF terms forwarded to the prioritization model's OKF-relevance signal.
    pub okf_terms: Vec<String>,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            max_tasks: 5,
            completed_ids: HashSet::new(),
            goal_keywords: Vec::new(),
            helix_score: None,
            okf_terms: Vec::new(),
        }
    }
}

// ─── Engine ────────────────────────────────────────────────────────────────────

/// The Task Selection Engine.
///
/// Scoring is delegated entirely to [`TaskPrioritizationModel`].
/// Use [`TaskSelectionEngine::with_model`] to inject a custom model
/// (e.g. with tuned weights for a specific HOH phase).
#[derive(Debug)]
pub struct TaskSelectionEngine {
    adapter: TaskListAdapter,
    model: TaskPrioritizationModel,
}

impl TaskSelectionEngine {
    /// Create with the default prioritization model.
    pub fn new(adapter: TaskListAdapter) -> Self {
        Self {
            adapter,
            model: TaskPrioritizationModel::new(),
        }
    }

    /// Create with a custom prioritization model (e.g. different weights).
    pub fn with_model(adapter: TaskListAdapter, model: TaskPrioritizationModel) -> Self {
        Self { adapter, model }
    }

    // ── Config helpers ──────────────────────────────────────────────────────────

    /// Build a [`SelectionConfig`] by auto-detecting completed task IDs from
    /// the current task list (status `"done"` or `"deferred"`).
    pub async fn config_from_adapter(
        adapter: &TaskListAdapter,
        max_tasks: usize,
        goal_keywords: Vec<String>,
    ) -> Result<SelectionConfig, HOHError> {
        let list = adapter.load().await?;
        Ok(Self::config_from_list(&list, max_tasks, goal_keywords))
    }

    /// Build a [`SelectionConfig`] from an already-loaded [`TaskList`].
    pub fn config_from_list(
        list: &TaskList,
        max_tasks: usize,
        goal_keywords: Vec<String>,
    ) -> SelectionConfig {
        let completed_ids: HashSet<u64> = list
            .tasks
            .iter()
            .flat_map(|t| {
                let mut ids = Vec::new();
                if matches!(t.status.as_str(), "done" | "deferred") {
                    ids.push(t.id);
                }
                for sub in &t.subtasks {
                    if matches!(sub.status.as_str(), "done" | "deferred") {
                        ids.push(sub.id);
                    }
                }
                ids
            })
            .collect();

        SelectionConfig {
            max_tasks,
            completed_ids,
            goal_keywords,
            helix_score: None,
            okf_terms: Vec::new(),
        }
    }

    // ── Core selection ──────────────────────────────────────────────────────────

    /// Load the task list and run selection with the given config.
    pub async fn select(
        &self,
        config: &SelectionConfig,
    ) -> Result<Vec<SelectedTask>, HOHError> {
        let list = self.adapter.load().await?;
        Ok(self.select_from_list(&list, config))
    }

    /// Run selection on an already-loaded list — pure computation, no I/O.
    ///
    /// Internally calls [`TaskPrioritizationModel::rank`] on the ready subset
    /// then converts [`RankedTask`]s to [`SelectedTask`]s.
    pub fn select_from_list(&self, list: &TaskList, config: &SelectionConfig) -> Vec<SelectedTask> {
        let graph = TaskDependencyGraph::from_tasks(&list.tasks);

        // Collect all pending tasks (top-level + one level of subtasks)
        let pending = Self::collect_pending(&list.tasks);

        // Filter to those whose dependencies are satisfied
        let ready_ids: HashSet<u64> = graph
            .get_ready_tasks(&config.completed_ids)
            .into_iter()
            .collect();

        let ready_pending: Vec<Task> = pending
            .into_iter()
            .filter(|t| ready_ids.contains(&t.id))
            .collect();

        // Build prioritization context from config
        let ctx = PrioritizationContext {
            goals: config.goal_keywords.clone(),
            helix_score: config.helix_score,
            okf_terms: config.okf_terms.clone(),
            completion_stats: None, // pass via select_with_history for history signal
        };

        // Delegate scoring + sorting to the prioritization model
        let mut selected: Vec<SelectedTask> = self
            .model
            .rank(&ready_pending, Some(&graph), &ctx)
            .into_iter()
            .map(|r| SelectedTask {
                task: r.task,
                score: r.score,
                reasoning: r.explanation,
            })
            .collect();

        selected.truncate(config.max_tasks);
        selected
    }

    /// Like [`select`], but feeds the [`TaskCompletionTracker`]'s statistics
    /// into the prioritization model's completion-history signal, and applies
    /// a word-overlap boost from recent high-quality completions.
    pub async fn select_with_history(
        &self,
        config: &SelectionConfig,
        tracker: &TaskCompletionTracker,
    ) -> Result<Vec<SelectedTask>, HOHError> {
        let list = self.adapter.load().await?;
        let graph = TaskDependencyGraph::from_tasks(&list.tasks);
        let pending = Self::collect_pending(&list.tasks);

        let ready_ids: HashSet<u64> = graph
            .get_ready_tasks(&config.completed_ids)
            .into_iter()
            .collect();

        let ready_pending: Vec<Task> = pending
            .into_iter()
            .filter(|t| ready_ids.contains(&t.id))
            .collect();

        // Include completion history in the prioritization context
        let ctx = PrioritizationContext {
            goals: config.goal_keywords.clone(),
            helix_score: config.helix_score,
            okf_terms: config.okf_terms.clone(),
            completion_stats: Some(tracker.get_stats()),
        };

        let mut selected: Vec<SelectedTask> = self
            .model
            .rank(&ready_pending, Some(&graph), &ctx)
            .into_iter()
            .map(|r| SelectedTask {
                task: r.task,
                score: r.score,
                reasoning: r.explanation,
            })
            .collect();

        // Extra word-overlap boost from individual high-quality completion titles
        // (complements the aggregate history signal already in the model)
        let recent = tracker.recent_completions(10);
        let hq_word_sets: Vec<HashSet<&str>> = recent
            .iter()
            .filter(|r| r.quality_score.unwrap_or(0.0) >= 0.8)
            .map(|r| r.title.split_whitespace().collect())
            .collect();

        for sel in &mut selected {
            let task_words: HashSet<&str> = sel.task.title.split_whitespace().collect();
            for hq_words in &hq_word_sets {
                if task_words.intersection(hq_words).count() >= 2 {
                    sel.score += 0.5;
                    sel.reasoning
                        .push("similar to recent high-quality completion (+0.50)".to_string());
                    break;
                }
            }
        }

        // Re-sort after the overlap boost
        selected.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.task.id.cmp(&b.task.id))
        });

        selected.truncate(config.max_tasks);
        Ok(selected)
    }

    // ── Helpers ─────────────────────────────────────────────────────────────────

    fn collect_pending(tasks: &[Task]) -> Vec<Task> {
        let mut out = Vec::new();
        for t in tasks {
            if t.status == "pending" {
                out.push(t.clone());
            }
            for sub in &t.subtasks {
                if sub.status == "pending" {
                    out.push(sub.clone());
                }
            }
        }
        out
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::tasklist_adapter::{Task, TaskList};
    use tempfile::tempdir;

    fn make_task(id: u64, title: &str, priority: &str, deps: Vec<u64>) -> Task {
        Task {
            id,
            title: title.to_string(),
            priority: priority.to_string(),
            status: "pending".to_string(),
            dependencies: deps,
            details: "Some implementation details here.".to_string(),
            test_strategy: "Run cargo test.".to_string(),
            ..Default::default()
        }
    }

    fn make_done(id: u64) -> Task {
        Task {
            id,
            title: format!("Done task {}", id),
            status: "done".to_string(),
            priority: "medium".to_string(),
            ..Default::default()
        }
    }

    fn list_with(tasks: Vec<Task>) -> TaskList {
        TaskList { tasks }
    }

    fn stub_engine() -> TaskSelectionEngine {
        let dir = tempdir().unwrap();
        let adapter = TaskListAdapter::new(dir.path().to_path_buf(), true);
        TaskSelectionEngine::new(adapter)
    }

    // ── Basic selection ─────────────────────────────────────────────────────────

    #[test]
    fn selects_only_pending_tasks() {
        let engine = stub_engine();
        let list = list_with(vec![
            make_done(1),
            make_task(2, "Do something", "high", vec![]),
        ]);
        let config = SelectionConfig::default();
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].task.id, 2);
    }

    #[test]
    fn respects_dependency_gates() {
        let engine = stub_engine();
        let list = list_with(vec![
            make_task(1, "Foundation", "high", vec![]),
            make_task(2, "Blocked task", "high", vec![1]),
        ]);
        let config = SelectionConfig::default(); // completed_ids is empty
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].task.id, 1);
    }

    #[test]
    fn unblocked_when_dep_is_done() {
        let engine = stub_engine();
        let list = list_with(vec![
            make_done(1),
            make_task(2, "Now unblocked", "medium", vec![1]),
        ]);
        let mut config = SelectionConfig::default();
        config.completed_ids.insert(1);
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].task.id, 2);
    }

    // ── Priority ordering ───────────────────────────────────────────────────────

    #[test]
    fn high_priority_beats_low_priority() {
        let engine = stub_engine();
        let list = list_with(vec![
            make_task(10, "Low prio", "low", vec![]),
            make_task(11, "High prio", "high", vec![]),
        ]);
        let config = SelectionConfig {
            max_tasks: 2,
            ..Default::default()
        };
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(picks[0].task.id, 11, "high priority should come first");
    }

    // ── Dependency pressure ─────────────────────────────────────────────────────

    #[test]
    fn dependency_pressure_boosts_score() {
        let engine = stub_engine();
        let mut tasks = vec![
            make_task(1, "Blocker", "medium", vec![]),
            make_task(2, "Free agent", "medium", vec![]),
            make_task(3, "Waiter A", "low", vec![1]),
            make_task(4, "Waiter B", "low", vec![1]),
            make_task(5, "Waiter C", "low", vec![1]),
        ];
        // Make waiters pending but with deps on task 1
        for t in &mut tasks[2..] { t.dependencies = vec![1]; }
        let list = list_with(tasks);
        let config = SelectionConfig { max_tasks: 5, ..Default::default() };
        let picks = engine.select_from_list(&list, &config);
        // Only tasks 1 and 2 are ready (waiters depend on 1, which isn't done)
        let score_1 = picks.iter().find(|p| p.task.id == 1).map(|p| p.score);
        let score_2 = picks.iter().find(|p| p.task.id == 2).map(|p| p.score);
        assert!(score_1 > score_2, "blocker (id=1) should score higher than free agent (id=2)");
    }

    // ── Goal keyword alignment ──────────────────────────────────────────────────

    #[test]
    fn goal_keyword_boosts_matching_task() {
        let engine = stub_engine();
        let mut t_match = make_task(20, "Refactor the architecture module", "medium", vec![]);
        t_match.description = "Improve architecture consistency.".to_string();
        let t_no_match = make_task(21, "Write release notes", "medium", vec![]);
        let list = list_with(vec![t_match, t_no_match]);
        let config = SelectionConfig {
            max_tasks: 2,
            goal_keywords: vec!["architecture".to_string()],
            ..Default::default()
        };
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(picks[0].task.id, 20, "keyword-matching task should rank first");
    }

    // ── Helix score forwarding ──────────────────────────────────────────────────

    #[test]
    fn good_helix_score_reflected_in_scores() {
        let engine = stub_engine();
        let list = list_with(vec![make_task(1, "Some task", "medium", vec![])]);

        let config_helix = SelectionConfig {
            helix_score: Some(0.9),
            ..Default::default()
        };
        let config_none = SelectionConfig::default();

        let with_helix = engine.select_from_list(&list, &config_helix);
        let without    = engine.select_from_list(&list, &config_none);
        assert!(!with_helix.is_empty() && !without.is_empty());
        assert!(
            with_helix[0].score > without[0].score,
            "good Helix score should produce higher score"
        );
    }

    // ── OKF terms forwarding ────────────────────────────────────────────────────

    #[test]
    fn okf_terms_boost_relevant_task() {
        let engine = stub_engine();
        let relevant  = make_task(30, "Implement OKF bundle sync", "medium", vec![]);
        let unrelated = make_task(31, "Update README", "medium", vec![]);
        let list = list_with(vec![relevant, unrelated]);
        let config = SelectionConfig {
            max_tasks: 2,
            okf_terms: vec!["okf".to_string(), "bundle".to_string()],
            ..Default::default()
        };
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(picks[0].task.id, 30);
    }

    // ── max_tasks cap ───────────────────────────────────────────────────────────

    #[test]
    fn respects_max_tasks_limit() {
        let engine = stub_engine();
        let tasks: Vec<Task> = (1..=20)
            .map(|i| make_task(i, &format!("Task {}", i), "medium", vec![]))
            .collect();
        let list = list_with(tasks);
        let config = SelectionConfig { max_tasks: 3, ..Default::default() };
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(picks.len(), 3);
    }

    // ── config_from_list ────────────────────────────────────────────────────────

    #[test]
    fn config_from_list_detects_done_ids() {
        let list = list_with(vec![
            make_done(1),
            make_done(2),
            make_task(3, "Still pending", "high", vec![1, 2]),
        ]);
        let config = TaskSelectionEngine::config_from_list(&list, 5, vec![]);
        assert!(config.completed_ids.contains(&1));
        assert!(config.completed_ids.contains(&2));
        assert!(!config.completed_ids.contains(&3));
    }

    // ── Reasoning is populated ──────────────────────────────────────────────────

    #[test]
    fn reasoning_lines_are_populated() {
        let engine = stub_engine();
        let list = list_with(vec![make_task(99, "A task", "high", vec![])]);
        let config = SelectionConfig::default();
        let picks = engine.select_from_list(&list, &config);
        assert!(!picks.is_empty());
        assert!(!picks[0].reasoning.is_empty(), "SelectedTask should include reasoning");
        assert!(
            picks[0].reasoning.iter().any(|r| r.contains("priority")),
            "reasoning should include a priority entry"
        );
    }

    // ── with_model constructor ──────────────────────────────────────────────────

    #[test]
    fn with_model_uses_custom_weights() {
        use crate::hoh::task_prioritization::PrioritizationWeights;

        // Zero everything except goal_alignment — only matching tasks score > 0
        let weights = PrioritizationWeights {
            static_priority: 0.0,
            dependency_pressure: 0.0,
            goal_alignment: 10.0,
            evaluation_feedback: 0.0,
            actionability: 0.0,
            test_strategy_bonus: 0.0,
            okf_relevance: 0.0,
            completion_history: 0.0,
            complexity_penalty: 0.0,
        };
        let dir = tempdir().unwrap();
        let adapter = TaskListAdapter::new(dir.path().to_path_buf(), true);
        let engine = TaskSelectionEngine::with_model(
            adapter,
            TaskPrioritizationModel::with_weights(weights),
        );

        let low_match  = make_task(1, "Low-priority architecture task", "low", vec![]);
        let high_plain = make_task(2, "High-priority plain task", "high", vec![]);
        let list = list_with(vec![low_match, high_plain]);
        let config = SelectionConfig {
            max_tasks: 2,
            goal_keywords: vec!["architecture".to_string()],
            ..Default::default()
        };
        let picks = engine.select_from_list(&list, &config);
        assert_eq!(
            picks[0].task.id, 1,
            "goal-matching low-priority task should win when only goal_alignment matters"
        );
    }

    // ── Async path ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn async_select_returns_results() {
        let dir = tempdir().unwrap();
        // simulation_mode = false so save() actually writes the file
        let adapter = TaskListAdapter::new(dir.path().to_path_buf(), false);

        let list = list_with(vec![
            make_task(1, "Alpha", "high", vec![]),
            make_task(2, "Beta", "medium", vec![1]),
        ]);
        adapter.save(&list).await.unwrap();

        let engine = TaskSelectionEngine::new(adapter.clone());
        let config = TaskSelectionEngine::config_from_adapter(&adapter, 5, vec![])
            .await
            .unwrap();
        let picks = engine.select(&config).await.unwrap();

        // Only task 1 is ready (task 2 depends on task 1 which is not done)
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].task.id, 1);
    }
}
