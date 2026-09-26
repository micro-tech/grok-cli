//! HOH Predictive Failure Modeling (Task 361.18)

use crate::hoh::state::{EvaluationReport, PatchSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePrediction {
    pub patch_id: String,
    pub failure_probability: f32,   // 0.0–1.0
    pub risk_factors: Vec<String>,
    pub confidence: f32,
}

/// Predict which patches are likely to cause failures based on historical signals.
pub fn predict_failures(
    patches: &[PatchSet],
    history: &[EvaluationReport],
) -> Vec<FailurePrediction> {
    let historical_fail_rate = if history.is_empty() { 0.3 } else {
        let failed = history.iter().filter(|e| e.test_passed == Some(false)).count();
        failed as f32 / history.len() as f32
    };

    patches.iter().map(|patch| {
        let mut prob = historical_fail_rate;
        let mut factors = Vec::new();

        // Large diffs are riskier
        if patch.diff_summary.len() > 500 {
            prob += 0.1;
            factors.push("large diff".to_string());
        }

        // Many files changed = higher risk
        if patch.files_changed.len() > 3 {
            prob += 0.1;
            factors.push(format!("{} files changed", patch.files_changed.len()));
        }

        // Risky file patterns
        for file in &patch.files_changed {
            let f = file.to_lowercase();
            if f.contains("auth") || f.contains("security") || f.contains("main") {
                prob += 0.15;
                factors.push(format!("sensitive file: {}", file));
            }
        }

        // Previous low helix scores suggest fragile area
        let avg_helix = history.iter()
            .filter_map(|e| e.helix_score)
            .fold((0.0f32, 0usize), |(sum, cnt), s| (sum + s, cnt + 1));
        if avg_helix.1 > 0 && (avg_helix.0 / avg_helix.1 as f32) < 0.4 {
            prob += 0.1;
            factors.push("historically low helix scores".to_string());
        }

        FailurePrediction {
            patch_id: patch.id.clone(),
            failure_probability: prob.clamp(0.0, 1.0),
            risk_factors: factors,
            confidence: if history.len() >= 3 { 0.75 } else { 0.4 },
        }
    }).collect()
}

/// Sort patches: safest first.
pub fn order_by_safety(patches: &mut Vec<PatchSet>, history: &[EvaluationReport]) {
    let predictions = predict_failures(patches, history);
    let prob_map: std::collections::HashMap<String, f32> = predictions.into_iter()
        .map(|p| (p.patch_id, p.failure_probability)).collect();
    patches.sort_by(|a, b| {
        let pa = prob_map.get(&a.id).copied().unwrap_or(0.5);
        let pb = prob_map.get(&b.id).copied().unwrap_or(0.5);
        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(id: &str, files: Vec<&str>) -> PatchSet {
        PatchSet { id: id.to_string(), files_changed: files.into_iter().map(String::from).collect(),
            diff_summary: String::new(), source: "test".to_string(), timestamp: 0, intended_content: None }
    }

    #[test]
    fn test_sensitive_file_increases_probability() {
        let patches = vec![patch("p1", vec!["src/auth/mod.rs"])];
        let preds = predict_failures(&patches, &[]);
        assert!(preds[0].failure_probability > 0.3);
    }

    #[test]
    fn test_safe_patch_lower_prob_than_risky() {
        let patches = vec![
            patch("safe", vec!["README.md"]),
            patch("risky", vec!["src/auth.rs", "src/security.rs", "src/main.rs", "src/lib.rs"]),
        ];
        let preds = predict_failures(&patches, &[]);
        let safe_prob = preds.iter().find(|p| p.patch_id == "safe").unwrap().failure_probability;
        let risky_prob = preds.iter().find(|p| p.patch_id == "risky").unwrap().failure_probability;
        assert!(risky_prob > safe_prob);
    }
}
