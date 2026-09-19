//! Self-correction and error-recovery for the reasoning engine.
//!
//! This module implements Task 99: bounded self-correction loops that detect
//! error signals in [`ReasoningEngineState`] and apply targeted plan revisions
//! to recover.  All correction logic is encapsulated in [`CorrectionEngine`],
//! which is safe to call repeatedly because it respects the `max_revisions`
//! guard already embedded in the engine state.
//!
//! # Design
//!
//! ```text
//!  loop {
//!      trigger = should_correct(state)?      // detect signal
//!      outcome = apply_correction(state, t)  // apply targeted fix
//!      if outcome == MaxRevisionsReached { break }
//!  }                                         // terminates by construction
//! ```
//!
//! The public entry points [`CorrectionEngine::maybe_correct`] and
//! [`CorrectionEngine::correct_until_stable`] compose these two primitives
//! with the loop and termination guards baked in.
//!
//! # Bounded-loop guarantee
//!
//! Every correction attempt calls [`ReasoningEngineState::revise_plan`], which
//! increments `revision_count`.  When `revision_count >= max_revisions` that
//! method returns [`PlanError::MaxRevisionsExceeded`] and the correction
//! engine surfaces [`CorrectionOutcome::MaxRevisionsReached`].  Both
//! [`maybe_correct`][CorrectionEngine::maybe_correct] and
//! [`correct_until_stable`][CorrectionEngine::correct_until_stable] stop on
//! that outcome, so infinite loops are impossible regardless of the caller's
//! behaviour.

use std::fmt;

use tracing::warn;

use super::state::{
    EngineState, PlanError, PlanStep, ReasoningEngineState, StepAction, StepStatus,
};

// ---------------------------------------------------------------------------
// CorrectionTrigger
// ---------------------------------------------------------------------------

/// A signal that indicates the reasoning engine should attempt self-correction.
///
/// Triggers are produced by [`CorrectionEngine::should_correct`] after
/// inspecting [`ReasoningEngineState`].  Each variant captures the information
/// needed by [`CorrectionEngine::apply_correction`] to build a targeted
/// recovery plan.
#[derive(Debug, Clone, PartialEq)]
pub enum CorrectionTrigger {
    /// A plan step failed with the given reason.
    StepFailed {
        /// Zero-based index of the failed step in the current plan.
        step_index: usize,
        /// Human-readable failure reason captured from [`StepStatus::Failed`].
        reason: String,
    },

    /// Overall uncertainty exceeded the configured threshold.
    HighUncertainty {
        /// Current uncertainty value (in `[0.0, 1.0]`).
        uncertainty: f32,
        /// Configured threshold that was breached.
        threshold: f32,
    },

    /// The plan is empty but the goal has not yet been achieved.
    EmptyPlan,

    /// An external feedback signal (e.g. the user said "that's wrong").
    ExternalFeedback(String),
}

