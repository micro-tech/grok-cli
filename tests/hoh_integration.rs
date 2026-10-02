//! HOH Integration Tests (Task 297.33)
//!
//! End-to-end tests using simulation mode. Verifies the full
//! planning → execution → evaluation → improvement loop with zero
//! side effects on the real source tree.

use grok_cli::hoh::simulation::{HOHSimulator, SimulationConfig, SimulationEvent};
use grok_cli::hoh::state::{IterationState, IterationStatus};
use std::path::PathBuf;
use std::time::SystemTime;

// ── helpers ─────────────────────────────────────────────────────────────────

fn project_root() -> PathBuf {
    // Integration tests run from the crate root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ── tests ────────────────────────────────────────────────────────────────────

/// A simulation run must finish (Completed, not Failed) and have a summary.
#[tokio::test]
async fn test_simulation_runs_end_to_end() {
    let mut sim = HOHSimulator::new(SimulationConfig::default());
    let state = sim.run_simulation(1).await;

    assert_ne!(
        state.status,
        IterationStatus::Failed,
        "simulation should not fail"
    );
    assert!(
        state.summary.is_some() && !state.summary.as_deref().unwrap().is_empty(),
        "summary must be non-empty"
    );
    assert!(
        !state.patches.is_empty(),
        "at least one patch must be generated"
    );
    assert!(
        !state.evaluations.is_empty(),
        "at least one evaluation must be recorded"
    );
}

/// Simulation must not modify any `src/` file.
#[tokio::test]
async fn test_simulation_zero_side_effects() {
    let root = project_root();
    let src = root.join("src");

    // Collect mtime snapshot before
    let before: Vec<(PathBuf, SystemTime)> = walkdir::WalkDir::new(&src)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            e.metadata().ok().and_then(|m| m.modified().ok()).map(|mt| (e.path().to_path_buf(), mt))
        })
        .collect();

    let mut sim = HOHSimulator::new(SimulationConfig::default());
    sim.run_simulation(99).await;

    // Check mtimes after
    for (path, before_time) in &before {
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(after_time) = meta.modified() {
                assert_eq!(
                    before_time, &after_time,
                    "simulation modified {path:?} — should be zero side effects"
                );
            }
        }
    }
}

/// The simulation result must record at least one event.
#[tokio::test]
async fn test_simulation_records_events() {
    let mut sim = HOHSimulator::new(SimulationConfig {
        record_events: true,
        seed: 7,
        max_steps: 3,
    });
    sim.run_simulation(2).await;

    let result = sim.get_result().expect("result must be present");
    assert!(
        !result.events.is_empty(),
        "at least one event must be recorded"
    );
    assert!(
        !result.would_have_changed.is_empty(),
        "would_have_changed must be non-empty"
    );

    // Verify event types are present
    let has_plan = result.events.iter().any(|e| matches!(e, SimulationEvent::PlanCreated { .. }));
    let has_eval = result.events.iter().any(|e| matches!(e, SimulationEvent::EvaluationComplete(_)));
    assert!(has_plan, "PlanCreated event missing");
    assert!(has_eval, "EvaluationComplete event missing");
}

/// Simulating the same iteration with the same seed gives identical scores.
#[tokio::test]
async fn test_simulation_deterministic_with_seed() {
    let cfg = SimulationConfig { seed: 42, max_steps: 3, record_events: true };
    let mut a = HOHSimulator::new(cfg.clone());
    let mut b = HOHSimulator::new(cfg);

    let sa = a.run_simulation(5).await;
    let sb = b.run_simulation(5).await;

    assert_eq!(
        sa.evaluations[0].helix_score,
        sb.evaluations[0].helix_score,
        "same seed must produce same helix score"
    );
    assert_eq!(
        sa.evaluations[0].test_passed,
        sb.evaluations[0].test_passed,
        "same seed must produce same test outcome"
    );
}

/// Multi-step simulation respects max_steps.
#[tokio::test]
async fn test_simulation_respects_max_steps() {
    let mut sim = HOHSimulator::new(SimulationConfig {
        max_steps: 2,
        seed: 1,
        record_events: true,
    });
    let state = sim.run_simulation(3).await;
    assert!(
        state.patches.len() <= 2,
        "patch count ({}) must not exceed max_steps (2)",
        state.patches.len()
    );
}

/// State from a fresh IterationState has sensible defaults.
#[test]
fn test_iteration_state_defaults() {
    let state = IterationState::new(42);
    assert_eq!(state.iteration_id, 42);
    assert_eq!(state.status, IterationStatus::Planning);
    assert!(state.patches.is_empty());
    assert!(state.evaluations.is_empty());
    assert!(state.summary.is_none());
}
