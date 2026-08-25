//! Reasoning Engine Observability — Task 100.
//!
//! Provides [`EngineObserver`] and [`ObserverConfig`] for structured logging
//! of reasoning engine state transitions, plan revisions, step completions,
//! and self-correction events.  Also exposes [`is_safe_to_log`] and
//! [`redact_state`] as best-effort privacy helpers.
//!
//! # Architecture
//!
//! ```text
//! EngineObserver
//!   ├── log_state_transition()  ──► tracing::info! / debug! / trace!
//!   ├── log_plan_revision()     ──► tracing::info! / debug!
//!   ├── log_step_complete()     ──► tracing::debug! / trace!
//!   └── log_correction()        ──► tracing::warn!  (always, unless Off/suppressed)
//! ```
//!
//! All string fields pass through [`ObserverConfig::redaction`] before being
//! emitted, ensuring that sensitive content is replaced with `[REDACTED]`
//! even when debug-level logging is enabled.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use grok_cli::engine::observability::{EngineObserver, ObserverConfig};
//! use grok_cli::engine::state::EngineState;
//!
//! // Production: suppressed, Summary level.
//! let obs = EngineObserver::production();
//!
//! // Development: Trace level, no suppression.
//! let obs = EngineObserver::debug_mode();
//!
//! obs.log_state_transition("engine-1", &EngineState::AnalyzeGoal, &EngineState::ExpandOptions, 0.5);
//! ```

use tracing::{debug, info, trace, warn};

use crate::engine::state::{EngineState, PlanStep, ReasoningEngineState};
use crate::rpl::{ReasoningLogLevel, RedactionConfig};

// ---------------------------------------------------------------------------
// ObserverConfig
// ---------------------------------------------------------------------------

/// Configuration for an [`EngineObserver`].
///
/// Controls verbosity, redaction rules, and whether the observer is globally
/// suppressed.  Use [`ObserverConfig::production`] in deployed environments
/// and [`ObserverConfig::debug_mode`] during development.
#[derive(Debug, Clone)]
pub struct ObserverConfig {
    /// Controls the verbosity of emitted log events.
    pub log_level: ReasoningLogLevel,

    /// Redaction rules applied to every string field before it is logged.
    ///
    /// Defaults to [`RedactionConfig::default_rules`], which covers
    /// `api_key`, `secret`, and `password` patterns.
    pub redaction: RedactionConfig,

    /// When `true`, all log events are suppressed unless the level is
    /// [`ReasoningLogLevel::Trace`] (which bypasses suppression but is
    /// typically filtered by the tracing subscriber in release builds).
    ///
    /// The safe default is `true`.
    pub suppressed: bool,
}

impl Default for ObserverConfig {
    /// Returns a production-safe configuration: `Summary` level, suppressed.
    fn default() -> Self {
        Self {
            log_level: ReasoningLogLevel::Summary,
            redaction: RedactionConfig::default_rules(),
            suppressed: true,
        }
    }
}

impl ObserverConfig {
    /// Production default: `Summary` level, suppressed, default redaction rules.
    ///
    /// Safe for deployed environments — no engine-internal detail leaks into
    /// logs.
    pub fn production() -> Self {
        Self::default()
    }

    /// Debug mode: `Trace` level, not suppressed, default redaction rules.
    ///
    /// Emits detailed trace events including serialised state variants.
    /// **Do not use in production.**
    pub fn debug_mode() -> Self {
        Self {
            log_level: ReasoningLogLevel::Trace,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        }
    }
}

// ---------------------------------------------------------------------------
// EngineObserver
// ---------------------------------------------------------------------------

