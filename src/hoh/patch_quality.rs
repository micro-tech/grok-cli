//! HOH Patch Quality Scoring (Task 361.31)

use crate::hoh::state::PatchSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchQualityScore {
    pub patch_id: String,
    pub score: f32,
    pub coherence: f32,
    pub completeness: f32,
    pub test_coverage: f32,
    pub documentation: f32,
    pub issues: Vec<String>,
}

pub fn score_patch(patch: &PatchSet) -> PatchQualityScore {
    let mut issues = Vec::new();
    let content = patch.intended_content.as_deref().unwrap_or(&patch.diff_summary);

    // Coherence: single purpose (fewer files = more focused)
    let coherence = match patch.files_changed.len() {
        0     => { issues.push("No files listed".to_string()); 0.0 }
        1     => 1.0,
        2..=3 => 0.8,
        4..=6 => 0.6,
        _     => { issues.push("Too many files — consider splitting".to_string()); 0.3 }
    };

    // Completeness: non-empty content, has meaningful summary
    let completeness = {
        let mut c = 0.0f32;
        if !content.is_empty()              { c += 0.5; }
        if patch.diff_summary.len() > 20    { c += 0.3; }
        if !patch.source.is_empty()         { c += 0.2; }
        if content.is_empty() { issues.push("Empty patch content".to_string()); }
        c
    };

    // Test coverage: does content mention tests?
    let test_coverage = if content.contains("#[test]") || content.contains("test_") || content.contains("assert!") {
        1.0
    } else if content.contains("mod tests") {
        0.7
    } else {
        issues.push("No test coverage detected in patch".to_string());
        0.0
    };

    // Documentation: doc comments present
    let documentation = if content.contains("///") || content.contains("//!") { 0.8 }
        else if content.contains("//") { 0.4 }
        else { issues.push("No documentation in patch".to_string()); 0.0 };

    let score = coherence * 0.3 + completeness * 0.3 + test_coverage * 0.25 + documentation * 0.15;

    PatchQualityScore {
        patch_id: patch.id.clone(),
        score, coherence, completeness, test_coverage, documentation, issues,
    }
}

pub fn score_all(patches: &[PatchSet]) -> Vec<PatchQualityScore> {
    patches.iter().map(score_patch).collect()
}

pub fn filter_low_quality(patches: &[PatchSet], threshold: f32) -> Vec<&PatchSet> {
    patches.iter().filter(|p| score_patch(p).score < threshold).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(id: &str, files: usize, content: &str) -> PatchSet {
        PatchSet {
            id: id.to_string(),
            files_changed: (0..files).map(|i| format!("src/f{}.rs", i)).collect(),
            diff_summary: "meaningful change description".to_string(),
            source: "hoh".to_string(), timestamp: 0,
            intended_content: Some(content.to_string()),
        }
    }

    #[test]
    fn test_good_patch_high_score() {
        let p = patch("p1", 1, "/// docs\nfn foo() {}\n#[test] fn test_foo() { assert!(true); }");
        assert!(score_patch(&p).score > 0.6);
    }

    #[test]
    fn test_empty_patch_low_score() {
        let p = patch("p2", 0, "");
        assert!(score_patch(&p).score < 0.4);
    }
}
