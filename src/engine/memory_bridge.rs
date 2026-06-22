//! Memory bridge — reasoning engine ↔ long-term memory integration.
//!
//! [`MemoryBridge`] provides the plumbing that lets the reasoning engine read
//! from and conditionally write to [`LongTermMemory`] without depending on the
//! full `MemoryStore` facade.
//!
//! # Responsibilities
//!
//! | Method | What it does |
//! |---|---|
//! | [`MemoryBridge::relevant_facts`] | Retrieve facts from long-term memory keyed on the engine's current goal. |
//! | [`MemoryBridge::should_write_memory`] | Decide whether the engine's output is reliable enough to persist. |
//! | [`MemoryBridge::build_summary`] | Format a summary string suitable for a new memory entry. |
//!
//! See `docs/engine_architecture.md` for the full design document.

use std::collections::HashSet;

use crate::engine::state::{EngineState, ReasoningEngineState};
use crate::memory::{LongTermMemory, MemoryEntry};

// ---------------------------------------------------------------------------
// MemoryBridgeConfig
// ---------------------------------------------------------------------------

/// Configuration that governs how [`MemoryBridge`] interacts with long-term
/// memory.
///
/// All fields have safe, conservative defaults that can be overridden for
/// testing or specialised use-cases.
#[derive(Debug, Clone)]
pub struct MemoryBridgeConfig {
    /// Uncertainty level **at or above** which the engine will **not** write
    /// to memory.
    ///
    /// When `state.uncertainty >= write_uncertainty_threshold` the result is
    /// considered too speculative to be worth persisting.
    ///
    /// Default: `0.6` (write only when the engine is reasonably confident).
    pub write_uncertainty_threshold: f32,

    /// Maximum number of memory facts to return per [`MemoryBridge::relevant_facts`]
    /// call, after deduplication.
    ///
    /// Default: `5`.
    pub max_facts: usize,

    /// Minimum relevance score `[0.0, 1.0]` a [`MemoryEntry`] must have to be
    /// included in the results of [`MemoryBridge::relevant_facts`].
    ///
    /// Entries whose [`MemoryEntry::relevance`] is `None` are treated as
    /// `0.0` and will pass the default threshold of `0.0` (include all).
    ///
    /// Default: `0.0` (no filtering).
    pub min_relevance: f32,
}

