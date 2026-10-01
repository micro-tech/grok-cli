//! HOH Simulation Mode (Task 297.25)
//!
//! Runs a complete HOH iteration in-memory with zero side effects on the real
//! filesystem. Used for safe experimentation, testing, and what-if analysis.

use crate::hoh::state::{EvaluationReport, HOHPlan, IterationState, IterationStatus, PatchSet};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Configuration for a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Maximum number of simulated steps (patch generations, test runs, etc.).
    pub max_steps: u32,
    /// Whether to record every event for later inspection.
    pub record_events: bool,
    /// Seed for deterministic randomness (uses simple hash-based simulation).
    pub seed: u64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            max_steps: 5,
            record_events: true,
            seed: 42,
        }
    }
}

/// Individual events recorded during a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationEvent {
    PlanCreated { goal_count: usize },
    PatchGenerated(String),
    TestRun { passed: bool, failures: u32 },
    EvaluationComplete(f32),
    ImprovementProposed(String),
    PhaseTransition { from: String, to: String },
}

/// Full results of a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub iteration_id: u64,
    pub events: Vec<SimulationEvent>,
    pub summary: String,
    /// File paths that *would* have been modified in a real run.
    pub would_have_changed: Vec<String>,
    pub simulated_helix_score: f32,
    pub simulated_test_pass: bool,
    pub duration_ms: u64,
}

/// Runs full HOH iterations in a sandboxed, zero-side-effect environment.
pub struct HOHSimulator {
    pub config: SimulationConfig,
    result: Option<SimulationResult>,
}

impl HOHSimulator {
    pub fn new(config: SimulationConfig) -> Self {
        Self {
            config,
            result: None,
        }
    }

    /// Run a complete simulated iteration, returning a populated IterationState.
    pub async fn run_simulation(&mut self, iteration: u64) -> IterationState {
        let start = now_secs();
        let mut state = IterationState::new(iteration);
        let mut events = Vec::new();

        // --- Planning phase ---
        state.status = IterationStatus::Planning;
        let goals = vec![
            format!("Sim: improve test coverage for iteration {}", iteration),
            "Sim: refactor module boundaries".to_string(),
            "Sim: update documentation".to_string(),
        ];
        state.plan = Some(HOHPlan {
            goals: goals.clone(),
            selected_tasks: vec![iteration, iteration + 1],
            experiments: vec!["experiment-A".to_string()],
            ..Default::default()
        });
        events.push(SimulationEvent::PlanCreated { goal_count: goals.len() });
        events.push(SimulationEvent::PhaseTransition {
            from: "Planning".to_string(),
            to: "Executing".to_string(),
        });

        // --- Execution phase (patch generation) ---
        state.status = IterationStatus::Executing;
        let would_have_changed: Vec<String> = vec![
            format!("src/hoh/sim_target_{}.rs", iteration),
            "src/hoh/mod.rs".to_string(),
        ];
        let steps = self.config.max_steps.min(3) as u64;
        for i in 0..steps {
            let patch = PatchSet {
                id: format!("sim-patch-{}-{}", iteration, i),
                files_changed: would_have_changed.clone(),
                diff_summary: format!("Simulated diff #{} for iteration {}", i, iteration),
                source: "simulation".to_string(),
                timestamp: start + i,
                intended_content: None,
            };
            events.push(SimulationEvent::PatchGenerated(patch.id.clone()));
            state.patches.push(patch);
        }
        events.push(SimulationEvent::PhaseTransition {
            from: "Executing".to_string(),
            to: "Testing".to_string(),
        });

        // --- Testing phase ---
        state.status = IterationStatus::Testing;
        // Deterministic: fail every 3rd iteration based on seed
        let test_passed = (iteration + self.config.seed) % 3 != 0;
        let failures = if test_passed { 0 } else { 2 };
        events.push(SimulationEvent::TestRun { passed: test_passed, failures });
        events.push(SimulationEvent::PhaseTransition {
            from: "Testing".to_string(),
            to: "Evaluating".to_string(),
        });

        // --- Evaluation phase ---
        state.status = IterationStatus::Evaluating;
        let helix_score = if test_passed {
            0.6 + (self.config.seed as f32 % 10.0) * 0.03
        } else {
            0.3 + (self.config.seed as f32 % 5.0) * 0.02
        };
        let eval = EvaluationReport {
            iteration_id: iteration,
            helix_score: Some(helix_score),
            test_passed: Some(test_passed),
            test_summary: if test_passed {
                "All simulated tests passed".to_string()
            } else {
                format!("{} simulated test failures", failures)
            },
            patch_count: state.patches.len(),
            files_changed_count: would_have_changed.len(),
            avg_diff_length: 128.0,
            ..Default::default()
        };
        events.push(SimulationEvent::EvaluationComplete(helix_score));
        state.evaluations.push(eval);

        // --- Improvement phase ---
        let improvements = vec![
            "Increase test coverage for hoh::state".to_string(),
            "Reduce clone() calls in planning phase".to_string(),
        ];
        for imp in &improvements {
            events.push(SimulationEvent::ImprovementProposed(imp.clone()));
        }
        if let Some(plan) = &mut state.plan {
            plan.improvement_suggestions = improvements.clone();
        }

        // --- Finalize ---
        let summary = format!(
            "Simulation #{}: {} patches generated, tests {}, helix={:.2}, would have changed {} files",
            iteration,
            state.patches.len(),
            if test_passed { "PASSED" } else { "FAILED" },
            helix_score,
            would_have_changed.len(),
        );
        state.mark_completed(summary.clone());

        let duration_ms = (now_secs() - start) * 1000;
        self.result = Some(SimulationResult {
            iteration_id: iteration,
            events,
            summary,
            would_have_changed,
            simulated_helix_score: helix_score,
            simulated_test_pass: test_passed,
            duration_ms,
        });

        tracing::info!(
            iteration,
            test_passed,
            helix_score,
            "[HOH Simulation] completed iteration"
        );
        state
    }

    pub fn get_result(&self) -> Option<&SimulationResult> {
        self.result.as_ref()
    }
}

/// Backward-compatible thin wrapper (used by outer_loop in simulation_mode).
pub async fn run_simulation(iteration: u64) -> IterationState {
    HOHSimulator::new(SimulationConfig::default())
        .run_simulation(iteration)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulation_produces_complete_state() {
        let mut sim = HOHSimulator::new(SimulationConfig::default());
        let state = sim.run_simulation(1).await;
        assert_eq!(state.status, IterationStatus::Completed);
        assert!(state.summary.is_some
());
        assert!(!state.patches.is_empty());
        assert!(!state.evaluations.is_empty());
    }

    #[tokio::test]
    async fn test_simulation_records_events() {
        let mut sim = HOHSimulator::new(SimulationConfig {
            record_events: true,
            ..Default::default()
        });
        sim.run_simulation(2).await;
        let result = sim.get_result().unwrap();
        assert!(!result.events.is_empty());
        assert!(!result.would_have_changed.is_empty());
    }

    #[tokio::test]
    async fn test_simulation_deterministic_with_seed() {
        let cfg = SimulationConfig { seed: 99
, ..Default::default() };
        let mut a = HOHSimulator::new(cfg.clone());
        let mut b = HOHSimulator::new(cfg);
        let sa = a.run_simulation(5).await;
        let sb = b.run_simulation(5).await;
        assert_eq!(
            sa.evaluations[0].helix_score,
            sb.evaluations[0].helix_score
        );
    }
}
