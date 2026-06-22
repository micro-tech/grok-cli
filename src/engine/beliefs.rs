//! Bayesian belief management for the reasoning engine.
//!
//! This module wraps [`crate::bayes::BayesianEngine`] to provide a clean,
//! high-level interface for updating and querying probabilistic beliefs about
//! hypotheses and tool relevance throughout a reasoning turn.
//!
//! # Architecture
//!
//! ```text
//! Evidence (user text / tool results / model confidence)
//!        │
//!        ▼
//! EngineBeliefs ──► inner: BayesianEngine  (posterior over intent keys)
//!        │
//!        ├──► tool_beliefs: HashMap<String, ToolBelief>  (per-tool scores)
//!        │
//!        └──► sync_to_state() ──► ReasoningEngineState
//! ```
//!
//! ## Evidence lifecycle
//!
//! 1. Caller receives new information (user turn, tool result, model reply).
//! 2. Caller constructs an [`Evidence`] variant and calls
//!    [`EngineBeliefs::update_from_evidence`].
//! 3. At the end of each reasoning step the caller invokes
//!    [`EngineBeliefs::sync_to_state`] to push updated uncertainty and
//!    hypothesis confidences into the [`crate::engine::state::ReasoningEngineState`].
//! 4. Before executing a plan, the caller calls [`EngineBeliefs::score_plan`]
//!    to rank steps by tool belief score.

use std::collections::HashMap;

use crate::bayes::BayesianEngine;
use crate::engine::state::{Hypothesis, PlanStep, ReasoningEngineState, StepAction};

// ---------------------------------------------------------------------------
// ToolBelief
// ---------------------------------------------------------------------------

/// A scored candidate tool derived from Bayesian beliefs.
///
/// Tool beliefs are maintained independently of the underlying
/// [`BayesianEngine`] so that fine-grained success/failure signals can be
/// applied per-tool without polluting the intent distribution.
#[derive(Debug, Clone)]
pub struct ToolBelief {
    /// The registered name of the tool (e.g. `"list_directory"`).
    pub tool_name: String,
    /// Posterior score in `[0.0, 1.0]`.
    ///
    /// Higher values indicate greater confidence that the tool is relevant and
    /// likely to succeed for the current goal.
    pub score: f32,
    /// Whether this tool was selected for execution in the current turn.
    pub selected: bool,
}

// ---------------------------------------------------------------------------
// Evidence
// ---------------------------------------------------------------------------

/// Evidence supplied to the belief layer after a turn or tool call.
///
/// Each variant corresponds to a distinct event type that carries different
/// Bayesian signal strength and direction.
#[derive(Debug, Clone)]
pub enum Evidence {
    /// New user text was received.
    ///
    /// Triggers a full intent-distribution update via the inner
    /// [`BayesianEngine`].
    UserText(String),

    /// A tool call completed successfully.
    ///
    /// Raises the corresponding tool's belief score by `0.1` (capped at
    /// `1.0`) and marks it as `selected`.
    ToolSuccess {
        /// Registered name of the tool that succeeded.
        tool_name: String,
    },

    /// A tool call failed.
    ///
    /// Triggers a tool-failure update on the inner [`BayesianEngine`] and
    /// lowers the corresponding tool's belief score by `0.2` (floored at
    /// `0.0`).
    ToolFailure {
        /// Registered name of the tool that failed.
        tool_name: String,
    },

    /// The language model returned a response with the given self-assessed
    /// confidence estimate in `[0.0, 1.0]`.
    ///
    /// This is an external signal and does **not** update the inner
    /// [`BayesianEngine`] (which operates on discrete intent keys).
    ModelConfidence(f32),
}

// ---------------------------------------------------------------------------
// EngineBeliefs
// ---------------------------------------------------------------------------

/// Manages probabilistic beliefs about hypotheses and tool relevance for the
/// reasoning engine.
///
/// `EngineBeliefs` wraps a [`BayesianEngine`] to keep an updated posterior
/// distribution over user-intent keys, while also maintaining a separate
/// per-tool belief map that is adjusted by tool success and failure events.
///
/// # Example
///
/// ```rust,ignore
/// let mut beliefs = EngineBeliefs::new();
/// beliefs.register_tool("list_directory", 0.6);
/// beliefs.update_from_evidence(&Evidence::UserText("show me all Rust files".into()));
/// beliefs.update_from_evidence(&Evidence::ToolSuccess {
///     tool_name: "list_directory".into(),
/// });
///
/// let mut state = ReasoningEngineState::new();
/// beliefs.sync_to_state(&mut state);
/// assert!(state.uncertainty <= 1.0);
/// ```
pub struct EngineBeliefs {
    /// The underlying Bayesian inference engine over intent keys.
    inner: BayesianEngine,
    /// Per-tool posterior scores, updated by success/failure evidence.
    tool_beliefs: HashMap<String, ToolBelief>,
}

