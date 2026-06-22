//! Reasoning Engine State Model.
//!
//! This module defines the finite-state-machine (FSM) types, plan structures,
//! hypotheses, and the top-level [`ReasoningEngineState`] that drives a single
//! reasoning turn inside the Grok CLI engine.
//!
//! # State machine overview
//!
//! ```text
//! AnalyzeGoal ──► ExpandOptions ──► EvaluateOptions ──► CommitPlan
//!                                         │                  │
//!                                         └──► Failed ◄──────┤
//!                                                            │
//!                                                    ExecuteStep(n)
//!                                                    ┌───────┤
//!                                                    │       ├──► ExecuteStep(n+1)
//!                                                    │       ├──► RevisePlan ──► CommitPlan
//!                                                    │       ├──► Complete
//!                                                    │       └──► Failed
//!                                                    └── (loop)
//! ```
//!
//! Terminal states: [`EngineState::Complete`] and [`EngineState::Failed`].
//! No transitions are permitted out of either terminal state.
//!
//! # Serialisation
//!
//! All types implement [`serde::Serialize`] / [`serde::Deserialize`].
//! The on-disk format is versioned via [`ENGINE_SCHEMA_VERSION`]; use
//! [`validate_version`] after deserialising from persistent storage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Version constant
// ---------------------------------------------------------------------------

/// Current schema version for [`ReasoningEngineState`].
///
/// Increment this constant — and update [`validate_version`] — whenever a
/// breaking change is made to the serialised layout of
/// [`ReasoningEngineState`].
pub const ENGINE_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// EngineState FSM enum
// ---------------------------------------------------------------------------

/// The finite-state-machine (FSM) state of the reasoning engine.
///
/// Each variant represents a distinct phase of a single reasoning turn.
/// [`EngineState::Complete`] and [`EngineState::Failed`] are **terminal**;
/// calling [`ReasoningEngineState::transition`] from either of those states
/// always returns [`TransitionError::InvalidTransition`].
///
/// The `"kind"` serde tag means every serialised state is self-describing,
/// e.g. `{"kind":"execute_step","step_index":2}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EngineState {
    /// Initial phase: parse and understand the user's goal.
    AnalyzeGoal,

    /// Generate candidate approaches for satisfying the goal.
    ExpandOptions,

    /// Score and filter the candidate approaches.
    EvaluateOptions,

    /// Lock in the highest-scoring plan before execution begins.
    CommitPlan,

    /// Execute the plan step at the given zero-based `step_index`.
    ExecuteStep {
        /// Zero-based index into [`ReasoningEngineState::plan`].
        step_index: usize,
    },

    /// Revise the current plan (e.g. after a step failure or new evidence).
    RevisePlan,

    /// The reasoning turn completed successfully. **Terminal state.**
    Complete,

    /// The reasoning turn failed unrecoverably. **Terminal state.**
    Failed {
        /// Human-readable description of why the turn failed.
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// StepAction
// ---------------------------------------------------------------------------

/// What a plan step asks the engine to do when it is executed.
///
/// The `"kind"` serde tag makes serialised actions self-describing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StepAction {
    /// Invoke a named tool with optional JSON-encoded arguments.
    UseTool {
        /// Registered name of the tool to invoke (e.g. `"list_directory"`).
        tool_name: String,
        /// Arbitrary JSON arguments forwarded verbatim to the tool.
        #[serde(default)]
        args: serde_json::Value,
    },

    /// Retrieve relevant memories from the memory store.
    QueryMemory {
        /// Semantic or keyword query string sent to the memory backend.
        query: String,
    },

    /// Issue a prompt to the backing language model.
    ModelCall {
        /// The prompt text to send.
        prompt: String,
    },

    /// Delegate work to a sub-agent (Task 127 multi-agent support).
    DelegateToSubAgent {
        /// The task description to give the sub-agent.
        task: String,
        /// Optional pre-existing agent ID (if the agent was already spawned).
        #[serde(default)]
        agent_id: Option<String>,
    },

    /// No-op placeholder; useful for testing and deferred steps.
    NoOp,
}

