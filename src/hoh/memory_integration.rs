//! HOH Memory Integration (Task 297.22)
//!
//! Persistent cross-iteration memory for HOH. Allows the outer loop to recall
//! key facts, decisions, and outcomes from previous iterations.

use crate::hoh::state::{HOHPlan, IterationState};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MEMORY_FILE: &str = ".grok/hoh/memory.json";
const MAX_ENTRIES: usize = 500;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A single persisted memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub iteration_id: u64,
    pub key: String,
    pub value: String,
    /// 0.0 (forget) to 1.0 (critical). Higher importance survives pruning.
    pub importance: f32,
    pub timestamp: u64,
}

/// HOH long-term memory store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HOHMemory {
    pub entries: Vec<MemoryEntry>,
    #[serde(skip)]
    pub project_root: PathBuf,
}

impl HOHMemory {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            entries: Vec::new(),
            project_root,
        }
    }

    fn memory_path(project_root: &Path) -> PathBuf {
        project_root.join(MEMORY_FILE)
    }

    /// Load memory from disk (falls back to empty on missing or corrupt file).
    pub fn load(project_root: &Path) -> Self {
        let path = Self::memory_path(project_root);
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<MemoryEntry>>(&s).ok())
            .unwrap_or_default();
        Self {
            entries,
            project_root: project_root.to_path_buf(),
        }
    }

    /// Persist memory to disk (pretty-printed JSON).
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::memory_path(&self.project_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&path, json)
    }

    /// Add a memory entry, pruning to MAX_ENTRIES by importance if needed.
    pub fn remember(&mut self, iteration_id: u64, key: &str, value: &str, importance: f32) {
        self.entries.push(MemoryEntry {
            iteration_id,
            key: key.to_string(),
            value: value.to_string(),
            importance: importance.clamp(0.0, 1.0),
            timestamp: now_secs(),
        });

        if self.entries.len() > MAX_ENTRIES {
            // Sort by importance desc, then timestamp desc; keep top MAX_ENTRIES
            self.entries.sort_by(|a, b| {
                b.importance
                    .partial_cmp(&a.importance)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.timestamp.cmp(&a.timestamp))
            });
            self.entries.truncate(MAX_ENTRIES);
        }
    }

    /// Recall entries whose key or value contains the query string.
    /// Results are sorted by importance descending.
    pub fn recall(&self, query: &str, top_n: usize) -> Vec<&MemoryEntry> {
        let q = query.to_lowercase();
        let mut matches: Vec<&MemoryEntry> = self
            .entries
            .iter()
            .filter(|e| {
                e.key.to_lowercase().contains(&q) || e.value.to_lowercase().contains(&q)
            })
            .collect();
        matches.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(top_n);
        matches
    }

    /// Store key facts from an iteration into memory automatically.
    pub fn ingest_iteration(&mut self, state: &IterationState) {
        let id = state.iteration_id;

        // Remember test outcome
        if let Some(eval) = state.evaluations.last() {
            if let Some(passed) = eval.test_passed {
                self.remember(
                    id,
                    &format!("iteration_{}_tests", id),
                    if passed { "passed" } else { "failed" },
                    0.7,
                );
            }
            if let Some(score) = eval.helix_score {
                self.remember(
                    id,
                    &format!("iteration_{}_helix", id),
                    &format!("{:.4}", score),
                    0.6,
                );
            }
        }

        // Remember status
        self.remember(
            id,
            &format!("iteration_{}_status", id),
            &format!("{:?}", state.status),
            0.5,
        );

        // Remember improvement proposals
        if let Some(plan) = &state.plan {
            for (i, imp) in plan.improvement_suggestions.iter().take(3).enumerate() {
                self.remember(
                    id,
                    &format!("iteration_{}_improvement_{}", id, i),
                    imp,
                    0.8,
                );
            }
        }
    }
}

/// Inject relevant memories into the iteration's planning context.
pub fn inject_into_plan(memory: &HOHMemory, state: &mut IterationState) {
    let recent = memory.recall("iteration", 5);
    if recent.is_empty() {
        return;
    }

    if state.plan.is_none() {
        state.plan = Some(HOHPlan::default());
    }

    if let Some(plan) = &mut state.plan {
        let summary: Vec<String> = recent
            .iter()
            .map(|e| format!("[iter {}] {}: {}", e.iteration_id, e.key, e.value))
            .collect();
        plan.goals.push(format!(
            "Memory context ({} entries): {}",
            recent.len(),
            summary.join(" | ")
        ));
    }

    tracing::debug!(
        iteration = state.iteration_id,
        recalled = recent.len(),
        "[HOH Memory] injected into plan"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remember_and_recall() {
        let tmp = std::env::temp_dir().join("hoh_memory_test");
        let mut mem = HOHMemory::new(tmp);
        mem.remember(1, "key_foo", "value_bar", 0.9);
        mem.remember(2, "key_baz", "value_qux", 0.5);

        let hits = mem.recall("foo", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, "key_foo");
    }

    #[test]
    fn test_inject_into_plan_adds_goal() {
        let tmp = std::env::temp_dir().join("hoh_memory_inject_test");
        let mut mem = HOHMemory::new(tmp);
        mem.remember(1, "iteration_1_status", "Completed", 0.5);

        let mut state = IterationState::new(2);
        inject_into_plan(&mem, &mut state);

        let plan = state.plan.expect("plan should be created");
        assert!(!plan.goals.is_empty());
        assert!(plan.goals.iter().any(|g| g.contains("Memory context")));
    }

    #[test]
    fn test_prune_on_exceed_max() {
        let tmp = std::env::temp_dir().join("hoh_memory_prune_test");
        let mut mem = HOHMemory::new(tmp);
        for i in 0..600u64 {
            mem.remember(i, &format!("k{}", i), "v", 0.5);
        }
        assert!(mem.entries.len() <= MAX_ENTRIES);
    }
}
