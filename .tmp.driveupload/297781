//! HOH Failure Recovery (Tasks 297.18 + 297.28)
//!
//! Classifies failures, chooses recovery strategies, applies backoff,
//! and saves partial state so the outer loop can continue safely.

use crate::hoh::state::{IterationState, IterationStatus};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Category of failure that occurred.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FailureKind {
    PlanningFailed,
    ExecutionCrashed(String),
    TestsAllFailed,
    EvaluationUnavailable,
    BudgetExhausted,
    UnknownError(String),
}

/// How the outer loop should respond to a failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryStrategy {
    Retry { attempts_left: u32 },
    Skip,
    RollbackAndRetry,
    Escalate(String),
    PartialContinue,
}

/// Record of a recovery action taken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub kind: FailureKind,
    pub strategy: RecoveryStrategy,
    pub message: String,
    pub timestamp: u64,
}

/// Map an error string to a `FailureKind`.
pub fn classify_failure(error: &str) -> FailureKind {
    let lower = error.to_lowercase();
    if lower.contains("plan") || lower.contains("no tasks") {
        FailureKind::PlanningFailed
    } else if lower.contains("budget") || lower.contains("limit") || lower.contains("quota") {
        FailureKind::BudgetExhausted
    } else if lower.contains("test") && (lower.contains("fail") || lower.contains("panic")) {
        FailureKind::TestsAllFailed
    } else if lower.contains("helix") || lower.contains("evaluation") || lower.contains("score") {
        FailureKind::EvaluationUnavailable
    } else if lower.contains("crash") || lower.contains("panic") || lower.contains("thread") {
        FailureKind::ExecutionCrashed(error.to_string())
    } else {
        FailureKind::UnknownError(error.to_string())
    }
}

/// Choose the best recovery strategy given the failure kind and retry count.
pub fn choose_recovery_strategy(kind: &FailureKind, attempt: u32) -> RecoveryStrategy {
    match kind {
        FailureKind::PlanningFailed => {
            if attempt < 2 {
                RecoveryStrategy::Retry { attempts_left: 2 - attempt }
            } else {
                RecoveryStrategy::Skip
            }
        }
        FailureKind::ExecutionCrashed(_) => {
            if attempt == 0 {
                RecoveryStrategy::RollbackAndRetry
            } else {
                RecoveryStrategy::Escalate("Execution crashed twice — human review needed".to_string())
            }
        }
        FailureKind::TestsAllFailed => RecoveryStrategy::PartialContinue,
        FailureKind::EvaluationUnavailable => RecoveryStrategy::Skip,
        FailureKind::BudgetExhausted => {
            RecoveryStrategy::Escalate("Budget limit reached — increase budget or reduce scope".to_string())
        }
        FailureKind::UnknownError(_) => {
            if attempt == 0 {
                RecoveryStrategy::Retry { attempts_left: 1 }
            } else {
                RecoveryStrategy::Escalate("Unknown error persists after retry".to_string())
            }
        }
    }
}

/// Apply exponential backoff: waits min(2^attempt, 30) seconds.
pub async fn apply_backoff(attempt: u32) {
    let secs = (1u64 << attempt.min(5)).min(30);
    tracing::info!(attempt, secs, "[HOH Recovery] backing off");
    tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
}

/// Main recovery entry point. Classifies the error, chooses a strategy,
/// updates the iteration state, and returns the action taken.
pub async fn recover_from_failure(
    state: &mut IterationState,
    error: &str,
    attempt: u32,
) -> RecoveryAction {
    let kind = classify_failure(error);
    let strategy = choose_recovery_strategy(&kind, attempt);

    let message = match &strategy {
        RecoveryStrategy::Retry { attempts_left } => {
            format!("Retrying ({} attempts left): {}", attempts_left, error)
        }
        RecoveryStrategy::Skip => format!("Skipping failed step: {}", error),
        RecoveryStrategy::RollbackAndRetry => {
            format!("Rolling back and retrying after: {}", error)
        }
        RecoveryStrategy::Escalate(reason) => {
            format!("Escalating — {}: {}", reason, error)
        }
        RecoveryStrategy::PartialContinue => {
            format!("Continuing partially despite: {}", error)
        }
    };

    // Update state
    match &strategy {
        RecoveryStrategy::Escalate(_) | RecoveryStrategy::Skip => {
            state.status = IterationStatus::Failed;
        }
        RecoveryStrategy::PartialContinue => {
            // Keep current status — partial work still counts
        }
        _ => {
            // Retry / RollbackAndRetry — leave status unchanged for outer loop to re-run
        }
    }

    // Always save partial summary
    let existing = state.summary.clone().unwrap_or_default();
    state.summary = Some(format!("{}\n[Recovery] {}", existing, message).trim().to_string());

    tracing::warn!(
        iteration = state.iteration_id,
        attempt,
        ?kind,
        "[HOH Recovery] {}", message
    );

    RecoveryAction {
        kind,
        strategy,
        message,
        timestamp: now_secs(),
    }
}

/// Convenience wrapper for single-attempt recovery (attempt=0).
pub async fn recover(state: &mut IterationState, error: &str) -> RecoveryAction {
    recover_from_failure(state, error, 0).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_failure_maps_correctly() {
        assert_eq!(classify_failure("planning error: no tasks"), FailureKind::PlanningFailed);
        assert_eq!(classify_failure("budget quota exceeded"), FailureKind::BudgetExhausted);
        assert!(matches!(classify_failure("test panicked"), FailureKind::TestsAllFailed));
        assert!(matches!(classify_failure("something weird"), FailureKind::UnknownError(_)));
    }

    #[test]
    fn test_choose_strategy_escalates_after_retries() {
        let kind = FailureKind::PlanningFailed;
        assert!(matches!(
            choose_recovery_strategy(&kind, 0),
            RecoveryStrategy::Retry { .. }
        ));
        assert_eq!(choose_recovery_strategy(&kind, 2), RecoveryStrategy::Skip);
    }

    #[tokio::test]
    async fn test_recover_updates_state_summary() {
        let mut state = IterationState::new(1);
        let action = recover(&mut state, "test panicked in module X").await;
        assert!(state.summary.is_some());
        assert!(!action.message.is_empty());
    }
}
