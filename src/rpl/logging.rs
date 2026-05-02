//! Reasoning Protocol Layer – trace logging.
//!
//! Provides [`ReasoningLogLevel`] and [`log_trace`], which together control
//! how much reasoning detail the RPL layer emits through [`tracing`].
//!
//! # Log levels
//!
//! | Level     | Tracing macro     | Fields emitted                                    |
//! |-----------|-------------------|---------------------------------------------------|
//! | `Off`     | *(silent)*        | nothing                                           |
//! | `Summary` | `tracing::info!`  | `trace_id`, `phase`, `uncertainty`                |
//! | `Debug`   | `tracing::debug!` | `trace_id`, `phase`, `uncertainty`, `goal`, `plan`, `tool_count` |
//! | `Trace`   | `tracing::trace!` | full JSON-serialised trace                        |
//!
//! `Summary` is the default and is intended for production use.  `Debug` and
//! `Trace` are for development only and may log sensitive reasoning details.

use crate::rpl::schema::ReasoningTrace;

// ---------------------------------------------------------------------------
// ReasoningLogLevel
// ---------------------------------------------------------------------------

/// Controls how much reasoning detail the RPL layer emits to the log.
///
/// Implement [`Default`] is derived as [`Summary`][Self::Summary], which is
/// safe for production deployments.
///
/// # Ordering
///
/// Verbosity increases from `Off` → `Summary` → `Debug` → `Trace`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReasoningLogLevel {
    /// Emit a full structured trace (all fields, every step).
    ///
    /// This level is intended for development use only and may log
    /// sensitive internal reasoning details.
    Trace,

    /// Emit key decisions: tool selections, plan, uncertainty.
    ///
    /// Useful during feature development and integration testing.
    Debug,

    /// Emit a single-line summary: `trace_id`, `phase`, `uncertainty`.
    ///
    /// Safe for production; this is the **default**.
    #[default]
    Summary,

    /// Do not emit any reasoning log events.
    Off,
}

// ---------------------------------------------------------------------------
// log_trace
// ---------------------------------------------------------------------------