impl fmt::Display for CorrectionTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StepFailed { step_index, reason } => {
                write!(f, "step {step_index} failed: {reason}")
            }
            Self::HighUncertainty {
                uncertainty,
                threshold,
            } => write!(f, "high uncertainty {uncertainty:.2} > {threshold:.2}"),
            Self::EmptyPlan => write!(f, "empty plan"),
            Self::ExternalFeedback(msg) => write!(f, "feedback: {msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// CorrectionOutcome
// ---------------------------------------------------------------------------

/// Outcome of a self-correction attempt.
///
/// Returned by [`CorrectionEngine::apply_correction`] and surfaced through
/// [`CorrectionEngine::maybe_correct`] and
/// [`CorrectionEngine::correct_until_stable`].
#[derive(Debug, Clone, PartialEq)]
pub enum CorrectionOutcome {
    /// The engine successfully applied a correction; the plan has been revised.
    Corrected {
        /// The engine's [`ReasoningEngineState::revision_count`] after the
        /// revision.
        revision_count: u32,
    },

    /// The engine's `max_revisions` limit has been reached; the correction was
    /// **not** applied.
    MaxRevisionsReached,

    /// The trigger did not warrant a correction (e.g. the engine is already in
    /// a terminal state).
    NotNeeded,
}

// ---------------------------------------------------------------------------
// CorrectionConfig
// ---------------------------------------------------------------------------

/// Configuration knobs for [`CorrectionEngine`].
#[derive(Debug, Clone)]
pub struct CorrectionConfig {
    /// Uncertainty value above which a [`CorrectionTrigger::HighUncertainty`]
    /// signal is emitted.
    ///
    /// Must be in `[0.0, 1.0]`.  Defaults to `0.75`.
    pub uncertainty_threshold: f32,
}

impl Default for CorrectionConfig {
    fn default() -> Self {
        Self {
            uncertainty_threshold: 0.75,
        }
    }
}

// ---------------------------------------------------------------------------
// CorrectionEngine
// ---------------------------------------------------------------------------

/// Detects failures and applies bounded self-correction to reasoning engine
/// state.
///
/// Construct via [`CorrectionEngine::new`] with an explicit
/// [`CorrectionConfig`], or use [`CorrectionEngine::default`] for sensible
/// defaults.
///
/// # Usage pattern
///
/// ```rust
/// use grok_cli::engine::correction::CorrectionEngine;
/// use grok_cli::engine::state::ReasoningEngineState;
///
/// let engine = CorrectionEngine::default();
/// let mut state = ReasoningEngineState::new().with_goal("test");
///
/// // Single-round, safe:
/// if let Some((trigger, outcome)) = engine.maybe_correct(&mut state) {
///     println!("corrected: {trigger} → {outcome:?}");
/// }
///
/// // Multi-round, bounded:
/// let log = engine.correct_until_stable(&mut state, 5);
/// ```
pub struct CorrectionEngine {
    config: CorrectionConfig,
}

impl CorrectionEngine {
    /// Create a new [`CorrectionEngine`] with the supplied [`CorrectionConfig`].
    pub fn new(config: CorrectionConfig) -> Self {
        Self { config }
    }

    // -----------------------------------------------------------------------
    // Detection
    // -----------------------------------------------------------------------

    /// Inspect `state` and return the first applicable [`CorrectionTrigger`],
    /// or `None` if no correction is needed.
    ///
    /// # Check order (first match wins)
    ///
    /// 1. `state.state == Failed { .. }` → `None` (terminal; cannot correct)
    /// 2. `state.state == Complete`      → `None` (terminal; nothing to correct)
    /// 3. `state.plan.is_empty() && state.goal.is_some()` →
    ///    [`CorrectionTrigger::EmptyPlan`]
    /// 4. For each step where `status == Failed { reason }` →
    ///    [`CorrectionTrigger::StepFailed`] (first occurrence)
    /// 5. `state.uncertainty > uncertainty_threshold` →
    ///    [`CorrectionTrigger::HighUncertainty`]
    /// 6. Otherwise → `None`
    pub fn should_correct(&self, state: &ReasoningEngineState) -> Option<CorrectionTrigger> {
        // 1 & 2 — terminal FSM states: correction is impossible or unnecessary.
        if matches!(
            state.state,
            EngineState::Failed { .. } | EngineState::Complete
        ) {
            return None;
        }

        // 3 — plan is empty but a goal still needs to be achieved.
        if state.plan.is_empty() && state.goal.is_some() {
            return Some(CorrectionTrigger::EmptyPlan);
        }

        // 4 — first failed step in the plan (scanning in order).
        for (i, step) in state.plan.iter().enumerate() {
            if let StepStatus::Failed { reason } = &step.status {
                return Some(CorrectionTrigger::StepFailed {
                    step_index: i,
                    reason: reason.clone(),
                });
            }
        }

        // 5 — aggregate uncertainty breaches the threshold.
        if state.uncertainty > self.config.uncertainty_threshold {
            return Some(CorrectionTrigger::HighUncertainty {
                uncertainty: state.uncertainty,
                threshold: self.config.uncertainty_threshold,
            });
        }

        None
    }

    // -----------------------------------------------------------------------
    // Correction
    // -----------------------------------------------------------------------

    /// Apply a correction based on `trigger`, mutating `state` in place.
    ///
    /// # Return value
    ///
    /// - [`CorrectionOutcome::Corrected`]  — plan was successfully revised.
    /// - [`CorrectionOutcome::MaxRevisionsReached`] — `revise_plan` rejected
    ///   the update because `revision_count >= max_revisions`; `state` is
    ///   **not** modified.
    ///
    /// # Recovery strategies by trigger variant
    ///
    /// | Trigger             | Recovery plan built                                  |
    /// |---------------------|------------------------------------------------------|
    /// | `StepFailed`        | Completed steps before index + `ModelCall("Recover from: …")` + pending steps after index |
    /// | `HighUncertainty`   | `ModelCall("Re-evaluate …")` prepended to pending steps |
    /// | `EmptyPlan`         | Single `ModelCall("No plan available; re-analyse goal: …")` |
    /// | `ExternalFeedback`  | `ModelCall("User feedback: …")` prepended to full plan |
    pub fn apply_correction(
        &self,
        state: &mut ReasoningEngineState,
        trigger: CorrectionTrigger,
    ) -> CorrectionOutcome {
        match trigger {
            // -----------------------------------------------------------------
            // StepFailed — rebuild the plan around the failure point.
            // -----------------------------------------------------------------
            CorrectionTrigger::StepFailed { step_index, reason } => {
                // Preserve completed steps that came before the failed step.
                let completed_before: Vec<PlanStep> = state
                    .plan
                    .iter()
                    .take(step_index)
                    .filter(|s| matches!(s.status, StepStatus::Completed))
                    .cloned()
                    .collect();

                // Preserve pending steps that come after the failed step.
                let pending_after: Vec<PlanStep> = state
                    .plan
                    .iter()
                    .skip(step_index.saturating_add(1))
                    .filter(|s| matches!(s.status, StepStatus::Pending))
                    .cloned()
                    .collect();

                // The recovery step asks the model to diagnose and continue.
                let recovery_prompt = format!("Recover from: {reason}");
                let recovery_step = PlanStep::new(
                    recovery_prompt.clone(),
                    StepAction::ModelCall {
                        prompt: recovery_prompt,
                    },
                );

                let mut recovery_plan = completed_before;
                recovery_plan.push(recovery_step);
                recovery_plan.extend(pending_after);

                match state.revise_plan(recovery_plan) {
                    Err(PlanError::MaxRevisionsExceeded(_)) => {
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Err(e) => {
                        // Unexpected variant — treat conservatively as terminal.
                        warn!("Unexpected plan error during StepFailed correction: {e}");
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Ok(()) => {}
                }

                // Drive the FSM through the standard revision cycle.
                self.do_revise_transitions(state);

                CorrectionOutcome::Corrected {
                    revision_count: state.revision_count,
                }
            }

            // -----------------------------------------------------------------
            // HighUncertainty — prepend a re-evaluation step to pending work.
            // -----------------------------------------------------------------
            CorrectionTrigger::HighUncertainty { .. } => {
                // Collect steps that have not yet been executed.
                let pending_steps: Vec<PlanStep> = state
                    .plan
                    .iter()
                    .filter(|s| matches!(s.status, StepStatus::Pending))
                    .cloned()
                    .collect();

                let reeval_prompt = "Re-evaluate due to high uncertainty";
                let reeval_step = PlanStep::new(
                    reeval_prompt,
                    StepAction::ModelCall {
                        prompt: reeval_prompt.to_owned(),
                    },
                );

                // Re-evaluation step goes to the front of the pending work.
                let mut new_plan = vec![reeval_step];
                new_plan.extend(pending_steps);

                match state.revise_plan(new_plan) {
                    Err(PlanError::MaxRevisionsExceeded(_)) => {
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Err(e) => {
                        warn!("Unexpected plan error during HighUncertainty correction: {e}");
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Ok(()) => {}
                }

                self.do_revise_transitions(state);

                CorrectionOutcome::Corrected {
                    revision_count: state.revision_count,
                }
            }

            // -----------------------------------------------------------------
            // EmptyPlan — seed the plan with a goal re-analysis step.
            // -----------------------------------------------------------------
            CorrectionTrigger::EmptyPlan => {
                let goal_str = state.goal.as_deref().unwrap_or("unknown goal");
                let prompt = format!("No plan available; re-analyse goal: {goal_str}");
                let seed_step = PlanStep::new(prompt.clone(), StepAction::ModelCall { prompt });

                match state.revise_plan(vec![seed_step]) {
                    Err(PlanError::MaxRevisionsExceeded(_)) => {
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Err(e) => {
                        warn!("Unexpected plan error during EmptyPlan correction: {e}");
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Ok(()) => {}
                }

                CorrectionOutcome::Corrected {
                    revision_count: state.revision_count,
                }
            }

            // -----------------------------------------------------------------
            // ExternalFeedback — prepend a feedback-processing step.
            // -----------------------------------------------------------------
            CorrectionTrigger::ExternalFeedback(msg) => {
                let prompt = format!("User feedback: {msg}");
                let feedback_step = PlanStep::new(prompt.clone(), StepAction::ModelCall { prompt });

                // Feedback step goes before all current plan steps.
                let mut new_plan = vec![feedback_step];
                new_plan.extend(state.plan.clone());

                match state.revise_plan(new_plan) {
                    Err(PlanError::MaxRevisionsExceeded(_)) => {
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Err(e) => {
                        warn!("Unexpected plan error during ExternalFeedback correction: {e}");
                        return CorrectionOutcome::MaxRevisionsReached;
                    }
                    Ok(()) => {}
                }

                CorrectionOutcome::Corrected {
                    revision_count: state.revision_count,
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Composite entry points
    // -----------------------------------------------------------------------

    /// Check whether a correction is needed and, if so, apply it atomically.
    ///
    /// Returns `Some((trigger, outcome))` when a [`CorrectionTrigger`] was
    /// found, or `None` when [`should_correct`][Self::should_correct] returned
    /// `None` (no correction is required).
    ///
    /// This is the safe single-round entry point.  Callers that loop on this
    /// method terminate naturally when:
    ///
    /// - `outcome == MaxRevisionsReached` (the plan cannot be revised further),
    ///   **or**
    /// - this method returns `None` (the state is stable or terminal).
    pub fn maybe_correct(
        &self,
        state: &mut ReasoningEngineState,
    ) -> Option<(CorrectionTrigger, CorrectionOutcome)> {
        let trigger = self.should_correct(state)?;
        let outcome = self.apply_correction(state, trigger.clone());
        Some((trigger, outcome))
    }

    /// Run [`maybe_correct`][Self::maybe_correct] in a bounded loop, stopping
    /// after `max_rounds` rounds **or** when no further trigger fires,
    /// whichever comes first.
    ///
    /// The loop also stops immediately when a
    /// [`CorrectionOutcome::MaxRevisionsReached`] outcome is observed, since
    /// further rounds cannot revise the plan anyway.
    ///
    /// # Returns
    ///
    /// A `Vec` of `(trigger, outcome)` pairs — one entry per round actually
    /// executed.  The vec will be empty if no trigger fires on the first round.
    ///
    /// # Safeguard
    ///
    /// Even if the caller ignores the returned outcomes, this method will
    /// never loop infinitely: it is bounded by both `max_rounds` and the
    /// engine's `max_revisions` counter.
    pub fn correct_until_stable(
        &self,
        state: &mut ReasoningEngineState,
        max_rounds: u32,
    ) -> Vec<(CorrectionTrigger, CorrectionOutcome)> {
        let mut results = Vec::new();

        for _ in 0..max_rounds {
            match self.maybe_correct(state) {
                None => break,
                Some((trigger, outcome)) => {
                    let is_terminal = outcome == CorrectionOutcome::MaxRevisionsReached;
                    results.push((trigger, outcome));
                    if is_terminal {
                        break;
                    }
                }
            }
        }

        results
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Attempt the `→ RevisePlan → CommitPlan` FSM transition sequence.
    ///
    /// These transitions are only valid when the engine is currently in an
    /// `ExecuteStep` state.  If the current state does not permit either
    /// transition (e.g. during unit tests that do not exercise the full FSM
    /// path), the failure is logged as a warning and execution continues — the
    /// plan revision has already been committed, so the correction remains
    /// useful even without the FSM advancement.
    fn do_revise_transitions(&self, state: &mut ReasoningEngineState) {
        if let Err(e) = state.transition(EngineState::RevisePlan) {
            warn!("Could not transition to RevisePlan after correction: {e}");
        } else if let Err(e) = state.transition(EngineState::CommitPlan) {
            warn!("Could not transition to CommitPlan after correction: {e}");
        }
    }
}

impl Default for CorrectionEngine {
    /// Create a [`CorrectionEngine`] with [`CorrectionConfig::default`]
    /// (uncertainty threshold of `0.75`).
    fn default() -> Self {
        Self::new(CorrectionConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{
        EngineState, PlanStep, ReasoningEngineState, StepAction, StepStatus,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Default engine with uncertainty threshold `0.75`.
    fn engine() -> CorrectionEngine {
        CorrectionEngine::default()
    }

    /// State with a single `Pending` `NoOp` step and a goal set.
    fn state_with_pending_step() -> ReasoningEngineState {
        let mut s = ReasoningEngineState::new().with_goal("test goal");
        s.plan.push(PlanStep::new("do something", StepAction::NoOp));
        s
    }

    /// State in `ExecuteStep { step_index }` with the plan already populated.
    fn executing_state(step_index: usize) -> ReasoningEngineState {
        let mut s = ReasoningEngineState::new().with_goal("execute goal");
        s.state = EngineState::ExecuteStep { step_index };
        s
    }

    // -----------------------------------------------------------------------
    // 99.1 — `should_correct` detection
    // -----------------------------------------------------------------------

    #[test]
    fn no_trigger_for_complete_state() {
        let mut state = ReasoningEngineState::new().with_goal("done");
        state.state = EngineState::Complete;
        // Give it many reasons to fire — terminal state must win.
        state.uncertainty = 0.99;

        assert_eq!(
            engine().should_correct(&state),
            None,
            "Complete state must never produce a trigger"
        );
    }

    #[test]
    fn no_trigger_for_failed_state() {
        let mut state = ReasoningEngineState::new().with_goal("done");
        state.state = EngineState::Failed {
            reason: "unrecoverable".to_owned(),
        };
        state.uncertainty = 0.99;

        assert_eq!(
            engine().should_correct(&state),
            None,
            "Failed state must never produce a trigger"
        );
    }

    #[test]
    fn empty_plan_triggers_empty_plan() {
        // Plan is empty and a goal exists → EmptyPlan fires before HighUncertainty.
        let mut state = ReasoningEngineState::new().with_goal("analyse something");
        state.uncertainty = 0.99; // would trigger HighUncertainty if EmptyPlan lost

        assert_eq!(
            engine().should_correct(&state),
            Some(CorrectionTrigger::EmptyPlan)
        );
    }

    #[test]
    fn failed_step_triggers_step_failed() {
        let mut state = state_with_pending_step();
        state.plan[0].status = StepStatus::Failed {
            reason: "network timeout".to_owned(),
        };

        assert_eq!(
            engine().should_correct(&state),
            Some(CorrectionTrigger::StepFailed {
                step_index: 0,
                reason: "network timeout".to_owned(),
            })
        );
    }

    #[test]
    fn high_uncertainty_triggers_high_uncertainty() {
        let mut state = state_with_pending_step();
        state.uncertainty = 0.9; // above default threshold 0.75

        let trigger = engine().should_correct(&state);
        assert!(
            matches!(
                trigger,
                Some(CorrectionTrigger::HighUncertainty { uncertainty, threshold })
                    if uncertainty > threshold
            ),
            "expected HighUncertainty trigger, got: {trigger:?}"
        );
    }

    #[test]
    fn no_trigger_when_all_conditions_clear() {
        let mut state = state_with_pending_step();
        state.uncertainty = 0.3; // well below 0.75 threshold

        assert_eq!(
            engine().should_correct(&state),
            None,
            "stable state with low uncertainty must produce no trigger"
        );
    }

    #[test]
    fn step_failed_fires_before_high_uncertainty() {
        // Both conditions true: StepFailed must win (checked first).
        let mut state = state_with_pending_step();
        state.uncertainty = 0.99;
        state.plan[0].status = StepStatus::Failed {
            reason: "boom".to_owned(),
        };

        assert!(
            matches!(
                engine().should_correct(&state),
                Some(CorrectionTrigger::StepFailed { .. })
            ),
            "StepFailed must take priority over HighUncertainty"
        );
    }

    // -----------------------------------------------------------------------
    // 99.2 — `apply_correction`
    // -----------------------------------------------------------------------

    #[test]
    fn apply_correction_step_failed_adds_model_call() {
        let mut state = executing_state(1);

        // Three-step plan: completed → failed → pending.
        let mut done = PlanStep::new("compile", StepAction::NoOp);
        done.status = StepStatus::Completed;
        let mut bad = PlanStep::new("deploy", StepAction::NoOp);
        bad.status = StepStatus::Failed {
            reason: "disk full".to_owned(),
        };
        let pending = PlanStep::new("notify", StepAction::NoOp);
        state.plan = vec![done, bad, pending];

        let trigger = CorrectionTrigger::StepFailed {
            step_index: 1,
            reason: "disk full".to_owned(),
        };
        let outcome = engine().apply_correction(&mut state, trigger);

        // Outcome must be Corrected.
        assert!(
            matches!(outcome, CorrectionOutcome::Corrected { .. }),
            "expected Corrected, got {outcome:?}"
        );

        // Plan must contain a recovery ModelCall that references the reason.
        let has_recovery = state.plan.iter().any(|s| {
            matches!(&s.action, StepAction::ModelCall { prompt } if prompt.contains("disk full"))
        });
        assert!(
            has_recovery,
            "plan must contain a recovery ModelCall for the failure reason"
        );

        // Completed step before the failure is preserved.
        let has_completed = state
            .plan
            .iter()
            .any(|s| matches!(s.status, StepStatus::Completed));
        assert!(
            has_completed,
            "completed steps before the failure must be preserved"
        );

        // Pending step after the failure is preserved.
        let has_trailing_pending = state
            .plan
            .iter()
            .any(|s| s.description == "notify" && matches!(s.status, StepStatus::Pending));
        assert!(
            has_trailing_pending,
            "pending steps after the failed step must be preserved"
        );

        // revision_count incremented to 1.
        assert_eq!(state.revision_count, 1);
    }

    #[test]
    fn apply_correction_max_revisions_returns_max_reached() {
        // max_revisions = 0 means the very first revision is rejected.
        let mut state = ReasoningEngineState::new()
            .with_goal("do it")
            .with_max_revisions(0);

        let trigger = CorrectionTrigger::StepFailed {
            step_index: 0,
            reason: "broken".to_owned(),
        };

        let outcome = engine().apply_correction(&mut state, trigger);
        assert_eq!(outcome, CorrectionOutcome::MaxRevisionsReached);
        // Plan must not have been touched.
        assert_eq!(state.revision_count, 0);
    }

    #[test]
    fn apply_correction_high_uncertainty_adds_reevaluate_step() {
        let mut state = state_with_pending_step();
        state.uncertainty = 0.9;

        let trigger = CorrectionTrigger::HighUncertainty {
            uncertainty: 0.9,
            threshold: 0.75,
        };
        let outcome = engine().apply_correction(&mut state, trigger);

        assert!(
            matches!(outcome, CorrectionOutcome::Corrected { .. }),
            "expected Corrected, got {outcome:?}"
        );

        // The first step of the revised plan must be the re-evaluation ModelCall.
        let first = state.plan.first().expect("revised plan must not be empty");
        assert!(
            matches!(
                &first.action,
                StepAction::ModelCall { prompt } if prompt.contains("Re-evaluate")
            ),
            "first step must be the re-evaluation ModelCall, got: {:?}",
            first.action
        );
    }

    #[test]
    fn apply_correction_empty_plan_adds_goal_step() {
        let mut state = ReasoningEngineState::new().with_goal("find the answer");
        // Plan is empty — trigger EmptyPlan directly.

        let outcome = engine().apply_correction(&mut state, CorrectionTrigger::EmptyPlan);

        assert!(
            matches!(outcome, CorrectionOutcome::Corrected { .. }),
            "expected Corrected, got {outcome:?}"
        );
        assert_eq!(state.plan.len(), 1, "plan must have exactly one seed step");

        let prompt = match &state.plan[0].action {
            StepAction::ModelCall { prompt } => prompt.as_str(),
            other => panic!("expected ModelCall, got {other:?}"),
        };
        assert!(
            prompt.contains("find the answer"),
            "seed step prompt must reference the goal: {prompt}"
        );
        assert!(
            prompt.contains("re-analyse goal"),
            "seed step prompt must mention re-analysis: {prompt}"
        );
    }

    #[test]
    fn apply_correction_empty_plan_uses_unknown_goal_when_none() {
        // goal is None → fall back to the literal string "unknown goal".
        let mut state = ReasoningEngineState::new(); // goal = None

        let outcome = engine().apply_correction(&mut state, CorrectionTrigger::EmptyPlan);

        assert!(matches!(outcome, CorrectionOutcome::Corrected { .. }));
        let prompt = match &state.plan[0].action {
            StepAction::ModelCall { prompt } => prompt.as_str(),
            other => panic!("expected ModelCall, got {other:?}"),
        };
        assert!(
            prompt.contains("unknown goal"),
            "prompt must fall back to 'unknown goal': {prompt}"
        );
    }

    #[test]
    fn apply_correction_feedback_adds_feedback_step() {
        let mut state = state_with_pending_step();
        let original_len = state.plan.len();

        let outcome = engine().apply_correction(
            &mut state,
            CorrectionTrigger::ExternalFeedback("wrong output".to_owned()),
        );

        assert!(
            matches!(outcome, CorrectionOutcome::Corrected { .. }),
            "expected Corrected, got {outcome:?}"
        );

        // Feedback step must be first.
        let first = &state.plan[0];
        assert!(
            matches!(
                &first.action,
                StepAction::ModelCall { prompt } if prompt.contains("wrong output")
            ),
            "first step must be the feedback ModelCall"
        );

        // Original steps preserved, shifted by one.
        assert_eq!(
            state.plan.len(),
            original_len + 1,
            "plan must be original steps plus one feedback step"
        );
    }

    #[test]
    fn apply_correction_revises_plan_and_increments_revision_count() {
        let mut state = state_with_pending_step();
        assert_eq!(state.revision_count, 0);

        engine().apply_correction(
            &mut state,
            CorrectionTrigger::ExternalFeedback("try again".to_owned()),
        );

        assert_eq!(
            state.revision_count, 1,
            "revision_count must increment by one"
        );
    }

    // -----------------------------------------------------------------------
    // 99.3 — Loop safeguards
    // -----------------------------------------------------------------------

    #[test]
    fn maybe_correct_returns_none_when_stable() {
        // Low uncertainty, no failed steps, non-empty plan → nothing fires.
        let mut state = state_with_pending_step();
        state.uncertainty = 0.1; // well below 0.75

        let result = engine().maybe_correct(&mut state);
        assert!(result.is_none(), "stable state must produce no correction");
    }

    #[test]
    fn maybe_correct_returns_some_when_trigger_fires() {
        let mut state = state_with_pending_step();
        state.uncertainty = 0.95; // above threshold

        let result = engine().maybe_correct(&mut state);
        assert!(
            result.is_some(),
            "high uncertainty must produce a correction"
        );

        let (trigger, outcome) = result.unwrap();
        assert!(matches!(trigger, CorrectionTrigger::HighUncertainty { .. }));
        assert!(matches!(outcome, CorrectionOutcome::Corrected { .. }));
    }

    #[test]
    fn correct_until_stable_stops_when_no_trigger() {
        // Terminal state: should_correct immediately returns None.
        let mut state = ReasoningEngineState::new().with_goal("already done");
        state.state = EngineState::Complete;

        let results = engine().correct_until_stable(&mut state, 100);
        assert!(
            results.is_empty(),
            "terminal state must produce an empty round log"
        );
    }

    #[test]
    fn correct_until_stable_stops_at_max_rounds() {
        // High uncertainty fires every round; max_revisions is high enough
        // that it never caps us before max_rounds does.
        let mut state = ReasoningEngineState::new()
            .with_goal("keep going")
            .with_max_revisions(100);

        state.uncertainty = 0.95; // consistently above 0.75 threshold
        // Seed a pending step so EmptyPlan does not mask the uncertainty trigger.
        state.plan.push(PlanStep::new("seed", StepAction::NoOp));

        let results = engine().correct_until_stable(&mut state, 3);
        assert_eq!(
            results.len(),
            3,
            "exactly max_rounds iterations must be performed"
        );

        for (_, outcome) in &results {
            assert!(
                matches!(outcome, CorrectionOutcome::Corrected { .. }),
                "every round must report Corrected within max_revisions budget"
            );
        }
    }

    #[test]
    fn correct_until_stable_stops_on_max_revisions_reached() {
        // max_revisions=1: round 1 → Corrected (revision_count 0→1),
        //                  round 2 → MaxRevisionsReached (1 >= 1).
        let mut state = ReasoningEngineState::new()
            .with_goal("attempt")
            .with_max_revisions(1);

        state.uncertainty = 0.95;
        state.plan.push(PlanStep::new("seed", StepAction::NoOp));

        let results = engine().correct_until_stable(&mut state, 100);

        // Loop must have stopped after the MaxRevisionsReached outcome.
        assert!(!results.is_empty(), "at least one round must execute");
        assert_eq!(
            results.last().unwrap().1,
            CorrectionOutcome::MaxRevisionsReached,
            "last outcome must be MaxRevisionsReached"
        );
        // No more than 2 rounds: one Corrected + one MaxRevisionsReached.
        assert!(
            results.len() <= 2,
            "loop must not continue past MaxRevisionsReached"
        );
    }

    // -----------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------

    #[test]
    fn display_step_failed() {
        let t = CorrectionTrigger::StepFailed {
            step_index: 3,
            reason: "timeout".to_owned(),
        };
        assert_eq!(t.to_string(), "step 3 failed: timeout");
    }

    #[test]
    fn display_high_uncertainty() {
        let t = CorrectionTrigger::HighUncertainty {
            uncertainty: 0.876_54,
            threshold: 0.75,
        };
        // "high uncertainty 0.88 > 0.75"
        assert_eq!(t.to_string(), "high uncertainty 0.88 > 0.75");
    }

    #[test]
    fn display_empty_plan() {
        assert_eq!(CorrectionTrigger::EmptyPlan.to_string(), "empty plan");
    }

    #[test]
    fn display_external_feedback() {
        let t = CorrectionTrigger::ExternalFeedback("wrong output".to_owned());
        assert_eq!(t.to_string(), "feedback: wrong output");
    }
}
