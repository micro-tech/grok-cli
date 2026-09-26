//! HOH Task Risk Assessment (Task 327.21)

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum RiskLevel { Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub task_id: u64,
    pub risk_level: RiskLevel,
    /// 0.0–1.0
    pub risk_score: f32,
    pub risk_factors: Vec<String>,
    pub mitigations: Vec<String>,
}

static HIGH_RISK_KEYWORDS: &[(&str, f32, &str)] = &[
    ("security",    0.20, "security-sensitive change"),
    ("auth",        0.20, "authentication/authorization change"),
    ("unsafe",      0.25, "unsafe Rust code"),
    ("breaking",    0.20, "breaking API change"),
    ("migration",   0.15, "data migration"),
    ("delete",      0.15, "data deletion"),
    ("public api",  0.15, "public API change"),
    ("crypto",      0.20, "cryptographic operation"),
    ("prod",        0.10, "production-affecting change"),
    ("refactor",    0.08, "large-scale refactor"),
];

pub fn assess_risk(task: &Task) -> RiskAssessment {
    let text = format!("{} {} {}", task.title, task.description, task.details).to_lowercase();
    let mut score: f32 = 0.0;
    let mut factors = Vec::new();
    let mut mitigations = Vec::new();

    for (kw, weight, reason) in HIGH_RISK_KEYWORDS {
        if text.contains(kw) {
            score += weight;
            factors.push(reason.to_string());
        }
    }

    // Large scope (many dependencies) → higher risk
    if task.dependencies.len() > 5 {
        score += 0.1;
        factors.push("many dependencies".to_string());
    }

    // No test strategy → higher risk
    if task.test_strategy.trim().is_empty() {
        score += 0.1;
        factors.push("no test strategy defined".to_string());
    }

    score = score.clamp(0.0, 1.0);

    // Generate mitigations
    if score > 0.15 { mitigations.push("Add comprehensive tests before merging".to_string()); }
    if text.contains("security") || text.contains("auth") {
        mitigations.push("Request security review".to_string());
    }
    if text.contains("breaking") || text.contains("public api") {
        mitigations.push("Provide migration guide and deprecation period".to_string());
    }
    if text.contains("migration") {
        mitigations.push("Take database snapshot before running migration".to_string());
    }
    if mitigations.is_empty() {
        mitigations.push("Standard review process".to_string());
    }

    let risk_level = match score {
        s if s < 0.15 => RiskLevel::Low,
        s if s < 0.35 => RiskLevel::Medium,
        s if s < 0.60 => RiskLevel::High,
        _             => RiskLevel::Critical,
    };

    RiskAssessment { task_id: task.id, risk_level, risk_score: score, risk_factors: factors, mitigations }
}

pub fn assess_all(tasks: &[Task]) -> Vec<RiskAssessment> {
    tasks.iter().map(assess_risk).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, title: &str, desc: &str) -> Task {
        Task { id, title: title.to_string(), description: desc.to_string(), ..Default::default() }
    }

    #[test]
    fn test_security_task_is_high_risk() {
        let task = t(1, "Update auth security system", "security-sensitive breaking change");
        let r = assess_risk(&task);
        assert!(r.risk_score > 0.3);
        assert!(!r.mitigations.is_empty());
    }

    #[test]
    fn test_simple_task_is_low_risk() {
        let task = t(2, "Update README", "Fix a typo in the documentation");
        let r = assess_risk(&task);
        assert_eq!(r.risk_level, RiskLevel::Low);
    }
}