/// Structured logger for reasoning engine transitions and decisions.
///
/// Integrates with the RPL suppression system: when `config.suppressed` is
/// `true` and no debug override is active, no engine-level log events are
/// emitted (beyond the tracing crate's own filtering).
///
/// # Suppression semantics
///
/// | `suppressed` | `log_level` | Events emitted?                              |
/// |-------------|-------------|----------------------------------------------|
/// | `false`     | `Off`       | No                                           |
/// | `false`     | any other   | Yes, at the configured level                 |
/// | `true`      | `Trace`     | Yes (trace-filtered by subscriber at runtime)|
/// | `true`      | any other   | No                                           |
///
/// # Usage
///
/// ```rust,ignore
/// use grok_cli::engine::observability::EngineObserver;
/// use grok_cli::engine::state::EngineState;
///
/// let obs = EngineObserver::debug_mode();
/// obs.log_state_transition("eng-1", &EngineState::AnalyzeGoal, &EngineState::ExpandOptions, 0.4);
/// ```
pub struct EngineObserver {
    config: ObserverConfig,
}

impl EngineObserver {
    /// Create a new [`EngineObserver`] with the given [`ObserverConfig`].
    pub fn new(config: ObserverConfig) -> Self {
        Self { config }
    }

    /// Returns a production-safe observer (`Summary` level, suppressed).
    ///
    /// No engine-internal details are emitted in this mode.
    pub fn production() -> Self {
        Self::new(ObserverConfig::production())
    }

    /// Returns a debug observer (`Trace` level, not suppressed).
    ///
    /// Emits detailed structured events including serialised state.
    /// **Do not use in production.**
    pub fn debug_mode() -> Self {
        Self::new(ObserverConfig::debug_mode())
    }

    /// Returns `true` if any log events will be emitted.
    ///
    /// Returns `false` when `config.suppressed && config.log_level != Trace`
    /// (`Trace` level always emits regardless of suppression, but is usually
    /// filtered out in release builds by the tracing subscriber).
    ///
    /// Also returns `false` when `config.log_level` is
    /// [`ReasoningLogLevel::Off`].
    pub fn is_active(&self) -> bool {
        if self.config.log_level == ReasoningLogLevel::Off {
            return false;
        }
        if self.config.suppressed && self.config.log_level != ReasoningLogLevel::Trace {
            return false;
        }
        true
    }

    /// Log a state transition.
    ///
    /// | Level     | Tracing macro     | Fields emitted                                           |
    /// |-----------|-------------------|----------------------------------------------------------|
    /// | `Off`     | *(silent)*        | nothing                                                  |
    /// | `Summary` | `tracing::info!`  | `engine_id`, `from`, `to` (state names)                  |
    /// | `Debug`   | `tracing::debug!` | above + `uncertainty`                                    |
    /// | `Trace`   | `tracing::trace!` | above + `from_json`, `to_json` (serialised variants)     |
    ///
    /// All string fields are passed through `config.redaction.apply_all`
    /// before logging.  Serialisation failures at `Trace` level are swallowed
    /// rather than propagated.
    pub fn log_state_transition(
        &self,
        engine_id: &str,
        from: &EngineState,
        to: &EngineState,
        uncertainty: f32,
    ) {
        if !self.is_active() {
            return;
        }

        let safe_id = self.config.redaction.apply_all(engine_id);
        let from_name = engine_state_name(from);
        let to_name = engine_state_name(to);

        match self.config.log_level {
            ReasoningLogLevel::Off => {}

            ReasoningLogLevel::Summary => {
                info!(
                    engine_id = %safe_id,
                    from      = %from_name,
                    to        = %to_name,
                    "engine: state transition"
                );
            }

            ReasoningLogLevel::Debug => {
                debug!(
                    engine_id   = %safe_id,
                    from        = %from_name,
                    to          = %to_name,
                    uncertainty = uncertainty,
                    "engine: state transition"
                );
            }

            ReasoningLogLevel::Trace => {
                let from_json = serde_json::to_string(from)
                    .unwrap_or_else(|e| format!("<serialisation error: {e}>"));
                let to_json = serde_json::to_string(to)
                    .unwrap_or_else(|e| format!("<serialisation error: {e}>"));
                trace!(
                    engine_id   = %safe_id,
                    from        = %from_name,
                    to          = %to_name,
                    uncertainty = uncertainty,
                    from_json   = %from_json,
                    to_json     = %to_json,
                    "engine: state transition (trace)"
                );
            }
        }
    }

