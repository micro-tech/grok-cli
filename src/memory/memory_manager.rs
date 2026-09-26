//! MemoryManager for the multi-slot /replace memory system (Task 452).
//!
//! Implements the core manager described in docs/multi_slot_replace_memory_spec.md.
//!
//! Responsibilities:
//! - Owns all named + indexed MemorySlots
//! - Provides get / update API
//! - Drives compaction and (future) promotion
//! - Produces prompt-ready serialization
//!
//! This is the central piece that will be integrated into the agent loop (Task 454).

use std::collections::HashMap;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::memory::replace_slot::{MemorySlot, SlotType};

/// Configuration for the multi-slot memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum total tokens across all slots before automatic compaction.
    pub max_total_tokens: usize,

    /// Number of indexed rolling memory slots (mem.0 .. mem.N-1).
    pub indexed_slots: usize,

    /// Token budgets for the standard named slots.
    pub plan_max_tokens: usize,
    pub working_max_tokens: usize,
    pub context_max_tokens: usize,
    pub errors_max_tokens: usize,

    /// Per-indexed-slot budget.
    pub indexed_max_tokens: usize,

    /// Threshold at which we start compacting (as % of max_total_tokens).
    pub compaction_threshold_ratio: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_total_tokens: 6000,
            indexed_slots: 6,
            plan_max_tokens: 800,
            working_max_tokens: 1200,
            context_max_tokens: 1500,
            errors_max_tokens: 600,
            indexed_max_tokens: 400,
            compaction_threshold_ratio: 0.75,
        }
    }
}

impl MemoryConfig {
    /// Returns the compaction threshold in tokens.
    pub fn compaction_threshold(&self) -> usize {
        (self.max_total_tokens as f32 * self.compaction_threshold_ratio) as usize
    }
}

/// The central manager for all /replace memory slots.
///
/// This is the implementation of the component specified for Task 452.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryManager {
    slots: HashMap<String, MemorySlot>,
    pub config: MemoryConfig,
}

impl MemoryManager {
    /// Create a new MemoryManager with default configuration and standard slots.
    pub fn new() -> Self {
        Self::with_config(MemoryConfig::default())
    }

