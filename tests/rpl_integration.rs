//! End-to-end integration tests for the Reasoning Protocol Layer (RPL).
//!
//! Tests exercise the complete public API surface accessible to downstream
//! crates: lifecycle management, serialisation round-trips, suppression
//! guards, redaction, validation, and interoperability with the skills
//! subsystem.
//!
//! All types are imported via `use grok_cli::rpl::*` to mirror real caller
//! usage and to verify that every required type is properly re-exported.

use grok_cli::rpl::*;

// ---------------------------------------------------------------------------
// Test harness helper
// ---------------------------------------------------------------------------

/// Build a paired [`RplLayer`] and [`SuppressionLayer`] for the given mode.
///
/// * `debug = false` → production config (logging off, suppression enforced)
/// * `debug = true`  → debug config (debug logging, suppression bypassed)
fn make_layer(debug: bool) -> (RplLayer, SuppressionLayer) {
    let rpl = RplLayer::new(RplConfig {
        log_level: if debug {
            ReasoningLogLevel::Debug
        } else {
            ReasoningLogLevel::Off
        },
        lenient_validation: true,
    });
    let sup = if debug {
        SuppressionLayer::debug()
    } else {
        SuppressionLayer::production()
    };
    (rpl, sup)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A [`ReasoningTrace`] must survive a JSON serialise → deserialise
/// round-trip with every field intact.
#[test]
fn schema_round_trip() {
    let original = ReasoningTrace::new(ReasoningPhase::Complete)
        .with_goal("round-trip goal")
        .with_context("round-trip context")
        .with_plan("round-trip plan")
        .with_uncertainty(0.42);

    let json = serde_json::to_string(&original).expect("ReasoningTrace serialisation must succeed");

    let recovered: ReasoningTrace =
        serde_json::from_str(&json).expect("ReasoningTrace deserialisation must succeed");

    assert_eq!(
        recovered.trace_id, original.trace_id,
        "trace_id must be identical after round-trip"
    );
    assert_eq!(
        recovered.goal, original.goal,
        "goal must be identical after round-trip"
    );
    assert_eq!(
        recovered.context, original.context,
        "context must be identical after round-trip"
    );
    assert_eq!(
        recovered.plan, original.plan,
        "plan must be identical after round-trip"
    );
    assert!(
        (recovered.uncertainty - original.uncertainty).abs() < f32::EPSILON,
        "uncertainty must be identical after round-trip"
    );
    assert_eq!(
        recovered.phase, original.phase,
        "phase must be identical after round-trip"
    );
    assert_eq!(
        recovered.suppressed, original.suppressed,
        "suppressed flag must be identical after round-trip"
    );
    assert_eq!(
        recovered.schema_version, original.schema_version,
        "schema_version must be identical after round-trip"
    );
}

/// Each call to [`RplLayer::on_pre_evaluate`] must yield a distinct
/// `trace_id`.
#[test]
fn trace_id_is_unique_per_trace() {
    let (layer, _) = make_layer(false);

    let trace_a = layer.on_pre_evaluate(None, None);
    let trace_b = layer.on_pre_evaluate(None, None);

    assert_ne!(
        trace_a.trace_id, trace_b.trace_id,
        "every on_pre_evaluate call must produce a unique trace_id"
    );
}

/// Calling the full lifecycle (`on_pre_evaluate` → two `on_tool_selection`
/// → `on_complete`) must produce a trace in the `Complete` phase with
/// exactly two tool evaluations.
#[test]
fn full_cpu_lifecycle_produces_complete_trace() {
    let (layer, _) = make_layer(false);

    let mut trace = layer.on_pre_evaluate(Some("list /tmp"), None);
    layer.on_tool_selection(
        &mut trace,
        "list_directory",
        true,
        None, // relevance_score (defaults to 1.0 when selected)
        Some("path argument detected"),
    );
    layer.on_tool_selection(&mut trace, "read_file", false, None, None);
    layer.on_complete(&mut trace);

    assert_eq!(
        trace.phase,
        ReasoningPhase::Complete,
        "phase must be Complete after on_complete"
    );
    assert_eq!(
        trace.tool_evaluations.len(),
        2,
        "trace must have exactly 2 tool evaluations after two on_tool_selection calls"
    );
}

/// In production mode the suppression guard must return `None` for a trace
/// that is suppressed (the default).
#[test]
fn suppression_blocks_trace_in_production_mode() {
    let (_, sup) = make_layer(false);
    let trace = ReasoningTrace::new(ReasoningPhase::Complete);

    assert!(
        trace.suppressed,
        "ReasoningTrace must be suppressed by default"
    );
    assert!(
        sup.guard(&trace).is_none(),
        "production SuppressionLayer must return None for a suppressed trace"
    );
}

/// In debug mode the suppression guard must return `Some` even for a trace
/// that is suppressed.
#[test]
fn debug_mode_exposes_suppressed_trace() {
    let (_, sup) = make_layer(true);
    let trace = ReasoningTrace::new(ReasoningPhase::Complete);

    assert!(
        trace.suppressed,
        "ReasoningTrace must be suppressed by default"
    );
    assert!(
        sup.guard(&trace).is_some(),
        "debug SuppressionLayer must return Some for a suppressed trace"
    );
}

/// An explicitly un-suppressed trace must always be visible, even under the
/// production suppression layer.
#[test]
fn unsuppressed_trace_always_visible() {
    let (_, sup) = make_layer(false);
    let mut trace = ReasoningTrace::new(ReasoningPhase::Complete);
    trace.suppressed = false;

    assert!(
        sup.guard(&trace).is_some(),
        "production SuppressionLayer must return Some when trace.suppressed is false"
    );
}

/// Applying redaction to a trace whose `goal` contains `"token: sk-abc123"`
/// must produce a goal that no longer contains the raw secret value.
#[test]
fn redaction_removes_api_key_from_goal() {
    let (_, sup) = make_layer(false);
    let trace = ReasoningTrace::new(ReasoningPhase::Complete).with_goal("token: sk-abc123");

    let redacted = sup.redact(&trace);
    let goal = redacted
        .goal
        .expect("goal must still be present (as [REDACTED]) after redaction");

    assert!(
        !goal.contains("sk-abc123"),
        "redacted goal must not contain the raw API key value; got: {goal:?}"
    );
}

/// [`validate`] must return `Err` when `uncertainty` is outside `[0.0, 1.0]`.
///
/// We bypass the clamping `with_uncertainty` builder and write the invalid
/// value directly to the public field.
#[test]
fn validation_rejects_invalid_uncertainty() {
    let mut trace = ReasoningTrace::new(ReasoningPhase::Complete);
    // Direct field write bypasses the builder's clamp to [0.0, 1.0].
    trace.uncertainty = 1.5;

    let result = validate(&trace);
    assert!(
        result.is_err(),
        "validate must return Err for uncertainty=1.5 (outside [0.0, 1.0])"
    );
}

/// [`AutoActivationEngine::check_with_reasoning`] called with empty skill
/// slices and `None` reasoning must return an empty result without panicking.
#[test]
fn skill_arbitration_check_with_reasoning_none_is_stable() {
    let engine = grok_cli::skills::AutoActivationEngine::default();

    let result = engine.check_with_reasoning(
        "",                        // input
        std::path::Path::new("."), // working_dir
        &[],                       // available_skills
        &[],                       // already_active
        None,                      // reasoning
    );

    assert!(
        result.is_empty(),
        "check_with_reasoning with empty skills and None reasoning must return an empty Vec"
    );
}

/// Calling [`log_trace`] with [`ReasoningLogLevel::Off`] must not panic
/// (smoke test; observable side-effects cannot be captured without a
/// tracing subscriber in this test harness).
#[test]
fn log_trace_off_emits_nothing_deterministically() {
    let trace = ReasoningTrace::new(ReasoningPhase::Complete)
        .with_goal("smoke test goal")
        .with_uncertainty(0.1);

    // Must not panic under any circumstances.
    log_trace(&trace, ReasoningLogLevel::Off);
}

/// Lifecycle hooks must advance the phase monotonically:
/// `PreEvaluation` → `ToolSelection` → `Complete`.
#[test]
fn deterministic_phase_ordering() {
    let (layer, _) = make_layer(false);

    let mut trace = layer.on_pre_evaluate(Some("phase ordering test"), None);
    assert_eq!(
        trace.phase,
        ReasoningPhase::PreEvaluation,
        "phase must be PreEvaluation immediately after on_pre_evaluate"
    );

    layer.on_tool_selection(&mut trace, "list_directory", true, None, None);
    assert_eq!(
        trace.phase,
        ReasoningPhase::ToolSelection,
        "phase must advance to ToolSelection after the first on_tool_selection call"
    );

    layer.on_complete(&mut trace);
    assert_eq!(
        trace.phase,
        ReasoningPhase::Complete,
        "phase must advance to Complete after on_complete"
    );
}

/// A fully executed lifecycle trace must remain suppressed in production mode
/// so that internal reasoning never leaks into user-facing output.
#[test]
fn regression_trace_does_not_appear_in_normal_response() {
    let (layer, sup) = make_layer(false);

    let mut trace = layer.on_pre_evaluate(Some("normal user request"), None);
    layer.on_tool_selection(
        &mut trace,
        "read_file",
        true,
        None, // relevance_score (defaults to 1.0 when selected)
        Some("user asked to read a file"),
    );
    layer.on_complete(&mut trace);

    assert!(
        trace.suppressed,
        "trace must remain suppressed=true after the full lifecycle"
    );
    assert!(
        sup.guard(&trace).is_none(),
        "production SuppressionLayer must hide the fully-executed trace from user-facing output"
    );
}
