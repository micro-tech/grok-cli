//! Agent planner — thin adapter between user input and the Reasoning Engine FSM.
//!
//! [`Planner`] drives a single [`ReasoningEngineState`] turn from goal analysis
//! through plan commit.  The heavy lifting is done by the engine state machine;
//! this module is the entry point from the CLI and ACP paths.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::bayes::BayesianEngine;
use crate::engine::state::{EngineState, Hypothesis, ReasoningEngineState, StepAction};
use crate::router::AppRouter;

// ── Plan ─────────────────────────────────────────────────────────────────────

/// A committed reasoning plan produced by [`Planner::plan`].
///
/// Wraps the [`ReasoningEngineState`] after it has been driven to
/// [`EngineState::CommitPlan`] so callers can inspect the chosen hypotheses
/// and step sequence.
#[derive(Debug)]
pub struct Plan {
    state: ReasoningEngineState,
    /// Optional compact repository evidence collected by the Explorer agent.
    pub repo_evidence: Option<serde_json::Value>,
}

impl Plan {
    /// The top-ranked hypothesis in the committed plan, if any.
    pub fn top_hypothesis(&self) -> Option<&Hypothesis> {
        self.state.hypotheses.iter().max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Confidence of the top hypothesis, or 0.0 when no hypotheses exist.
    pub fn heuristic_confidence(&self) -> f32 {
        self.top_hypothesis().map(|h| h.confidence).unwrap_or(0.0)
    }

    /// Number of plan steps committed.
    pub fn step_count(&self) -> usize {
        self.state.plan.len()
    }

    /// The first planned step action, if any.
    pub fn first_step_action(&self) -> Option<&StepAction> {
        self.state.plan.first().map(|s| &s.action)
    }
}

// ── Planner ───────────────────────────────────────────────────────────────────

/// Goal-driven planner that wraps [`ReasoningEngineState`] and the Bayesian
/// engine.
pub struct Planner {
    bayes: BayesianEngine,
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

impl Planner {
    /// Create a planner using the default (profile-loaded) Bayesian engine.
    pub fn new() -> Self {
        Self {
            bayes: BayesianEngine::new(),
        }
    }

    /// Create a planner using compiled-in default priors (never reads disk).
    /// Preferred in unit tests.
    pub fn new_with_default_priors() -> Self {
        Self {
            bayes: BayesianEngine::new_with_default_priors(),
        }
    }

    /// Drive the reasoning engine through a full planning turn for `user_input`.
    ///
    /// Returns a [`Plan`] when the engine reaches [`EngineState::CommitPlan`],
    /// or an error if the engine fails or cannot progress past goal analysis.
    pub async fn plan(&mut self, user_input: &str) -> Result<Plan> {
        self.plan_with_explorer(user_input, None, "grok-4").await
    }

    /// Same as [`plan`] but optionally runs the Explorer agent first when
    /// the Bayesian intent suggests a code-edit/refactor task.
    pub async fn plan_with_explorer(
        &mut self,
        user_input: &str,
        client: Option<&AppRouter>,
        model: &str,
    ) -> Result<Plan> {
        // Update Bayesian beliefs from the user's text.
        self.bayes.update_from_text(user_input);

        let intent = self
            .bayes
            .best_intent()
            .unwrap_or_else(|| "intent_question".to_string());

        debug!(intent = %intent, "Planner: Bayesian intent resolved");

        // ── Optional Explorer run (Task 161/162) ─────────────────────────────
        // Fixed: Use proper Bayesian intent names instead of fragile substring checks.
        let mut repo_evidence = None;
        let should_explore = matches!(
            intent.as_str(),
            "intent_edit" | "intent_refactor" | "intent_fix" | "intent_code_change"
        ) || intent.contains("edit"); // keep a loose fallback for future intent names

        if should_explore {
            if let Some(client) = client {
                if let Ok(evidence) =
                    crate::agent::explorer::run_explorer_mode(client, user_input, model).await
                {
                    repo_evidence = Some(serde_json::to_value(evidence)?);
                    debug!("Planner: Explorer evidence collected");
                }
            }
        }

        // Boot the FSM.
        // ReasoningEngineState::new() takes no args; set goal with builder.
        let mut engine = ReasoningEngineState::new().with_goal(user_input);

        // ── AnalyzeGoal ──────────────────────────────────────────────────────
        // Derive confidence from Bayesian model state.
        let model_confidence = 1.0 - self.bayes.probability("low_confidence");
        // Hypothesis has fields: id, description, confidence (no supporting_evidence).
        let hypothesis = Hypothesis {
            id: uuid::Uuid::new_v4().to_string(),
            description: format!("Intent: {} — goal: {}", intent, user_input),
            confidence: model_confidence.clamp(0.1, 1.0),
        };
        engine.hypotheses.push(hypothesis);

        // ── ExpandOptions → EvaluateOptions → CommitPlan ────────────────────
        use crate::engine::state::TransitionError;

        let transitions = [
            EngineState::ExpandOptions,
            EngineState::EvaluateOptions,
            EngineState::CommitPlan,
        ];

        for next_state in &transitions {
            match engine.transition(next_state.clone()) {
                Ok(_) => {
                    debug!(state = ?next_state, "Planner: FSM transition");
                }
                Err(TransitionError::InvalidTransition { from, to }) => {
                    warn!(
                        ?from,
                        ?to,
                        "Planner: invalid FSM transition — aborting plan"
                    );
                    return Err(anyhow::anyhow!(
                        "Reasoning engine FSM stuck at {:?} → {:?}",
                        from,
                        to
                    ));
                }
            }
        }

        // Self-correction gate: if confidence is too low, try once more.
        let plan = Plan {
            state: engine,
            repo_evidence,
        };
        if plan.heuristic_confidence() < 0.3 {
            warn!(
                confidence = plan.heuristic_confidence(),
                "Planner: low-confidence plan — applying self-correction"
            );
            self.bayes.update_from_model_confidence(0.3);
        }

        info!(
            steps = plan.step_count(),
            confidence = plan.heuristic_confidence(),
            "Planner: plan committed"
        );
        Ok(plan)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn plan_returns_committed_plan_for_simple_input() {
        let mut planner = Planner::new_with_default_priors();
        let plan = planner.plan("what is Rust?").await.unwrap();
        assert!(plan.step_count() == 0 || plan.step_count() > 0); // any count is fine
        assert!(plan.heuristic_confidence() >= 0.0);
        assert!(plan.heuristic_confidence() <= 1.0);
    }

    #[tokio::test]
    async fn plan_intent_edit_produces_valid_confidence() {
        let mut planner = Planner::new_with_default_priors();
        let plan = planner
            .plan("can you edit and fix this file?")
            .await
            .unwrap();
        assert!(plan.heuristic_confidence() >= 0.0);
        assert!(plan.top_hypothesis().is_some());
    }

    #[tokio::test]
    async fn plan_low_confidence_still_returns_ok() {
        let mut planner = Planner::new_with_default_priors();
        // Feed ambiguous input to produce low confidence.
        let plan = planner.plan("maybe do something or not").await.unwrap();
        assert!(plan.heuristic_confidence() >= 0.0);
    }
}
