//! HOH Multi-Agent Patch Fusion (Task 361.37)

use crate::hoh::state::PatchSet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedPatch {
    pub id: String,
    pub contributing_patches: Vec<String>,
    pub patch: PatchSet,
    pub fusion_strategy: FusionStrategy,
    pub conflicts_resolved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FusionStrategy { Union, Intersection, Sequential, Merged }

/// Fuse multiple patches from different agents into one coherent patch.
pub fn fuse_patches(patches: &[PatchSet], strategy: FusionStrategy) -> FusedPatch {
    let ids: Vec<String> = patches.iter().map(|p| p.id.clone()).collect();
    let mut conflicts_resolved = 0usize;

    let (files, summary) = match strategy {
        FusionStrategy::Union => {
            // All files from all patches, deduplicated
            let mut all_files: Vec<String> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            for p in patches {
                for f in &p.files_changed {
                    if seen.insert(f.clone()) { all_files.push(f.clone()); }
                    else { conflicts_resolved += 1; }
                }
            }
            let summary = patches.iter().map(|p| p.diff_summary.as_str()).collect::<Vec<_>>().join(" | ");
            (all_files, format!("[Fused/Union] {}", &summary[..summary.len().min(300)]))
        }
        FusionStrategy::Intersection => {
            // Only files touched by ALL patches
            let mut common: HashSet<String> = patches.first()
                .map(|p| p.files_changed.iter().cloned().collect())
                .unwrap_or_default();
            for p in patches.iter().skip(1) {
                let set: HashSet<String> = p.files_changed.iter().cloned().collect();
                common = common.intersection(&set).cloned().collect();
            }
            conflicts_resolved = patches.iter().map(|p| p.files_changed.len()).sum::<usize>()
                .saturating_sub(common.len());
            (common.into_iter().collect(), "[Fused/Intersection] Core changes only".to_string())
        }
        FusionStrategy::Sequential => {
            // Files in order, each patch applied after the previous
            let mut files = Vec::new();
            let mut summaries = Vec::new();
            for (i, p) in patches.iter().enumerate() {
                files.extend(p.files_changed.iter().cloned());
                summaries.push(format!("[{}] {}", i + 1, p.diff_summary));
            }
            files.sort();
            files.dedup();
            (files, summaries.join(" → "))
        }
        FusionStrategy::Merged => {
            // Smart merge: union of files, combined summaries
            let mut all_files: Vec<String> = patches.iter()
                .flat_map(|p| p.files_changed.iter().cloned())
                .collect::<HashSet<_>>().into_iter().collect();
            all_files.sort();
            let summary = format!("Merged {} patches: {}",
                patches.len(),
                patches.iter().map(|p| &p.diff_summary[..p.diff_summary.len().min(80)])
                    .collect::<Vec<_>>().join("; "));
            (all_files, summary)
        }
    };

    let fused = PatchSet {
        id: format!("fused-{}", now()),
        files_changed: files,
        diff_summary: summary,
        source: "patch_fusion".to_string(),
        timestamp: now(),
        intended_content: None,
    };

    FusedPatch { id: fused.id.clone(), contributing_patches: ids, patch: fused, fusion_strategy: strategy, conflicts_resolved }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str, files: Vec<&str>, summary: &str) -> PatchSet {
        PatchSet { id: id.to_string(), files_changed: files.into_iter().map(String::from).collect(),
            diff_summary: summary.to_string(), source: "test".to_string(), timestamp: 0, intended_content: None }
    }

    #[test]
    fn test_union_fusion_includes_all_files() {
        let patches = vec![p("a", vec!["x.rs", "y.rs"], "A"), p("b", vec!["y.rs", "z.rs"], "B")];
        let fused = fuse_patches(&patches, FusionStrategy::Union);
        assert!(fused.patch.files_changed.len() == 3); // x, y, z
        assert!(fused.conflicts_resolved == 1); // y.rs duplicated
    }

    #[test]
    fn test_intersection_only_common_files() {
        let patches = vec![p("a", vec!["x.rs", "common.rs"], "A"), p("b", vec!["y.rs", "common.rs"], "B")];
        let fused = fuse_patches(&patches, FusionStrategy::Intersection);
        assert_eq!(fused.patch.files_changed, vec!["common.rs"]);
    }
}
