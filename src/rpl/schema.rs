//! Reasoning Protocol Layer – schema types.
//!
//! Defines the versioned data structures that carry a single reasoning trace
//! through the CPU router's tool-execution loop.  All types implement
//! [`serde::Serialize`] / [`serde::Deserialize`] so they can be persisted or
//! forwarded to external observability pipelines.
//!
//! # Versioning
//!
//! The [`RPL_SCHEMA_VERSION`] constant must be incremented whenever a breaking
//! change is made to the serialised layout.  The validation layer enforces
//! that every in-flight trace carries the expected version.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Version constant
// ---------------------------------------------------------------------------

/// Current schema version for [`ReasoningTrace`].
///
/// [`crate::rpl::validation::validate`] rejects traces whose
/// `schema_version` does not match this value.
pub const RPL_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Component types
// ---------------------------------------------------------------------------

/// The evaluation result for a single candidate tool considered during a
/// routing decision.
///
/// One [`ToolEvaluation`] is appended to [`ReasoningTrace::tool_evaluations`]
/// for every tool the RPL layer inspects, whether or not it is ultimately
/// selected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEvaluation {
    /// The registered name of the tool (e.g. `"list_directory"`).
    pub tool_name: String,

    /// Relevance score in the range `[0.0, 1.0]`.
    ///
    /// `0.0` means completely irrelevant to the current goal;
    /// `1.0` means the tool is the ideal choice.
    pub relevance_score: f32,

    /// Optional human-readable explanation of why this score was assigned.
    pub reason: Option<String>,

    /// Whether this tool was ultimately selected for execution in this cycle.
    pub selected: bool,
}

/// A memory entry that was retrieved and scored during the reasoning phase.
///
/// One [`MemoryConsideration`] is appended to
/// [`ReasoningTrace::memory_considerations`] for every memory key the engine
/// evaluates against the current goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsideration {
    /// The key under which the memory entry is stored in the memory backend.
    pub memory_key: String,

    /// Relevance score in the range `[0.0, 1.0]`.
    ///
    /// `0.0` means this memory is completely unrelated to the current goal;
    /// `1.0` means it is directly applicable.
    pub relevance_score: f32,

    /// Optional short summary of what this memory entry contains.
    pub summary: Option<String>,
}

// ---------------------------------------------------------------------------
// Phase enum
// ---------------------------------------------------------------------------

/// Which CPU lifecycle phase the [`ReasoningTrace`] was captured in.
///
/// Phases advance monotonically from [`PreEvaluation`][Self::PreEvaluation]
/// through [`Complete`][Self::Complete].  A trace may be serialised at any
/// intermediate phase for debugging or observability purposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningPhase {
    /// Trace created before any tool or memory evaluation has started.
    PreEvaluation,
    /// The router is currently scoring and selecting candidate tools.
    ToolSelection,
    /// The router is querying and scoring memory entries.
    MemoryLookup,
    /// Tools have been selected; the router is assembling the execution plan.
    ActionPlanning,
    /// All reasoning steps are complete; the trace is ready for logging.
    Complete,
}

// ---------------------------------------------------------------------------
// ReasoningTrace
// ---------------------------------------------------------------------------

