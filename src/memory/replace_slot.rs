//! Multi-slot /replace memory implementation (Task 451+).
//!
//! This module defines the core `MemorySlot` struct and `SlotType` enum
//! as specified in docs/multi_slot_replace_memory_spec.md (Task 450).
//!
//! The design gives the agent explicit, named, bounded memory that can be
//! addressed via `/replace[plan]`, `/replace[working]`, `/replace[mem.0]`, etc.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The semantic type of a memory slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotType {
    /// High-level goals and current plan. Highest priority, rarely compacted.
    Plan,
    /// Active scratchpad / current task state.
    Working,
    /// Retrieved facts, code snippets, and context.
    Context,
    /// Recent errors, diagnostics, and failure information.
    Errors,
    /// Rolling indexed short-term memory (mem.0, mem.1, ...).
    Indexed(u32),
    /// Future extension / custom slots.
    Custom(String),
}

impl SlotType {
    /// Returns the canonical name prefix used in prompts and /replace syntax.
    pub fn as_prefix(&self) -> String {
        match self {
            SlotType::Plan => "plan".to_string(),
            SlotType::Working => "working".to_string(),
            SlotType::Context => "context".to_string(),
            SlotType::Errors => "errors".to_string(),
            SlotType::Indexed(n) => format!("mem.{}", n),
            SlotType::Custom(s) => s.clone(),
        }
    }

    /// Whether this slot type should be aggressively preserved during compaction.
    pub fn is_high_priority(&self) -> bool {
        matches!(self, SlotType::Plan | SlotType::Working)
    }
}

/// A single named, bounded memory slot.
///
/// This is the fundamental unit of the multi-slot /replace memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySlot {
    /// Canonical slot name used for addressing (e.g. "plan", "mem.0").
    pub name: String,

    /// The actual content stored in the slot.
    pub content: String,

    /// Hard token limit for this slot (approximate; enforcement is soft in v1).
    pub max_tokens: usize,

    /// Last time the slot was written to.
    pub last_updated: DateTime<Utc>,

    /// Semantic category of the slot.
    pub slot_type: SlotType,
}

