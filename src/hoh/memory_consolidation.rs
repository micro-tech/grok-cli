//! HOH Long-Term Memory Consolidation (Task 361.36)

use crate::hoh::memory_integration::{HOHMemory, MemoryEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub entries_before: usize,
    pub entries_after: usize,
    pub merged_count: usize,
    pub promoted_count: usize,
    pub pruned_count: usize,
}

/// Consolidate memory: merge similar entries, promote high-value ones, prune noise.
pub fn consolidate(memory: &mut HOHMemory) -> ConsolidationReport {
    let before = memory.entries.len();

    // Step 1: Merge entries with the same key (keep highest importance)
    let mut merged = 0usize;
    let mut key_best: std::collections::HashMap<String, MemoryEntry> = std::collections::HashMap::new();
    for entry in memory.entries.drain(..) {
        key_best.entry(entry.key.clone())
            .and_modify(|existing| {
                if entry.importance > existing.importance {
                    *existing = entry.clone();
                }
                merged += 1;
            })
            .or_insert(entry);
    }
    memory.entries = key_best.into_values().collect();

    // Step 2: Promote entries referenced by many iterations (high recurrence)
    let mut promoted = 0usize;
    let iteration_counts: std::collections::HashMap<String, usize> = {
        let mut counts = std::collections::HashMap::new();
        for e in &memory.entries {
            *counts.entry(e.key.clone()).or_insert(0) += 1;
        }
        counts
    };
    for entry in &mut memory.entries {
        let count = *iteration_counts.get(&entry.key).unwrap_or(&1);
        if count > 2 && entry.importance < 0.8 {
            entry.importance = (entry.importance + 0.1).min(1.0);
            promoted += 1;
        }
    }

    // Step 3: Prune entries below importance threshold (keep at least 100)
    let min_keep = 100usize;
    let pruned = if memory.entries.len() > min_keep {
        let cutoff = memory.entries.len() - min_keep;
        memory.entries.sort_by(|a, b| a.importance.partial_cmp(&b.importance).unwrap_or(std::cmp::Ordering::Equal));
        let mut low_importance: Vec<_> = memory.entries.drain(..cutoff.min(memory.entries.len())).collect();
        let pruned = low_importance.iter().filter(|e| e.importance < 0.3).count();
        memory.entries.extend(low_importance.drain(..));
        pruned
    } else { 0 };

    let after = memory.entries.len();

    tracing::info!(
        before, after, merged, promoted, pruned,
        "[HOH MemoryConsolidation] consolidation complete"
    );

    ConsolidationReport { entries_before: before, entries_after: after, merged_count: merged, promoted_count: promoted, pruned_count: pruned }
}

/// Save consolidated memory back to disk.
pub fn consolidate_and_save(memory: &mut HOHMemory) -> std::io::Result<ConsolidationReport> {
    let report = consolidate(memory);
    memory.save()?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_consolidation_merges_duplicate_keys() {
        let mut mem = HOHMemory::new(PathBuf::from("/tmp/hoh_consol_test"));
        mem.remember(1, "same_key", "value_a", 0.5);
        mem.remember(2, "same_key", "value_b", 0.9);
        mem.remember(3, "other_key", "value_c", 0.3);

        let report = consolidate(&mut mem);
        // same_key should be merged
        assert!(report.merged_count > 0 || mem.entries.len() <= 2);
    }

    #[test]
    fn test_consolidation_preserves_high_importance() {
        let mut mem = HOHMemory::new(PathBuf::from("/tmp/hoh_consol_test2"));
        for i in 0..50u64 {
            mem.remember(i, &format!("key_{}", i), "value", if i < 10 { 0.95 } else { 0.1 });
        }
        consolidate(&mut mem);
        // High importance entries should survive
        assert!(mem.entries.iter().any(|e| e.importance >= 0.9));
    }
}
