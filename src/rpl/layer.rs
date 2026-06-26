//! Reasoning Protocol Layer – lifecycle management.
//!
//! [`RplLayer`] is the primary entry point for the RPL subsystem.  It manages
//! the lifecycle of a [`ReasoningTrace`] across a single `route_with_tools`
//! invocation, from initial construction through tool selection, and finally
//! validation and logging on completion.
//!
//! # Typical call sequence
//!
//! ```text
//! let layer = RplLayer::with_default_config();
//!
//! // 1. At the start of route_with_tools
//! let mut trace = layer.on_pre_evaluate(Some("list /tmp"), None);
//!
//! // 2. For each tool considered during the loop
//! layer.on_tool_selection(&mut trace, "list_directory", true, Some(0.92), Some("path arg"));
//! layer.on_tool_selection(&mut trace, "read_file", false, None, None);
//!
//! // 3. When the loop finishes (success or max-iterations)
//! layer.on_complete(&mut trace);
//! ```
//!
//! Integration with `CpuRouter::route_with_tools` is handled externally;
//! this module only manages the trace data and emits logs.

use tracing::{debug, warn};

use crate::rpl::{
    logging::{log_trace, ReasoningLogLevel},
    schema::{ReasoningPhase, ReasoningTrace, ToolEvaluation},
    validation::validate,
};

// ---------------------------------------------------------------------------
// RplConfig
// ---------------------------------------------------------------------------

/// Configuration for the [`RplLayer`].
///
/// Use [`Default::default()`] to obtain safe production defaults:
/// - [`log_level`][Self::log_level]: [`ReasoningLogLevel::Summary`]
/// - [`lenient_validation`][Self::lenient_validation]: `true`
#[derive(Debug, Clone)]
pub struct RplConfig {
    /// Controls how much reasoning detail is emitted to the tracing log.
    ///
    /// See [`ReasoningLogLevel`] for the available verbosity levels.
    /// Defaults to [`ReasoningLogLevel::Summary`].
    pub log_level: ReasoningLogLevel,

    /// When `true`, validation errors are logged as warnings but do **not**
    /// abort execution.  The trace lifecycle continues normally.
    ///
    /// When `false`, the first call to [`RplLayer::on_complete`] that
    /// produces validation errors will log them, but the trace is still
    /// returned to the caller (aborting the parent router is the caller's
    /// responsibility).
    ///
    /// Defaults to `true` to avoid breaking the tool-execution loop due to
    /// a malformed trace in production.
    pub lenient_validation: bool,
}

impl Default for RplConfig {
    fn default() -> Self {
        Self {
            log_level: ReasoningLogLevel::Summary,
            lenient_validation: true,
        }
    }
}

// ---------------------------------------------------------------------------
// RplLayer
// ---------------------------------------------------------------------------

/// The Reasoning Protocol Layer.
///
/// Attach to a `CpuRouter` via `CpuRouter::with_rpl` (implemented externally)
/// to enable structured reasoning traces across the tool-execution loop.
///
/// # Thread safety
///
/// [`RplLayer`] holds no mutable state; a single instance can be shared
/// across concurrent routing calls.  Each call to [`on_pre_evaluate`]
/// produces an independent [`ReasoningTrace`] owned by the caller.
///
/// [`on_pre_evaluate`]: RplLayer::on_pre_evaluate
#[derive(Debug, Clone)]
pub struct RplLayer {
    config: RplConfig,
}

impl RplLayer {
    /// Create a new [`RplLayer`] with the supplied `config`.
    pub fn new(config: RplConfig) -> Self {
        Self { config }
    }

    /// Create a new [`RplLayer`] with [`RplConfig::default`].
    ///
    /// Equivalent to `RplLayer::new(RplConfig::default())`.
    pub fn with_default_config() -> Self {
        Self::new(RplConfig::default())
    }

    /// Called at the very start of a `route_with_tools` invocation.
    ///
    /// Creates and returns a fresh [`ReasoningTrace`] in the
    /// [`PreEvaluation`][ReasoningPhase::PreEvaluation] phase, pre-populated
    /// with the supplied `goal` and `context` if provided.
    ///
    /// The returned trace is **not** logged at this stage; logging happens in
    /// [`on_complete`][Self::on_complete] once the full picture is available.
    ///
    /// # Arguments
    ///
    /// * `goal`    – High-level goal of the routing call (e.g. a condensed
    ///   version of the user's message).  `None` if unknown.
    /// * `context` – Optional additional context (e.g. list of active skills
    ///   or a conversation summary).
    pub fn on_pre_evaluate(&self, goal: Option<&str>, context: Option<&str>) -> ReasoningTrace {
        let mut trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation);

