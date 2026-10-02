//! Multi-slot /replace memory configuration (Tasks 450-459).
//!
//! Controls the JAZ-style short-term structured working memory system:
//! plan, working, context, errors, mem.0..N

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Master switch for the entire multi-slot memory system.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Hard cap on total tokens across all memory slots before aggressive compaction.
    #[serde(default = "default_max_total_tokens")]
    pub max_total_tokens: usize,

    // Per-slot budgets (approximate tokens)
    #[serde(default = "default_plan_max")]
    pub plan_max_tokens: usize,

    #[serde(default = "default_working_max")]
    pub working_max_tokens: usize,

    #[serde(default = "default_context_max")]
    pub context_max_tokens: usize,

    #[serde(default = "default_errors_max")]
    pub errors_max_tokens: usize,

    /// Budget for each rolling indexed slot (mem.0, mem.1, ...)
    #[serde(default = "default_indexed_max")]
    pub indexed_max_tokens: usize,

    /// How many indexed rolling slots to maintain (mem.0 .. mem.N-1)
    #[serde(default = "default_indexed_slots")]
    pub indexed_slots: usize,

    /// When total_tokens > (max_total_tokens * ratio) we trigger compaction.
    #[serde(default = "default_compaction_ratio")]
    pub compaction_threshold_ratio: f32,

    /// Automatically attempt OKF promotion for stable long-lived content.
    #[serde(default = "default_true")]
    pub auto_promote_to_okf: bool,

    /// Minimum token length before a slot is considered for promotion.
    #[serde(default = "default_promote_min_tokens")]
    pub promote_min_tokens: usize,
}

fn default_true() -> bool { true }
fn default_max_total_tokens() -> usize { 6000 }
fn default_plan_max() -> usize { 800 }
fn default_working_max() -> usize { 1200 }
fn default_context_max() -> usize { 1500 }
fn default_errors_max() -> usize { 600 }
fn default_indexed_max() -> usize { 400 }
fn default_indexed_slots() -> usize { 6 }
fn default_compaction_ratio() -> f32 { 0.75 }
fn default_promote_min_tokens() -> usize { 80 }

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_total_tokens: default_max_total_tokens(),
            plan_max_tokens: default_plan_max(),
            working_max_tokens: default_working_max(),
            context_max_tokens: default_context_max(),
            errors_max_tokens: default_errors_max(),
            indexed_max_tokens: default_indexed_max(),
            indexed_slots: default_indexed_slots(),
            compaction_threshold_ratio: default_compaction_ratio(),
            auto_promote_to_okf: true,
            promote_min_tokens: default_promote_min_tokens(),
        }
    }
}

impl MemoryConfig {
    /// Returns the absolute token threshold at which compaction should start.
    pub fn compaction_threshold(&self) -> usize {
        (self.max_total_tokens as f32 * self.compaction_threshold_ratio) as usize
    }

    /// Convert this config into the in-memory MemoryManager config.
    pub fn to_memory_manager_config(&self) -> crate::memory::memory_manager::MemoryConfig {
        crate::memory::memory_manager::MemoryConfig {
            max_total_tokens: self.max_total_tokens,
            plan_max_tokens: self.plan_max_tokens,
            working_max_tokens: self.working_max_tokens,
            context_max_tokens: self.context_max_tokens,
            errors_max_tokens: self.errors_max_tokens,
            indexed_max_tokens: self.indexed_max_tokens,
            indexed_slots: self.indexed_slots,
            compaction_threshold_ratio: self.compaction_threshold_ratio,
        }
    }
}