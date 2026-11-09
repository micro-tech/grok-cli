//! HOH Iteration State and related data structures (Task 297.2)

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IterationState {
    pub iteration_id: u64,
    pub started_at: u64,
    pub status: IterationStatus,
    pub plan: Option<HOHPlan>,
    pub patches: Vec<PatchSet>,
    pub evaluations: Vec<EvaluationReport>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum IterationStatus {
    #[default]
    Planning,
    Executing,
    Testing,
    Evaluating,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HOHPlan {
    pub goals: Vec<String>,
    pub selected_tasks: Vec<u64>,
    pub experiments: Vec<String>,
    /// Architectural evolution proposals generated this cycle (361.1)
    #[serde(default)]
    pub architecture_proposals: Vec<String>,
    /// Self-refinement proposals for HOH internals (361.2)
    #[serde(default)]
    pub self_refinement_proposals: Vec<String>,
    /// Concrete refactoring actions from 361.3
    #[serde(default)]
    pub refactoring_actions: Vec<String>,
    /// A: Materialized new task IDs from refactoring actions this cycle
    #[serde(default)]
    pub materialized_task_ids: Vec<u64>,
    /// C: Patch stubs generated from high-confidence refactoring actions (361.3)
    #[serde(default)]
    pub generated_patch_stubs: Vec<String>,
    /// D/E: Specialized sub-agent routings performed (361.4)
    #[serde(default)]
    pub specialized_agent_routes: Vec<String>,
    /// 361.5: Continual / meta improvement suggestions generated at the end of previous cycle
    #[serde(default)]
    pub improvement_suggestions: Vec<String>,
    /// 401: Novel creative ideas generated this cycle
    #[serde(default)]
    pub creative_ideas: Vec<crate::hoh::creativity::CreativityIdea>,
    /// 402: Generative architecture designs proposed this cycle (built on top of 401)
    #[serde(default)]
    pub architecture_designs: Vec<crate::hoh::generative_designer::ArchitectureDesign>,
    /// 403: Agent retirement / hibernation decisions and events this cycle
    #[serde(default)]
    pub agent_lifecycle_events: Vec<crate::hoh::agent_lifecycle::LifecycleEvent>,
    /// 404: Agent births — newly spawned specialized agents this cycle
    #[serde(default)]
    pub agent_birth_events: Vec<crate::hoh::agent_birth::BirthEvent>,
    /// 405: Agent evolution events this cycle
    #[serde(default)]
    pub agent_evolution_events: Vec<crate::hoh::agent_evolution::EvolutionEvent>,
    /// 406: Multi-domain reasoning outputs
    #[serde(default)]
    pub multi_domain_outputs: Vec<crate::hoh::multi_domain::DomainOutput>,
    /// 407: Governance decisions / blocks
    #[serde(default)]
    pub governance_decisions: Vec<String>,
    /// 408: Ethics check results
    #[serde(default)]
    pub ethics_checks: Vec<String>,
    /// 409: Meta-plans for improving HOH itself
    #[serde(default)]
    pub meta_plans: Vec<crate::hoh::meta_planning::MetaPlan>,
    /// 410: Meta-evaluation of HOH performance
    #[serde(default)]
    pub meta_evaluation: Option<crate::hoh::meta_evaluation::MetaEvaluation>,

    /// 361.8: Multi-agent simulation outcomes / what-if predictions for this cycle
    #[serde(default)]
    pub simulation_outcomes: Vec<crate::hoh::multi_agent_simulation::SimulationOutcome>,

    /// 361.7 + 361.8: Results from the unified Multi-Agent Orchestrator (delegation, simulation, skill evolution)
    #[serde(default)]
    pub orchestration_results: Vec<crate::hoh::multi_agent_orchestrator::OrchestrationResult>,

    /// 361.9: Long-term strategic goals that span many iterations (Harness-of-Harness long-horizon direction)
    #[serde(default)]
    pub long_term_strategies: Vec<crate::hoh::long_term_strategy::StrategicGoal>,

    /// 361.11: Transferable patterns extracted from this project for use in other projects
    #[serde(default)]
    pub cross_project_patterns: Vec<crate::hoh::cross_project_knowledge::TransferablePattern>,

    /// 361.11: Patterns from other projects that look applicable here (via OKF / memory)
    #[serde(default)]
    pub cross_project_transfers: Vec<crate::hoh::cross_project_knowledge::CrossProjectTransfer>,

    /// 361.0101 / 370: Multi-Project Orchestrator results (work that spans multiple projects)
    #[serde(default)]
    pub multi_project_result: Option<crate::hoh::multi_project_orchestrator::MultiProjectOrchestrationResult>,

    /// 361.0101: Projects currently known to the multi-project orchestrator in this plan
    #[serde(default)]
    pub registered_projects: Vec<crate::hoh::multi_project_orchestrator::ProjectRef>,

    /// 453 / 401: Creative ideas turned into real TaskMutation::AddTask (so they become actual tasks)
    #[serde(default)]
    pub creative_task_mutations: Vec<crate::hoh::task_mutation::TaskMutation>,

    /// 411: Evaluation-driven self-improvement proposals generated this cycle.
    /// These are concrete, reviewable suggestions for prompts, heuristics, or agent behavior.
    #[serde(default)]
    pub self_improvement_proposals: Vec<crate::hoh::self_improvement::SelfImprovementProposal>,

    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatchSet {
    pub id: String,
    pub files_changed: Vec<String>,
    pub diff_summary: String,
    pub source: String, // "inner_harness", "hoh", etc.
    pub timestamp: u64,
    /// Optional: when we want the patch to carry the *intended full content* for files.
    /// Used by patch_applier when present. Falls back to diff_summary if empty.
    #[serde(default)]
    pub intended_content: Option<String>,
}

impl PatchSet {
    /// Helper to get the content that should be written for this patch.
    ///
    /// Kept for callers that only preview content length. Prefer
    /// [`PatchSet::content_for`] when actually writing files, since it is
    /// correct for multi-file patches.
    pub fn content_to_apply(&self) -> String {
        self.intended_content
            .clone()
            .unwrap_or_else(|| self.diff_summary.clone())
    }

    /// Resolve the content to write for a specific file in this patch.
    ///
    /// A `PatchSet` carries a single `intended_content` blob, which is only
    /// meaningful when the patch targets exactly one file. Writing that blob to
    /// every entry of a multi-file `files_changed` would overwrite them all with
    /// identical content (silent data loss), so this returns `None` for
    /// multi-file patches until per-file content is modeled.
    pub fn content_for(&self, _path: &str) -> Option<String> {
        if self.files_changed.len() != 1 {
            return None;
        }
        match &self.intended_content {
            Some(content) => Some(content.clone()),
            None if !self.diff_summary.trim().is_empty() => Some(self.diff_summary.clone()),
            None => None,
        }
    }
}

/// The apply-gate threshold for the meta-improvement score (H7/H10).
/// An iteration may only apply real patches when its meta score is
/// strictly above this value. It is a named constant (not a magic number)
/// so the FullAuto nudge (H10) and any other consumer cannot silently
/// re-derive it from a different threshold.
pub const META_SCORE_APPLY_GATE: f32 = 0.70;

/// Neutral base for the meta-improvement score (H7). Deliberately well
/// below [`META_SCORE_APPLY_GATE`]: an iteration that only produces
/// artifacts scores here and does not open the apply gate.
pub const META_SCORE_BASE: f32 = 0.50;

/// Bonus when the iteration's tests pass (green is maintained, not
/// improved). Small on purpose: passing tests alone must not open the gate.
pub const META_SCORE_TEST_PASS_BONUS: f32 = 0.05;

/// Bonus when a previously-failing test now passes — the one concrete,
/// auditable improvement signal wired into the score today (H7). Combined
/// with the base and the pass bonus it is the only path above the gate:
/// 0.50 + 0.05 + 0.20 = 0.75 > 0.70.
pub const META_SCORE_IMPROVEMENT_BONUS: f32 = 0.20;

/// Penalty when the iteration's tests fail.
pub const META_SCORE_TEST_FAIL_PENALTY: f32 = 0.10;

/// The canonical meta-improvement score (H7/H11).
///
/// This is the ONLY meta-score computation in the codebase. `evaluate_phase`
/// calls it, and the former duplicate
/// (`ArchitectureEvolutionEngine::compute_meta_improvement_score`, a
/// proposal-count/success-rate composite) was deleted. If a new outcome
/// signal is wired in (clippy warning count, a task-list bug closed),
/// extend THIS function — do not add a second scorer.
///
/// Formula (all inputs are test outcomes, the only outcome signal the
/// evaluator receives today):
/// - base 0.50 (neutral, below the 0.70 gate)
/// - tests pass: +0.05
/// - previously-failing tests now pass: +0.20 (real, verifiable improvement)
/// - tests fail: -0.10
/// - no test data: +0.00
///
/// Activity is deliberately NOT scored: producing tasks, patch stubs,
/// routes, or refactoring actions earns nothing. An iteration that only
/// produces artifacts scores 0.50–0.55 and does not open the apply gate.
/// Only a demonstrated improvement (a failing test now passing) crosses
/// it, at 0.75.
pub fn compute_meta_score(test_passed: Option<bool>, prev_test_passed: Option<bool>) -> f32 {
    let mut score = META_SCORE_BASE;
    match test_passed {
        Some(true) => {
            score += META_SCORE_TEST_PASS_BONUS;
            if prev_test_passed == Some(false) {
                score += META_SCORE_IMPROVEMENT_BONUS;
            }
        }
        Some(false) => {
            score -= META_SCORE_TEST_FAIL_PENALTY;
        }
        None => {}
    }
    score.clamp(0.0, 0.95)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationReport {
    pub iteration_id: u64,
    /// Local patch-shape score (0.4–0.92), NOT a Helix judgment. It is
    /// produced by `hoh::helix::evaluate_patch_shape`, a local heuristic
    /// over patch count / diff size / file diversity. No live Helix call is
    /// made (H8). Consumers use it as a weak patch-quality signal only.
    pub helix_score: Option<f32>,
    pub internal_metrics: std::collections::HashMap<String, f32>,
    pub notes: String,
    /// 361: Architectural proposals considered this iteration
    #[serde(default)]
    pub architecture_proposals_evaluated: Vec<String>,
    /// 361.2: Meta-level improvement score (how much HOH improved its own capabilities)
    #[serde(default)]
    pub meta_improvement_score: Option<f32>,
    /// 361.5 / patch quality: lightweight metrics on patches produced this iteration
    #[serde(default)]
    pub patch_count: usize,
    #[serde(default)]
    pub files_changed_count: usize,
    #[serde(default)]
    pub avg_diff_length: f32,
    #[serde(default)]
    pub test_passed: Option<bool>,

    // === Rich testing feedback (next HOH hardening step) ===
    /// Full or substantial raw test output (from run_basic_tests or real harness test run).
    /// This is what gets fed into continual improvement and 361.5 meta loops.
    /// Previously we only kept a bool + tiny summary — now we keep the actual output
    /// so the planner / meta systems can see real compiler errors, failures, etc.
    #[serde(default)]
    pub test_output: Option<String>,

    /// A concise but informative summary extracted from the test run.
    #[serde(default)]
    pub test_summary: String,
}

impl IterationState {
    pub fn new(iteration_id: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            iteration_id,
            started_at: now,
            status: IterationStatus::Planning,
            ..Default::default()
        }
    }

    pub fn mark_completed(&mut self, summary: String) {
        self.status = IterationStatus::Completed;
        self.summary = Some(summary);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HOHError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Other: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_score_artifacts_only_stays_below_gate() {
        // H7: activity earns nothing. Tests green (maintained, not improved)
        // and no test data both score below the apply gate.
        assert!(compute_meta_score(Some(true), Some(true)) < META_SCORE_APPLY_GATE);
        assert!(compute_meta_score(Some(true), None) < META_SCORE_APPLY_GATE);
        assert!(compute_meta_score(None, None) < META_SCORE_APPLY_GATE);
        // Failing tests score below the gate too.
        assert!(compute_meta_score(Some(false), Some(true)) < META_SCORE_APPLY_GATE);
    }

    #[test]
    fn meta_score_real_improvement_crosses_gate() {
        // H7: the only path above the gate is a demonstrated improvement —
        // a previously-failing test now passing.
        let score = compute_meta_score(Some(true), Some(false));
        assert!(
            score > META_SCORE_APPLY_GATE,
            "improvement should cross the gate, got {score}"
        );
        // And it is exactly base + pass + improvement (auditable formula).
        assert!((score - (META_SCORE_BASE + META_SCORE_TEST_PASS_BONUS + META_SCORE_IMPROVEMENT_BONUS)).abs() < f32::EPSILON);
    }

    #[test]
    fn meta_score_maintained_green_does_not_cross_gate() {
        // Passing tests that were already passing is maintenance, not
        // improvement — it must not open the apply gate.
        let score = compute_meta_score(Some(true), Some(true));
        assert!((score - (META_SCORE_BASE + META_SCORE_TEST_PASS_BONUS)).abs() < f32::EPSILON);
        assert!(score < META_SCORE_APPLY_GATE);
    }
}