/// Emit a structured log event for a [`ReasoningTrace`] via [`tracing`].
///
/// The verbosity of the emitted event is governed by `level`:
///
/// - [`Off`][ReasoningLogLevel::Off] — nothing is emitted.
/// - [`Summary`][ReasoningLogLevel::Summary] — a single `tracing::info!`
///   event carrying `trace_id`, `phase` (as a string), and `uncertainty`.
/// - [`Debug`][ReasoningLogLevel::Debug] — a `tracing::debug!` event that
///   additionally carries `goal`, `plan`, and `tool_count`.
/// - [`Trace`][ReasoningLogLevel::Trace] — a `tracing::trace!` event whose
///   `trace_json` field contains the full JSON-serialised trace.  If
///   serialisation fails the error is logged instead and no panic occurs.
///
/// # Notes
///
/// - Suppressed traces ([`ReasoningTrace::suppressed`] == `true`) are emitted
///   at every level except `Off`.  The suppression flag governs *user-facing*
///   output (handled upstream); it does not silence the observability log.
/// - This function is intentionally synchronous and infallible: logging
///   failures are swallowed rather than propagated to the caller.
pub fn log_trace(trace: &ReasoningTrace, level: ReasoningLogLevel) {
    match level {
        ReasoningLogLevel::Off => {
            // Intentionally silent.
        }

        ReasoningLogLevel::Summary => {
            // Derive a stable string representation of the phase without
            // pulling in serde here – use the Debug impl which is always
            // available.
            let phase = phase_label(trace);
            tracing::info!(
                trace_id = %trace.trace_id,
                phase     = %phase,
                uncertainty = trace.uncertainty,
                "rpl: reasoning trace summary"
            );
        }

        ReasoningLogLevel::Debug => {
            let phase = phase_label(trace);
            tracing::debug!(
                trace_id    = %trace.trace_id,
                phase       = %phase,
                uncertainty = trace.uncertainty,
                goal        = ?trace.goal,
                plan        = ?trace.plan,
                tool_count  = trace.tool_evaluations.len(),
                "rpl: reasoning trace debug"
            );
        }

        ReasoningLogLevel::Trace => {
            // Serialise the whole trace to JSON.  If it fails (e.g. a custom
            // serialiser panicked), fall back to a warning rather than
            // propagating an error or panicking.
            match serde_json::to_string(trace) {
                Ok(json) => {
                    tracing::trace!(
                        trace_json = %json,
                        "rpl: reasoning trace (full)"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        trace_id   = %trace.trace_id,
                        error      = %err,
                        "rpl: failed to serialise reasoning trace to JSON"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return a compact, human-readable label for the trace's current phase
/// without depending on `serde`.
#[inline]
fn phase_label(trace: &ReasoningTrace) -> &'static str {
    use crate::rpl::schema::ReasoningPhase;
    match trace.phase {
        ReasoningPhase::PreEvaluation => "pre_evaluation",
        ReasoningPhase::ToolSelection => "tool_selection",
        ReasoningPhase::MemoryLookup => "memory_lookup",
        ReasoningPhase::ActionPlanning => "action_planning",
        ReasoningPhase::Complete => "complete",
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpl::schema::{ReasoningPhase, ReasoningTrace, ToolEvaluation};

    // ── ReasoningLogLevel defaults ───────────────────────────────────────────

    /// The default variant must be `Summary`.
    #[test]
    fn default_level_is_summary() {
        assert_eq!(ReasoningLogLevel::default(), ReasoningLogLevel::Summary);
    }

    /// All variants must compare equal only to themselves.
    #[test]
    fn variants_are_distinct() {
        let all = [
            ReasoningLogLevel::Trace,
            ReasoningLogLevel::Debug,
            ReasoningLogLevel::Summary,
            ReasoningLogLevel::Off,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // ── phase_label ─────────────────────────────────────────────────────────

    /// Every `ReasoningPhase` variant must produce a non-empty label.
    #[test]
    fn phase_label_covers_all_variants() {
        let phases = [
            ReasoningPhase::PreEvaluation,
            ReasoningPhase::ToolSelection,
            ReasoningPhase::MemoryLookup,
            ReasoningPhase::ActionPlanning,
            ReasoningPhase::Complete,
        ];
        for phase in phases {
            let trace = ReasoningTrace::new(phase);
            let label = phase_label(&trace);
            assert!(!label.is_empty(), "phase_label must not be empty");
        }
    }

    /// `phase_label` must return the expected snake_case strings.
    #[test]
    fn phase_label_returns_snake_case() {
        let cases = [
            (ReasoningPhase::PreEvaluation, "pre_evaluation"),
            (ReasoningPhase::ToolSelection, "tool_selection"),
            (ReasoningPhase::MemoryLookup, "memory_lookup"),
            (ReasoningPhase::ActionPlanning, "action_planning"),
            (ReasoningPhase::Complete, "complete"),
        ];
        for (phase, expected) in cases {
            let trace = ReasoningTrace::new(phase);
            assert_eq!(phase_label(&trace), expected);
        }
    }

    // ── log_trace smoke tests ────────────────────────────────────────────────
    //
    // These tests verify that `log_trace` does not panic for any combination
    // of level and trace state.  We cannot easily assert on tracing output in
    // unit tests without a test subscriber, so we focus on the contract that
    // the function must be infallible.

    fn build_full_trace() -> ReasoningTrace {
        let mut trace = ReasoningTrace::new(ReasoningPhase::Complete)
            .with_goal("list files in /tmp")
            .with_context("user session")
            .with_plan("call list_directory")
            .with_uncertainty(0.3);

        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "list_directory".to_string(),
            relevance_score: 0.9,
            reason: Some("path argument detected".to_string()),
            selected: true,
        });
        trace
    }

    /// `log_trace` with `Off` must not panic.
    #[test]
    fn log_trace_off_does_not_panic() {
        let trace = build_full_trace();
        log_trace(&trace, ReasoningLogLevel::Off);
    }

    /// `log_trace` with `Summary` must not panic.
    #[test]
    fn log_trace_summary_does_not_panic() {
        let trace = build_full_trace();
        log_trace(&trace, ReasoningLogLevel::Summary);
    }

    /// `log_trace` with `Debug` must not panic.
    #[test]
    fn log_trace_debug_does_not_panic() {
        let trace = build_full_trace();
        log_trace(&trace, ReasoningLogLevel::Debug);
    }

    /// `log_trace` with `Trace` must not panic.
    #[test]
    fn log_trace_trace_does_not_panic() {
        let trace = build_full_trace();
        log_trace(&trace, ReasoningLogLevel::Trace);
    }

    /// `log_trace` must be infallible for a minimal (default) trace.
    #[test]
    fn log_trace_minimal_trace_does_not_panic() {
        let trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation);
        for level in [
            ReasoningLogLevel::Off,
            ReasoningLogLevel::Summary,
            ReasoningLogLevel::Debug,
            ReasoningLogLevel::Trace,
        ] {
            log_trace(&trace, level);
        }
    }

    /// `log_trace` must be infallible even when `suppressed = false`.
    #[test]
    fn log_trace_unsuppressed_trace_does_not_panic() {
        let mut trace = ReasoningTrace::new(ReasoningPhase::Complete)
            .with_goal("some goal")
            .with_uncertainty(0.1);
        trace.suppressed = false;

        log_trace(&trace, ReasoningLogLevel::Summary);
        log_trace(&trace, ReasoningLogLevel::Debug);
        log_trace(&trace, ReasoningLogLevel::Trace);
    }
}
