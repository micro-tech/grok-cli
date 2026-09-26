//! HOH Patch Risk Scoring (Task 361.32)

use crate::hoh::state::PatchSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum PatchRiskLevel { Safe, Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchRiskScore {
    pub patch_id: String,
    pub risk_level: PatchRiskLevel,
    pub risk_score: f32,
    pub factors: Vec<String>,
    pub required_approvals: u32,
}

static RISKY_PATTERNS: &[(&str, f32, &str)] = &[
    ("auth",         0.20, "authentication-related change"),
    ("security",     0.20, "security-sensitive area"),
    ("unsafe",       0.25, "unsafe Rust code"),
    ("breaking",     0.20, "breaking change"),
    ("migration",    0.15, "data migration"),
    ("main.rs",      0.10, "modifies entry point"),
    ("config",       0.10, "configuration change"),
    ("Cargo.toml",   0.15, "dependency change"),
    ("delete",       0.15, "deletion operation"),
    ("drop",         0.10, "resource drop"),
];

pub fn score_risk(patch: &PatchSet) -> PatchRiskScore {
    let text = format!("{} {}", patch.diff_summary, patch.files_changed.join(" ")).to_lowercase();
    let mut risk_score: f32 = 0.0;
    let mut factors = Vec::new();

    for (pattern, weight, desc) in RISKY_PATTERNS {
        if text.contains(pattern) {
            risk_score += weight;
            factors.push(desc.to_string());
        }
    }

    // Many files = higher risk
    if patch.files_changed.len() > 5 {
        risk_score += 0.15;
        factors.push(format!("{} files changed", patch.files_changed.len()));
    }

    risk_score = risk_score.clamp(0.0, 1.0);

    let risk_level = match risk_score {
        s if s < 0.1  => PatchRiskLevel::Safe,
        s if s < 0.25 => PatchRiskLevel::Low,
        s if s < 0.45 => PatchRiskLevel::Medium,
        s if s < 0.65 => PatchRiskLevel::High,
        _             => PatchRiskLevel::Critical,
    };

    let required_approvals = match &risk_level {
        PatchRiskLevel::Safe     => 0,
        PatchRiskLevel::Low      => 0,
        PatchRiskLevel::Medium   => 1,
        PatchRiskLevel::High     => 2,
        PatchRiskLevel::Critical => 3,
    };

    PatchRiskScore { patch_id: patch.id.clone(), risk_level, risk_score, factors, required_approvals }
}

pub fn score_all_risk(patches: &[PatchSet]) -> Vec<PatchRiskScore> {
    patches.iter().map(score_risk).collect()
}

pub fn needs_human_review(patch: &PatchSet) -> bool {
    score_risk(patch).required_approvals > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(id: &str, summary: &str, files: Vec<&str>) -> PatchSet {
        PatchSet { id: id.to_string(), diff_summary: summary.to_string(),
            files_changed: files.into_iter().map(String::from).collect(),
            source: "test".to_string(), timestamp: 0, intended_content: None }
    }

    #[test]
    fn test_safe_patch() {
        let p = patch("p1", "Update README typo", vec!["README.md"]);
        let s = score_risk(&p);
        assert_eq!(s.risk_level, PatchRiskLevel::Safe);
        assert_eq!(s.required_approvals, 0);
    }

    #[test]
    fn test_security_patch_high_risk() {
        let p = patch("p2", "Refactor auth security module with breaking changes", vec!["src/auth.rs", "src/security.rs"]);
        let s = score_risk(&p);
        assert!(s.risk_score >= 0.45);
        assert!(s.required_approvals >= 2);
    }
}