    /// Log a plan revision.
    ///
    /// | Level     | Tracing macro     | Fields emitted                                      |
    /// |-----------|-------------------|-----------------------------------------------------|
    /// | `Off`     | *(silent)*        | nothing                                             |
    /// | `Summary` | `tracing::info!`  | `engine_id`, `revision_count`, `step_count`         |
    /// | `Debug`+  | `tracing::debug!` | above + `reason` (redacted)                         |
    ///
    /// `Debug` and `Trace` both use `tracing::debug!`; the reason string is
    /// always redacted before emission.
    pub fn log_plan_revision(
        &self,
        engine_id: &str,
        revision_count: u32,
        new_step_count: usize,
        reason: &str,
    ) {
        if !self.is_active() {
            return;
        }

        let safe_id = self.config.redaction.apply_all(engine_id);
        let safe_reason = self.config.redaction.apply_all(reason);

        match self.config.log_level {
            ReasoningLogLevel::Off => {}

            ReasoningLogLevel::Summary => {
                info!(
                    engine_id      = %safe_id,
                    revision_count = revision_count,
                    step_count     = new_step_count,
                    "engine: plan revised"
                );
            }

            ReasoningLogLevel::Debug | ReasoningLogLevel::Trace => {
                debug!(
                    engine_id      = %safe_id,
                    revision_count = revision_count,
                    step_count     = new_step_count,
                    reason         = %safe_reason,
                    "engine: plan revised"
                );
            }
        }
    }

    /// Log completion of a plan step.
    ///
    /// | Level     | Tracing macro     | Fields emitted                                   |
    /// |-----------|-------------------|--------------------------------------------------|
    /// | `Off`     | *(silent)*        | nothing                                          |
    /// | `Summary` | *(silent)*        | step completions are too granular for summary    |
    /// | `Debug`   | `tracing::debug!` | `engine_id`, `step_id`, `status`                 |
    /// | `Trace`   | `tracing::trace!` | above + `result` (redacted, empty if `None`)     |
    pub fn log_step_complete(&self, engine_id: &str, step: &PlanStep) {
        if !self.is_active() {
            return;
        }

        let safe_id = self.config.redaction.apply_all(engine_id);
        let status_name = step_status_name(&step.status);

        match self.config.log_level {
            ReasoningLogLevel::Off | ReasoningLogLevel::Summary => {}

            ReasoningLogLevel::Debug => {
                debug!(
                    engine_id = %safe_id,
                    step_id   = %step.step_id,
                    status    = %status_name,
                    "engine: step complete"
                );
            }

            ReasoningLogLevel::Trace => {
                let safe_result = step
                    .result
                    .as_deref()
                    .map(|r| self.config.redaction.apply_all(r))
                    .unwrap_or_default();
                trace!(
                    engine_id = %safe_id,
                    step_id   = %step.step_id,
                    status    = %status_name,
                    result    = %safe_result,
                    "engine: step complete (trace)"
                );
            }
        }
    }

    /// Log a self-correction event.
    ///
    /// Always emits [`tracing::warn!`] for any level other than `Off`
    /// (corrections indicate unexpected situations that warrant attention
    /// regardless of the configured verbosity level).
    ///
    /// | Level | Tracing macro    | Fields emitted                                          |
    /// |-------|------------------|---------------------------------------------------------|
    /// | `Off` | *(silent)*       | nothing                                                 |
    /// | any   | `tracing::warn!` | `engine_id`, `trigger` (redacted), `revision_count`     |
    ///
    /// The `trigger` string is passed through redaction before emission.
    pub fn log_correction(&self, engine_id: &str, trigger: &str, revision_count: u32) {
        if !self.is_active() {
            return;
        }

        let safe_id = self.config.redaction.apply_all(engine_id);
        let safe_trigger = self.config.redaction.apply_all(trigger);

        warn!(
            engine_id      = %safe_id,
            trigger        = %safe_trigger,
            revision_count = revision_count,
            "engine: self-correction triggered"
        );
    }
}

