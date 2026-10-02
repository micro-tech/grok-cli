//! HOH Conflict Resolver (Task 297.24)
//!
//! Detects overlapping patches from multiple agents or consecutive iterations
//! and applies a resolution strategy to keep the patch set coherent.

use crate::hoh::state::{IterationState, PatchSet};
use serde::{Deserialize, Serialize};

/// What kind of conflict was found.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictType {
    /// Two patches touch the same file(s).
    SameFileModified,
    /// Patches touch files that have a known dependency.
    DependencyConflict,
    /// Higher-level semantic overlap (described by a message).
    SemanticConflict(String),
    /// No conflict.
    None,
}

/// A detected conflict between two patches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchConflict {
    pub patch_a_id: String,
    pub patch_b_id: String,
    pub conflict_type: ConflictType,
    /// Files that are touched by both patches.
    pub affected_files: Vec<String>,
}

/// How to resolve a conflict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResolutionStrategy {
    KeepFirst,
    KeepLast,
    Merge,
    EscalateToHuman(String),
    Discard(String),
}

/// Record of a conflict and its chosen resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflict: PatchConflict,
    pub strategy: ResolutionStrategy,
    pub resolved: bool,
}

/// Scan a slice of patches and return all pairs that conflict.
pub fn detect_conflicts(patches: &[PatchSet]) -> Vec<PatchConflict> {
    let mut conflicts = Vec::new();
    for i in 0..patches.len() {
        for j in (i + 1)..patches.len() {
            let a = &patches[i];
            let b = &patches[j];
            let shared: Vec<String> = a
                .files_changed
                .iter()
                .filter(|f| b.files_changed.contains(f))
                .cloned()
                .collect();
            if !shared.is_empty() {
                conflicts.push(PatchConflict {
                    patch_a_id: a.id.clone(),
                    patch_b_id: b.id.clone(),
                    conflict_type: ConflictType::SameFileModified,
                    affected_files: shared,
                });
            }
        }
    }
    conflicts
}

/// Choose the best resolution strategy for a given conflict.
pub fn resolve_conflict(conflict: &PatchConflict, _iteration: u64) -> ResolutionStrategy {
    match &conflict.conflict_type {
        ConflictType::SameFileModified => ResolutionStrategy::KeepLast,
        ConflictType::DependencyConflict => {
            ResolutionStrategy::EscalateToHuman(
                "Dependency conflict requires human review".to_string(),
            )
        }
        ConflictType::SemanticConflict(_) => ResolutionStrategy::Merge,
        ConflictType::None => ResolutionStrategy::KeepFirst,
    }
}

/// Apply resolutions: remove patches that have a `Discard` strategy.
pub fn apply_resolutions(patches: &mut Vec<PatchSet>, resolutions: &[ConflictResolution]) {
    let discard_ids: Vec<&str> = resolutions
        .iter()
        .filter_map(|r| {
            if matches!(&r.strategy, ResolutionStrategy::Discard(_)) {
                Some(r.conflict.patch_a_id.as_str())
            } else {
                None
            }
        })
        .collect();

    if !discard_ids.is_empty() {
        patches.retain(|p| !discard_ids.contains(&p.id.as_str()));
    }

    for r in resolutions {
        tracing::info!(
            patch_a = r.conflict.patch_a_id,
            patch_b = r.conflict.patch_b_id,
            strategy = ?r.strategy,
            "[HOH ConflictResolver] applied resolution"
        );
    }
}

/// Detect, resolve, and apply all conflicts in an iteration's patch set.
/// Returns a log of every resolution made.
pub fn resolve_all(state: &mut IterationState) -> Vec<ConflictResolution> {
    let conflicts = detect_conflicts(&state.patches);
    if conflicts.is_empty() {
        tracing::debug!(
            iteration = state.iteration_id,
            "[HOH ConflictResolver] no conflicts"
        );
        return Vec::new();
    }

    tracing::info!(
        iteration = state.iteration_id,
        conflicts = conflicts.len(),
        "[HOH ConflictResolver] resolving conflicts"
    );

    let resolutions: Vec<ConflictResolution> = conflicts
        .into_iter()
        .map(|c| {
            let strategy = resolve_conflict(&c, state.iteration_id);
            ConflictResolution {
                conflict: c,
                strategy,
                resolved: true,
            }
        })
        .collect();

    apply_resolutions(&mut state.patches, &resolutions);
    resolutions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_patch(id: &str, files: Vec<&str>) -> PatchSet {
        PatchSet {
            id: id.to_string(),
            files_changed: files.into_iter().map(String::from).collect(),
            diff_summary: String::new(),
            source: "test".to_string(),
            timestamp: 0,
            intended_content: None,
        }
    }

    #[test]
    fn test_detect_conflicts_finds_overlap() {
        let patches = vec![
            make_patch("p1", vec!["src/a.rs", "src/b.rs"]),
            make_patch("p2", vec!["src/b.rs", "src/c.rs"]),
            make_patch("p3", vec!["src/d.rs"]),
        ];
        let conflicts = detect_conflicts(&patches);
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].affected_files.contains(&"src/b.rs".to_string()));
    }

    #[test]
    fn test_resolve_same_file_keeps_last() {
        let conflict = PatchConflict {
            patch_a_id: "p1".to_string(),
            patch_b_id: "p2".to_string(),
            conflict_type: ConflictType::SameFileModified,
            affected_files: vec!["src/a.rs".to_string()],
        };
        assert_eq!(resolve_conflict(&conflict, 1), ResolutionStrategy::KeepLast);
    }

    #[test]
    fn test_resolve_all_no_panic_on_empty() {
        let mut state = IterationState::new(1);
        let resolutions = resolve_all(&mut state);
        assert!(resolutions.is_empty());
    }
}