// ---------------------------------------------------------------------------
// StepStatus
// ---------------------------------------------------------------------------

/// Execution status of a single [`PlanStep`].
///
/// Status advances monotonically: `Pending → InProgress → Completed | Failed`.
/// A step may also be `Skipped` if the engine bypasses it (e.g. after a
/// plan revision).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StepStatus {
    /// Step has not yet been started.
    Pending,

    /// Step is currently executing.
    InProgress,

    /// Step finished successfully.
    Completed,

    /// Step finished with an error.
    Failed {
        /// Human-readable description of why the step failed.
        reason: String,
    },

    /// Step was intentionally bypassed (e.g. superseded by a plan revision).
    Skipped,
}

// ---------------------------------------------------------------------------
// PlanStep
// ---------------------------------------------------------------------------

/// A single step in the reasoning engine's execution plan.
///
/// Steps are created by the planner and stored in
/// [`ReasoningEngineState::plan`]. Each step carries a unique UUID, a
/// human-readable description, the [`StepAction`] to perform, the current
/// [`StepStatus`], and an optional result string populated after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Unique identifier (UUID v4) for this step.
    pub step_id: String,

    /// Human-readable description of what this step does.
    pub description: String,

    /// The action this step should perform when executed.
    pub action: StepAction,

    /// Current execution status of this step.
    pub status: StepStatus,

    /// Optional output produced by the step after successful completion.
    pub result: Option<String>,
}