impl Default for EngineObserver {
    /// Returns a production-safe observer.  See [`EngineObserver::production`].
    fn default() -> Self {
        Self::production()
    }
}

// ---------------------------------------------------------------------------
// Privacy helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the engine state contains no obviously sensitive content.
///
/// Checks [`ReasoningEngineState::goal`] and every [`PlanStep::description`]
/// and [`PlanStep::result`] against the patterns that the default
/// [`RedactionConfig`] would redact (`api_key`, `secret`, `password`
/// patterns).
///
/// This is a best-effort check intended for use in debug assertions and test
/// helpers; it is not a substitute for proper secret management.
///
/// # Note
///
/// This function constructs a temporary [`RedactionConfig`] on each call.
/// Avoid calling it in tight hot paths.
pub fn is_safe_to_log(state: &ReasoningEngineState) -> bool {
    let redaction = RedactionConfig::default_rules();

    if let Some(goal) = &state.goal
        && redaction.apply_all(goal) != *goal
    {
        return false;
    }

    for step in &state.plan {
        if redaction.apply_all(&step.description) != step.description {
            return false;
        }
        if let Some(result) = &step.result
            && redaction.apply_all(result) != *result
        {
            return false;
        }
    }

    true
}

/// Apply redaction to all string fields of a [`ReasoningEngineState`] clone
/// and return the redacted copy.
///
/// The following fields are redacted using the provided [`RedactionConfig`]:
///
/// - [`ReasoningEngineState::goal`] (if `Some`)
/// - Each [`PlanStep::description`]
/// - Each [`PlanStep::result`] (if `Some`)
///
/// IDs, enum variants, timestamps, and numeric fields are **not** modified.
///
/// # Example
///
/// ```rust,ignore
/// use grok_cli::engine::observability::redact_state;
/// use grok_cli::engine::state::ReasoningEngineState;
/// use grok_cli::rpl::RedactionConfig;
///
/// let mut state = ReasoningEngineState::new();
/// state.goal = Some("Connect using api_key=secret123".to_string());
///
/// let rules = RedactionConfig::default_rules();
/// let safe = redact_state(&state, &rules);
/// assert!(safe.goal.unwrap().contains("[REDACTED]"));
/// ```
pub fn redact_state(
    state: &ReasoningEngineState,
    redaction: &RedactionConfig,
) -> ReasoningEngineState {
    let mut out = state.clone();

    if let Some(goal) = &out.goal {
        out.goal = Some(redaction.apply_all(goal));
    }

    for step in &mut out.plan {
        step.description = redaction.apply_all(&step.description);
        if let Some(result) = &step.result {
            step.result = Some(redaction.apply_all(result));
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Returns a short, stable, human-readable name for an [`EngineState`] variant.
///
/// Used in log field values to avoid leaking internal details (e.g. step
/// indices or failure reason strings) into structured log output.
#[inline]
fn engine_state_name(state: &EngineState) -> &'static str {
    match state {
        EngineState::AnalyzeGoal => "AnalyzeGoal",
        EngineState::ExpandOptions => "ExpandOptions",
        EngineState::EvaluateOptions => "EvaluateOptions",
        EngineState::CommitPlan => "CommitPlan",
        EngineState::ExecuteStep { .. } => "ExecuteStep",
        EngineState::RevisePlan => "RevisePlan",
        EngineState::Complete => "Complete",
        EngineState::Failed { .. } => "Failed",
    }
}

/// Returns a short, stable, human-readable name for a
/// [`crate::engine::state::StepStatus`] variant.
///
/// Used in log field values to keep output compact and consistent.
#[inline]
fn step_status_name(status: &crate::engine::state::StepStatus) -> &'static str {
    use crate::engine::state::StepStatus;
    match status {
        StepStatus::Pending => "Pending",
        StepStatus::InProgress => "InProgress",
        StepStatus::Completed => "Completed",
        StepStatus::Failed { .. } => "Failed",
        StepStatus::Skipped => "Skipped",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{PlanStep, ReasoningEngineState, StepAction, StepStatus};
    use crate::rpl::{ReasoningLogLevel, RedactionConfig};

    // ── ObserverConfig / suppression ─────────────────────────────────────────

    /// A production observer must start suppressed.
    #[test]
    fn production_observer_is_suppressed() {
        let obs = EngineObserver::production();
        assert!(
            obs.config.suppressed,
            "production observer must be suppressed"
        );
    }

    /// A debug-mode observer must not be suppressed.
    #[test]
    fn debug_observer_is_not_suppressed() {
        let obs = EngineObserver::debug_mode();
        assert!(
            !obs.config.suppressed,
            "debug observer must not be suppressed"
        );
    }

    // ── is_active ────────────────────────────────────────────────────────────

    /// `Off` level must always yield an inactive observer, regardless of suppression.
    #[test]
    fn is_active_returns_false_for_off_level() {
        let obs = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Off,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        });
        assert!(
            !obs.is_active(),
            "Off level must be inactive regardless of suppression"
        );
    }

    /// A debug-mode observer (Trace, not suppressed) must be active.
    #[test]
    fn is_active_returns_true_for_debug_mode() {
        let obs = EngineObserver::debug_mode();
        assert!(obs.is_active(), "debug_mode observer must be active");
    }

    /// A suppressed observer at Summary level must be inactive.
    #[test]
    fn is_active_returns_false_for_suppressed_summary() {
        let obs = EngineObserver::production(); // suppressed + Summary
        assert!(
            !obs.is_active(),
            "production (suppressed, Summary) observer must be inactive"
        );
    }

    /// A suppressed observer at Trace level must still be active
    /// (Trace bypasses suppression — the subscriber filters it at runtime).
    #[test]
    fn is_active_returns_true_for_suppressed_trace() {
        let obs = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Trace,
            redaction: RedactionConfig::default_rules(),
            suppressed: true,
        });
        assert!(
            obs.is_active(),
            "Trace level must be active even when suppressed"
        );
    }

    // ── log_state_transition smoke tests ─────────────────────────────────────

    /// `log_state_transition` at `Off` must not panic.
    #[test]
    fn log_state_transition_off_does_not_panic() {
        let obs = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Off,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        });
        obs.log_state_transition(
            "test-engine",
            &EngineState::AnalyzeGoal,
            &EngineState::ExpandOptions,
            0.5,
        );
    }

    /// `log_state_transition` at `Summary` must not panic.
    #[test]
    fn log_state_transition_summary_does_not_panic() {
        let obs = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Summary,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        });
        obs.log_state_transition(
            "test-engine",
            &EngineState::ExpandOptions,
            &EngineState::EvaluateOptions,
            0.4,
        );
    }

    /// `log_state_transition` at `Debug` must not panic.
    #[test]
    fn log_state_transition_debug_does_not_panic() {
        let obs = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Debug,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        });
        obs.log_state_transition(
            "test-engine",
            &EngineState::CommitPlan,
            &EngineState::ExecuteStep { step_index: 0 },
            0.2,
        );
    }

    /// `log_state_transition` at `Trace` must not panic (includes JSON serialisation).
    #[test]
    fn log_state_transition_trace_does_not_panic() {
        let obs = EngineObserver::debug_mode();
        obs.log_state_transition(
            "test-engine",
            &EngineState::ExecuteStep { step_index: 2 },
            &EngineState::Complete,
            0.1,
        );
    }

    // ── log_plan_revision smoke test ──────────────────────────────────────────

    /// `log_plan_revision` must not panic for any combination of level and reason.
    #[test]
    fn log_plan_revision_does_not_panic() {
        // Summary level
        let obs_summary = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Summary,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        });
        obs_summary.log_plan_revision("engine-s", 1, 4, "step 2 timed out");

        // Debug level
        let obs_debug = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Debug,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        });
        obs_debug.log_plan_revision(
            "engine-d",
            2,
            5,
            "step 3 failed, retrying with different tool",
        );

        // Trace level (debug_mode)
        let obs_trace = EngineObserver::debug_mode();
        obs_trace.log_plan_revision("engine-t", 3, 6, "unexpected tool error on step 4");
    }

    // ── log_step_complete smoke test ──────────────────────────────────────────

    /// `log_step_complete` must not panic for all status variants at Trace level.
    #[test]
    fn log_step_complete_does_not_panic() {
        let obs = EngineObserver::debug_mode();

        let mut step = PlanStep::new(
            "Call list_directory on /tmp",
            StepAction::UseTool {
                tool_name: "list_directory".to_string(),
                args: serde_json::json!({ "path": "/tmp" }),
            },
        );
        step.status = StepStatus::Completed;
        step.result = Some("files: a.txt, b.txt".to_string());
        obs.log_step_complete("test-engine", &step);

        // Failed variant
        let mut step_failed = PlanStep::new("No-op step", StepAction::NoOp);
        step_failed.status = StepStatus::Failed {
            reason: "mock failure".to_string(),
        };
        obs.log_step_complete("test-engine", &step_failed);

        // Skipped variant
        let mut step_skipped = PlanStep::new("Skipped step", StepAction::NoOp);
        step_skipped.status = StepStatus::Skipped;
        obs.log_step_complete("test-engine", &step_skipped);
    }

    // ── log_correction smoke test ─────────────────────────────────────────────

    /// `log_correction` must not panic for any active configuration.
    #[test]
    fn log_correction_does_not_panic() {
        // debug_mode — should warn
        let obs = EngineObserver::debug_mode();
        obs.log_correction("test-engine", "unexpected tool failure on step 2", 1);

        // Off level — should be silent (no panic)
        let obs_off = EngineObserver::new(ObserverConfig {
            log_level: ReasoningLogLevel::Off,
            redaction: RedactionConfig::default_rules(),
            suppressed: false,
        });
        obs_off.log_correction("test-engine", "trigger text", 0);

        // Production (suppressed) — should be silent (no panic)
        let obs_prod = EngineObserver::production();
        obs_prod.log_correction("test-engine", "trigger text", 2);
    }

    // ── is_safe_to_log ────────────────────────────────────────────────────────

    /// A state with only clean content must return `true`.
    #[test]
    fn is_safe_to_log_clean_state_returns_true() {
        let mut state = ReasoningEngineState::new();
        state.goal = Some("List all files in the project directory".to_string());
        assert!(is_safe_to_log(&state), "clean state must be safe to log");
    }

    /// A state whose goal contains an `api_key=…` pattern must return `false`.
    #[test]
    fn is_safe_to_log_sensitive_goal_returns_false() {
        let mut state = ReasoningEngineState::new();
        state.goal = Some("Connect to service using api_key=supersecret123".to_string());
        assert!(
            !is_safe_to_log(&state),
            "state with api_key in goal must not be safe to log"
        );
    }

    /// A state whose plan step description contains a `password=…` pattern must
    /// return `false`.
    #[test]
    fn is_safe_to_log_sensitive_step_returns_false() {
        let mut state = ReasoningEngineState::new();
        let step = PlanStep::new("Login with password=hunter2 to the API", StepAction::NoOp);
        state.plan.push(step);
        assert!(
            !is_safe_to_log(&state),
            "state with password in step description must not be safe to log"
        );
    }

    // ── redact_state ──────────────────────────────────────────────────────────

    /// `redact_state` must remove a sensitive token from the goal field.
    #[test]
    fn redact_state_removes_sensitive_goal() {
        let mut state = ReasoningEngineState::new();
        state.goal = Some("Authenticate with token=mytoken123".to_string());

        let redaction = RedactionConfig::default_rules();
        let redacted = redact_state(&state, &redaction);

        let goal = redacted
            .goal
            .expect("goal must remain Some after redaction");
        assert!(
            !goal.contains("mytoken123"),
            "redacted goal must not contain the raw token value; got: {goal}"
        );
        assert!(
            goal.contains("[REDACTED]"),
            "redacted goal must contain [REDACTED] placeholder; got: {goal}"
        );
    }

    /// `redact_state` must remove sensitive values from plan step descriptions
    /// and result fields.
    #[test]
    fn redact_state_removes_sensitive_step_description() {
        let mut state = ReasoningEngineState::new();

        let mut step = PlanStep::new(
            "Connect using password=hunter2 to authenticate",
            StepAction::NoOp,
        );
        step.result = Some("Response includes secret=xyz captured".to_string());
        state.plan.push(step);

        let redaction = RedactionConfig::default_rules();
        let redacted = redact_state(&state, &redaction);

        let desc = &redacted.plan[0].description;
        assert!(
            !desc.contains("hunter2"),
            "redacted description must not contain the raw password; got: {desc}"
        );
        assert!(
            desc.contains("[REDACTED]"),
            "redacted description must contain [REDACTED]; got: {desc}"
        );

        let result = redacted.plan[0]
            .result
            .as_deref()
            .expect("result must remain Some after redaction");
        assert!(
            !result.contains("xyz"),
            "redacted result must not contain the raw secret value; got: {result}"
        );
        assert!(
            result.contains("[REDACTED]"),
            "redacted result must contain [REDACTED]; got: {result}"
        );
    }

    /// `redact_state` must not modify the original state.
    #[test]
    fn redact_state_does_not_modify_original() {
        let mut state = ReasoningEngineState::new();
        state.goal = Some("Use api_key=topsecret".to_string());

        let redaction = RedactionConfig::default_rules();
        let _redacted = redact_state(&state, &redaction);

        // Original must be unchanged.
        assert_eq!(
            state.goal.as_deref(),
            Some("Use api_key=topsecret"),
            "original state must not be mutated by redact_state"
        );
    }

    /// `redact_state` with a `None` goal must leave `goal` as `None`.
    #[test]
    fn redact_state_none_goal_stays_none() {
        let state = ReasoningEngineState::new(); // goal is None by default
        let redaction = RedactionConfig::default_rules();
        let redacted = redact_state(&state, &redaction);
        assert!(
            redacted.goal.is_none(),
            "None goal must remain None after redaction"
        );
    }

    // ── engine_state_name coverage ────────────────────────────────────────────

    /// Every `EngineState` variant must produce a non-empty, stable name.
    #[test]
    fn engine_state_name_covers_all_variants() {
        let variants: &[EngineState] = &[
            EngineState::AnalyzeGoal,
            EngineState::ExpandOptions,
            EngineState::EvaluateOptions,
            EngineState::CommitPlan,
            EngineState::ExecuteStep { step_index: 0 },
            EngineState::RevisePlan,
            EngineState::Complete,
            EngineState::Failed {
                reason: "test".to_string(),
            },
        ];
        for variant in variants {
            let name = engine_state_name(variant);
            assert!(!name.is_empty(), "engine_state_name must not be empty");
        }
    }

    // ── step_status_name coverage ─────────────────────────────────────────────

    /// Every `StepStatus` variant must produce a non-empty, stable name.
    #[test]
    fn step_status_name_covers_all_variants() {
        use crate::engine::state::StepStatus;
        let variants = [
            StepStatus::Pending,
            StepStatus::InProgress,
            StepStatus::Completed,
            StepStatus::Failed {
                reason: "err".to_string(),
            },
            StepStatus::Skipped,
        ];
        for variant in &variants {
            let name = step_status_name(variant);
            assert!(!name.is_empty(), "step_status_name must not be empty");
        }
    }
}
