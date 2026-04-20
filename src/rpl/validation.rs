//! Reasoning Protocol Layer – trace validation.
//!
//! [`validate`] performs a full, **non-short-circuiting** inspection of a
//! [`ReasoningTrace`] and collects every problem it finds into a single
//! `Vec<ValidationError>`.  This lets callers log or display all issues at
//! once rather than discovering them one at a time.
//!
//! # Usage
//!
//! ```rust,ignore
//! match validate(&trace) {
//!     Ok(()) => { /* trace is clean */ }
//!     Err(errors) => {
//!         for e in &errors {
//!             tracing::warn!("RPL validation: {e}");
//!         }
//!     }
//! }
//! ```

use std::collections::HashSet;

use thiserror::Error;

use crate::rpl::schema::{RPL_SCHEMA_VERSION, ReasoningTrace};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// A single validation problem found inside a [`ReasoningTrace`].
///
/// [`validate`] may return multiple errors in one call; inspect the full
/// `Vec` rather than matching only the first element.
#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    /// The trace's `trace_id` field is an empty string.
    #[error("trace_id is empty")]
    EmptyTraceId,

    /// The top-level `uncertainty` value is outside the closed interval
    /// `[0.0, 1.0]`.
    #[error("uncertainty {0} is outside [0.0, 1.0]")]
    InvalidUncertainty(f32),

    /// A [`ToolEvaluation`][crate::rpl::schema::ToolEvaluation]'s
    /// `relevance_score` is outside `[0.0, 1.0]`.
    ///
    /// Fields: `(score, tool_name)`.
    #[error("tool relevance score {0} for '{1}' is outside [0.0, 1.0]")]
    InvalidToolScore(f32, String),

    /// A [`MemoryConsideration`][crate::rpl::schema::MemoryConsideration]'s
    /// `relevance_score` is outside `[0.0, 1.0]`.
    ///
    /// Fields: `(score, memory_key)`.
    #[error("memory relevance score {0} for '{1}' is outside [0.0, 1.0]")]
    InvalidMemoryScore(f32, String),

    /// The trace's `schema_version` does not match [`RPL_SCHEMA_VERSION`].
    ///
    /// Fields: `(found_version, expected_version)`.
    #[error("unsupported schema_version {0} (expected {1})")]
    UnsupportedVersion(u32, u32),

    /// Two or more [`ToolEvaluation`][crate::rpl::schema::ToolEvaluation]
    /// entries share the same `tool_name`, which is not allowed within a
    /// single trace.
    #[error("duplicate tool evaluation for '{0}'")]
    DuplicateToolEvaluation(String),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate a [`ReasoningTrace`], collecting **all** errors found.
