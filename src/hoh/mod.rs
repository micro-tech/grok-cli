//! Harness-of-Harness (HOH) - Multi-Day Autonomous Development Outer Loop
//!
//! This module implements the outer orchestration layer for long-running,
//! self-improving development cycles. HOH plans, delegates to the inner
//! Grok-CLI harness, captures patches, evaluates, and iterates.

pub mod outer_loop;
pub mod state;
pub mod planner;
pub mod simulation;
pub mod patch_capture;
pub mod testing;
pub mod helix;
pub mod continual_improvement;
pub mod tool_control;
pub mod iteration_folders;
pub mod timeline;
pub mod safety;
pub mod recovery;
pub mod multi_agent;
pub mod cli;
pub mod tasklist_adapter;
pub mod task_dependency_graph;
pub mod task_mutation;
pub mod evolution_engine;
pub mod task_completion_tracker;

pub use outer_loop::HOHManager;
pub use state::{IterationState, PatchSet};

/// Top-level HOH configuration and entry points.
#[derive(Debug, Clone, Default)]
pub struct HOHConfig {
    pub simulation_mode: bool,
    pub max_iterations_per_day: u32,
    pub autonomy_level: AutonomyLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutonomyLevel {
    Observe,
    Propose,
    #[default]
    ExecuteWithApproval,
    FullAuto,
}