//! Helix Integration as Independent Evaluator (Task 297.7)
//!
//! Now consumes real PatchSets and produces richer EvaluationReport
//! including patch quality metrics. Part of 361.5 feedback.

use crate::hoh::state::{EvaluationReport, PatchSet};

/// Evaluate patches using a Helix-style objective scorer.
/// Computes a helix_score influenced by patch volume, focus (diff size), and diversity.
pub async fn evaluate_with_helix(patches: &[PatchSet]) -> EvaluationReport {
    let patch_count = patches.len();
    let files_changed: usize = patches.iter().map(|p| p.files_changed.len()).sum();
    let total_diff: usize = patches.iter().map(|p| p.diff_summary.len()).sum();
    let avg_diff = if patch_count > 0 { total_diff as f32 / patch_count as f32 } else { 0.0 };

    // Base helix score
    let mut helix: f32 = 0.65;

    // Volume signal: some work happened
    if patch_count > 0 {
        helix += 0.05;
    }
    if patch_count >= 3 {
        helix += 0.05;
    }

    // Focus signal: smaller, targeted diffs are better
    if avg_diff > 0.0 && avg_diff < 350.0 {
        helix += 0.08;
    } else if avg_diff > 1000.0 {
        helix -= 0.06;
    }

    // Diversity: touching multiple files in a coherent way
    if files_changed > patch_count && files_changed < patch_count * 4 {
        helix += 0.04;
    }

    // Cap it
    helix = helix.clamp(0.4, 0.92);

    let mut notes = format!(
        "Helix: {} patches, {} files, avg_diff {:.0}",
        patch_count, files_changed, avg_diff
    );

    if avg_diff < 300.0 && patch_count > 0 {
        notes.push_str(" | Good patch focus");
    }

    EvaluationReport {
        iteration_id: 0,
        helix_score: Some(helix),
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
    }
}