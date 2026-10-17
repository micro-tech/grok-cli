//! HOH Patch Semantic Validation (Task 361.34)

use crate::hoh::state::PatchSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SemanticValidity { Valid, Invalid(String), Uncertain(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticValidation {
    pub patch_id: String,
    pub validity: SemanticValidity,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
    pub confidence: f32,
}

/// Validate a patch semantically (without running the compiler).
pub fn validate_semantics(patch: &PatchSet) -> SemanticValidation {
    let content = patch.intended_content.as_deref().unwrap_or(&patch.diff_summary);
    let mut passed = Vec::new();
    let mut failed = Vec::new();

    // Check 1: Source claim matches files
    if !patch.source.is_empty() { passed.push("Source field set".to_string()); }
    else { failed.push("Missing source field".to_string()); }

    // Check 2: Files listed exist in content (rough)
    for file in &patch.files_changed {
        let ext = std::path::Path::new(file).extension()
            .and_then(|e| e.to_str()).unwrap_or("");
        if ext == "rs" && content.contains("fn ") {
            passed.push(format!("Rust content matches .rs file {}", file));
        }
    }

    // Check 3: Balanced braces
    let open  = content.chars().filter(|&c| c == '{').count();
    let close = content.chars().filter(|&c| c == '}').count();
    if open == close {
        passed.push("Balanced braces".to_string());
    } else {
        failed.push(format!("Unbalanced braces: {} open, {} close", open, close));
    }

    // Check 4: No obvious syntax errors (very rough heuristic)
    let has_syntax_issue = content.contains("fn fn ") || content.contains("let let ")
        || content.contains("pub pub ");
    if has_syntax_issue {
        failed.push("Obvious syntax duplication detected".to_string());
    } else {
        passed.push("No obvious syntax duplication".to_string());
    }

    // Check 5: Semantic consistency — if diff_summary says "add X", content should mention X
    let key = patch.diff_summary.split_whitespace()
        .find(|w| w.len() > 4 && w.chars().all(|c| c.is_alphabetic()))
        .map(|w| w.to_lowercase());
    if let Some(kw) = key {
        if content.to_lowercase().contains(&kw) {
            passed.push(format!("Content consistent with summary keyword '{}'", kw));
        }
    }

    let confidence = passed.len() as f32 / (passed.len() + failed.len()).max(1) as f32;

    let validity = if failed.is_empty() {
        SemanticValidity::Valid
    } else if failed.len() == 1 && content.len() < 10 {
        SemanticValidity::Uncertain(failed[0].clone())
    } else {
        SemanticValidity::Invalid(failed.join("; "))
    };

    SemanticValidation { patch_id: patch.id.clone(), validity, checks_passed: passed, checks_failed: failed, confidence }
}

pub fn validate_all(patches: &[PatchSet]) -> Vec<SemanticValidation> {
    patches.iter().map(validate_semantics).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(id: &str, summary: &str, content: &str) -> PatchSet {
        PatchSet { id: id.to_string(), diff_summary: summary.to_string(),
            files_changed: vec!["src/lib.rs".to_string()],
            source: "hoh".to_string(), timestamp: 0,
            intended_content: Some(content.to_string()) }
    }

    #[test]
    fn test_balanced_braces_passes() {
        let p = patch("p1", "add compute function",
            "fn compute() { let x = 1; { x } }");
        let v = validate_semantics(&p);
        assert!(v.checks_passed.iter().any(|c| c.contains("Balanced")));
    }

    #[test]
    fn test_unbalanced_braces_fails() {
        let p = patch("p2", "bad code", "fn foo() { { { }");
        let v = validate_semantics(&p);
        assert!(v.checks_failed.iter().any(|c| c.contains("Unbalanced")));
    }
}
