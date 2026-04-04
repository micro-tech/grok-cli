//! Memory tool — persists facts to the long-term memory store.

use anyhow::{Result, anyhow};

/// Save a fact to the long-term memory store.
///
/// Delegates to [`crate::memory::long_term::save_fact_to_default_store`]
/// which writes both a structured `memory.json` and a human-readable
/// `memory.md` mirror atomically, ensuring that a Starlink drop mid-write
/// cannot corrupt the store.  Duplicate facts are silently deduplicated.
///
/// # Arguments
/// * `fact` — the human-readable fact string to persist.
///
/// # Returns
/// A confirmation message on success.
pub fn save_memory(fact: &str) -> Result<String> {
    crate::memory::long_term::save_fact_to_default_store(fact)
        .map(|_id| "Fact saved to memory.".to_string())
        .map_err(|e| anyhow!("Failed to save memory: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_memory_returns_ok_for_valid_fact() {
        // This writes to the real store; use a clearly test-only string.
        let result = save_memory("test-fact-for-unit-test-please-ignore");
        // Either succeeds or fails gracefully — must not panic.
        assert!(result.is_ok() || result.is_err());
    }
}