impl Default for MemoryBridgeConfig {
    /// Returns the default configuration:
    ///
    /// - `write_uncertainty_threshold` = `0.6`
    /// - `max_facts` = `5`
    /// - `min_relevance` = `0.0` (include all facts)
    fn default() -> Self {
        Self {
            write_uncertainty_threshold: 0.6,
            max_facts: 5,
            min_relevance: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryBridge
// ---------------------------------------------------------------------------

/// Bridges the reasoning engine to the long-term memory system.
///
/// `MemoryBridge` provides three key capabilities:
///
/// 1. **Retrieve** relevant facts from [`LongTermMemory`] keyed on the
///    engine's current goal ([`Self::relevant_facts`]).
/// 2. **Decide** whether the engine's output should be persisted to memory
///    ([`Self::should_write_memory`]).
/// 3. **Format** a summary string suitable for writing as a new memory entry
///    ([`Self::build_summary`]).
///
/// # Example
///
/// ```rust
/// use grok_cli::engine::memory_bridge::MemoryBridge;
/// use grok_cli::engine::state::ReasoningEngineState;
///
/// let bridge = MemoryBridge::default();
/// let state  = ReasoningEngineState::new();
/// // Returns false — state is AnalyzeGoal, not Complete.
/// assert!(!bridge.should_write_memory(&state));
/// ```
pub struct MemoryBridge {
    config: MemoryBridgeConfig,
}

impl MemoryBridge {
    /// Create a new [`MemoryBridge`] with the supplied configuration.
    pub fn new(config: MemoryBridgeConfig) -> Self {
        Self { config }
    }

    /// Retrieve memory entries relevant to the engine's current goal.
    ///
    /// # Behaviour
    ///
    /// 1. Returns an empty `Vec` immediately when `state.goal` is `None`.
    /// 2. Extracts up to **five** keywords from the goal: words that are four
    ///    or more characters long, taken in order of appearance
    ///    (lower-cased).
    /// 3. For each keyword, calls [`LongTermMemory::search`] and accumulates
    ///    the returned references.
    /// 4. Deduplicates the accumulated results by [`MemoryEntry::id`] — the
    ///    first occurrence wins.
    /// 5. Applies the [`MemoryBridgeConfig::min_relevance`] filter: entries
    ///    whose [`MemoryEntry::relevance`] is `None` are treated as `0.0`.
    /// 6. Limits the final result to at most [`MemoryBridgeConfig::max_facts`]
    ///    entries.
    /// 7. Appends each returned entry's `id` to `state.memory_references`,
    ///    skipping any that are already recorded there.
    ///
    /// Returns references whose lifetimes are tied to `memory`.
    pub fn relevant_facts<'a>(
        &self,
        state: &mut ReasoningEngineState,
        memory: &'a LongTermMemory,
    ) -> Vec<&'a MemoryEntry> {
        // 1. Early return when there is no goal to derive keywords from.
        let goal = match &state.goal {
            Some(g) => g.clone(),
            None => return Vec::new(),
        };

        // 2. Extract keywords: words of >= 4 chars, up to 5.
        let keywords: Vec<String> = goal
            .split_whitespace()
            .filter(|w| w.len() >= 4)
            .take(5)
            .map(|w| w.to_lowercase())
            .collect();

        // 3 + 4. Search for each keyword and deduplicate by entry ID.
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut results: Vec<&'a MemoryEntry> = Vec::new();

        for keyword in &keywords {
            for entry in memory.search(keyword) {
                // 5. Relevance filter — None is treated as 0.0.
                let relevance = entry.relevance.unwrap_or(0.0);
                if relevance < self.config.min_relevance {
                    continue;
                }

                // Deduplicate: insert returns false when already present.
                if seen_ids.insert(entry.id.clone()) {
                    results.push(entry);
                }
            }
        }

        // 6. Cap at max_facts.
        results.truncate(self.config.max_facts);

        // 7. Record referenced IDs in the engine state.
        for entry in &results {
            if !state.memory_references.contains(&entry.id) {
                state.memory_references.push(entry.id.clone());
            }
        }

        results
    }

    /// Decide whether the engine should write a memory entry after this turn.
    ///
    /// Returns `true` only when **all three** of the following conditions hold:
    ///
    /// | Condition | Rationale |
    /// |---|---|
    /// | `state.uncertainty < write_uncertainty_threshold` | Engine is confident enough to trust the output. |
    /// | `state.state == EngineState::Complete` | The reasoning turn finished successfully. |
    /// | `state.revision_count == 0` | The plan was not revised, so the result is clean. |
    pub fn should_write_memory(&self, state: &ReasoningEngineState) -> bool {
        state.uncertainty < self.config.write_uncertainty_threshold
            && state.state == EngineState::Complete
            && state.revision_count == 0
    }

    /// Build a summary string suitable for writing to long-term memory.
    ///
    /// # Format
    ///
    /// ```text
    /// [Engine <engine_id>] Goal: <goal> | Steps: <n> | Uncertainty: <u:.2>
    /// ```
    ///
    /// Returns `None` when `state.goal` is `None` (there is nothing
    /// meaningful to summarise).
    pub fn build_summary(&self, state: &ReasoningEngineState) -> Option<String> {
        let goal = state.goal.as_deref()?;
        Some(format!(
            "[Engine {}] Goal: {} | Steps: {} | Uncertainty: {:.2}",
            state.engine_id,
            goal,
            state.plan.len(),
            state.uncertainty,
        ))
    }
}

impl Default for MemoryBridge {
    /// Returns a [`MemoryBridge`] using [`MemoryBridgeConfig::default`].
    fn default() -> Self {
        Self::new(MemoryBridgeConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{EngineState, ReasoningEngineState};
    use crate::memory::{LongTermMemory, MemorySource};
    use tempfile::tempdir;

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Build a [`ReasoningEngineState`] that satisfies all three conditions
    /// required for [`MemoryBridge::should_write_memory`] to return `true`.
    fn confident_complete_state() -> ReasoningEngineState {
        let mut s = ReasoningEngineState::new();
        // Set directly — bypassing the FSM transition guard is intentional in
        // unit tests that isolate the function under test.
        s.state = EngineState::Complete;
        s.uncertainty = 0.1;
        s.revision_count = 0;
        s
    }

    // ── should_write_memory ───────────────────────────────────────────────────

    /// High uncertainty (0.9) must prevent the bridge from recommending a
    /// memory write even when the engine state is otherwise ready.
    #[test]
    fn should_write_memory_returns_false_when_uncertain() {
        let bridge = MemoryBridge::default();
        let mut state = confident_complete_state();
        state.uncertainty = 0.9; // above the 0.6 threshold
        assert!(
            !bridge.should_write_memory(&state),
            "should not write memory when uncertainty is 0.9"
        );
    }

    /// A non-terminal engine state (e.g. `AnalyzeGoal`) must prevent a memory
    /// write even when uncertainty is low.
    #[test]
    fn should_write_memory_returns_false_when_not_complete() {
        let bridge = MemoryBridge::default();
        let mut state = confident_complete_state();
        state.state = EngineState::AnalyzeGoal;
        assert!(
            !bridge.should_write_memory(&state),
            "should not write memory when state is AnalyzeGoal"
        );
    }

    /// A revised plan (`revision_count > 0`) must prevent a memory write
    /// because the result may no longer represent a clean, single-pass
    /// reasoning outcome.
    #[test]
    fn should_write_memory_returns_false_when_revised() {
        let bridge = MemoryBridge::default();
        let mut state = confident_complete_state();
        state.revision_count = 1;
        assert!(
            !bridge.should_write_memory(&state),
            "should not write memory when revision_count is 1"
        );
    }

    /// When all three conditions are satisfied the bridge must recommend
    /// writing a memory entry.
    #[test]
    fn should_write_memory_returns_true_when_confident_complete() {
        let bridge = MemoryBridge::default();
        let state = confident_complete_state();
        assert!(
            bridge.should_write_memory(&state),
            "should write memory: uncertainty=0.1, state=Complete, revision_count=0"
        );
    }

    // ── build_summary ─────────────────────────────────────────────────────────

    /// `build_summary` must return `None` when the engine has no goal set.
    #[test]
    fn build_summary_returns_none_without_goal() {
        let bridge = MemoryBridge::default();
        let state = ReasoningEngineState::new(); // goal is None
        assert!(
            bridge.build_summary(&state).is_none(),
            "expected None when goal is None"
        );
    }

    /// The summary string must contain both the engine ID and the goal text.
    #[test]
    fn build_summary_contains_engine_id_and_goal() {
        let bridge = MemoryBridge::default();
        let mut state = ReasoningEngineState::new();
        state.goal = Some("analyse the codebase structure".to_string());

        let summary = bridge
            .build_summary(&state)
            .expect("summary should be Some when goal is set");

        assert!(
            summary.contains(&state.engine_id),
            "summary must contain engine_id; got: {summary}"
        );
        assert!(
            summary.contains("analyse the codebase structure"),
            "summary must contain the goal text; got: {summary}"
        );
    }

    // ── relevant_facts ────────────────────────────────────────────────────────

    /// When the engine state has no goal, `relevant_facts` must return an
    /// empty slice without touching the memory store.
    #[test]
    fn relevant_facts_returns_empty_without_goal() {
        let bridge = MemoryBridge::default();
        let mut state = ReasoningEngineState::new(); // goal is None

        let dir = tempdir().expect("failed to create tempdir");
        let memory =
            LongTermMemory::load_or_create_at(dir.path()).expect("failed to open memory store");

        let facts = bridge.relevant_facts(&mut state, &memory);
        assert!(
            facts.is_empty(),
            "expected empty result when goal is None, got {} entries",
            facts.len()
        );
    }

    /// After calling `relevant_facts` with a goal that matches stored facts,
    /// each returned entry's ID must appear in `state.memory_references`.
    #[test]
    fn relevant_facts_records_memory_references() {
        let bridge = MemoryBridge::default();
        let mut state = ReasoningEngineState::new();
        // The keywords extracted will be: "analyse" (7), "codebase" (8),
        // "carefully" (9) — all >= 4 chars, limit 5.
        state.goal = Some("analyse the codebase carefully".to_string());

        let dir = tempdir().expect("failed to create tempdir");
        let mut memory =
            LongTermMemory::load_or_create_at(dir.path()).expect("failed to open memory store");

        // Save a fact whose text contains "analyse" so it will be returned
        // when we search for that keyword.
        memory
            .save_fact(
                "analyse code quality regularly",
                MemorySource::User,
                vec!["engineering".to_string()],
            )
            .expect("failed to save fact");

        let facts = bridge.relevant_facts(&mut state, &memory);

        assert!(
            !facts.is_empty(),
            "expected at least one matching fact for keyword 'analyse'"
        );
        assert!(
            !state.memory_references.is_empty(),
            "memory_references should be populated after relevant_facts"
        );

        // Every returned entry's ID must be recorded in memory_references.
        for entry in &facts {
            assert!(
                state.memory_references.contains(&entry.id),
                "entry id {} not found in memory_references",
                entry.id
            );
        }
    }
}
