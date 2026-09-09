//! HOH Task Prioritization Model (Task 327.5)
//!
//! A standalone, reusable, multi-signal model that scores and ranks HOH tasks.
//! Used by both [`TaskSelectionEngine`] and [`HOHPlanner`] to produce stable,
//! explainable orderings.
//!
//! # Signals (default weights in parentheses)
//!
//! | # | Signal | Default weight | Source |
//! |---|--------|---------------|--------|
//! | 1 | Static priority | ×3.0 | task.priority |
//! | 2 | Dependency pressure | ×1.5 per waiter | dependency graph |
//! | 3 | Goal alignment | ×2.0 per keyword hit | HOH goals |
//! | 4 | Evaluation feedback | ×2.0 | Helix score |
//! | 5 | Actionability | ×2.0 | details + testStrategy completeness |
//! | 6 | Test-strategy bonus | ×2.0 | non-empty testStrategy |
//! | 7 | Completion history | ×1.5 | TaskCompletionTracker stats |
//! | 8 | OKF relevance | ×1.0 per term hit | OKF lookup results |
//! | 9 | Complexity penalty | −0.5 per 500 chars over 1 000 | details length |
//!
//! # Quick start
//!
//! ```
//! use grok_cli::hoh::task_prioritization::{
//!     TaskPrioritizationModel, PrioritizationContext,
//! };
//! use grok_cli::hoh::tasklist_adapter::Task;
//!
//! let model = TaskPrioritizationModel::new();
//! let tasks: Vec<Task> = vec![];          // your pending tasks
//! let ctx   = PrioritizationContext::default();
//! let ranked = model.rank(&tasks, None, &ctx);
//! for r in &ranked {
//!     println!("[{:.1}] #{} — {}", r.score, r.task.id, r.task.title);
//!     for line in &r.explanation { println!("  {}", line); }
//! }
//! ```

use crate::hoh::task_completion_tracker::CompletionStats;
use crate::hoh::task_dependency_graph::TaskDependencyGraph;
use crate::hoh::tasklist_adapter::Task;
use std::collections::HashMap;

// ─── Weights ───────────────────────────────────────────────────────────────────

/// Tunable weights for each scoring signal.
///
/// All weights are non-negative multipliers applied to the normalised signal
/// value.  Set a weight to `0.0` to disable that signal entirely.
#[derive(Debug, Clone, PartialEq)]
pub struct PrioritizationWeights {
    /// Multiplied by the priority base (high=3, medium=2, low=1).
    pub static_priority: f32,
    /// Added per task that is waiting (blocked) on this one.
    pub dependency_pressure: f32,
    /// Added per HOH goal keyword found in title + description + details.
    pub goal_alignment: f32,
    /// Scales how much a recent Helix evaluation score shifts task priorities.
    ///
    /// Positive Helix → boosts tasks aligned with current good work.
    /// Negative Helix → boosts tasks that would fix the failure area.
    pub evaluation_feedback: f32,
    /// Added when a task has both non-trivial `details` AND a `testStrategy`.
    pub actionability: f32,
    /// Added when `testStrategy` is non-empty (even without full details).
    pub test_strategy_bonus: f32,
    /// Applied per OKF term found in the task text.
    pub okf_relevance: f32,
    /// Added when completion history shows recent high-quality work.
    pub completion_history: f32,
    /// Subtracted per 500 chars of `details` over a 1 000-char threshold.
    /// Penalises huge, unfocused tasks.
    pub complexity_penalty: f32,
}

impl Default for PrioritizationWeights {
    fn default() -> Self {
        Self {
            static_priority: 3.0,
            dependency_pressure: 1.5,
            goal_alignment: 2.0,
            evaluation_feedback: 2.0,
            actionability: 2.0,
            test_strategy_bonus: 2.0,
            okf_relevance: 1.0,
            completion_history: 1.5,
            complexity_penalty: 0.5,
        }
    }
}

// ─── Signals ───────────────────────────────────────────────────────────────────