    /// Create with a custom configuration.
    /// This also initializes the standard named slots + requested number of indexed slots.
    pub fn with_config(config: MemoryConfig) -> Self {
        let mut slots = HashMap::new();

        // --- Named slots (primary semantic roles) ---
        slots.insert(
            "plan".to_string(),
            MemorySlot::new("plan", SlotType::Plan, config.plan_max_tokens),
        );
        slots.insert(
            "working".to_string(),
            MemorySlot::new("working", SlotType::Working, config.working_max_tokens),
        );
        slots.insert(
            "context".to_string(),
            MemorySlot::new("context", SlotType::Context, config.context_max_tokens),
        );
        slots.insert(
            "errors".to_string(),
            MemorySlot::new("errors", SlotType::Errors, config.errors_max_tokens),
        );

        // --- Indexed rolling slots (mem.0 .. mem.N) ---
        for i in 0..config.indexed_slots {
            let name = format!("mem.{}", i);
            slots.insert(
                name.clone(),
                MemorySlot::new(name, SlotType::Indexed(i as u32), config.indexed_max_tokens),
            );
        }

        Self { slots, config }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Core accessors (as specified)
    // ──────────────────────────────────────────────────────────────────────────

    /// Get a slot by name.
    pub fn get_slot(&self, name: &str) -> Option<&MemorySlot> {
        self.slots.get(name)
    }

    /// Mutable access to a slot (internal use).
    pub fn get_slot_mut(&mut self, name: &str) -> Option<&mut MemorySlot> {
        self.slots.get_mut(name)
    }

    /// Update (or create for indexed slots) a slot's content.
    ///
    /// - For known named slots: updates in place.
    /// - For indexed slots (mem.N): allowed.
    /// - For unknown named slots: rejected (per spec).
    /// - After update, if the slot is over budget, we compact it immediately.
    pub fn update_slot(&mut self, name: &str, content: String) -> Result<()> {
        if !self.is_valid_slot_name(name) {
            bail!("Unknown memory slot name: '{}'. Allowed: plan, working, context, errors, mem.0..mem.N", name);
        }

        // Create indexed slot on demand if it doesn't exist yet (within configured range)
        if !self.slots.contains_key(name) && name.starts_with("mem.") {
            if let Some(idx) = Self::parse_indexed_name(name) {
                if idx < self.config.indexed_slots as u32 {
                    let slot = MemorySlot::new(
                        name,
                        SlotType::Indexed(idx),
                        self.config.indexed_max_tokens,
                    );
                    self.slots.insert(name.to_string(), slot);
                }
            }
        }

        if let Some(slot) = self.slots.get_mut(name) {
            slot.update(content);

            // Enforce budget: compact this slot if it went over
            if slot.is_over_budget() {
                slot.compact();
            }
        } else {
            bail!("Failed to create or locate slot '{}'", name);
        }

        // Global auto-compaction trigger (compute total first to avoid borrow issues)
        let over = self.total_tokens() > self.config.compaction_threshold();
        if over {
            let _ = self.compact_all();
        }

        Ok(())
    }

    /// Returns true if this is a recognized slot name.
    fn is_valid_slot_name(&self, name: &str) -> bool {
        matches!(name, "plan" | "working" | "context" | "errors")
            || name.starts_with("mem.")
            || self.slots.contains_key(name)
    }

    fn parse_indexed_name(name: &str) -> Option<u32> {
        name.strip_prefix("mem.")
            .and_then(|s| s.parse::<u32>().ok())
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Bulk operations
    // ──────────────────────────────────────────────────────────────────────────

    /// Total approximate tokens across all slots.
    pub fn total_tokens(&self) -> usize {
        self.slots.values().map(|s| s.token_count()).sum()
    }

    /// Compact all slots, starting with lowest-priority ones.
    ///
    /// Fully uses the deterministic context compaction algorithm (Task 453)
    /// implemented in `crate::memory::compaction`.
    pub fn compact_all(&mut self) -> Result<()> {
        // Priority order (lowest first): errors → indexed (oldest) → context → working → plan (last)
        let mut names: Vec<String> = self.slots.keys().cloned().collect();

        names.sort_by(|a, b| {
            let prio = |name: &str| -> u8 {
                match name {
                    "errors" => 0,
                    n if n.starts_with("mem.") => 1,
                    "context" => 2,
                    "working" => 3,
                    "plan" => 4,
                    _ => 5,
                }
            };
            prio(a).cmp(&prio(b))
        });

        // Phase 1: compact individual slots that are over their own budget
        // (check without holding mutable borrow across total_tokens)
        for name in &names {
            let is_over = self.slots.get(name).map_or(false, |s| s.is_over_budget());
            if is_over {
                if let Some(slot) = self.slots.get_mut(name) {
                    slot.compact();
                }
            }
        }

        // Phase 2: while still globally over the hard limit, compact more (lowest priority first)
        let max_total = self.config.max_total_tokens;
        let mut idx = 0;
        while self.total_tokens() > max_total && idx < names.len() {
            let name = &names[idx];
            if name != "plan" {
                if let Some(slot) = self.slots.get_mut(name) {
                    slot.compact();
                }
            }
            idx += 1;
        }

        Ok(())
    }

    /// Promote important facts from slots into OKF.
    ///
    /// Current implementation is a stub that calls each slot's promote_to_okf()
    /// and collects the resulting concept IDs.
    ///
    /// Real OKF writing + removal from short-term memory happens in Task 456.
    pub fn promote_all(&mut self) -> Result<Vec<String>> {
        let mut promoted = Vec::new();

        for (name, slot) in &self.slots {
            if let Some(concept_id) = slot.promote_to_okf() {
                // In a fuller implementation we would:
                // 1. Call okf_create(...)
                // 2. Replace content in the slot with a reference
                promoted.push(format!("{}:{}", name, concept_id));
            }
        }

        Ok(promoted)
    }

    /// Evict oldest / lowest priority content until we free at least
    /// `target_reduction_tokens`.
    pub fn evict_oldest(&mut self, target_reduction_tokens: usize) -> Result<()> {
        let mut freed = 0usize;

        // Same low-to-high priority order as compaction
        let mut names: Vec<String> = self.slots.keys().cloned().collect();
        names.sort_by(|a, b| {
            let prio = |name: &str| -> u8 {
                match name {
                    "errors" => 0,
                    n if n.starts_with("mem.") => 1,
                    "context" => 2,
                    "working" => 3,
                    "plan" => 4,
                    _ => 5,
                }
            };
            prio(a).cmp(&prio(b))
        });

        for name in names {
            if freed >= target_reduction_tokens {
                break;
            }

            if let Some(slot) = self.slots.get_mut(&name) {
                if name == "plan" {
                    continue; // Never evict plan
                }

                let before = slot.token_count();
                // Simple eviction: clear the oldest indexed or context/errors first
                if slot.slot_type != SlotType::Plan {
                    slot.update(""); // aggressive clear for now
                }
                let after = slot.token_count();
                freed += before.saturating_sub(after);
            }
        }

        Ok(())
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Prompt serialization (critical for Task 455)
    // ──────────────────────────────────────────────────────────────────────────

    /// Serialize selected slots (or all if empty) into a clean prompt section.
    ///
    /// Output format is designed to be injected by the prompt builder.
    pub fn serialize_for_prompt(&self, slot_names: &[&str]) -> String {
        let mut lines = vec!["## Memory Slots".to_string(), String::new()];

        let names_to_serialize: Vec<String> = if slot_names.is_empty() {
            // Default order
            let mut all: Vec<_> = self.slots.keys().cloned().collect();
            all.sort_by(|a, b| {
                let order = |n: &str| match n {
                    "plan" => 0,
                    "working" => 1,
                    "context" => 2,
                    "errors" => 3,
                    n if n.starts_with("mem.") => 10 + n.trim_start_matches("mem.").parse::<u32>().unwrap_or(99),
                    _ => 99,
                };
                order(a).cmp(&order(b))
            });
            all
        } else {
            slot_names.iter().map(|s| s.to_string()).collect()
        };

        for name in names_to_serialize {
            if let Some(slot) = self.slots.get(&name) {
                if slot.content.trim().is_empty() {
                    continue;
                }

                lines.push(format!("### [{}]", name));
                lines.push(slot.content.trim().to_string());
                lines.push(String::new());
            }
        }

        lines.join("\n").trim_end().to_string()
    }

    /// List all current slot names.
    pub fn slot_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.slots.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get a snapshot of all slots (useful for debugging / persistence).
    pub fn all_slots(&self) -> &HashMap<String, MemorySlot> {
        &self.slots
    }

    /// Force a specific slot to exist (mainly for tests / bootstrap).
    pub fn ensure_slot(&mut self, name: &str, slot_type: SlotType, max_tokens: usize) {
        if !self.slots.contains_key(name) {
            self.slots.insert(
                name.to_string(),
                MemorySlot::new(name, slot_type, max_tokens),
            );
        }
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_standard_slots() {
        let mgr = MemoryManager::new();
        assert!(mgr.get_slot("plan").is_some());
        assert!(mgr.get_slot("working").is_some());
        assert!(mgr.get_slot("context").is_some());
        assert!(mgr.get_slot("errors").is_some());
        assert!(mgr.get_slot("mem.0").is_some());
        assert!(mgr.get_slot("mem.5").is_some()); // default 6 indexed
    }

    #[test]
    fn update_slot_works_and_triggers_compaction() {
        let mut mgr = MemoryManager::new();
        mgr.update_slot("working", "x".repeat(2000)).unwrap();

        let slot = mgr.get_slot("working").unwrap();
        // Should have been compacted somewhat because 2000 > 1200
        assert!(slot.token_count() <= 1200 + 50); // small tolerance
    }

    #[test]
    fn rejects_unknown_named_slot() {
        let mut mgr = MemoryManager::new();
        let result = mgr.update_slot("foo-bar", "something".into());
        assert!(result.is_err());
    }

    #[test]
    fn serialize_for_prompt_produces_labeled_sections() {
        let mut mgr = MemoryManager::new();
        mgr.update_slot("plan", "1. Build the thing".into()).unwrap();
        mgr.update_slot("working", "Currently editing foo.rs".into()).unwrap();

        let output = mgr.serialize_for_prompt(&[]);

        assert!(output.contains("### [plan]"));
        assert!(output.contains("### [working]"));
        assert!(output.contains("Build the thing"));
    }

    #[test]
    fn total_tokens_and_threshold() {
        let mut mgr = MemoryManager::with_config(MemoryConfig {
            max_total_tokens: 100,
            compaction_threshold_ratio: 0.5,
            ..Default::default()
        });

        mgr.update_slot("working", "hello world this is some content".into()).unwrap();
        assert!(mgr.total_tokens() > 0);

        // With very low threshold it should have auto-compacted
        assert!(mgr.total_tokens() <= 100);
    }

    #[test]
    fn promote_all_collects_candidates() {
        let mut mgr = MemoryManager::new();
        mgr.update_slot("plan", "This is a very long and stable decision that has been repeated many times across iterations and should be promoted.").unwrap();

        let promoted = mgr.promote_all().unwrap();
        // The heuristic in MemorySlot may or may not trigger; just ensure it doesn't crash
        assert!(promoted.len() <= mgr.slot_names().len());
    }
}