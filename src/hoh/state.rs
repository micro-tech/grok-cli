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
    pub fn content_to_apply(&self) -> String {
        self.intended_content
            .clone()
            .unwrap_or_else(|| self.diff_summary.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationReport {
    pub iteration_id: u64,
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