        if let Some(g) = goal {
            trace = trace.with_goal(g);
        }
        if let Some(c) = context {
            trace = trace.with_context(c);
        }

        debug!(
            trace_id = %trace.trace_id,
            "rpl: pre-evaluation trace created"
        );

        trace
    }

    /// Called when a tool is being considered or selected during the loop.
    ///
    /// Appends a [`ToolEvaluation`] to the trace, advances the phase to
    /// [`ToolSelection`][ReasoningPhase::ToolSelection] if it is still at
    /// [`PreEvaluation`][ReasoningPhase::PreEvaluation], and emits a
    /// `tracing::debug!` event.
    ///
    /// # Arguments
    ///
    /// * `trace`            – The active reasoning trace for this routing call.
    /// * `tool_name`        – The registered name of the tool being evaluated.
    /// * `selected`         – Whether this tool was chosen for execution.
    /// * `relevance_score`  – Optional score (0.0–1.0). If `None`, defaults to
    ///                        1.0 when `selected`, else 0.0.
    /// * `reason`           – Optional explanation for the selection decision.
    pub fn on_tool_selection(
        &self,
        trace: &mut ReasoningTrace,
        tool_name: &str,
        selected: bool,
        relevance_score: Option<f32>,
        reason: Option<&str>,
    ) {
        // Advance phase if we're still in pre-evaluation.
        if trace.phase == ReasoningPhase::PreEvaluation {
            trace.phase = ReasoningPhase::ToolSelection;
        }

        // Preserve caller-provided score when meaningful; otherwise fall back
        // to the simple selected/not-selected default.
        let relevance_score = relevance_score.unwrap_or_else(|| {
            if selected { 1.0_f32 } else { 0.0_f32 }
        });

        let eval = ToolEvaluation {
            tool_name: tool_name.to_string(),
            relevance_score,
            reason: reason.map(str::to_string),
            selected,
        };

        debug!(
            trace_id  = %trace.trace_id,
            tool_name = %tool_name,
            selected  = selected,
            relevance = relevance_score,
            reason    = ?reason,
            "rpl: tool evaluated"
        );

        trace.add_tool_evaluation(eval);
    }

    /// Called when the tool-execution loop completes (success or
    /// max-iterations reached).
    ///
    /// This method:
    ///
    /// 1. Advances the trace phase to [`Complete`][ReasoningPhase::Complete].
    /// 2. Runs [`validate`] on the trace.
    ///    - If [`lenient_validation`][RplConfig::lenient_validation] is `true`,
    ///      any errors are logged as `tracing::warn!` events and execution
    ///      continues.
    ///    - If `lenient_validation` is `false`, errors are still logged but
    ///      no panic or abort is triggered from this layer.
    /// 3. Emits the trace via [`log_trace`] at the configured
    ///    [`log_level`][RplConfig::log_level].
    ///
    /// The trace is mutated in place so the caller can inspect the final
    /// state after this call returns.
    pub fn on_complete(&self, trace: &mut ReasoningTrace) {
        // Advance to the terminal phase.
        trace.phase = ReasoningPhase::Complete;

        // Validate – collect all errors in one pass.
        match validate(trace) {
            Ok(()) => {
                debug!(
                    trace_id = %trace.trace_id,
                    "rpl: trace validated successfully"
                );
            }
            Err(ref errors) => {
                // Always log validation errors regardless of lenient mode.
                for err in errors {
                    warn!(
                        trace_id = %trace.trace_id,
                        lenient  = self.config.lenient_validation,
                        error    = %err,
                        "rpl: trace validation error"
                    );
                }

                if !self.config.lenient_validation {
                    // In strict mode we've already logged every error above.
                    // Aborting is the caller's responsibility; we return
                    // normally so the trace is still available for inspection.
                    warn!(
                        trace_id = %trace.trace_id,
                        error_count = errors.len(),
                        "rpl: strict validation failed — caller should abort"
                    );
                }
            }
        }

        // Emit the completed trace at the configured log level.
        log_trace(trace, self.config.log_level);
    }

    /// Return a reference to the layer's configuration.
    pub fn config(&self) -> &RplConfig {
        &self.config
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpl::schema::{MemoryConsideration, ReasoningPhase};

    // ── RplConfig defaults ───────────────────────────────────────────────────

    /// Default config must have `Summary` log level and lenient validation.
    #[test]
    fn default_config_values() {
        let cfg = RplConfig::default();
        assert_eq!(cfg.log_level, ReasoningLogLevel::Summary);
        assert!(
            cfg.lenient_validation,
            "lenient_validation must default to true"
        );
    }

    // ── RplLayer construction ────────────────────────────────────────────────

    /// `new` and `with_default_config` must produce equivalent instances.
    #[test]
    fn new_and_with_default_config_are_equivalent() {
        let a = RplLayer::new(RplConfig::default());
        let b = RplLayer::with_default_config();

        assert_eq!(a.config().log_level, b.config().log_level);
        assert_eq!(a.config().lenient_validation, b.config().lenient_validation);
    }

    /// `config()` must return the same values that were passed to `new`.
    #[test]
    fn config_accessor_returns_stored_config() {
        let cfg = RplConfig {
            log_level: ReasoningLogLevel::Debug,
            lenient_validation: false,
        };
        let layer = RplLayer::new(cfg.clone());

        assert_eq!(layer.config().log_level, cfg.log_level);
        assert_eq!(layer.config().lenient_validation, cfg.lenient_validation);
    }

    // ── on_pre_evaluate ──────────────────────────────────────────────────────

    /// `on_pre_evaluate` with `None` args must produce a trace with no goal
    /// or context.
    #[test]
    fn on_pre_evaluate_none_args() {
        let layer = RplLayer::with_default_config();
        let trace = layer.on_pre_evaluate(None, None);

        assert_eq!(trace.phase, ReasoningPhase::PreEvaluation);
        assert!(trace.goal.is_none(), "goal must be None");
        assert!(trace.context.is_none(), "context must be None");
        assert!(!trace.trace_id.is_empty(), "trace_id must be populated");
    }

    /// `on_pre_evaluate` with `Some` args must store them in the trace.
    #[test]
    fn on_pre_evaluate_stores_goal_and_context() {
        let layer = RplLayer::with_default_config();
        let trace = layer.on_pre_evaluate(Some("list /tmp"), Some("session ctx"));

        assert_eq!(trace.goal.as_deref(), Some("list /tmp"));
        assert_eq!(trace.context.as_deref(), Some("session ctx"));
    }

    /// Each call to `on_pre_evaluate` must produce a unique `trace_id`.
    #[test]
    fn on_pre_evaluate_produces_unique_trace_ids() {
        let layer = RplLayer::with_default_config();

        let ids: Vec<String> = (0..4)
            .map(|_| layer.on_pre_evaluate(None, None).trace_id)
            .collect();

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "trace IDs must be unique");
            }
        }
    }

    // ── on_tool_selection ────────────────────────────────────────────────────

    /// `on_tool_selection` must append one evaluation per call.
    #[test]
    fn on_tool_selection_appends_evaluation() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(None, None);

        layer.on_tool_selection(&mut trace, "read_file", true, None, Some("file path in goal"));
        layer.on_tool_selection(&mut trace, "list_directory", false, None, None);

        assert_eq!(trace.tool_evaluations.len(), 2);
        assert_eq!(trace.tool_evaluations[0].tool_name, "read_file");
        assert!(trace.tool_evaluations[0].selected);
        assert_eq!(trace.tool_evaluations[1].tool_name, "list_directory");
        assert!(!trace.tool_evaluations[1].selected);
    }

    /// The first call to `on_tool_selection` must advance the phase from
    /// `PreEvaluation` to `ToolSelection`.
    #[test]
    fn on_tool_selection_advances_phase_from_pre_evaluation() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(None, None);

        assert_eq!(trace.phase, ReasoningPhase::PreEvaluation);
        layer.on_tool_selection(&mut trace, "any_tool", false, None, None);
        assert_eq!(trace.phase, ReasoningPhase::ToolSelection);
    }

    /// Subsequent calls to `on_tool_selection` must not regress the phase.
    #[test]
    fn on_tool_selection_does_not_regress_phase() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(None, None);

        layer.on_tool_selection(&mut trace, "tool_a", false, None, None);
        // Manually advance phase past ToolSelection.
        trace.phase = ReasoningPhase::ActionPlanning;

        layer.on_tool_selection(&mut trace, "tool_b", true, None, None);
        // Phase must remain at ActionPlanning, not regress to ToolSelection.
        assert_eq!(trace.phase, ReasoningPhase::ActionPlanning);
    }

    /// `selected = true` must produce `relevance_score = 1.0`.
    #[test]
    fn on_tool_selection_selected_sets_score_one() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(None, None);

        layer.on_tool_selection(&mut trace, "chosen_tool", true, None, None);
        let score = trace.tool_evaluations[0].relevance_score;
        assert!(
            (score - 1.0).abs() < f32::EPSILON,
            "selected tool must have relevance_score = 1.0, got {score}"
        );
    }

    /// `selected = false` must produce `relevance_score = 0.0`.
    #[test]
    fn on_tool_selection_not_selected_sets_score_zero() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(None, None);

        layer.on_tool_selection(&mut trace, "skipped_tool", false, None, None);
        let score = trace.tool_evaluations[0].relevance_score;
        assert!(
            score.abs() < f32::EPSILON,
            "unselected tool must have relevance_score = 0.0, got {score}"
        );
    }

    // ── on_complete ──────────────────────────────────────────────────────────

    /// `on_complete` must set the phase to `Complete`.
    #[test]
    fn on_complete_sets_phase_to_complete() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(None, None);

        layer.on_complete(&mut trace);
        assert_eq!(trace.phase, ReasoningPhase::Complete);
    }

    /// `on_complete` must not panic on a valid trace.
    #[test]
    fn on_complete_does_not_panic_on_valid_trace() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(Some("goal"), Some("ctx"));
        layer.on_tool_selection(&mut trace, "list_directory", true, None);
        layer.on_complete(&mut trace);
        // If we reach here without panic the test passes.
    }

    /// `on_complete` must not panic on a trace with an invalid field when
    /// `lenient_validation = true`.
    #[test]
    fn on_complete_lenient_does_not_panic_on_invalid_trace() {
        let cfg = RplConfig {
            log_level: ReasoningLogLevel::Off,
            lenient_validation: true,
        };
        let layer = RplLayer::new(cfg);
        let mut trace = layer.on_pre_evaluate(None, None);

        // Deliberately corrupt the trace.
        trace.uncertainty = 9.9;

        // Should log a warning and continue — must not panic.
        layer.on_complete(&mut trace);
        assert_eq!(trace.phase, ReasoningPhase::Complete);
    }

    /// `on_complete` must not panic in strict mode either; it only logs.
    #[test]
    fn on_complete_strict_logs_but_does_not_panic_on_invalid_trace() {
        let cfg = RplConfig {
            log_level: ReasoningLogLevel::Off,
            lenient_validation: false,
        };
        let layer = RplLayer::new(cfg);
        let mut trace = layer.on_pre_evaluate(None, None);

        // Deliberately corrupt the trace.
        trace.uncertainty = -5.0;

        // Must log and return normally; aborting is the caller's responsibility.
        layer.on_complete(&mut trace);
        assert_eq!(trace.phase, ReasoningPhase::Complete);
    }

    // ── full lifecycle ───────────────────────────────────────────────────────

    /// A complete lifecycle (pre_evaluate → tool_selection × N → complete)
    /// must leave the trace in the `Complete` phase with all evaluations
    /// preserved in insertion order.
    #[test]
    fn full_lifecycle_produces_consistent_trace() {
        let layer = RplLayer::with_default_config();

        let mut trace = layer.on_pre_evaluate(Some("find large files"), Some("home dir"));
        layer.on_tool_selection(&mut trace, "list_directory", true, None, Some("dir arg"));
        layer.on_tool_selection(&mut trace, "read_file", false, None, None);
        layer.on_tool_selection(&mut trace, "shell_exec", false, None, Some("blocked by policy"));
        layer.on_complete(&mut trace);

        assert_eq!(trace.phase, ReasoningPhase::Complete);
        assert_eq!(trace.goal.as_deref(), Some("find large files"));
        assert_eq!(trace.context.as_deref(), Some("home dir"));
        assert_eq!(trace.tool_evaluations.len(), 3);
        assert_eq!(trace.tool_evaluations[0].tool_name, "list_directory");
        assert_eq!(trace.tool_evaluations[1].tool_name, "read_file");
        assert_eq!(trace.tool_evaluations[2].tool_name, "shell_exec");
        assert!(trace.tool_evaluations[0].selected);
        assert!(!trace.tool_evaluations[1].selected);
        assert!(!trace.tool_evaluations[2].selected);
    }

    /// A memory consideration added directly to the trace must survive
    /// to `on_complete` unchanged.
    #[test]
    fn memory_considerations_survive_complete() {
        let layer = RplLayer::with_default_config();
        let mut trace = layer.on_pre_evaluate(None, None);

        trace.add_memory_consideration(MemoryConsideration {
            memory_key: "recent_files".to_string(),
            relevance_score: 0.7,
            summary: Some("list of recently edited files".to_string()),
        });

        layer.on_complete(&mut trace);

        assert_eq!(trace.memory_considerations.len(), 1);
        assert_eq!(trace.memory_considerations[0].memory_key, "recent_files");
    }
}
