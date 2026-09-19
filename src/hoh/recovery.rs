//! HOH Failure Recovery (Task 297.18 + 297.28)

use crate::hoh::state::IterationState;

pub async fn recover_from_failure(state: &mut IterationState, error: &str) {
    state.status = crate::hoh::state::IterationStatus::Failed;
    state.summary = Some(format!("Recovered from: {}", error));
}