///
/// Returns `Ok(())` when the trace is fully valid, or
/// `Err(Vec<ValidationError>)` containing every problem that was detected.
/// The vector is guaranteed to be non-empty when `Err` is returned.
///
/// # Rules checked
///
/// | Rule | Error variant |
/// |------|---------------|
/// | `trace_id` must not be empty | [`ValidationError::EmptyTraceId`] |
/// | `uncertainty` ∈ `[0.0, 1.0]` | [`ValidationError::InvalidUncertainty`] |
/// | Each tool `relevance_score` ∈ `[0.0, 1.0]` | [`ValidationError::InvalidToolScore`] |
/// | Each memory `relevance_score` ∈ `[0.0, 1.0]` | [`ValidationError::InvalidMemoryScore`] |
/// | `schema_version` == [`RPL_SCHEMA_VERSION`] | [`ValidationError::UnsupportedVersion`] |
/// | No duplicate `tool_name` within `tool_evaluations` | [`ValidationError::DuplicateToolEvaluation`] |
pub fn validate(trace: &ReasoningTrace) -> Result<(), Vec<ValidationError>> {
    let mut errors: Vec<ValidationError> = Vec::new();

    // ── schema version ──────────────────────────────────────────────────────
    if trace.schema_version != RPL_SCHEMA_VERSION {
        errors.push(ValidationError::UnsupportedVersion(
            trace.schema_version,
            RPL_SCHEMA_VERSION,
        ));
    }

    // ── trace_id ────────────────────────────────────────────────────────────
    if trace.trace_id.trim().is_empty() {
        errors.push(ValidationError::EmptyTraceId);
    }

    // ── uncertainty ─────────────────────────────────────────────────────────
    if !is_unit_interval(trace.uncertainty) {
        errors.push(ValidationError::InvalidUncertainty(trace.uncertainty));
    }

    // ── tool evaluations ────────────────────────────────────────────────────
    // Check relevance scores AND collect names for duplicate detection in a
    // single pass to avoid iterating the vec twice.
    let mut seen_tool_names: HashSet<&str> = HashSet::with_capacity(trace.tool_evaluations.len());

    for eval in &trace.tool_evaluations {
        if !is_unit_interval(eval.relevance_score) {
            errors.push(ValidationError::InvalidToolScore(
                eval.relevance_score,
                eval.tool_name.clone(),
            ));
        }

        // `insert` returns false when the value was already present.
        if !seen_tool_names.insert(eval.tool_name.as_str()) {
            errors.push(ValidationError::DuplicateToolEvaluation(
                eval.tool_name.clone(),
            ));
        }
    }

    // ── memory considerations ───────────────────────────────────────────────
    for mem in &trace.memory_considerations {
        if !is_unit_interval(mem.relevance_score) {
            errors.push(ValidationError::InvalidMemoryScore(
                mem.relevance_score,
                mem.memory_key.clone(),
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns `true` when `v` is in the closed interval `[0.0, 1.0]`.
///
/// NaN is treated as out-of-range (both comparisons fail for NaN).
#[inline]
fn is_unit_interval(v: f32) -> bool {
    v >= 0.0 && v <= 1.0
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpl::schema::{MemoryConsideration, ReasoningPhase, ReasoningTrace, ToolEvaluation};

    // ── helpers ─────────────────────────────────────────────────────────────

    /// Build a minimal valid trace with no evaluations.
    fn valid_trace() -> ReasoningTrace {
        ReasoningTrace::new(ReasoningPhase::Complete).with_uncertainty(0.5)
    }

    /// Assert that exactly one error of the expected variant is present.
    fn assert_single_error(errors: &[ValidationError], expected: &ValidationError) {
        assert!(
            errors.contains(expected),
            "expected {expected:?} in errors, got {errors:?}"
        );
    }

    // ── valid trace ──────────────────────────────────────────────────────────

    /// A freshly constructed trace with defaults must pass validation.
    #[test]
    fn valid_trace_passes() {
        let trace = valid_trace();
        assert!(
            validate(&trace).is_ok(),
            "a default trace should be valid, got: {:?}",
            validate(&trace)
        );
    }

    // ── EmptyTraceId ─────────────────────────────────────────────────────────

    /// A trace with an empty `trace_id` must fail with [`ValidationError::EmptyTraceId`].
    #[test]
    fn empty_trace_id_is_rejected() {
        let mut trace = valid_trace();
        trace.trace_id = String::new();

        let errors = validate(&trace).expect_err("empty trace_id must be invalid");
        assert_single_error(&errors, &ValidationError::EmptyTraceId);
    }

    /// A whitespace-only `trace_id` is also considered empty.
    #[test]
    fn whitespace_only_trace_id_is_rejected() {
        let mut trace = valid_trace();
        trace.trace_id = "   ".to_string();

        let errors = validate(&trace).expect_err("whitespace-only trace_id must be invalid");
        assert_single_error(&errors, &ValidationError::EmptyTraceId);
    }

    // ── InvalidUncertainty ───────────────────────────────────────────────────

    /// `uncertainty` below `0.0` must produce [`ValidationError::InvalidUncertainty`].
    #[test]
    fn uncertainty_below_zero_is_rejected() {
        let mut trace = valid_trace();
        // Bypass the clamping builder by writing directly to the field.
        trace.uncertainty = -0.1;

        let errors = validate(&trace).expect_err("negative uncertainty must be invalid");
        assert_single_error(&errors, &ValidationError::InvalidUncertainty(-0.1));
    }

    /// `uncertainty` above `1.0` must produce [`ValidationError::InvalidUncertainty`].
    #[test]
    fn uncertainty_above_one_is_rejected() {
        let mut trace = valid_trace();
        trace.uncertainty = 1.1;

        let errors = validate(&trace).expect_err("uncertainty > 1.0 must be invalid");
        assert_single_error(&errors, &ValidationError::InvalidUncertainty(1.1));
    }

    /// Boundary values `0.0` and `1.0` must be accepted.
    #[test]
    fn uncertainty_at_boundaries_is_accepted() {
        for boundary in [0.0_f32, 1.0] {
            let mut trace = valid_trace();
            trace.uncertainty = boundary;
            assert!(
                validate(&trace).is_ok(),
                "uncertainty={boundary} should be valid"
            );
        }
    }

    // ── InvalidToolScore ─────────────────────────────────────────────────────

    /// A tool with `relevance_score < 0.0` must produce [`ValidationError::InvalidToolScore`].
    #[test]
    fn tool_score_below_zero_is_rejected() {
        let mut trace = valid_trace();
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "bad_tool".to_string(),
            relevance_score: -0.5,
            reason: None,
            selected: false,
        });

        let errors = validate(&trace).expect_err("negative tool score must be invalid");
        assert_single_error(
            &errors,
            &ValidationError::InvalidToolScore(-0.5, "bad_tool".to_string()),
        );
    }

    /// A tool with `relevance_score > 1.0` must produce [`ValidationError::InvalidToolScore`].
    #[test]
    fn tool_score_above_one_is_rejected() {
        let mut trace = valid_trace();
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "over_tool".to_string(),
            relevance_score: 1.5,
            reason: None,
            selected: false,
        });

        let errors = validate(&trace).expect_err("tool score > 1.0 must be invalid");
        assert_single_error(
            &errors,
            &ValidationError::InvalidToolScore(1.5, "over_tool".to_string()),
        );
    }

    /// A tool with a valid `relevance_score` must not generate an error.
    #[test]
    fn valid_tool_score_is_accepted() {
        let mut trace = valid_trace();
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "good_tool".to_string(),
            relevance_score: 0.75,
            reason: Some("path argument detected".to_string()),
            selected: true,
        });

        assert!(validate(&trace).is_ok());
    }

    // ── InvalidMemoryScore ───────────────────────────────────────────────────

    /// A memory entry with `relevance_score < 0.0` must produce
    /// [`ValidationError::InvalidMemoryScore`].
    #[test]
    fn memory_score_below_zero_is_rejected() {
        let mut trace = valid_trace();
        trace.add_memory_consideration(MemoryConsideration {
            memory_key: "bad_key".to_string(),
            relevance_score: -0.1,
            summary: None,
        });

        let errors = validate(&trace).expect_err("negative memory score must be invalid");
        assert_single_error(
            &errors,
            &ValidationError::InvalidMemoryScore(-0.1, "bad_key".to_string()),
        );
    }

    /// A memory entry with `relevance_score > 1.0` must produce
    /// [`ValidationError::InvalidMemoryScore`].
    #[test]
    fn memory_score_above_one_is_rejected() {
        let mut trace = valid_trace();
        trace.add_memory_consideration(MemoryConsideration {
            memory_key: "over_key".to_string(),
            relevance_score: 2.0,
            summary: None,
        });

        let errors = validate(&trace).expect_err("memory score > 1.0 must be invalid");
        assert_single_error(
            &errors,
            &ValidationError::InvalidMemoryScore(2.0, "over_key".to_string()),
        );
    }

    /// A memory entry with a valid `relevance_score` must not generate an error.
    #[test]
    fn valid_memory_score_is_accepted() {
        let mut trace = valid_trace();
        trace.add_memory_consideration(MemoryConsideration {
            memory_key: "project_layout".to_string(),
            relevance_score: 0.8,
            summary: Some("directory structure".to_string()),
        });

        assert!(validate(&trace).is_ok());
    }

    // ── UnsupportedVersion ───────────────────────────────────────────────────

    /// A `schema_version` that does not match [`RPL_SCHEMA_VERSION`] must be
    /// rejected with [`ValidationError::UnsupportedVersion`].
    #[test]
    fn wrong_schema_version_is_rejected() {
        let mut trace = valid_trace();
        trace.schema_version = 99;

        let errors = validate(&trace).expect_err("wrong schema_version must be invalid");
        assert_single_error(
            &errors,
            &ValidationError::UnsupportedVersion(99, RPL_SCHEMA_VERSION),
        );
    }

    /// The correct [`RPL_SCHEMA_VERSION`] must be accepted.
    #[test]
    fn correct_schema_version_is_accepted() {
        let mut trace = valid_trace();
        trace.schema_version = RPL_SCHEMA_VERSION;
        assert!(validate(&trace).is_ok());
    }

    // ── DuplicateToolEvaluation ──────────────────────────────────────────────

    /// Two [`ToolEvaluation`] entries with the same `tool_name` must produce
    /// [`ValidationError::DuplicateToolEvaluation`].
    #[test]
    fn duplicate_tool_name_is_rejected() {
        let mut trace = valid_trace();
        let eval = ToolEvaluation {
            tool_name: "list_directory".to_string(),
            relevance_score: 0.9,
            reason: None,
            selected: true,
        };
        trace.add_tool_evaluation(eval.clone());
        trace.add_tool_evaluation(eval);

        let errors = validate(&trace).expect_err("duplicate tool_name must be invalid");
        assert_single_error(
            &errors,
            &ValidationError::DuplicateToolEvaluation("list_directory".to_string()),
        );
    }

    /// Distinct tool names within the same trace must not trigger the
    /// duplicate check.
    #[test]
    fn distinct_tool_names_are_accepted() {
        let mut trace = valid_trace();
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "read_file".to_string(),
            relevance_score: 0.8,
            reason: None,
            selected: true,
        });
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "list_directory".to_string(),
            relevance_score: 0.4,
            reason: None,
            selected: false,
        });

        assert!(validate(&trace).is_ok());
    }

    // ── multiple errors collected ────────────────────────────────────────────

    /// When a trace has several problems, *all* of them must appear in the
    /// returned error vec (non-short-circuiting behaviour).
    #[test]
    fn multiple_errors_are_all_collected() {
        let mut trace = valid_trace();

        // Inject multiple simultaneous faults.
        trace.trace_id = String::new(); // EmptyTraceId
        trace.uncertainty = -1.0; // InvalidUncertainty
        trace.schema_version = 0; // UnsupportedVersion

        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "dup_tool".to_string(),
            relevance_score: 0.5,
            reason: None,
            selected: false,
        });
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "dup_tool".to_string(), // duplicate
            relevance_score: 2.0,              // also invalid score
            reason: None,
            selected: false,
        });

        let errors = validate(&trace).expect_err("multiple errors expected");

        assert!(
            errors.contains(&ValidationError::EmptyTraceId),
            "missing EmptyTraceId"
        );
        assert!(
            errors.contains(&ValidationError::InvalidUncertainty(-1.0)),
            "missing InvalidUncertainty"
        );
        assert!(
            errors.contains(&ValidationError::UnsupportedVersion(0, RPL_SCHEMA_VERSION)),
            "missing UnsupportedVersion"
        );
        assert!(
            errors.contains(&ValidationError::DuplicateToolEvaluation(
                "dup_tool".to_string()
            )),
            "missing DuplicateToolEvaluation"
        );
        assert!(
            errors.contains(&ValidationError::InvalidToolScore(
                2.0,
                "dup_tool".to_string()
            )),
            "missing InvalidToolScore for dup_tool"
        );
    }
}