impl MemorySlot {
    /// Create a new empty slot.
    pub fn new(name: impl Into<String>, slot_type: SlotType, max_tokens: usize) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            content: String::new(),
            max_tokens,
            last_updated: Utc::now(),
            slot_type,
        }
    }

    /// Create a slot with initial content.
    pub fn with_content(
        name: impl Into<String>,
        slot_type: SlotType,
        max_tokens: usize,
        content: impl Into<String>,
    ) -> Self {
        let mut slot = Self::new(name, slot_type, max_tokens);
        slot.update(content);
        slot
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Core mutation API (required by Task 451)
    // ──────────────────────────────────────────────────────────────────────────

    /// Replace the entire content of the slot and update the timestamp.
    ///
    /// Note: In a full implementation this should enforce `max_tokens`.
    /// For now we just store and let compaction handle overflow.
    pub fn update(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.last_updated = Utc::now();
    }

    /// Append content to the existing slot (useful for incremental updates).
    pub fn append(&mut self, additional: impl AsRef<str>) {
        if !self.content.is_empty() {
            self.content.push('\n');
        }
        self.content.push_str(additional.as_ref());
        self.last_updated = Utc::now();
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Introspection / utility methods
    // ──────────────────────────────────────────────────────────────────────────

    /// Rough token count estimate (≈4 characters per token).
    /// Real implementation may use a proper tokenizer later.
    pub fn token_count(&self) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        // Simple but effective heuristic used elsewhere in the codebase
        ((self.content.len() as f64) / 4.0).ceil() as usize
    }

    /// Returns true if the slot is currently over its token budget.
    pub fn is_over_budget(&self) -> bool {
        self.token_count() > self.max_tokens
    }

    /// Returns how many tokens we are over budget (0 if within limit).
    pub fn overage(&self) -> usize {
        self.token_count().saturating_sub(self.max_tokens)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Methods required by the spec (stubs for now — will be filled in 452/453)
    // ──────────────────────────────────────────────────────────────────────────

    /// Produce a condensed version of the content.
    /// Used by compaction (Task 453).
    ///
    /// This is a deterministic, heuristic-based summarizer.
    /// Target reduction: 50-80% on verbose input while keeping key meaning.
    pub fn summarize(&self) -> String {
        crate::memory::compaction::compact_content(&self.content, self.slot_type.clone(), self.max_tokens)
    }

    /// In-place compaction of this slot.
    /// Applies the context compaction algorithm (Task 453).
    pub fn compact(&mut self) {
        if !self.is_over_budget() {
            return;
        }

        let target = (self.max_tokens as f64 * 0.85) as usize; // aim for ~15% headroom
        let compacted = crate::memory::compaction::compact_content(&self.content, self.slot_type.clone(), target);

        if compacted.len() < self.content.len() {
            self.content = compacted;
            self.last_updated = Utc::now();
        }
    }

    /// Attempt to promote stable facts from this slot into OKF.
    /// Returns Some(OKF concept id) on success.
    ///
    /// Actual promotion logic lives in the OKF promotion pipeline (Task 456).
    pub fn promote_to_okf(&self) -> Option<String> {
        // Placeholder — real implementation will call okf_create
        // and return the created concept ID.
        if self.content.len() > 300 && self.token_count() > 50 {
            // Heuristic: long stable content might be worth promoting
            Some(format!("memory/{}", self.name.replace('.', "-")))
        } else {
            None
        }
    }
}

impl Default for MemorySlot {
    fn default() -> Self {
        Self::new("working", SlotType::Working, 1200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_slot_is_empty() {
        let slot = MemorySlot::new("plan", SlotType::Plan, 800);
        assert_eq!(slot.name, "plan");
        assert!(slot.content.is_empty());
        assert_eq!(slot.max_tokens, 800);
        assert_eq!(slot.slot_type, SlotType::Plan);
        assert_eq!(slot.token_count(), 0);
    }

    #[test]
    fn update_sets_content_and_timestamp() {
        let mut slot = MemorySlot::new("working", SlotType::Working, 1200);
        let before = slot.last_updated;

        std::thread::sleep(std::time::Duration::from_millis(1));
        slot.update("Current task: implement MemorySlot");

        assert_eq!(slot.content, "Current task: implement MemorySlot");
        assert!(slot.last_updated > before);
        assert!(slot.token_count() > 0);
    }

    #[test]
    fn append_works() {
        let mut slot = MemorySlot::new("context", SlotType::Context, 1500);
        slot.update("First fact.");
        slot.append("Second fact.");

        assert!(slot.content.contains("First fact."));
        assert!(slot.content.contains("Second fact."));
    }

    #[test]
    fn slot_type_prefixes() {
        assert_eq!(SlotType::Plan.as_prefix(), "plan");
        assert_eq!(SlotType::Indexed(3).as_prefix(), "mem.3");
        assert_eq!(SlotType::Custom("scratch".into()).as_prefix(), "scratch");
    }

    #[test]
    fn high_priority_slots() {
        assert!(SlotType::Plan.is_high_priority());
        assert!(SlotType::Working.is_high_priority());
        assert!(!SlotType::Errors.is_high_priority());
    }

    #[test]
    fn token_count_and_over_budget() {
        let mut slot = MemorySlot::new("test", SlotType::Working, 10);
        slot.update("This is a reasonably long piece of text that should exceed the tiny budget.");

        assert!(slot.is_over_budget());
        assert!(slot.overage() > 0);
    }

    #[test]
    fn summarize_reduces_size() {
        let mut slot = MemorySlot::new("context", SlotType::Context, 1000);
        let long_text = "x".repeat(800);
        slot.update(long_text);

        let summary = slot.summarize();
        assert!(summary.len() < slot.content.len());
    }

    #[test]
    fn promote_to_okf_heuristic() {
        let mut slot = MemorySlot::new("plan", SlotType::Plan, 800);
        slot.update("A very long and stable architectural decision that has survived many iterations and should probably be promoted to OKF.");

        let promoted = slot.promote_to_okf();
        assert!(promoted.is_some());
    }
}