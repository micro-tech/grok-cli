//! Helix Integration as Independent Evaluator (Task 297.7)

use crate::hoh::state::EvaluationReport;

/// Stub for calling Helix (or local evaluator) for objective scoring.
pub async fn evaluate_with_helix(_patches: &[crate::hoh::state::PatchSet]) -> EvaluationReport {
    EvaluationReport {
        iteration_id: 0,
        helix_score: Some(0.65),
        internal_metrics: Default::default(),
        notes: "Helix evaluation stub (Task 297.7)".to_string(),
    }
}