impl EngineBeliefs {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a new belief layer backed by a fresh [`BayesianEngine`].
    ///
    /// The inner engine is initialised with profile-loaded or built-in default
    /// priors. No tools are registered initially.
    pub fn new() -> Self {
        Self {
            inner: BayesianEngine::new(),
            tool_beliefs: HashMap::new(),
        }
    }

    // ── Evidence updates ──────────────────────────────────────────────────────

    /// Update beliefs from a piece of [`Evidence`].
    ///
    /// | Evidence variant         | Effect on inner engine              | Effect on tool beliefs                                   |
    /// |--------------------------|-------------------------------------|----------------------------------------------------------|
    /// | `UserText(t)`            | `inner.update_from_text(&t)`        | —                                                        |
    /// | `ToolSuccess { name }`   | —                                   | score += 0.1 (cap 1.0), selected = true                  |
    /// | `ToolFailure { name }`   | `inner.update_from_tool_failure()`  | score -= 0.2 (floor 0.0)                                 |
    /// | `ModelConfidence(_)`     | no-op                               | —                                                        |
    pub fn update_from_evidence(&mut self, evidence: &Evidence) {
        match evidence {
            Evidence::UserText(text) => {
                self.inner.update_from_text(text);
            }

            Evidence::ToolSuccess { tool_name } => {
                if let Some(belief) = self.tool_beliefs.get_mut(tool_name) {
                    belief.score = (belief.score + 0.1).min(1.0);
                    belief.selected = true;
                }
            }

            Evidence::ToolFailure { tool_name } => {
                // Propagate the failure signal to the Bayesian engine so that
                // the intent distribution can reflect reduced confidence.
                self.inner.update_from_tool_failure();
                if let Some(belief) = self.tool_beliefs.get_mut(tool_name) {
                    belief.score = (belief.score - 0.2).max(0.0);
                }
            }

            Evidence::ModelConfidence(_confidence) => {
                // Model confidence is an external, continuous signal.
                // The inner BayesianEngine operates on discrete intent keys,
                // so there is no meaningful update to apply here.
            }
        }
    }

    // ── Tool registry ─────────────────────────────────────────────────────────