/// The individual signal contributions for one task.
///
/// `total()` sums them into the final composite score.
#[derive(Debug, Clone, Default)]
pub struct PrioritySignals {
    pub static_priority: f32,
    pub dependency_pressure: f32,
    pub goal_alignment: f32,
    pub evaluation_feedback: f32,
    pub actionability: f32,
    pub test_strategy_bonus: f32,
    pub okf_relevance: f32,
    pub completion_history: f32,
    /// Negative — reduces score for large, complex tasks.
    pub complexity_penalty: f32,
}

impl PrioritySignals {
    /// Composite score — sum of all signals.
    pub fn total(&self) -> f32 {
        self.static_priority
            + self.dependency_pressure
            + self.goal_alignment
            + self.evaluation_feedback
            + self.actionability
            + self.test_strategy_bonus
            + self.okf_relevance
            + self.completion_history
            + self.complexity_penalty // already ≤ 0
    }
}

// ─── Context ───────────────────────────────────────────────────────────────────

/// Dynamic context for one ranking run — injected from the outer HOH loop.
#[derive(Debug, Clone, Default)]
pub struct PrioritizationContext {
    /// Current HOH goals (keywords matched against task text for alignment).
    pub goals: Vec<String>,
    /// Most recent Helix evaluation score (0.0 = poor, 1.0 = excellent).
    /// `None` means no evaluation data is available.
    pub helix_score: Option<f32>,
    /// OKF concept terms from recent lookups (for relevance scoring).
    pub okf_terms: Vec<String>,
    /// Aggregate completion statistics from `TaskCompletionTracker`.
    pub completion_stats: Option<CompletionStats>,
}

// ─── Output ────────────────────────────────────────────────────────────────────

/// One task after ranking: score, full signal breakdown, and human-readable
/// explanation lines (one per signal that contributed ≠ 0).
#[derive(Debug, Clone)]
pub struct RankedTask {
    pub task: Task,
    /// Composite score (sum of all weighted signals). Higher is better.
    pub score: f32,
    /// Individual signal contributions (before weights).
    pub signals: PrioritySignals,
    /// One explanation line per active signal — for logging and debugging.
    pub explanation: Vec<String>,
}

// ─── Model ─────────────────────────────────────────────────────────────────────

/// The standalone Task Prioritization Model.
///
/// Instantiate once, call [`rank`](Self::rank) or
/// [`score_task`](Self::score_task) as needed.
#[derive(Debug, Clone)]
pub struct TaskPrioritizationModel {
    pub weights: PrioritizationWeights,
}

impl Default for TaskPrioritizationModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskPrioritizationModel {
    /// Create a model with default weights.
    pub fn new() -> Self {
        Self {
            weights: PrioritizationWeights::default(),
        }
    }

    /// Create a model with custom weights.
    pub fn with_weights(weights: PrioritizationWeights) -> Self {
        Self { weights }
    }

    // ── Public API ──────────────────────────────────────────────────────────────

