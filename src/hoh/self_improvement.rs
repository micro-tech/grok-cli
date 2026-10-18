//! Evaluation-Driven Self-Improvement (Task 411)
//!
//! Turns evaluation results, patch outcomes, and history into concrete,
//! reviewable proposals for improving prompts, heuristics, and agent behavior.
//!
//! All proposals are human-review gated. No automatic application.

use crate::hoh::state::IterationState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImprovementTarget {
    PlannerPrompt,
    HeuristicScoring,
    TestStrategyEnforcement,
    PatchQuality,
    AgentBehavior,
    ContextHandling,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementProposal {
    pub id: String,
    pub target: ImprovementTarget,
    pub title: String,
    pub description: String,
    pub rationale: String,
    pub confidence: f32, // 0.0 - 1.0
    pub expected_impact: String,
    pub suggested_change: String,
}

impl SelfImprovementProposal {
    pub fn new(
        target: ImprovementTarget,
        title: impl Into<String>,
        description: impl Into<String>,
        rationale: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            target,
            title: title.into(),
            description: description.into(),
            rationale: rationale.into(),
            confidence: confidence.clamp(0.0, 1.0),
            expected_impact: String::new(),
            suggested_change: String::new(),
        }
    }

    pub fn with_suggestion(mut self, change: impl Into<String>, impact: impl Into<String>) -> Self {
        self.suggested_change = change.into();
        self.expected_impact = impact.into();
        self
    }
}

/// Main entry point for Task 411.
/// Analyzes the current iteration state (especially the latest evaluation + history)
/// and produces a small number of concrete, actionable self-improvement proposals.
pub fn generate_self_improvement_proposals(state: &IterationState) -> Vec<SelfImprovementProposal> {
    let mut proposals = Vec::new();

    let last_eval = state.evaluations.last();

    // === Signal 1: Low meta-improvement or declining quality ===
    if let Some(eval) = last_eval {
        if let Some(meta) = eval.meta_improvement_score {
            if meta < 0.65 {
                proposals.push(
                    SelfImprovementProposal::new(
                        ImprovementTarget::HeuristicScoring,
                        "Strengthen test_strategy signal in prioritization",
                        "Recent meta score is low. Many cycles are producing changes without strong test coverage.",
                        format!("Meta-improvement score was only {:.2}. We should bias the planner harder toward tasks that declare a testStrategy.", meta),
                        0.78,
                    )
                    .with_suggestion(
                        "In task_prioritization.rs and planner, add +0.15–0.25 boost when task has non-empty test_strategy.",
                        "Higher quality changes and fewer regressions in future iterations.",
                    ),
                );
            }
        }

        // Patch quality signals
        if eval.avg_diff_length > 900.0 && eval.patch_count > 2 {
            proposals.push(
                SelfImprovementProposal::new(
                    ImprovementTarget::PatchQuality,
                    "Add patch size / focus heuristic",
                    "We are producing large diffs on average. This increases risk and review burden.",
                    format!("Average diff length was {:.0} chars across {} patches.", eval.avg_diff_length, eval.patch_count),
                    0.72,
                )
                .with_suggestion(
                    "In execution / patch_capture phases, warn or down-score when a single change touches > 4 files or > 600 chars of diff.",
                    "Smaller, more focused changes that are easier to validate.",
                ),
            );
        }

        if let Some(false) = eval.test_passed {
            proposals.push(
                SelfImprovementProposal::new(
                    ImprovementTarget::TestStrategyEnforcement,
                    "Require test_strategy before materializing refactors",
                    "Tests failed this cycle. We should not proceed with refactoring actions unless a test_strategy is present.",
                    "Real test failures were observed. The loop is currently too eager to mutate code.",
                    0.85,
                )
                .with_suggestion(
                    "In autonomous_refactoring and planner materialization: only promote actions to materialized tasks if the source task has a non-trivial testStrategy field.",
                    "Fewer broken refactors and stronger regression protection.",
                ),
            );
        }
    }

    // === Signal 2: Too many patches / thrashing ===
    if state.patches.len() > 6 {
        proposals.push(
            SelfImprovementProposal::new(
                ImprovementTarget::AgentBehavior,
                "Cap number of patches per iteration",
                "High patch volume often indicates the agent is thrashing instead of focusing.",
                format!("{} patches were generated this iteration.", state.patches.len()),
                0.65,
            )
            .with_suggestion(
                "Add a soft cap (e.g. 5) in the execution loop and HOH planner. Prefer quality over quantity.",
                "Cleaner iterations and easier post-mortems.",
            ),
        );
    }

    // === Signal 3: Recurring improvement_suggestions theme ===
    if let Some(plan) = &state.plan {
        let has_test_focus = plan.improvement_suggestions.iter().any(|s| {
            s.to_lowercase().contains("test") || s.to_lowercase().contains("strategy")
        });

        if has_test_focus && proposals.iter().all(|p| !matches!(p.target, ImprovementTarget::TestStrategyEnforcement)) {
            proposals.push(
                SelfImprovementProposal::new(
                    ImprovementTarget::PlannerPrompt,
                    "Inject 'Write test_strategy first' rule into planner system prompt",
                    "Multiple cycles are surfacing the need for better test planning.",
                    "The continual improvement loop keeps recommending stronger test_strategy usage.",
                    0.70,
                )
                .with_suggestion(
                    "Add a permanent rule in the HOH planner prompt: 'Before proposing any code change or refactor, explicitly state a test_strategy (even if simple).'",
                    "Agents will naturally produce better-validated work.",
                ),
            );
        }
    }

    // Keep it small and high-signal (max 5)
    proposals.truncate(5);

    // Add a stable id prefix for easier tracking
    for (i, p) in proposals.iter_mut().enumerate() {
        if !p.id.starts_with("411-") {
            p.id = format!("411-{}", p.id);
        }
        if p.title.is_empty() {
            p.title = format!("Self-improvement #{i}");
        }
    }

    proposals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::state::{EvaluationReport, IterationState};

    #[test]
    fn produces_proposals_on_low_meta_score() {
        let mut state = IterationState::new(1);
        state.evaluations.push(EvaluationReport {
            meta_improvement_score: Some(0.52),
            ..Default::default()
        });

        let props = generate_self_improvement_proposals(&state);
        assert!(!props.is_empty());
        assert!(props.iter().any(|p| matches!(p.target, ImprovementTarget::HeuristicScoring)));
    }

    #[test]
    fn produces_patch_quality_proposal_on_large_diffs() {
        let mut state = IterationState::new(2);
        state.evaluations.push(EvaluationReport {
            avg_diff_length: 1200.0,
            patch_count: 4,
            ..Default::default()
        });

        let props = generate_self_improvement_proposals(&state);
        assert!(props.iter().any(|p| matches!(p.target, ImprovementTarget::PatchQuality)));
    }
}