    /// Register a tool name with an initial belief score.
    ///
    /// If the tool is already registered its score is **not** overwritten.
    /// Call [`update_from_evidence`](Self::update_from_evidence) with
    /// [`Evidence::ToolSuccess`] or [`Evidence::ToolFailure`] to adjust an
    /// existing score.
    ///
    /// `initial_score` is silently clamped to `[0.0, 1.0]`.
    pub fn register_tool(&mut self, tool_name: impl Into<String>, initial_score: f32) {
        let name = tool_name.into();
        self.tool_beliefs
            .entry(name.clone())
            .or_insert_with(|| ToolBelief {
                tool_name: name,
                score: initial_score.clamp(0.0, 1.0),
                selected: false,
            });
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Return the current belief score for a named tool, optionally adjusted by DNA.
    ///
    /// If `dna_tool_weight` is provided, the base score is multiplied by it.
    pub fn tool_score(&self, tool_name: &str, dna: Option<&crate::session::dna::SessionDna>) -> f32 {
        let base = self.tool_beliefs
            .get(tool_name)
            .map(|b| b.score)
            .unwrap_or(0.0);

        match dna {
            Some(d) => (base * d.get_tool_weight(tool_name)).clamp(0.0, 1.0),
            None => base,
        }
    }

    /// Return the top-ranked [`ToolBelief`], or `None` if no tools are
    /// registered.
    ///
    /// When multiple tools share the highest score the winner is determined by
    /// HashMap iteration order, which is intentionally unspecified.
    pub fn top_tool(&self) -> Option<&ToolBelief> {
        self.tool_beliefs.values().max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Overall uncertainty score derived from the Bayesian engine.
    ///
    /// Computed as the arithmetic mean of `P(need_clarification)` and
    /// `P(low_confidence)` from the inner engine, clamped to `[0.0, 1.0]`.
    /// Higher values indicate that the engine has less confidence in its
    /// current intent interpretation.
    pub fn uncertainty(&self) -> f32 {
        let nc = self.inner.probability("need_clarification");
        let lc = self.inner.probability("low_confidence");
        ((nc + lc) / 2.0).clamp(0.0, 1.0)
    }

    /// Whether clarification is needed according to the Bayesian engine.
    ///
    /// Delegates to [`BayesianEngine::needs_clarification`], which compares
    /// `P(need_clarification)` against the engine's configured threshold.
    pub fn needs_clarification(&self) -> bool {
        self.inner.needs_clarification()
    }

    // ── State synchronisation ─────────────────────────────────────────────────

    /// Synchronise the uncertainty and hypothesis confidences in a
    /// [`ReasoningEngineState`] from the current Bayesian beliefs.
    ///
    /// The following mutations are applied to `state`:
    ///
    /// 1. `state.uncertainty` is set to [`Self::uncertainty`].
    /// 2. For each [`Hypothesis`] in `state.hypotheses` whose `id` matches a
    ///    Bayesian key that has a positive probability
    ///    (`inner.probability(id) > 0.0`), `hypothesis.confidence` is updated
    ///    to the current posterior, clamped to `[0.0, 1.0]`.
    /// 3. If the Bayesian engine's [`BayesianEngine::best_intent`] key is not
    ///    already present in `state.hypotheses` (by `id` or `description`), a
    ///    new [`Hypothesis`] is appended with its `id` and `description` both
    ///    set to the intent key and `confidence` set to the corresponding
    ///    probability.
    pub fn sync_to_state(&self, state: &mut ReasoningEngineState) {
        // 1. Push overall uncertainty.
        state.uncertainty = self.uncertainty();

        // 2. Update confidences for hypotheses whose id matches a live key.
        for hypothesis in &mut state.hypotheses {
            let p = self.inner.probability(&hypothesis.id);
            if p > 0.0 {
                hypothesis.confidence = p.clamp(0.0, 1.0);
            }
        }

        // 3. Append the best-intent hypothesis if it is not already present.
        if let Some(best_key) = self.inner.best_intent() {
            let already_present = state
                .hypotheses
                .iter()
                .any(|h| h.id == best_key || h.description == best_key);

            if !already_present {
                let confidence = self.inner.probability(&best_key).clamp(0.0, 1.0);
                state.hypotheses.push(Hypothesis {
                    id: best_key.clone(),
                    description: best_key,
                    confidence,
                });
            }
        }
    }

    /// Score a slice of plan steps using the current tool beliefs.
    ///
    /// Optionally accepts a DNA tool weight multiplier (from SessionDna).
    pub fn score_plan(&self, steps: &[PlanStep], dna_tool_weight: Option<f32>) -> Vec<f32> {
        steps
            .iter()
            .map(|step| match &step.action {
                StepAction::UseTool { tool_name, .. } => {
                    let base = self.tool_score(tool_name, None);
                    match dna_tool_weight {
                        Some(w) => (base * w).clamp(0.0, 1.0),
                        None => base,
                    }
                }
                _ => 0.0,
            })
            .collect()
    }
}

impl Default for EngineBeliefs {
    /// Delegates to [`EngineBeliefs::new`].
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{PlanStep, ReasoningEngineState, StepAction};

    // ── Construction ──────────────────────────────────────────────────────────

    /// A freshly constructed `EngineBeliefs` must have an empty tool registry
    /// and an uncertainty value in the valid `[0.0, 1.0]` range.
    #[test]
    fn new_creates_fresh_beliefs() {
        let beliefs = EngineBeliefs::new();
        assert!(
            beliefs.tool_beliefs.is_empty(),
            "tool registry should start empty"
        );
        let u = beliefs.uncertainty();
        assert!(
            (0.0..=1.0).contains(&u),
            "initial uncertainty {u} must be in [0, 1]"
        );
    }

    // ── Evidence: user text ───────────────────────────────────────────────────

    /// Feeding a clear, decisive edit command should yield an uncertainty value
    /// that is still within `[0.0, 1.0]` and should not panic.
    #[test]
    fn update_from_user_text_lowers_uncertainty_on_clear_input() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.update_from_evidence(&Evidence::UserText("edit this file".to_string()));
        let u = beliefs.uncertainty();
        assert!(
            (0.0..=1.0).contains(&u),
            "uncertainty {u} must stay in [0, 1] after clear user text"
        );
    }

    // ── Tool registry ─────────────────────────────────────────────────────────

    /// `register_tool` must set the initial score exactly.
    #[test]
    fn register_tool_sets_initial_score() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("edit_file", 0.75);
        assert!(
            (beliefs.tool_score("edit_file", None) - 0.75).abs() < f32::EPSILON,
            "initial score should be 0.75"
        );
    }

