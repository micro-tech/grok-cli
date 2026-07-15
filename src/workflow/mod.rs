//! Workflow tracing layer for Grok-CLI.
//!
//! Records the full lifecycle of a code-generation + validation workflow:
//! User prompt → LLM code → tool runs (check/clippy/test) → decision → return to LLM or user.
//!
//! This is the foundation for Task 232 and later tasks (TUI viewer, /trace command, JSON persistence, task-graph parallelization).

pub mod persistence;
pub mod runner;
pub mod trace;
pub mod trace_command;

pub use persistence::{
    ensure_workflows_dir, list_traces, load_latest_trace, load_trace, save_trace, trace_file_summary,
    workflows_dir,
};
pub use runner::run_cargo_validation_workflow;
pub use trace::{WorkflowStep, WorkflowTrace};
pub use trace_command::handle_trace_command;