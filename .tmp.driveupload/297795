//! HOH Regression Recovery Engine (Task 361.21)

use crate::hoh::state::{EvaluationReport, IterationState, PatchSet};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryKind { Revert, Bisect, CompensatingPatch, Escalate }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionRecoveryPlan {
    pub regression_iteration: u64,
    pub kind: RecoveryKind,
    pub description: String,
    pub patches: Vec<PatchSet>,
}

/// Detect regression by comparing current eval to previous evals.
pub fn detect_regression(current: &EvaluationReport, history: &[EvaluationReport]) -> bool {
    if history.is_empty() { return false; }
    let prev_avg_score: f32 = history.iter()
        .filter_map(|e| e.helix_score)
        .sum::<f32>() / history.len().max(1) as f32;

    let current_score = current.helix_score.unwrap_or(0.0);
    let drop = prev_avg_score - current_score;

    let test_regressed = history.iter().any(|e| e.test_passed == Some(true))
        && current.test_passed == Some(false);

    drop > 0.15 || test_regressed
}

/// Create a recovery plan for a detected regression.
pub fn create_recovery_plan(
    state: &IterationState,
    regressed_iteration: u64,
) -> RegressionRecoveryPlan {
    // Determine strategy based on available context
    let (kind, description) = if !state.patches.is_empty() {
        (RecoveryKind::Revert,
         format!("Revert {} patches from iteration {}", state.patches.len(), regressed_iteration))
    } else {
        (RecoveryKind::Escalate,
         format!("Regression in iteration {} — no patches to revert, escalate to human", regressed_iteration))
    };

    // Generate revert patches (stub — would use git in real impl)
    let patches: Vec<PatchSet> = state.patches.iter().map(|p| PatchSet {
        id: format!("revert-{}", p.id),
        files_changed: p.files_changed.clone(),
        diff_summary: format!("REVERT: {}", p.diff_summary),
        source: "regression_recovery".to_string(),
        timestamp: now(),
        intended_content: None,
    }).collect();

    RegressionRecoveryPlan { regression_iteration: regressed_iteration, kind, description, patches }
}

/// Apply recovery: push revert patches into state.
pub fn apply_recovery(state: &mut IterationState, plan: &RegressionRecoveryPlan) {
    for p in &plan.patches {
        state.patches.push(p.clone());
    }
    state.summary = Some(format!(
        "[Regression Recovery] {} — {}",
        match plan.kind { RecoveryKind::Revert => "Reverted", RecoveryKind::Escalate => "Escalated",
            RecoveryKind::Bisect => "Bisecting", RecoveryKind::CompensatingPatch => "Compensating" },
        plan.description
    ));
    tracing::warn!(
        iteration = state.iteration_id, regression_at = plan.regression_iteration,
        "[HOH RegressionRecovery] applied recovery plan"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(score: f32, passed: bool) -> EvaluationReport {
        EvaluationReport { helix_score: Some(score), test_passed: Some(passed), ..Default::default() }
    }

    #[test]
    fn test_regression_detected_on_score_drop() {
        let history = vec![eval(0.8, true), eval(0.75, true)];
        let current = eval(0.5, false);
        assert!(detect_regression(&current, &history));
    }

    #[test]
    fn test_no_regression_if_stable() {
        let history = vec![eval(0.7, true)];
        let current = eval(0.72, true);
        assert!(!detect_regression(&current, &history));
    }
}
