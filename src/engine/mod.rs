//! Reasoning Engine — active decision-making component of Grok CLI.
//!
//! The reasoning engine orchestrates goal analysis, planning, Bayesian
//! belief updates, memory retrieval, tool selection, and self-correction
//! across the CPU tool-execution loop.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                     ReasoningEngineState                         │
//! │                                                                  │
//! │  AnalyzeGoal → ExpandOptions → EvaluateOptions → CommitPlan     │
//! │       │                              │                │          │
//! │       │                           Failed         ExecuteStep(n) │
//! │       │                                          ┌─────┤        │
//! │       │                                          │     ├─► n+1  │
//! │       │                                          │     ├─► RevisePlan → CommitPlan
//! │       │                                          │     ├─► Complete    │
//! │       │                                          │     └─► Failed      │
//! │       └──────────────────────────────────────────┴─────────────────────┘
//! └──────────────────────────────────────────────────────────────────┘
//!          │               │               │             │
//!        beliefs        planner      memory_bridge  arbitration
//!          │               │               │             │
//!       correction    observability
//! ```
//!
//! # Module layout
//!
//! | Module             | Status       | Implements                                  |
//! |--------------------|--------------|---------------------------------------------|
//! | [`state`]          | **Complete** | FSM, plan model, hypotheses (Task 94)       |
//! | [`beliefs`]        | Stub         | Bayesian belief management (Task 95)        |
//! | [`planner`]        | Stub         | Goal-driven plan construction (Task 96)     |
//! | [`memory_bridge`]  | Stub         | MemoryStore integration (Task 97)           |
//! | [`arbitration`]    | Stub         | Skill arbitration integration (Task 98)     |
//! | [`correction`]     | Stub         | Self-correction and recovery (Task 99)      |
//! | [`observability`]  | Stub         | Structured transition logging (Task 100)    |
//!
//! See `docs/engine_architecture.md` for the full design document.

// ---------------------------------------------------------------------------
// Submodule declarations
// ---------------------------------------------------------------------------

/// Skill arbitration integration.
///
/// Stub — full implementation in Task 98.
pub mod execution;

/// Bayesian belief management for the reasoning engine.
///
/// Stub — full implementation in Task 95.
pub mod beliefs;

/// Self-correction and error-recovery logic.
///
/// Stub — full implementation in Task 99.
pub mod correction;

/// Bridge between the reasoning engine and the `MemoryStore` backend.
///
/// Stub — full implementation in Task 97.
pub mod memory_bridge;

/// Structured logging and observability for engine state transitions.
///
/// Stub — full implementation in Task 100.
pub mod observability;

/// Goal-driven plan construction.
///
/// Stub — full implementation in Task 96.
pub mod planner;

/// Finite-state-machine types, plan model, hypotheses, and the top-level
/// [`ReasoningEngineState`].
///
/// **Fully implemented** as part of Task 94.
pub mod state;

// ---------------------------------------------------------------------------
// Flat re-exports — public surface of the engine crate
// ---------------------------------------------------------------------------

// From beliefs.rs (Task 95)
pub use beliefs::{EngineBeliefs, Evidence, ToolBelief};

// From planner.rs (Task 96)
pub use planner::{PlanBuilder, PlanBuilderConfig, ToolHint};

// From memory_bridge.rs (Task 97)
pub use memory_bridge::{MemoryBridge, MemoryBridgeConfig};

// From arbitration.rs (Task 98)
pub use execution::execute_delegate_to_sub_agent;

// From correction.rs (Task 99)
pub use correction::{CorrectionConfig, CorrectionEngine, CorrectionOutcome, CorrectionTrigger};

// From observability.rs (Task 100)
pub use observability::{EngineObserver, ObserverConfig, is_safe_to_log, redact_state};

/// Re-exports of all public state types and constants.
pub use state::{
    ENGINE_SCHEMA_VERSION, EngineState, Hypothesis, PlanError, PlanStep, ReasoningEngineState,
    StepAction, StepStatus, TransitionError, validate_version,
};
