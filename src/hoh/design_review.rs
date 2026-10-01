//! HOH Autonomous Design Review (Task 361.29)

use crate::hoh::state::PatchSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReviewVerdict { Approved, ApprovedWithSuggestions, RequestChanges, Rejected }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignReview {
    pub patch_id: String,
    pub verdict: ReviewVerdict,
    pub score: f32,
    pub findings: Vec<ReviewFinding>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub severity: FindingSeverity,
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FindingSeverity { Info, Warning, Error }

static ANTI_PATTERNS: &[(&str, FindingSeverity, &str, &str)] = &[
    ("unwrap()",          FindingSeverity::Warning, "error-handling",   "Use ? or match instead of .unwrap()"),
    ("panic!(",           FindingSeverity::Error,   "robustness",       "Avoid panic! in library code"),
    ("todo!()",           FindingSeverity::Warning, "completeness",     "TODO marker left in code"),
    ("unsafe ",           FindingSeverity::Error,   "safety",           "Unsafe code requires justification comment"),
    ("clone()",           FindingSeverity::Info,    "performance",      "Excessive cloning may hurt performance"),
    ("as usize",          FindingSeverity::Warning, "correctness",      "Numeric cast may truncate — use try_from"),
    (".expect(",          FindingSeverity::Warning, "error-handling",   "Prefer ? over .expect() in fallible paths"),
];

/// Review a patch for design quality issues.
pub fn review_patch(patch: &PatchSet, content_hint: Option<&str>) -> DesignReview {
    let text = content_hint.unwrap_or(&patch.diff_summary);
    let mut findings = Vec::new();
    let mut score: f32 = 1.0;

    for (pattern, severity, category, message) in ANTI_PATTERNS {
        if text.contains(pattern) {
            let penalty = match severity {
                FindingSeverity::Error   => 0.2,
                FindingSeverity::Warning => 0.1,
                FindingSeverity::Info    => 0.02,
            };
            score -= penalty;
            findings.push(ReviewFinding {
                severity: severity.clone(),
                category: category.to_string(),
                message: message.to_string(),
            });
        }
    }

    // Reward good patterns
    if text.contains("Result<") || text.contains("Option<") { score += 0.05; }
    if text.contains("#[cfg(test)]") || text.contains("#[test]") { score += 0.1; }
    if text.contains("///") { score += 0.05; }

    score = score.clamp(0.0, 1.0);

    let errors = findings.iter().filter(|f| f.severity == FindingSeverity::Error).count();
    let warnings = findings.iter().filter(|f| f.severity == FindingSeverity::Warning).count();

    let verdict = if errors > 0 {
        ReviewVerdict::RequestChanges
    } else if score < 0.5 {
        ReviewVerdict::Rejected
    } else if warnings > 0 {
        ReviewVerdict::ApprovedWithSuggestions
    } else {
        ReviewVerdict::Approved
    };

    let summary = format!(
        "Score: {:.2} | {} error(s), {} warning(s), {} info | Verdict: {:?}",
        score, errors, warnings,
        findings.iter().filter(|f| f.severity == FindingSeverity::Info).count(),
        verdict
    );

    DesignReview { patch_id: patch.id.clone(), verdict, score, findings, summary }
}

pub fn review_all(patches: &[PatchSet]) -> Vec<DesignReview> {
    patches.iter().map(|p| review_patch(p, None)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(id: &str, content: &str) -> PatchSet {
        PatchSet { id: id.to_string(), diff_summary: content.to_string(),
            files_changed: vec![], source: "test".to_string(), timestamp: 0, intended_content: None }
    }

    #[test]
    fn test_clean_code_approved() {
        let p = patch("p1", "fn compute(x: u32) -> Result<u32, Error> { /// doc\n#[test] }");
        let review = review_patch(&p, None);
        assert!(matches!(review.verdict, ReviewVerdict::Approved | ReviewVerdict::ApprovedWithSuggestions));
        assert!(review.score > 0.7);
    }

    #[test]
    fn test_panic_causes_request_changes() {
        let p = patch("p2", "fn bad() { panic!(\"oh no\"); unsafe { *ptr = 0; } }");
        let review = review_patch(&p, None);
        assert_eq!(review.verdict, ReviewVerdict::RequestChanges);
    }
}