    /// Rank a slice of tasks, returning them sorted descending by score.
    ///
    /// Ties are broken by `task.id` ascending so the ordering is fully
    /// deterministic given the same inputs.
    ///
    /// * `graph` — optional dependency graph; used for the dependency-pressure
    ///   signal.  Pass `None` to skip that signal.
    /// * `ctx`   — dynamic context (goals, Helix score, OKF terms, history).
    pub fn rank(
        &self,
        tasks: &[Task],
        graph: Option<&TaskDependencyGraph>,
        ctx: &PrioritizationContext,
    ) -> Vec<RankedTask> {
        let pressure = Self::build_pressure_map(graph);

        let mut ranked: Vec<RankedTask> = tasks
            .iter()
            .map(|t| self.score_task(t, &pressure, ctx))
            .collect();

        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.task.id.cmp(&b.task.id))
        });

        ranked
    }

    /// Score a single task and return the full [`RankedTask`] (signals +
    /// explanation).  Useful when you want per-task detail without ranking a
    /// full list.
    pub fn score_task(
        &self,
        task: &Task,
        pressure: &HashMap<u64, usize>,
        ctx: &PrioritizationContext,
    ) -> RankedTask {
        let w = &self.weights;
        let mut sig = PrioritySignals::default();
        let mut explanation: Vec<String> = Vec::new();

        // ── 1. Static priority ─────────────────────────────────────────────────
        let priority_base: f32 = match task.priority.as_str() {
            "high" => 3.0,
            "medium" => 2.0,
            "low" => 1.0,
            other => {
                tracing::debug!(
                    "TaskPrioritizationModel: unknown priority '{}' for task {}",
                    other,
                    task.id
                );
                1.5
            }
        };
        sig.static_priority = priority_base * w.static_priority;
        explanation.push(format!(
            "priority={} → +{:.2}",
            task.priority, sig.static_priority
        ));

        // ── 2. Dependency pressure ─────────────────────────────────────────────
        let waiters = *pressure.get(&task.id).unwrap_or(&0);
        if waiters > 0 {
            sig.dependency_pressure = (waiters as f32).min(5.0) * w.dependency_pressure;
            explanation.push(format!(
                "{} waiter(s) → +{:.2}",
                waiters, sig.dependency_pressure
            ));
        }

        // ── 3. Goal alignment ──────────────────────────────────────────────────
        if !ctx.goals.is_empty() {
            let haystack = format!(
                "{} {} {}",
                task.title.to_lowercase(),
                task.description.to_lowercase(),
                task.details.to_lowercase()
            );
            let hits = ctx
                .goals
                .iter()
                .filter(|g| haystack.contains(g.to_lowercase().as_str()))
                .count();
            if hits > 0 {
                sig.goal_alignment = hits.min(4) as f32 * w.goal_alignment;
                explanation.push(format!(
                    "{} goal keyword(s) → +{:.2}",
                    hits, sig.goal_alignment
                ));
            }
        }

        // ── 4. Evaluation feedback (Helix score) ───────────────────────────────
        //
        // Strategy: if the last iteration scored well, modestly boost all tasks
        // (we're on a good trajectory, keep momentum).  If it scored poorly,
        // give a larger boost to tasks whose text mentions the failure domain
        // (e.g. "test", "fix", "debug", "quality").
        if let Some(helix) = ctx.helix_score {
            let helix_clamped = helix.clamp(0.0, 1.0);
            let text = format!("{} {}", task.title.to_lowercase(), task.details.to_lowercase());
            let is_remediation = text.contains("test")
                || text.contains("fix")
                || text.contains("debug")
                || text.contains("quality")
                || text.contains("error")
                || text.contains("fail");

            let raw = if helix_clamped >= 0.6 {
                // Good iteration — small global momentum boost
                (helix_clamped - 0.5) * w.evaluation_feedback
            } else {
                // Poor iteration — larger boost for remediation-flavoured tasks
                if is_remediation {
                    (0.6 - helix_clamped) * w.evaluation_feedback * 2.0
                } else {
                    0.0
                }
            };

            if raw.abs() > 0.01 {
                sig.evaluation_feedback = raw;
                explanation.push(format!(
                    "helix={:.2} {} → {}{:.2}",
                    helix_clamped,
                    if is_remediation { "(remediation)" } else { "" },
                    if raw >= 0.0 { "+" } else { "" },
                    raw
                ));
            }
        }

        // ── 5. Actionability ──────────────────────────────────────────────────
        let has_details = task.details.len() > 50;
        let has_strategy = !task.test_strategy.trim().is_empty();
        if has_details && has_strategy {
            sig.actionability = w.actionability;
            explanation.push(format!(
                "well-defined (details + testStrategy) → +{:.2}",
                sig.actionability
            ));
        } else if has_details || has_strategy {
            sig.actionability = w.actionability * 0.5;
            explanation.push(format!(
                "partially defined → +{:.2}",
                sig.actionability
            ));
        }

        // ── 6. Test-strategy bonus ─────────────────────────────────────────────
        if has_strategy && task.test_strategy.len() > 20 {
            sig.test_strategy_bonus = w.test_strategy_bonus;
            explanation.push(format!(
                "testStrategy present → +{:.2}",
                sig.test_strategy_bonus
            ));
        }

        // ── 7. OKF relevance ──────────────────────────────────────────────────
        if !ctx.okf_terms.is_empty() {
            let haystack = format!(
                "{} {} {}",
                task.title.to_lowercase(),
                task.description.to_lowercase(),
                task.details.to_lowercase()
            );
            let hits = ctx
                .okf_terms
                .iter()
                .filter(|term| haystack.contains(term.to_lowercase().as_str()))
                .count();
            if hits > 0 {
                sig.okf_relevance = hits.min(3) as f32 * w.okf_relevance;
                explanation.push(format!(
                    "{} OKF term(s) → +{:.2}",
                    hits, sig.okf_relevance
                ));
            }
        }

        // ── 8. Completion history ─────────────────────────────────────────────
        if let Some(stats) = &ctx.completion_stats {
            if stats.total_completed > 0 {
                let quality_rate =
                    stats.high_quality_count as f32 / stats.total_completed as f32;
                if quality_rate > 0.6 {
                    sig.completion_history = quality_rate * w.completion_history;
                    explanation.push(format!(
                        "history quality {:.0}% → +{:.2}",
                        quality_rate * 100.0,
                        sig.completion_history
                    ));
                }
            }
        }

        // ── 9. Complexity penalty ─────────────────────────────────────────────
        let detail_len = task.details.len();
        if detail_len > 1_000 {
            let excess_chunks = ((detail_len - 1_000) / 500) as f32;
            sig.complexity_penalty = -(excess_chunks.min(6.0) * w.complexity_penalty);
            explanation.push(format!(
                "details {} chars → {:.2}",
                detail_len, sig.complexity_penalty
            ));
        }

        let score = sig.total();
        RankedTask {
            task: task.clone(),
            score,
            signals: sig,
            explanation,
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────────

    /// Build a `task_id → number_of_waiters` map from a dependency graph.
    pub fn build_pressure_map(graph: Option<&TaskDependencyGraph>) -> HashMap<u64, usize> {
        graph
            .map(|g| {
                g.reverse_edges
                    .iter()
                    .map(|(&id, dependents)| (id, dependents.len()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Convenience: build a [`PrioritizationContext`] from the available data
    /// the HOH planner holds.
    pub fn context_from(
        goals: Vec<String>,
        helix_score: Option<f32>,
        okf_terms: Vec<String>,
        completion_stats: Option<CompletionStats>,
    ) -> PrioritizationContext {
        PrioritizationContext {
            goals,
            helix_score,
            okf_terms,
            completion_stats,
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::tasklist_adapter::Task;

    fn task(id: u64, title: &str, priority: &str, details: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            priority: priority.to_string(),
            status: "pending".to_string(),
            details: details.to_string(),
            test_strategy: "Run cargo test.".to_string(),
            ..Default::default()
        }
    }

    fn ctx() -> PrioritizationContext {
        PrioritizationContext::default()
    }

    // ── Priority ordering ───────────────────────────────────────────────────────

    #[test]
    fn high_ranks_above_low_when_otherwise_equal() {
        let model = TaskPrioritizationModel::new();
        let tasks = vec![
            task(1, "Low task", "low", "Short."),
            task(2, "High task", "high", "Short."),
        ];
        let ranked = model.rank(&tasks, None, &ctx());
        assert_eq!(ranked[0].task.id, 2, "high-priority task should rank first");
    }

    #[test]
    fn medium_ranks_between_high_and_low() {
        let model = TaskPrioritizationModel::new();
        let tasks = vec![
            task(1, "Low", "low", "x"),
            task(2, "Medium", "medium", "x"),
            task(3, "High", "high", "x"),
        ];
        let ranked = model.rank(&tasks, None, &ctx());
        let ids: Vec<u64> = ranked.iter().map(|r| r.task.id).collect();
        assert_eq!(ids, vec![3, 2, 1]);
    }

    // ── Dependency pressure ─────────────────────────────────────────────────────

    #[test]
    fn dependency_pressure_boosts_blocker() {
        use crate::hoh::task_dependency_graph::TaskDependencyGraph;

        let model = TaskPrioritizationModel::new();
        // Build a graph: task 2, 3, 4 all depend on task 1
        let tasks = vec![
            task(1, "Blocker", "medium", "x"),
            task(2, "Waiter A", "medium", "x"),
            task(3, "Waiter B", "medium", "x"),
            task(4, "Waiter C", "medium", "x"),
            task(5, "Free agent", "medium", "x"),
        ];

        let mut all = tasks.clone();
        all[1].dependencies = vec![1];
        all[2].dependencies = vec![1];
        all[3].dependencies = vec![1];

        let graph = TaskDependencyGraph::from_tasks(&all);
        let ranked = model.rank(&tasks[..2], Some(&graph), &ctx());

        let score_blocker = ranked.iter().find(|r| r.task.id == 1).unwrap().score;
        let score_free    = ranked.iter().find(|r| r.task.id == 2).unwrap().score;
        assert!(
            score_blocker > score_free,
            "blocker (id=1, 3 waiters) should outscore free agent (id=2)"
        );
    }

    // ── Goal alignment ──────────────────────────────────────────────────────────

    #[test]
    fn goal_keyword_boosts_matching_task() {
        let model = TaskPrioritizationModel::new();
        let tasks = vec![
            task(1, "Refactor the architecture layer", "medium", "Improve architecture."),
            task(2, "Write release notes", "medium", "Prepare changelog."),
        ];
        let c = PrioritizationContext {
            goals: vec!["architecture".to_string()],
            ..Default::default()
        };
        let ranked = model.rank(&tasks, None, &c);
        assert_eq!(ranked[0].task.id, 1, "architecture-matching task should rank first");
    }

    // ── Helix feedback ──────────────────────────────────────────────────────────

    #[test]
    fn good_helix_score_boosts_all_tasks() {
        let model = TaskPrioritizationModel::new();
        let t = task(1, "Some task", "medium", "Details.");
        let c_helix = PrioritizationContext {
            helix_score: Some(0.9),
            ..Default::default()
        };
        let c_none = PrioritizationContext::default();

        let with_helix = model.score_task(&t, &HashMap::new(), &c_helix);
        let without    = model.score_task(&t, &HashMap::new(), &c_none);
        assert!(
            with_helix.score > without.score,
            "good Helix score should boost task score"
        );
    }

    #[test]
    fn poor_helix_score_boosts_remediation_tasks() {
        let model = TaskPrioritizationModel::new();
        let remediation = task(1, "Fix failing tests", "medium", "Debug test failures.");
        let normal       = task(2, "Add new feature", "medium", "Implement feature X.");

        let c = PrioritizationContext {
            helix_score: Some(0.2), // poor
            ..Default::default()
        };
        let r_remediation = model.score_task(&remediation, &HashMap::new(), &c);
        let r_normal      = model.score_task(&normal, &HashMap::new(), &c);
        assert!(
            r_remediation.score > r_normal.score,
            "remediation task should score higher when Helix is poor"
        );
    }

    // ── OKF relevance ───────────────────────────────────────────────────────────

    #[test]
    fn okf_terms_boost_relevant_tasks() {
        let model = TaskPrioritizationModel::new();
        let relevant = task(1, "Implement OKF bundle sync", "medium", "Use okf_lookup.");
        let unrelated = task(2, "Update README", "medium", "Edit docs.");
        let c = PrioritizationContext {
            okf_terms: vec!["okf".to_string(), "bundle".to_string()],
            ..Default::default()
        };
        let ranked = model.rank(&[relevant, unrelated], None, &c);
        assert_eq!(ranked[0].task.id, 1);
    }

    // ── Complexity penalty ──────────────────────────────────────────────────────

    #[test]
    fn large_task_penalised() {
        let model = TaskPrioritizationModel::new();
        let big   = task(1, "Big task", "high", &"x".repeat(3_000));
        let small = task(2, "Small task", "high", "Short details.");

        let r_big   = model.score_task(&big, &HashMap::new(), &ctx());
        let r_small = model.score_task(&small, &HashMap::new(), &ctx());

        assert!(
            r_small.score > r_big.score,
            "small (focused) task should outscore a huge task at equal priority"
        );
        assert!(
            r_big.signals.complexity_penalty < 0.0,
            "big task should have a negative complexity_penalty signal"
        );
    }

    // ── Completion history ──────────────────────────────────────────────────────

    #[test]
    fn high_quality_history_boosts_score() {
        let model = TaskPrioritizationModel::new();
        let t = task(1, "Some task", "medium", "Details.");

        let c_with_history = PrioritizationContext {
            completion_stats: Some(CompletionStats {
                total_completed: 10,
                high_quality_count: 9, // 90%
                ..Default::default()
            }),
            ..Default::default()
        };
        let c_no_history = PrioritizationContext::default();

        let with = model.score_task(&t, &HashMap::new(), &c_with_history);
        let without = model.score_task(&t, &HashMap::new(), &c_no_history);
        assert!(
            with.score > without.score,
            "high-quality completion history should boost score"
        );
    }

    // ── Explanation ─────────────────────────────────────────────────────────────

    #[test]
    fn explanation_covers_active_signals() {
        let model = TaskPrioritizationModel::new();
        let t = task(1, "Architecture task", "high", "Implementation details here.");
        let c = PrioritizationContext {
            goals: vec!["architecture".to_string()],
            helix_score: Some(0.8),
            okf_terms: vec!["architecture".to_string()],
            ..Default::default()
        };
        let ranked = model.score_task(&t, &HashMap::new(), &c);
        assert!(!ranked.explanation.is_empty());
        assert!(
            ranked.explanation.iter().any(|e| e.contains("priority")),
            "priority signal must appear in explanation"
        );
        assert!(
            ranked.explanation.iter().any(|e| e.contains("goal")),
            "goal alignment signal must appear in explanation"
        );
    }

    // ── Stability (determinism) ─────────────────────────────────────────────────

    #[test]
    fn same_inputs_produce_same_ordering() {
        let model = TaskPrioritizationModel::new();
        let tasks = vec![
            task(10, "Alpha", "medium", "x"),
            task(20, "Beta", "high", "y"),
            task(30, "Gamma", "low", "z"),
        ];
        let c = PrioritizationContext::default();
        let r1 = model.rank(&tasks, None, &c);
        let r2 = model.rank(&tasks, None, &c);
        let ids1: Vec<u64> = r1.iter().map(|r| r.task.id).collect();
        let ids2: Vec<u64> = r2.iter().map(|r| r.task.id).collect();
        assert_eq!(ids1, ids2, "ranking must be deterministic");
    }

    // ── Custom weights ──────────────────────────────────────────────────────────

    #[test]
    fn custom_weights_change_relative_scores() {
        // Disable all signals except goal_alignment
        let w = PrioritizationWeights {
            static_priority: 0.0,
            dependency_pressure: 0.0,
            goal_alignment: 10.0, // only this matters
            evaluation_feedback: 0.0,
            actionability: 0.0,
            test_strategy_bonus: 0.0,
            okf_relevance: 0.0,
            completion_history: 0.0,
            complexity_penalty: 0.0,
        };
        let model = TaskPrioritizationModel::with_weights(w);
        let tasks = vec![
            task(1, "Matches the goal", "low", "goal-relevant content"),
            task(2, "Unrelated task", "high", "nothing relevant"),
        ];
        let c = PrioritizationContext {
            goals: vec!["goal".to_string()],
            ..Default::default()
        };
        let ranked = model.rank(&tasks, None, &c);
        assert_eq!(
            ranked[0].task.id, 1,
            "with goal_alignment-only weights, matching task should win over higher priority"
        );
    }

    // ── context_from helper ─────────────────────────────────────────────────────

    #[test]
    fn context_from_builds_correctly() {
        let ctx = TaskPrioritizationModel::context_from(
            vec!["testing".to_string()],
            Some(0.7),
            vec!["okf_term".to_string()],
            None,
        );
        assert_eq!(ctx.goals.len(), 1);
        assert_eq!(ctx.helix_score, Some(0.7));
        assert_eq!(ctx.okf_terms.len(), 1);
        assert!(ctx.completion_stats.is_none());
    }
}
