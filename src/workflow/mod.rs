//! Workflow tracing layer for Grok-CLI.
//!
//! Records the full lifecycle of a code-generation + validation workflow:
//! User prompt → LLM code → tool runs (check/clippy/test) → decision → return to LLM or user.
//!
//! This is the foundation for Task 232 and later tasks (TUI viewer, /trace command, JSON persistence, task-graph parallelization).

pub mod runner;
pub mod trace;

pub use runner::run_cargo_validation_workflow;
pub use trace::{WorkflowStep, WorkflowTrace};