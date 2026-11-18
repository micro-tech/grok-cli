//! Local patch-shape heuristic (Task 297.7, renamed by H8 / task 491).
//!
//! This was formerly `evaluate_with_helix`, but it never called Helix:
//! it is a local heuristic over patch shape (focus and diversity). It is
//! NOT an independent evaluation and must not be presented as one. A real
//! Helix/LLM cross-check is a follow-up requiring a live backend, which
//! is not available in the sandbox.
//!
//! Now consumes real PatchSets and produces richer EvaluationReport
//! including patch quality metrics. Part of 361.5 feedback.

use crate::hoh::state::{EvaluationReport, PatchSet};

/// Score patches with a local patch-shape heuristic.
/// Computes a score influenced by patch focus (diff size) and diversity.
/// Patch volume is deliberately NOT rewarded: producing more patches is
/// activity, not improvement (H8 / task 491).
pub async fn evaluate_patch_shape(patches: &[PatchSet]) -> EvaluationReport {
    let patch_count = patches.len();
    let files_changed: usize = patches.iter().map(|p| p.files_changed.len()).sum();
    let total_diff: usize = patches.iter().map(|p| p.diff_summary.len()).sum();
    let avg_diff = if patch_count > 0 { total_diff as f32 / patch_count as f32 } else { 0.0 };

    // Base score for the patch-shape heuristic.
    let mut score: f32 = 0.65;

    // Focus signal: smaller, targeted diffs are better.
    if avg_diff > 0.0 && avg_diff < 350.0 {
        score += 0.08;
    } else if avg_diff > 1000.0 {
        score -= 0.06;
    }

    // Diversity: touching multiple files in a coherent way.
    if files_changed > patch_count && files_changed < patch_count * 4 {
        score += 0.04;
    }

    // Cap it
    score = score.clamp(0.4, 0.92);

    let mut notes = format!(
        "Patch shape: {} patches, {} files, avg_diff {:.0}",
        patch_count, files_changed, avg_diff
    );

    if avg_diff < 300.0 && patch_count > 0 {
        notes.push_str(" | Good patch focus");
    }

    EvaluationReport {
        iteration_id: 0,
        helix_score: Some(score),
        internal_metrics: {
            let mut m = std::collections::HashMap::new();
            m.insert("patch_count".to_string(), patch_count as f32);
            m.insert("files_changed".to_string(), files_changed as f32);
            m.insert("avg_diff_length".to_string(), avg_diff);
            m
        },
        notes,
        architecture_proposals_evaluated: vec![],
        meta_improvement_score: None,
        patch_count,
        files_changed_count: files_changed,
        avg_diff_length: avg_diff,
        test_passed: None,
        // New rich testing fields
        test_summary: String::new(),
        test_output: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::state::PatchSet;

    fn patch(files: usize, diff_len: usize) -> PatchSet {
        PatchSet {
            id: "test".to_string(),
            files_changed: (0..files).map(|i| format!("file{}.rs", i)).collect(),
            diff_summary: "x".repeat(diff_len),
            source: "test".to_string(),
            timestamp: 0,
            intended_content: None,
        }
    }

    #[tokio::test]
    async fn patch_volume_is_not_rewarded() {
        // H8 / task 491: one patch vs three patches with the same shape
        // (one file each, same avg diff) must score the same — producing
        // more patches is activity, not improvement.
        let one = evaluate_patch_shape(&[patch(1, 100)]).await;
        let three =
            evaluate_patch_shape(&[patch(1, 100), patch(1, 100), patch(1, 100)]).await;
        assert_eq!(one.helix_score, three.helix_score);
    }

    #[tokio::test]
    async fn focused_diffs_score_higher_than_sprawling_ones() {
        // Same patch count, focused diffs (avg 100) beat sprawling ones (avg 1500).
        let focused = evaluate_patch_shape(&[patch(1, 100)]).await;
        let sprawling = evaluate_patch_shape(&[patch(1, 1500)]).await;
        assert!(focused.helix_score.unwrap() > sprawling.helix_score.unwrap());
    }
}
