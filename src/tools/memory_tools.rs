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
/// * `fact` — the human-readable fact string to persist.  Leading/trailing
///   whitespace is stripped before storage.  An empty (or whitespace-only)
///   string is rejected with an error.
///
/// # Returns
/// A confirmation message on success.
pub fn save_memory(fact: &str) -> Result<String> {
    let fact = fact.trim();
    if fact.is_empty() {
        tracing::warn!("memory_tools::save_memory: rejected — fact is empty");
        return Err(anyhow::anyhow!("save_memory: fact must not be empty"));
    }

    crate::memory::long_term::save_fact_to_default_store(fact)
        .map(|_id| "Fact saved to memory.".to_string())
        .map_err(|e| {
            tracing::warn!(error = %e, "memory_tools: save_memory failed");
            anyhow!("Failed to save memory: {}", e)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_memory_empty_fact_returns_err() {
        let result = save_memory("");
        assert!(result.is_err(), "empty fact must return Err");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must not be empty"),
            "error message must mention 'must not be empty'"
        );
    }

    #[test]
    fn save_memory_whitespace_only_returns_err() {
        let result = save_memory("   \t\n  ");
        assert!(result.is_err(), "whitespace-only fact must return Err");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must not be empty"),
            "error message must mention 'must not be empty'"
        );
    }
}
