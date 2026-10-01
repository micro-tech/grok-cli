//! HOH Patch Optimization (Task 361.33)

use crate::hoh::state::PatchSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedPatch {
    pub original_id: String,
    pub optimized: PatchSet,
    pub changes_made: Vec<String>,
    pub size_reduction_pct: f32,
}

/// Optimize a patch: deduplicate, trim noise, split if too large.
pub fn optimize_patch(patch: &PatchSet) -> OptimizedPatch {
    let mut changes_made = Vec::new();
    let original_size = patch.diff_summary.len() + patch.files_changed.len() * 20;

    // Deduplicate files_changed
    let mut unique_files: Vec<String> = Vec::new();
    for f in &patch.files_changed {
        if !unique_files.contains(f) { unique_files.push(f.clone()); }
    }
    if unique_files.len() < patch.files_changed.len() {
        changes_made.push(format!("Removed {} duplicate file entries", patch.files_changed.len() - unique_files.len()));
    }

    // Trim overly long summaries
    let max_summary = 500usize;
    let summary = if patch.diff_summary.len() > max_summary {
        changes_made.push(format!("Trimmed summary from {} to {} chars", patch.diff_summary.len(), max_summary));
        patch.diff_summary.chars().take(max_summary).collect::<String>() + "..."
    } else {
        patch.diff_summary.clone()
    };

    // Strip empty lines from intended content
    let content = patch.intended_content.as_ref().map(|c| {
        let stripped = c.lines()
            .filter(|l| !l.trim().is_empty() || c.lines().count() < 10)
            .collect::<Vec<_>>()
            .join("\n");
        if stripped.len() < c.len() {
            changes_made.push("Removed blank lines from patch content".to_string());
        }
        stripped
    });

    let optimized = PatchSet {
        id: format!("{}-opt", patch.id),
        files_changed: unique_files,
        diff_summary: summary,
        source: patch.source.clone(),
        timestamp: patch.timestamp,
        intended_content: content,
    };

    let new_size = optimized.diff_summary.len() + optimized.files_changed.len() * 20;
    let reduction = if original_size == 0 { 0.0 }
        else { (1.0 - new_size as f32 / original_size as f32).max(0.0) * 100.0 };

    if changes_made.is_empty() { changes_made.push("No optimizations needed".to_string()); }

    OptimizedPatch {
        original_id: patch.id.clone(),
        optimized,
        changes_made,
        size_reduction_pct: reduction,
    }
}

/// Split an oversized patch into smaller focused patches.
pub fn split_patch(patch: &PatchSet, max_files: usize) -> Vec<PatchSet> {
    if patch.files_changed.len() <= max_files { return vec![patch.clone()]; }

    patch.files_changed
        .chunks(max_files)
        .enumerate()
        .map(|(i, chunk)| PatchSet {
            id: format!("{}-split-{}", patch.id, i),
            files_changed: chunk.to_vec(),
            diff_summary: format!("[Part {}/{}] {}", i + 1,
                (patch.files_changed.len() + max_files - 1) / max_files,
                patch.diff_summary),
            source: patch.source.clone(),
            timestamp: patch.timestamp,
            intended_content: patch.intended_content.clone(),
        })
        .collect()
}

pub fn optimize_all(patches: &[PatchSet]) -> Vec<OptimizedPatch> {
    patches.iter().map(optimize_patch).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(id: &str, files: Vec<&str>) -> PatchSet {
        PatchSet { id: id.to_string(), files_changed: files.into_iter().map(String::from).collect(),
            diff_summary: "x".repeat(600), source: "test".to_string(), timestamp: 0, intended_content: None }
    }

    #[test]
    fn test_deduplication() {
        let p = patch("p1", vec!["a.rs", "b.rs", "a.rs"]);
        let opt = optimize_patch(&p);
        assert_eq!(opt.optimized.files_changed.len(), 2);
        assert!(opt.changes_made.iter().any(|c| c.contains("duplicate")));
    }

    #[test]
    fn test_summary_trimmed() {
        let p = patch("p2", vec!["a.rs"]);
        let opt = optimize_patch(&p);
        assert!(opt.optimized.diff_summary.len() <= 503); // 500 + "..."
    }

    #[test]
    fn test_split_patch() {
        let p = patch("p3", vec!["a.rs","b.rs","c.rs","d.rs","e.rs","f.rs"]);
        let parts = split_patch(&p, 3);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].files_changed.len(), 3);
    }
}
