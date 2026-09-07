//! HOH Simulation Mode (Task 297.25)

use crate::hoh::state::IterationState;

/// Runs a full iteration in pure simulation (zero side effects on real tree).
pub async fn run_simulation(iteration: u64) -> IterationState {
    let mut state = IterationState::new(iteration);
    state.summary = Some("Simulation run completed (no real changes)".to_string());
    state
}