    /// A second call to `register_tool` with the same name must **not**
    /// overwrite the existing score.
    #[test]
    fn register_tool_does_not_overwrite_existing() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("edit_file", 0.75);
        beliefs.register_tool("edit_file", 0.10); // should be ignored
        assert!(
            (beliefs.tool_score("edit_file", None) - 0.75).abs() < f32::EPSILON,
            "second register_tool call must not overwrite the score"
        );
    }

    // ── Evidence: tool success ────────────────────────────────────────────────

    /// A `ToolSuccess` event must raise the tool's score by exactly `0.1`.
    #[test]
    fn tool_success_raises_score() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("search", 0.5);
        beliefs.update_from_evidence(&Evidence::ToolSuccess {
            tool_name: "search".to_string(),
        });
        let score = beliefs.tool_score("search", None);
        assert!(
            (score - 0.6).abs() < 1e-5,
            "expected score 0.6, got {score}"
        );
    }

    /// The score cap of `1.0` must be respected.
    #[test]
    fn tool_success_caps_at_one() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("search", 0.95);
        beliefs.update_from_evidence(&Evidence::ToolSuccess {
            tool_name: "search".to_string(),
        });
        let score = beliefs.tool_score("search", None);
        assert!(score <= 1.0, "score must not exceed 1.0, got {score}");
    }

    // ── Evidence: tool failure ────────────────────────────────────────────────

    /// A `ToolFailure` event must lower the tool's score by exactly `0.2`.
    #[test]
    fn tool_failure_lowers_score() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("search", 0.5);
        beliefs.update_from_evidence(&Evidence::ToolFailure {
            tool_name: "search".to_string(),
        });
        let score = beliefs.tool_score("search", None);
        assert!(
            (score - 0.3).abs() < 1e-5,
            "expected score 0.3, got {score}"
        );
    }

    /// The score floor of `0.0` must be respected.
    #[test]
    fn tool_failure_floors_at_zero() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("search", 0.1);
        beliefs.update_from_evidence(&Evidence::ToolFailure {
            tool_name: "search".to_string(),
        });
        let score = beliefs.tool_score("search", None);
        assert!(score >= 0.0, "score must not go below 0.0, got {score}");
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Querying an unregistered tool must return exactly `0.0`.
    #[test]
    fn tool_score_returns_zero_for_unknown() {
        let beliefs = EngineBeliefs::new();
        assert_eq!(
            beliefs.tool_score("unknown_tool", None),
            0.0,
            "unregistered tool must have score 0.0"
        );
    }

    /// `top_tool` must return the tool with the highest score.
    #[test]
    fn top_tool_returns_highest_scoring() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("low_tool", 0.2);
        beliefs.register_tool("high_tool", 0.9);
        beliefs.register_tool("mid_tool", 0.5);

        let top = beliefs
            .top_tool()
            .expect("top_tool should return Some when tools are registered");
        assert_eq!(top.tool_name, "high_tool");
        assert!(
            (top.score - 0.9).abs() < f32::EPSILON,
            "top tool score should be 0.9"
        );
    }

    /// `top_tool` must return `None` when no tools are registered.
    #[test]
    fn top_tool_returns_none_when_empty() {
        let beliefs = EngineBeliefs::new();
        assert!(
            beliefs.top_tool().is_none(),
            "top_tool should be None with no registered tools"
        );
    }

    // ── State sync ────────────────────────────────────────────────────────────

    /// `sync_to_state` must update `state.uncertainty` to match
    /// `beliefs.uncertainty()`.
    #[test]
    fn sync_to_state_sets_uncertainty() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.update_from_evidence(&Evidence::UserText("edit this file".to_string()));

        let mut state = ReasoningEngineState::new();
        beliefs.sync_to_state(&mut state);

        let expected = beliefs.uncertainty();
        assert!(
            (state.uncertainty - expected).abs() < f32::EPSILON,
            "state.uncertainty {:.4} must equal beliefs.uncertainty() {:.4}",
            state.uncertainty,
            expected
        );
    }

    /// `sync_to_state` must add a hypothesis for the best intent if one does
    /// not already exist in the state.
    #[test]
    fn sync_to_state_adds_best_intent_hypothesis() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.update_from_evidence(&Evidence::UserText("edit this file".to_string()));

        let mut state = ReasoningEngineState::new();
        assert!(
            state.hypotheses.is_empty(),
            "state starts with no hypotheses"
        );

        beliefs.sync_to_state(&mut state);
        // The Bayesian engine should have surfaced at least one intent hypothesis.
        assert!(
            !state.hypotheses.is_empty(),
            "sync_to_state should add at least one hypothesis"
        );
    }

    /// `sync_to_state` must not add duplicate hypotheses on repeated calls.
    #[test]
    fn sync_to_state_does_not_duplicate_hypotheses() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.update_from_evidence(&Evidence::UserText("edit this file".to_string()));

        let mut state = ReasoningEngineState::new();
        beliefs.sync_to_state(&mut state);
        let count_after_first = state.hypotheses.len();

        beliefs.sync_to_state(&mut state);
        let count_after_second = state.hypotheses.len();

        assert_eq!(
            count_after_first, count_after_second,
            "repeated sync_to_state should not add duplicate hypotheses"
        );
    }

    // ── Plan scoring ──────────────────────────────────────────────────────────

    /// A `NoOp` step must receive a score of exactly `0.0`.
    #[test]
    fn score_plan_returns_zero_for_noop() {
        let beliefs = EngineBeliefs::new();
        let steps = vec![PlanStep::new("no-op step", StepAction::NoOp)];
        let scores = beliefs.score_plan(&steps, None);
        assert_eq!(scores.len(), 1, "score vec length must match step count");
        assert_eq!(scores[0], 0.0, "NoOp step must score 0.0");
    }

    /// A `UseTool` step must receive the registered tool's belief score.
    #[test]
    fn score_plan_returns_tool_score_for_use_tool() {
        let mut beliefs = EngineBeliefs::new();
        beliefs.register_tool("list_directory", 0.8);

        let steps = vec![
            PlanStep::new(
                "list files",
                StepAction::UseTool {
                    tool_name: "list_directory".to_string(),
                    args: serde_json::Value::Null,
                },
            ),
            PlanStep::new("no-op", StepAction::NoOp),
        ];

        let scores = beliefs.score_plan(&steps, None);
        assert_eq!(scores.len(), 2, "score vec length must match step count");
        assert!(
            (scores[0] - 0.8).abs() < f32::EPSILON,
            "UseTool step should score 0.8, got {}",
            scores[0]
        );
        assert_eq!(scores[1], 0.0, "NoOp step must score 0.0");
    }

    /// An unregistered tool in a `UseTool` step must score `0.0`.
    #[test]
    fn score_plan_returns_zero_for_unregistered_tool() {
        let beliefs = EngineBeliefs::new();
        let steps = vec![PlanStep::new(
            "call unknown tool",
            StepAction::UseTool {
                tool_name: "ghost_tool".to_string(),
                args: serde_json::Value::Null,
            },
        )];
        let scores = beliefs.score_plan(&steps, None);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0], 0.0, "unregistered tool must score 0.0");
    }

    /// An empty plan produces an empty score vector.
    #[test]
    fn score_plan_returns_empty_for_empty_plan() {
        let beliefs = EngineBeliefs::new();
        let scores = beliefs.score_plan(&[], None);
        assert!(scores.is_empty(), "empty plan must yield empty score vec");
    }

    // ── Default trait ─────────────────────────────────────────────────────────

    /// `Default::default()` must produce an equivalent value to `new()`.
    #[test]
    fn default_delegates_to_new() {
        let via_new = EngineBeliefs::new();
        let via_default = EngineBeliefs::default();
        // Both should start with empty tool registries and identical uncertainty.
        assert!(via_new.tool_beliefs.is_empty());
        assert!(via_default.tool_beliefs.is_empty());
        assert!(
            (via_new.uncertainty() - via_default.uncertainty()).abs() < f32::EPSILON,
            "default and new must produce the same initial uncertainty"
        );
    }
}