/// A full reasoning trace captured during a single `route_with_tools` call.
///
/// Every public field is serialised; the trace is safe to write to disk or
/// forward to an observability pipeline.
///
/// ## Construction
///
/// Use [`ReasoningTrace::new`] and the fluent builder methods:
///
/// ```rust,ignore
/// let mut trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation)
///     .with_goal("list files in /tmp")
///     .with_uncertainty(0.3);
///
/// trace.add_tool_evaluation(ToolEvaluation { .. });
/// ```
///
/// ## Suppression
///
/// [`suppressed`][Self::suppressed] defaults to `true` so internal reasoning
/// is never accidentally surfaced in user-facing output.  Set it to `false`
/// only in developer/debug configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTrace {
    /// Schema version of this trace; validated against [`RPL_SCHEMA_VERSION`].
    pub schema_version: u32,

    /// UUID v4 correlation ID that links this trace to CPU-router log entries.
    pub trace_id: String,

    /// The high-level goal the CPU router is trying to accomplish, if known.
    pub goal: Option<String>,

    /// Additional context passed into the routing call (e.g. a conversation
    /// history summary or the list of active skills).
    pub context: Option<String>,

    /// All tool evaluations considered during this trace, in insertion order.
    pub tool_evaluations: Vec<ToolEvaluation>,

    /// All memory entries considered during this trace, in insertion order.
    pub memory_considerations: Vec<MemoryConsideration>,

    /// The final action plan produced by the reasoning engine, if any.
    pub plan: Option<String>,

    /// Overall uncertainty score in the range `[0.0, 1.0]`.
    ///
    /// `0.0` means the router is fully confident in its decisions;
    /// `1.0` means maximum uncertainty (e.g. no relevant tools or memory
    /// were found).
    pub uncertainty: f32,

    /// Timestamp at which this trace was created (UTC).
    pub created_at: DateTime<Utc>,

    /// The lifecycle phase this trace was recorded in.
    pub phase: ReasoningPhase,

    /// When `true`, this trace is suppressed from user-facing output.
    ///
    /// Defaults to `true` to prevent accidental leakage of internal reasoning
    /// to end-users.
    pub suppressed: bool,
}

impl ReasoningTrace {
    /// Create a new trace in the given `phase` with safe defaults.
    ///
    /// | Field            | Default value               |
    /// |------------------|-----------------------------|
    /// | `schema_version` | [`RPL_SCHEMA_VERSION`]      |
    /// | `trace_id`       | fresh UUID v4               |
    /// | `uncertainty`    | `0.5`                       |
    /// | `suppressed`     | `true`                      |
    /// | `created_at`     | current UTC time            |
    /// | all others       | `None` / empty              |
    pub fn new(phase: ReasoningPhase) -> Self {
        Self {
            schema_version: RPL_SCHEMA_VERSION,
            // UUID v4 generation is infallible with the `v4` feature enabled.
            trace_id: Uuid::new_v4().to_string(),
            goal: None,
            context: None,
            tool_evaluations: Vec::new(),
            memory_considerations: Vec::new(),
            plan: None,
            uncertainty: 0.5,
            created_at: Utc::now(),
            phase,
            suppressed: true,
        }
    }

