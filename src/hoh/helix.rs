//! Local Patch-Shape Scoring (Task 297.7, renamed 2026-11-09 — H8)
//!
//! This module does NOT call Helix and is NOT an independent evaluation.
//! There is no live Helix/LLM backend wired in here. It scores patch
//! *shape* locally (diff size, file diversity) and returns an
//! EvaluationReport whose `helix_score` field carries that local score.
//! The name `helix_score` is a pre-existing field name kept for the
//! consumers that read it (evolution_engine, failure_prediction,
//! long_term_strategy, memory_integration); see its doc comment in
//! `hoh::state::EvaluationReport`. Wiring a real Helix/reviewer call is a
//! follow-up for John, not part of this rename.
//!
//! The former name was `evaluate_with_helix`. It was renamed because the
//! name, the "Helix independent evaluation" log line, and the Doc/HOH.md
//! claim all said Helix was judging the patches when it never was (H8).

use crate::hoh::state::{EvaluationReport, PatchSet};

/// Score patches by local shape: focused diffs and coherent file diversity.
/// No Helix call is made; the returned `helix_score` is this local score.
///
/// The patch-volume reward is gone (H8): producing more patches earned
/// +0.05 for any patch and +0.05 more for >=3, which rewarded volume
/// rather than quality. Patch count is still recorded in the report for
/// traceability, but it does not move the score.
pub async fn evaluate_patch_shape(patches: &[PatchSet]) -> EvaluationReport {
    let patch_count = patches.len();
    let files_changed: usize = patches.iter().map(|p| p.files_changed.len()).sum();
    let total_diff: usize = patches.iter().map(|p| p.diff_summary.len()).sum();
    let avg_diff = if patch_count > 0 { total_diff as f32 / patch_count as f32 } else { 0.0 };

    // Neutral base (not 0.65): this is a weak patch-shape signal, and it
    // must not inflate the meta score or the apply gate on its own.
    let mut score: f32 = 0.50;

    // Focus signal: smaller, targeted diffs are better
    if avg_diff > 0.0 && avg_diff < 350.0 {
        score += 0.08;
    } else if avg_diff > 1000.0 {
        score -= 0.06;
    }

    // Diversity: touching multiple files in a coherent way
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

    fn patch(id: &str, diff_len: usize, files: usize) -> PatchSet {
        PatchSet {
            id: id.into(),
            files_changed: (0..files).map(|i| format!("file{i}.rs")).collect(),
            diff_summary: "x".repeat(diff_len),
            source: "test".into(),
            timestamp: 0,
            intended_content: Some("content".into()),
        }
    }

    #[tokio::test]
    async fn patch_shape_does_not_reward_volume() {
        // H8: the volume reward is gone. One patch and five patches with
        // the same focused diff shape must score identically — producing
        // more patches earns nothing.
        let one = evaluate_patch_shape(&[patch("a", 100, 1)]).await;
        let five = evaluate_patch_shape(&[
            patch("a", 100, 1),
            patch("b", 100, 1),
            patch("c", 100, 1),
            patch("d", 100, 1),
            patch("e", 100, 1),
        ])
        .await;
        assert_eq!(one.helix_score, five.helix_score);
    }

    #[tokio::test]
    async fn patch_shape_rewards_focus_not_size() {
        // A focused diff scores above a huge one; both are local shape
        // signals, not a Helix judgment.
        let focused = evaluate_patch_shape(&[patch("a", 100, 1)]).await;
        let huge = evaluate_patch_shape(&[patch("a", 2000, 1)]).await;
        assert!(focused.helix_score.unwrap() > huge.helix_score.unwrap());
    }
}