impl PlanStep {
    /// Create a new [`PlanStep`] in [`StepStatus::Pending`] with a fresh UUID.
    ///
    /// # Example
    ///
    /// ```rust
    /// use grok_cli::engine::state::{PlanStep, StepAction, StepStatus};
    ///
    /// let step = PlanStep::new("List files in /tmp", StepAction::UseTool {
    ///     tool_name: "list_directory".to_owned(),
    ///     args: serde_json::json!({ "path": "/tmp" }),
    /// });
    /// assert_eq!(step.status, StepStatus::Pending);
    /// assert!(step.result.is_none());
    /// ```
    pub fn new(description: impl Into<String>, action: StepAction) -> Self {
        Self {
            step_id: Uuid::new_v4().to_string(),
            description: description.into(),
            action,
            status: StepStatus::Pending,
            result: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Hypothesis
// ---------------------------------------------------------------------------

/// A hypothesis about what the user intends, with a Bayesian confidence score.
///
/// Hypotheses are stored in [`ReasoningEngineState::hypotheses`] and updated
/// as the engine gathers evidence during the `AnalyzeGoal` and `ExpandOptions`
/// phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Unique identifier (UUID v4) for this hypothesis.
    pub id: String,

    /// Human-readable description of this hypothesis.
    pub description: String,

    /// Bayesian posterior confidence, clamped to `[0.0, 1.0]`.
    ///
    /// `0.0` = definitely wrong, `1.0` = definitely correct.
    pub confidence: f32,
}

impl Hypothesis {
    /// Create a new [`Hypothesis`] with a fresh UUID.
    ///
    /// `confidence` is clamped to `[0.0, 1.0]`; values outside that range
    /// are silently clamped rather than returning an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use grok_cli::engine::state::Hypothesis;
    ///
    /// let h = Hypothesis::new("User wants to list files", 0.85);
    /// assert!((h.confidence - 0.85).abs() < f32::EPSILON);
    ///
    /// // Values are clamped, not rejected.
    /// let h2 = Hypothesis::new("Overconfident", 2.0);
    /// assert_eq!(h2.confidence, 1.0);
    /// ```
    pub fn new(description: impl Into<String>, confidence: f32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned when a state transition is not permitted by the FSM.
///
/// All terminal-state transitions ([`EngineState::Complete`] or
/// [`EngineState::Failed`] → anything) produce this error, as do any other
/// `(from, to)` pairs not listed in [`ReasoningEngineState::transition`].
#[derive(Debug, thiserror::Error)]
pub enum TransitionError {
    /// The requested `from → to` transition is not defined in the FSM.
    #[error("invalid transition from {from} to {to}")]
    InvalidTransition {
        /// Name of the state the engine was in.
        from: String,
        /// Name of the state that was requested.
        to: String,
    },
}

/// Error returned when a plan operation cannot be completed.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    /// Attempted to operate on a step in an empty plan.
    #[error("plan is empty")]
    EmptyPlan,

    /// The requested step index is beyond the end of the plan.
    #[error("step index {0} out of bounds (plan has {1} steps)")]
    StepOutOfBounds(usize, usize),

    /// The plan has been revised more times than [`ReasoningEngineState::max_revisions`] allows.
    #[error("max revisions ({0}) exceeded")]
    MaxRevisionsExceeded(u32),
}

// ---------------------------------------------------------------------------
// ReasoningEngineState
// ---------------------------------------------------------------------------

/// The full state of the reasoning engine for a single reasoning turn.
///
/// This struct is the single source of truth for the engine's FSM state,
/// current plan, hypotheses, uncertainty estimate, and audit metadata for one
/// complete reasoning turn.  It is designed to be serialised to JSON for
/// logging, observability, and crash recovery.
///
/// # Lifecycle
///
/// 1. Call [`ReasoningEngineState::new`] (optionally with builder methods).
/// 2. Drive the FSM forward via [`transition`][Self::transition].
/// 3. Mutate plan steps via [`mark_step_complete`][Self::mark_step_complete]
///    / [`mark_step_failed`][Self::mark_step_failed].
/// 4. Revise the plan via [`revise_plan`][Self::revise_plan] from
///    [`EngineState::RevisePlan`].
///
/// # Serialisation
///
/// Deserialised instances should be validated with [`validate_version`] to
/// catch schema-version mismatches before use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEngineState {
    /// Serialisation compatibility version.
    ///
    /// Always set to [`ENGINE_SCHEMA_VERSION`] on creation.
    /// Use [`validate_version`] after deserialising from persistent storage.
    pub schema_version: u32,

    /// Unique identifier (UUID v4) for this engine instance.
    ///
    /// This ID links the engine state to the corresponding RPL `trace_id`
    /// so that reasoning traces and engine states can be correlated in logs.
    pub engine_id: String,

    /// Current FSM state.
    pub state: EngineState,

    /// The user's goal as understood by the engine, if set.
    pub goal: Option<String>,

    /// Current set of intent hypotheses.
    ///
    /// Ordered by descending [`Hypothesis::confidence`] after each Bayesian
    /// update.  May be empty during early FSM phases.
    pub hypotheses: Vec<Hypothesis>,

    /// Ordered list of plan steps to be executed.
    pub plan: Vec<PlanStep>,

    /// Zero-based index of the step currently being executed.
    ///
    /// Points into [`Self::plan`]. Advanced by the engine as each step
    /// completes or is skipped.
    pub current_step_index: usize,

    /// Names of the tools selected for use during this reasoning turn.
    pub selected_tools: Vec<String>,

    /// Memory keys referenced during this reasoning turn.
    pub memory_references: Vec<String>,

    /// Current aggregate uncertainty estimate, in `[0.0, 1.0]`.
    ///
    /// `0.0` = fully certain, `1.0` = completely uncertain.
    pub uncertainty: f32,

    /// Number of times the plan has been revised via [`revise_plan`][Self::revise_plan].
    pub revision_count: u32,

    /// Maximum number of plan revisions permitted before the engine aborts.
    ///
    /// When `revision_count >= max_revisions`, [`revise_plan`][Self::revise_plan]
    /// returns [`PlanError::MaxRevisionsExceeded`].
    pub max_revisions: u32,

    /// UTC timestamp when this state was created.
    pub created_at: DateTime<Utc>,

    /// UTC timestamp of the most recent mutation.
    ///
    /// Updated automatically by any mutating method via [`touch`][Self::touch].
    pub updated_at: DateTime<Utc>,
}

impl Default for ReasoningEngineState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReasoningEngineState {
    /// Create a new [`ReasoningEngineState`] in [`EngineState::AnalyzeGoal`]
    /// with sensible defaults.
    ///
    /// - `schema_version` = [`ENGINE_SCHEMA_VERSION`]
    /// - `engine_id` = freshly-generated UUID v4
    /// - `uncertainty` = `0.5` (maximum-entropy prior)
    /// - `max_revisions` = `3`
    /// - `revision_count`, `current_step_index` = `0`
    /// - All collections empty
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            schema_version: ENGINE_SCHEMA_VERSION,
            engine_id: Uuid::new_v4().to_string(),
            state: EngineState::AnalyzeGoal,
            goal: None,
            hypotheses: Vec::new(),
            plan: Vec::new(),
            current_step_index: 0,
            selected_tools: Vec::new(),
            memory_references: Vec::new(),
            uncertainty: 0.5,
            revision_count: 0,
            max_revisions: 3,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the goal string and return `self` (builder pattern).
    ///
    /// # Example
    ///
    /// ```rust
    /// use grok_cli::engine::state::ReasoningEngineState;
    ///
    /// let state = ReasoningEngineState::new()
    ///     .with_goal("list all Rust files in the workspace");
    /// assert_eq!(state.goal.as_deref(), Some("list all Rust files in the workspace"));
    /// ```
    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }

    /// Set `max_revisions` and return `self` (builder pattern).
    ///
    /// # Example
    ///
    /// ```rust
    /// use grok_cli::engine::state::ReasoningEngineState;
    ///
    /// let state = ReasoningEngineState::new().with_max_revisions(5);
    /// assert_eq!(state.max_revisions, 5);
    /// ```
    pub fn with_max_revisions(mut self, max: u32) -> Self {
        self.max_revisions = max;
        self
    }

    /// Attempt a state transition, enforcing the FSM's allowed edges.
    ///
    /// # Valid transitions
    ///
    /// | Current state      | Allowed next states                              |
    /// |--------------------|--------------------------------------------------|
    /// | `AnalyzeGoal`      | `ExpandOptions`                                  |
    /// | `ExpandOptions`    | `EvaluateOptions`                                |
    /// | `EvaluateOptions`  | `CommitPlan`, `Failed`                           |
    /// | `CommitPlan`       | `ExecuteStep { step_index: 0 }`, `Failed`        |
    /// | `ExecuteStep(_)`   | `ExecuteStep(_)`, `RevisePlan`, `Complete`, `Failed` |
    /// | `RevisePlan`       | `CommitPlan`, `Failed`                           |
    /// | `Complete`         | *(terminal — always [`TransitionError`])*        |
    /// | `Failed`           | *(terminal — always [`TransitionError`])*        |
    ///
    /// On success the internal state is updated and [`updated_at`][Self::updated_at]
    /// is refreshed. On failure the state is **not** modified.
    ///
    /// # Errors
    ///
    /// Returns [`TransitionError::InvalidTransition`] if the `(current, next)`
    /// pair is not listed in the table above.
    pub fn transition(&mut self, next: EngineState) -> Result<(), TransitionError> {
        let allowed = match (&self.state, &next) {
            // Linear forward chain
            (EngineState::AnalyzeGoal, EngineState::ExpandOptions) => true,
            (EngineState::ExpandOptions, EngineState::EvaluateOptions) => true,
            // EvaluateOptions branches
            (EngineState::EvaluateOptions, EngineState::CommitPlan) => true,
            (EngineState::EvaluateOptions, EngineState::Failed { .. }) => true,
            // CommitPlan branches
            (EngineState::CommitPlan, EngineState::ExecuteStep { .. }) => true,
            (EngineState::CommitPlan, EngineState::Failed { .. }) => true,
            // ExecuteStep can loop, revise, finish, or fail
            (EngineState::ExecuteStep { .. }, EngineState::ExecuteStep { .. }) => true,
            (EngineState::ExecuteStep { .. }, EngineState::RevisePlan) => true,
            (EngineState::ExecuteStep { .. }, EngineState::Complete) => true,
            (EngineState::ExecuteStep { .. }, EngineState::Failed { .. }) => true,
            // RevisePlan branches
            (EngineState::RevisePlan, EngineState::CommitPlan) => true,
            (EngineState::RevisePlan, EngineState::Failed { .. }) => true,
            // Terminal states: no outgoing edges.
            (EngineState::Complete, _) => false,
            (EngineState::Failed { .. }, _) => false,
            // Everything else is invalid.
            _ => false,
        };

        if allowed {
            self.state = next;
            self.touch();
            Ok(())
        } else {
            Err(TransitionError::InvalidTransition {
                from: engine_state_name(&self.state).to_owned(),
                to: engine_state_name(&next).to_owned(),
            })
        }
    }

    /// Mark the current plan step as [`StepStatus::Completed`] with an
    /// optional result string.
    ///
    /// Advances [`updated_at`][Self::updated_at] on success.
    ///
    /// # Errors
    ///
    /// - [`PlanError::EmptyPlan`] — the plan has no steps at all.
    /// - [`PlanError::StepOutOfBounds`] — `current_step_index` is past the
    ///   end of the plan.
    pub fn mark_step_complete(&mut self, result: Option<String>) -> Result<(), PlanError> {
        let idx = self.current_step_index;
        let plan_len = self.plan.len();

        if plan_len == 0 {
            return Err(PlanError::EmptyPlan);
        }
        if idx >= plan_len {
            return Err(PlanError::StepOutOfBounds(idx, plan_len));
        }

        self.plan[idx].status = StepStatus::Completed;
        self.plan[idx].result = result;
        self.touch();
        Ok(())
    }

    /// Mark the current plan step as [`StepStatus::Failed`] with a reason.
    ///
    /// Advances [`updated_at`][Self::updated_at] on success.
    ///
    /// # Errors
    ///
    /// - [`PlanError::EmptyPlan`] — the plan has no steps at all.
    /// - [`PlanError::StepOutOfBounds`] — `current_step_index` is past the
    ///   end of the plan.
    pub fn mark_step_failed(&mut self, reason: impl Into<String>) -> Result<(), PlanError> {
        let idx = self.current_step_index;
        let plan_len = self.plan.len();

        if plan_len == 0 {
            return Err(PlanError::EmptyPlan);
        }
        if idx >= plan_len {
            return Err(PlanError::StepOutOfBounds(idx, plan_len));
        }

        self.plan[idx].status = StepStatus::Failed {
            reason: reason.into(),
        };
        self.touch();
        Ok(())
    }

    /// Replace the plan with `new_steps` and increment [`revision_count`][Self::revision_count].
    ///
    /// Also resets [`current_step_index`][Self::current_step_index] to `0` so
    /// execution restarts from the beginning of the revised plan.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError::MaxRevisionsExceeded`] if
    /// `revision_count >= max_revisions` before the revision is applied.
    /// The plan is **not** modified in that case.
    pub fn revise_plan(&mut self, new_steps: Vec<PlanStep>) -> Result<(), PlanError> {
        if self.revision_count >= self.max_revisions {
            return Err(PlanError::MaxRevisionsExceeded(self.max_revisions));
        }
        self.plan = new_steps;
        self.current_step_index = 0;
        self.revision_count += 1;
        self.touch();
        Ok(())
    }

    /// Returns a shared reference to the current plan step, or `None` if the
    /// plan is empty or `current_step_index` is out of bounds.
    pub fn current_step(&self) -> Option<&PlanStep> {
        self.plan.get(self.current_step_index)
    }

    /// Returns a mutable reference to the current plan step, or `None` if the
    /// plan is empty or `current_step_index` is out of bounds.
    pub fn current_step_mut(&mut self) -> Option<&mut PlanStep> {
        self.plan.get_mut(self.current_step_index)
    }

    /// Update [`updated_at`][Self::updated_at] to the current UTC instant.
    ///
    /// Called internally by every mutating method.
    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns a short, stable, human-readable name for an [`EngineState`] variant.
///
/// Used when formatting [`TransitionError`] messages so that the error text
/// does not include Rust `Debug` output (which would expose internal fields
/// such as step indices and failure reasons in user-facing logs).
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

// ---------------------------------------------------------------------------
// Serialisation helper
// ---------------------------------------------------------------------------

/// Validate that `state.schema_version` matches [`ENGINE_SCHEMA_VERSION`].
///
/// Call this immediately after deserialising a [`ReasoningEngineState`] from
/// persistent storage to detect forward-compatibility issues before any
/// mutation occurs.
///
/// # Errors
///
/// Returns `Err(String)` with a human-readable message when the versions do
/// not match.
///
/// # Example
///
/// ```rust
/// use grok_cli::engine::state::{ReasoningEngineState, validate_version};
///
/// // Example (in real code you would have the JSON string):
/// // let state: ReasoningEngineState = serde_json::from_str(&raw_json)?;
/// // validate_version(&state).map_err(|e| anyhow::anyhow!(e))?;
/// ```
pub fn validate_version(state: &ReasoningEngineState) -> Result<(), String> {
    if state.schema_version == ENGINE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(format!(
            "schema version mismatch: expected {ENGINE_SCHEMA_VERSION}, got {}",
            state.schema_version
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    #[test]
    fn new_creates_analyze_goal_state() {
        let s = ReasoningEngineState::new();

        assert_eq!(s.state, EngineState::AnalyzeGoal);
        assert_eq!(s.schema_version, ENGINE_SCHEMA_VERSION);
        assert!(s.goal.is_none(), "goal should start as None");
        assert!(s.plan.is_empty(), "plan should start empty");
        assert!(s.hypotheses.is_empty(), "hypotheses should start empty");
        assert!(s.selected_tools.is_empty());
        assert!(s.memory_references.is_empty());
        assert_eq!(s.revision_count, 0);
        assert_eq!(s.current_step_index, 0);
        assert_eq!(s.max_revisions, 3);
        // Uncertainty should be the maximum-entropy prior.
        assert!((s.uncertainty - 0.5).abs() < f32::EPSILON);
        // engine_id must be a non-empty string.
        assert!(!s.engine_id.is_empty());
        // Timestamps should be roughly equal at construction time.
        let delta = s.updated_at.signed_duration_since(s.created_at);
        assert!(delta.num_milliseconds() >= 0);
    }

    // -----------------------------------------------------------------------
    // Valid transition
    // -----------------------------------------------------------------------

    #[test]
    fn valid_transition_analyze_to_expand() {
        let mut s = ReasoningEngineState::new();
        let result = s.transition(EngineState::ExpandOptions);
        assert!(result.is_ok(), "AnalyzeGoal → ExpandOptions must succeed");
        assert_eq!(s.state, EngineState::ExpandOptions);
    }

    // -----------------------------------------------------------------------
    // Terminal-state transitions
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_transition_complete_to_anything() {
        let mut s = ReasoningEngineState::new();
        // Force the state into the terminal Complete variant.
        s.state = EngineState::Complete;

        for next in [
            EngineState::AnalyzeGoal,
            EngineState::ExpandOptions,
            EngineState::RevisePlan,
            EngineState::Failed {
                reason: "x".to_owned(),
            },
        ] {
            let result = s.transition(next);
            assert!(result.is_err(), "Complete must reject all transitions");
            let msg = result.unwrap_err().to_string();
            assert!(
                msg.contains("Complete"),
                "error message should mention 'Complete', got: {msg}"
            );
        }
    }

    #[test]
    fn invalid_transition_failed_to_anything() {
        let mut s = ReasoningEngineState::new();
        s.state = EngineState::Failed {
            reason: "unrecoverable".to_owned(),
        };

        for next in [
            EngineState::AnalyzeGoal,
            EngineState::RevisePlan,
            EngineState::Complete,
        ] {
            let result = s.transition(next);
            assert!(result.is_err(), "Failed must reject all transitions");
            let msg = result.unwrap_err().to_string();
            assert!(
                msg.contains("Failed"),
                "error message should mention 'Failed', got: {msg}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Plan step operations
    // -----------------------------------------------------------------------

    #[test]
    fn mark_step_complete_on_empty_plan_returns_error() {
        let mut s = ReasoningEngineState::new();
        // No steps added — must return EmptyPlan.
        let result = s.mark_step_complete(None);
        assert!(
            matches!(result, Err(PlanError::EmptyPlan)),
            "expected EmptyPlan, got {result:?}"
        );
    }

    #[test]
    fn mark_step_complete_stores_result_and_status() {
        let mut s = ReasoningEngineState::new();
        s.plan = vec![PlanStep::new("do something", StepAction::NoOp)];

        s.mark_step_complete(Some("output text".to_owned()))
            .unwrap();

        let step = &s.plan[0];
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.result.as_deref(), Some("output text"));
    }

    // -----------------------------------------------------------------------
    // Plan revision
    // -----------------------------------------------------------------------

    #[test]
    fn revise_plan_increments_revision_count() {
        let mut s = ReasoningEngineState::new().with_max_revisions(5);
        assert_eq!(s.revision_count, 0);

        s.revise_plan(vec![PlanStep::new("step one", StepAction::NoOp)])
            .unwrap();
        assert_eq!(s.revision_count, 1);

        s.revise_plan(vec![PlanStep::new("step two", StepAction::NoOp)])
            .unwrap();
        assert_eq!(s.revision_count, 2);
    }

    #[test]
    fn revise_plan_resets_current_step_index() {
        let mut s = ReasoningEngineState::new().with_max_revisions(5);
        s.plan = vec![
            PlanStep::new("a", StepAction::NoOp),
            PlanStep::new("b", StepAction::NoOp),
        ];
        s.current_step_index = 1;

        s.revise_plan(vec![PlanStep::new("revised", StepAction::NoOp)])
            .unwrap();

        assert_eq!(s.current_step_index, 0, "index must reset after revision");
    }

    #[test]
    fn revise_plan_exceeds_max_returns_error() {
        let mut s = ReasoningEngineState::new().with_max_revisions(1);

        // First revision must succeed.
        s.revise_plan(vec![PlanStep::new("s1", StepAction::NoOp)])
            .expect("first revision should succeed");
        assert_eq!(s.revision_count, 1);

        // Second revision must fail because revision_count (1) >= max_revisions (1).
        let result = s.revise_plan(vec![PlanStep::new("s2", StepAction::NoOp)]);
        assert!(
            matches!(result, Err(PlanError::MaxRevisionsExceeded(1))),
            "expected MaxRevisionsExceeded(1), got {result:?}"
        );
        // The plan must NOT have been modified.
        assert_eq!(s.plan.len(), 1);
        assert_eq!(s.plan[0].description, "s1");
    }

    // -----------------------------------------------------------------------
    // Serialisation
    // -----------------------------------------------------------------------

    #[test]
    fn serialization_round_trip() {
        let original = ReasoningEngineState::new()
            .with_goal("list all Rust source files")
            .with_max_revisions(7);

        let json = serde_json::to_string(&original).expect("serialise must succeed");
        let restored: ReasoningEngineState =
            serde_json::from_str(&json).expect("deserialise must succeed");

        assert_eq!(original.engine_id, restored.engine_id);
        assert_eq!(original.goal, restored.goal);
        assert_eq!(original.schema_version, restored.schema_version);
        assert_eq!(original.max_revisions, restored.max_revisions);
        assert_eq!(original.state, restored.state);
        assert_eq!(original.revision_count, restored.revision_count);
        assert_eq!(original.uncertainty, restored.uncertainty);
    }

    #[test]
    fn serialization_round_trip_with_steps_and_hypotheses() {
        let mut s = ReasoningEngineState::new().with_goal("find errors");
        s.plan = vec![
            PlanStep::new(
                "step A",
                StepAction::UseTool {
                    tool_name: "grep".to_owned(),
                    args: serde_json::json!({ "pattern": "error" }),
                },
            ),
            PlanStep::new(
                "step B",
                StepAction::QueryMemory {
                    query: "previous grep results".to_owned(),
                },
            ),
        ];
        s.hypotheses = vec![
            Hypothesis::new("User wants error lines", 0.9),
            Hypothesis::new("User wants warning lines", 0.3),
        ];

        let json = serde_json::to_string_pretty(&s).expect("serialise");
        let restored: ReasoningEngineState = serde_json::from_str(&json).expect("deserialise");

        assert_eq!(restored.plan.len(), 2);
        assert_eq!(restored.hypotheses.len(), 2);
        assert_eq!(restored.plan[0].description, "step A");
        assert_eq!(restored.hypotheses[0].description, "User wants error lines");
    }

    // -----------------------------------------------------------------------
    // Version validation
    // -----------------------------------------------------------------------

    #[test]
    fn validate_version_accepts_current_version() {
        let s = ReasoningEngineState::new();
        assert!(
            validate_version(&s).is_ok(),
            "current schema version must be accepted"
        );
    }

    #[test]
    fn validate_version_rejects_wrong_version() {
        let mut s = ReasoningEngineState::new();
        s.schema_version = 999;

        let result = validate_version(&s);
        assert!(result.is_err(), "wrong version must be rejected");

        let msg = result.unwrap_err();
        assert!(
            msg.contains("999"),
            "error message must mention the actual version, got: {msg}"
        );
        assert!(
            msg.contains(&ENGINE_SCHEMA_VERSION.to_string()),
            "error message must mention the expected version, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Hypothesis
    // -----------------------------------------------------------------------

    #[test]
    fn hypothesis_confidence_stored_correctly() {
        let h = Hypothesis::new("User wants to list files", 0.85);
        assert!(
            (h.confidence - 0.85).abs() < f32::EPSILON,
            "confidence 0.85 should be stored exactly"
        );
        assert!(!h.id.is_empty(), "id must be a non-empty UUID string");
        assert_eq!(h.description, "User wants to list files");

        // Values > 1.0 are clamped.
        let h_high = Hypothesis::new("Overconfident", 2.0);
        assert!(
            (h_high.confidence - 1.0).abs() < f32::EPSILON,
            "confidence above 1.0 must be clamped to 1.0"
        );

        // Values < 0.0 are clamped.
        let h_low = Hypothesis::new("Negative", -0.5);
        assert!(
            (h_low.confidence - 0.0).abs() < f32::EPSILON,
            "confidence below 0.0 must be clamped to 0.0"
        );
    }

    // -----------------------------------------------------------------------
    // Builder methods
    // -----------------------------------------------------------------------

    #[test]
    fn builder_methods_set_fields() {
        let s = ReasoningEngineState::new()
            .with_goal("do a thing")
            .with_max_revisions(10);

        assert_eq!(s.goal.as_deref(), Some("do a thing"));
        assert_eq!(s.max_revisions, 10);
    }

    // -----------------------------------------------------------------------
    // current_step helpers
    // -----------------------------------------------------------------------

    #[test]
    fn current_step_returns_none_on_empty_plan() {
        let s = ReasoningEngineState::new();
        assert!(s.current_step().is_none());
    }

    #[test]
    fn current_step_returns_correct_step() {
        let mut s = ReasoningEngineState::new();
        s.plan = vec![
            PlanStep::new("first", StepAction::NoOp),
            PlanStep::new("second", StepAction::NoOp),
        ];
        s.current_step_index = 1;

        let step = s.current_step().expect("should return second step");
        assert_eq!(step.description, "second");
    }
}