    /// Set the high-level goal for this trace.
    ///
    /// Returns `self` to allow method chaining.
    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }

    /// Set additional routing context for this trace.
    ///
    /// Returns `self` to allow method chaining.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set the action plan for this trace.
    ///
    /// Returns `self` to allow method chaining.
    pub fn with_plan(mut self, plan: impl Into<String>) -> Self {
        self.plan = Some(plan.into());
        self
    }

    /// Set the uncertainty score, **clamping** the supplied value to
    /// `[0.0, 1.0]`.
    ///
    /// Values below `0.0` are raised to `0.0`; values above `1.0` are lowered
    /// to `1.0`.  This guarantees the invariant expected by
    /// [`crate::rpl::validation::validate`].
    ///
    /// Returns `self` to allow method chaining.
    pub fn with_uncertainty(mut self, uncertainty: f32) -> Self {
        self.uncertainty = uncertainty.clamp(0.0, 1.0);
        self
    }

    /// Advance the trace to a new lifecycle `phase`.
    ///
    /// Returns `self` to allow method chaining.
    pub fn with_phase(mut self, phase: ReasoningPhase) -> Self {
        self.phase = phase;
        self
    }

    /// Append a [`ToolEvaluation`] to [`Self::tool_evaluations`].
    pub fn add_tool_evaluation(&mut self, eval: ToolEvaluation) {
        self.tool_evaluations.push(eval);
    }

    /// Append a [`MemoryConsideration`] to [`Self::memory_considerations`].
    pub fn add_memory_consideration(&mut self, mem: MemoryConsideration) {
        self.memory_considerations.push(mem);
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── new() defaults ──────────────────────────────────────────────────────

    /// `new()` must populate every field with its documented default value.
    #[test]
    fn new_sets_all_defaults() {
        let trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation);

        assert_eq!(
            trace.schema_version, RPL_SCHEMA_VERSION,
            "schema_version must equal RPL_SCHEMA_VERSION"
        );
        assert!(!trace.trace_id.is_empty(), "trace_id must not be empty");
        assert_eq!(
            trace.phase,
            ReasoningPhase::PreEvaluation,
            "phase must match constructor argument"
        );
        assert!(
            trace.suppressed,
            "suppressed must default to true for safety"
        );
        assert!(
            (trace.uncertainty - 0.5).abs() < f32::EPSILON,
            "uncertainty must default to 0.5"
        );
        assert!(trace.goal.is_none(), "goal must default to None");
        assert!(trace.context.is_none(), "context must default to None");
        assert!(trace.plan.is_none(), "plan must default to None");
        assert!(
            trace.tool_evaluations.is_empty(),
            "tool_evaluations must start empty"
        );
        assert!(
            trace.memory_considerations.is_empty(),
            "memory_considerations must start empty"
        );
    }

    /// Every call to `new()` must produce a distinct `trace_id`.
    #[test]
    fn new_generates_unique_trace_ids() {
        let ids: Vec<String> = (0..8)
            .map(|_| ReasoningTrace::new(ReasoningPhase::PreEvaluation).trace_id)
            .collect();

        // All IDs must be pairwise unique.
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "trace IDs must be unique");
            }
        }
    }

    // ── with_uncertainty clamping ───────────────────────────────────────────

    /// Values below `0.0` must be clamped to exactly `0.0`.
    #[test]
    fn with_uncertainty_clamps_below_zero() {
        let trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation).with_uncertainty(-1.0);
        assert_eq!(trace.uncertainty, 0.0);
    }

    /// Values above `1.0` must be clamped to exactly `1.0`.
    #[test]
    fn with_uncertainty_clamps_above_one() {
        let trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation).with_uncertainty(2.5);
        assert_eq!(trace.uncertainty, 1.0);
    }

    /// An in-range value must pass through unchanged.
    #[test]
    fn with_uncertainty_accepts_valid_values() {
        for &v in &[0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            let trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation).with_uncertainty(v);
            assert!(
                (trace.uncertainty - v).abs() < f32::EPSILON,
                "expected {v}, got {}",
                trace.uncertainty
            );
        }
    }

    // ── builder methods ─────────────────────────────────────────────────────

    /// Builder setters must store the supplied values.
    #[test]
    fn builder_methods_store_values() {
        let trace = ReasoningTrace::new(ReasoningPhase::ActionPlanning)
            .with_goal("list /tmp")
            .with_context("user session context")
            .with_plan("call list_directory");

        assert_eq!(trace.goal.as_deref(), Some("list /tmp"));
        assert_eq!(trace.context.as_deref(), Some("user session context"));
        assert_eq!(trace.plan.as_deref(), Some("call list_directory"));
    }

    /// `with_phase` must replace the current phase.
    #[test]
    fn with_phase_replaces_phase() {
        let trace =
            ReasoningTrace::new(ReasoningPhase::PreEvaluation).with_phase(ReasoningPhase::Complete);
        assert_eq!(trace.phase, ReasoningPhase::Complete);
    }

    // ── mutation helpers ────────────────────────────────────────────────────

    /// `add_tool_evaluation` must append an entry and preserve order.
    #[test]
    fn add_tool_evaluation_appends_in_order() {
        let mut trace = ReasoningTrace::new(ReasoningPhase::ToolSelection);

        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "read_file".to_string(),
            relevance_score: 0.9,
            reason: Some("file path mentioned".to_string()),
            selected: true,
        });
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "list_directory".to_string(),
            relevance_score: 0.4,
            reason: None,
            selected: false,
        });

        assert_eq!(trace.tool_evaluations.len(), 2);
        assert_eq!(trace.tool_evaluations[0].tool_name, "read_file");
        assert_eq!(trace.tool_evaluations[1].tool_name, "list_directory");
    }

    /// `add_memory_consideration` must append an entry and preserve order.
    #[test]
    fn add_memory_consideration_appends_in_order() {
        let mut trace = ReasoningTrace::new(ReasoningPhase::MemoryLookup);

        trace.add_memory_consideration(MemoryConsideration {
            memory_key: "project_layout".to_string(),
            relevance_score: 0.8,
            summary: Some("directory structure".to_string()),
        });
        trace.add_memory_consideration(MemoryConsideration {
            memory_key: "recent_errors".to_string(),
            relevance_score: 0.3,
            summary: None,
        });

        assert_eq!(trace.memory_considerations.len(), 2);
        assert_eq!(trace.memory_considerations[0].memory_key, "project_layout");
        assert_eq!(trace.memory_considerations[1].memory_key, "recent_errors");
    }

    // ── serialisation round-trip ────────────────────────────────────────────

    /// A full trace must survive a `serde_json` round-trip with identical
    /// field values.
    #[test]
    fn serialisation_round_trip() {
        let mut original = ReasoningTrace::new(ReasoningPhase::ToolSelection)
            .with_goal("list files in /tmp")
            .with_context("user asked to explore directory")
            .with_plan("call list_directory tool")
            .with_uncertainty(0.2);

        original.add_tool_evaluation(ToolEvaluation {
            tool_name: "list_directory".to_string(),
            relevance_score: 0.95,
            reason: Some("path argument detected".to_string()),
            selected: true,
        });
        original.add_memory_consideration(MemoryConsideration {
            memory_key: "cwd".to_string(),
            relevance_score: 0.6,
            summary: Some("current working directory".to_string()),
        });

        let json = serde_json::to_string(&original).expect("ReasoningTrace must serialise to JSON");

        let restored: ReasoningTrace =
            serde_json::from_str(&json).expect("JSON must deserialise to ReasoningTrace");

        assert_eq!(restored.trace_id, original.trace_id);
        assert_eq!(restored.schema_version, original.schema_version);
        assert_eq!(restored.goal, original.goal);
        assert_eq!(restored.context, original.context);
        assert_eq!(restored.plan, original.plan);
        assert!(
            (restored.uncertainty - original.uncertainty).abs() < f32::EPSILON,
            "uncertainty must survive round-trip"
        );
        assert_eq!(restored.phase, original.phase);
        assert_eq!(restored.suppressed, original.suppressed);
        assert_eq!(
            restored.tool_evaluations.len(),
            original.tool_evaluations.len()
        );
        assert_eq!(
            restored.memory_considerations.len(),
            original.memory_considerations.len()
        );
        assert_eq!(
            restored.tool_evaluations[0].tool_name,
            original.tool_evaluations[0].tool_name
        );
    }

    // ── ReasoningPhase serialisation ────────────────────────────────────────

    /// All `ReasoningPhase` variants must serialise to `snake_case` strings.
    #[test]
    fn phase_serialises_to_snake_case() {
        let cases = [
            (ReasoningPhase::PreEvaluation, "\"pre_evaluation\""),
            (ReasoningPhase::ToolSelection, "\"tool_selection\""),
            (ReasoningPhase::MemoryLookup, "\"memory_lookup\""),
            (ReasoningPhase::ActionPlanning, "\"action_planning\""),
            (ReasoningPhase::Complete, "\"complete\""),
        ];

        for (phase, expected) in cases {
            let json = serde_json::to_string(&phase)
                .unwrap_or_else(|e| panic!("failed to serialise {phase:?}: {e}"));
            assert_eq!(json, expected, "unexpected serialisation for {phase:?}");
        }
    }

    /// `snake_case` strings must deserialise back to the correct variants.
    #[test]
    fn phase_deserialises_from_snake_case() {
        let cases = [
            ("\"pre_evaluation\"", ReasoningPhase::PreEvaluation),
            ("\"tool_selection\"", ReasoningPhase::ToolSelection),
            ("\"memory_lookup\"", ReasoningPhase::MemoryLookup),
            ("\"action_planning\"", ReasoningPhase::ActionPlanning),
            ("\"complete\"", ReasoningPhase::Complete),
        ];

        for (json, expected) in cases {
            let phase: ReasoningPhase = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("failed to deserialise {json}: {e}"));
            assert_eq!(phase, expected);
        }
    }
}
