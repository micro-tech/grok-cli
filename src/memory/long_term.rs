//! Long-term memory — persistent fact store.
//!
//! [`LongTermMemory`] replaces the bare `save_memory()` append in
//! `acp/tools.rs` with a proper, structured, queryable fact store.
//!
//! ## Storage layout
//!
//! ```text
//! ~/.grok/
//!   memory.json   ← structured store  (machine-readable, primary source)
//!   memory.md     ← human-readable mirror (regenerated on every save)
//! ```
//!
//! The JSON file is the canonical source.  The Markdown mirror exists so that
//! existing context-loading code (which scans for `memory.md`) keeps working
//! unchanged.
//!
//! ## Starlink resilience
//!
//! All disk writes go through an atomic rename pattern:
//! write to `<file>.tmp` first, then rename over the real file.  A satellite
//! drop mid-write therefore never corrupts the live store.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, warn};

use crate::memory::types::{MemoryEntry, MemorySource};

// ── Constants ────────────────────────────────────────────────────────────────

const MEMORY_JSON: &str = "memory.json";
const MEMORY_MD: &str = "memory.md";
const MAX_FACTS_IN_PROMPT: usize = 20;

// ─────────────────────────────────────────────────────────────────────────────

/// Persistent, structured long-term fact store.
///
/// # Example
/// ```rust,no_run
/// use grok_cli::memory::long_term::LongTermMemory;
/// use grok_cli::memory::types::MemorySource;
///
/// let mut mem = LongTermMemory::load_or_create().unwrap();
/// mem.save_fact("user prefers dark mode", MemorySource::User, vec![]).unwrap();
///
/// let facts = mem.search("dark");
/// println!("{} matching facts", facts.len());
///
/// let prompt_section = mem.to_prompt_section(10);
/// ```
#[derive(Debug)]
pub struct LongTermMemory {
    entries: Vec<MemoryEntry>,
    store_path: PathBuf,
    mirror_path: PathBuf,
}

impl LongTermMemory {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Load from the default location (`~/.grok/memory.json`), creating an
    /// empty store if the file does not yet exist.
    pub fn load_or_create() -> Result<Self> {
        let dir = grok_dir()?;
        Self::load_or_create_at(dir)
    }

    /// Load from an explicit directory (useful for tests).
    pub fn load_or_create_at(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let store_path = dir.join(MEMORY_JSON);
        let mirror_path = dir.join(MEMORY_MD);

        let entries = if store_path.exists() {
            load_json(&store_path)?
        } else {
            Vec::new()
        };

        debug!(
            path = %store_path.display(),
            count = entries.len(),
            "LongTermMemory: loaded"
        );

        Ok(Self {
            entries,
            store_path,
            mirror_path,
        })
    }

    // ── Mutation ─────────────────────────────────────────────────────────────

    /// Persist a new fact and flush both the JSON store and the Markdown mirror.
    ///
    /// Returns the ID of the newly created [`MemoryEntry`].
    ///
    /// Duplicate detection: if a fact with identical text already exists the
    /// call is a no-op and the existing ID is returned.
    pub fn save_fact(
        &mut self,
        fact: &str,
        source: MemorySource,
        tags: Vec<String>,
    ) -> Result<String> {
        let fact = fact.trim();

        // Deduplicate by exact text match.
        if let Some(existing) = self.entries.iter().find(|e| e.fact == fact) {
            debug!(id = %existing.id, "LongTermMemory: duplicate fact — skipping");
            return Ok(existing.id.clone());
        }

        let entry = MemoryEntry::new(fact, source).with_tags(tags);
        let id = entry.id.clone();

        self.entries.push(entry);
        self.flush()
            .context("LongTermMemory: failed to flush after save_fact")?;

        debug!(id = %id, "LongTermMemory: saved new fact");
        Ok(id)
    }

    /// Remove a fact by ID.  Returns `true` if an entry was actually removed.
    pub fn remove(&mut self, id: &str) -> Result<bool> {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        let removed = self.entries.len() < before;

        if removed {
            self.flush()
                .context("LongTermMemory: failed to flush after remove")?;
        }

        Ok(removed)
    }

    /// Replace the tags on an existing entry.  Returns `false` if not found.
    pub fn update_tags(&mut self, id: &str, tags: Vec<String>) -> Result<bool> {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.tags = tags;
            self.flush()?;
            return Ok(true);
        }
        Ok(false)
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    /// Return all stored entries in insertion order.
    pub fn all(&self) -> &[MemoryEntry] {
        &self.entries
    }

    /// Return entries whose fact text or tags contain `query` (case-insensitive
    /// substring match).  Results are sorted by recency (newest first).
    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let q = query.to_lowercase();
        let mut results: Vec<&MemoryEntry> = self
            .entries
            .iter()
            .filter(|e| {
                e.fact.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect();

        // Newest first.
        results.sort_by_key(|e| std::cmp::Reverse(e.created_at));
        results
    }

    /// Return entries that match **all** of the supplied tags.
    pub fn by_tags(&self, tags: &[&str]) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| {
                tags.iter()
                    .all(|t| e.tags.iter().any(|et| et.eq_ignore_ascii_case(t)))
            })
            .collect()
    }

    /// Return entries from a specific source.
    pub fn by_source(&self, source: &MemorySource) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| &e.source == source)
            .collect()
    }

    /// Number of stored facts.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    // ── Prompt injection ─────────────────────────────────────────────────────

    /// Build a Markdown section suitable for injection into a system prompt.
    ///
    /// At most `max_facts` entries are included; most-recently-created entries
    /// are preferred.  Returns an empty string when there are no facts.
    pub fn to_prompt_section(&self, max_facts: usize) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let limit = max_facts.min(MAX_FACTS_IN_PROMPT);

        // Most recent first for the prompt.
        let mut sorted: Vec<&MemoryEntry> = self.entries.iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.created_at));

        let lines: Vec<String> = sorted
            .iter()
            .take(limit)
            .map(|e| e.to_prompt_line())
            .collect();

        format!("\n\n## Remembered Facts\n\n{}\n", lines.join("\n"))
    }

    /// Convenience wrapper using the default [`MAX_FACTS_IN_PROMPT`] limit.
    pub fn to_prompt_section_default(&self) -> String {
        self.to_prompt_section(MAX_FACTS_IN_PROMPT)
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Atomically flush the canonical JSON store and regenerate the Markdown
    /// mirror.  Uses a `.tmp` file + rename so a mid-write crash or Starlink
    /// drop never corrupts the live store.
    fn flush(&self) -> Result<()> {
        // Ensure the parent directory exists.
        if let Some(parent) = self.store_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }

        // ── Write JSON ───────────────────────────────────────────────────────
        atomic_write(
            &self.store_path,
            &serde_json::to_string_pretty(&self.entries)
                .context("serialising memory entries to JSON")?,
        )
        .context("writing memory.json")?;

        // ── Write Markdown mirror ─────────────────────────────────────────────
        atomic_write(&self.mirror_path, &self.to_markdown()).context("writing memory.md")?;

        debug!(
            json  = %self.store_path.display(),
            md    = %self.mirror_path.display(),
            count = self.entries.len(),
            "LongTermMemory: flushed"
        );

        Ok(())
    }

    /// Render all entries as a Markdown bullet list.
    fn to_markdown(&self) -> String {
        if self.entries.is_empty() {
            return "# Grok Memory\n\n*(no facts saved yet)*\n".to_string();
        }

        let mut lines = vec!["# Grok Memory\n".to_string()];

        // Group by source for readability.
        let mut user_facts: Vec<&MemoryEntry> = Vec::new();
        let mut inferred: Vec<&MemoryEntry> = Vec::new();
        let mut system: Vec<&MemoryEntry> = Vec::new();

        for e in &self.entries {
            match e.source {
                MemorySource::User => user_facts.push(e),
                MemorySource::Inferred => inferred.push(e),
                MemorySource::System => system.push(e),
            }
        }

        if !user_facts.is_empty() {
            lines.push("## User Facts\n".to_string());
            for e in &user_facts {
                lines.push(format!(
                    "- {} *({})*",
                    e.fact,
                    e.created_at.format("%Y-%m-%d")
                ));
            }
            lines.push(String::new());
        }

        if !inferred.is_empty() {
            lines.push("## Inferred Facts\n".to_string());
            for e in &inferred {
                lines.push(format!("- {}", e.fact));
            }
            lines.push(String::new());
        }

        if !system.is_empty() {
            lines.push("## System Context\n".to_string());
            for e in &system {
                lines.push(format!("- {}", e.fact));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

// ── Free-function helper kept for backward-compatibility with acp/tools.rs ───

/// Save a single fact to the default long-term store.
///
/// This is the new implementation backing the `save_memory` tool in
/// `acp/tools.rs`.  Callers that used the old append-only version can switch
/// to this without any API changes on their side.
pub fn save_fact_to_default_store(fact: &str) -> Result<String> {
    let mut mem = LongTermMemory::load_or_create()?;
    mem.save_fact(fact, MemorySource::User, Vec::new())
}

/// Load all facts from the default store as a prompt-ready Markdown section.
pub fn load_prompt_section() -> String {
    match LongTermMemory::load_or_create() {
        Ok(mem) => mem.to_prompt_section_default(),
        Err(e) => {
            warn!("LongTermMemory: could not load for prompt injection: {e}");
            String::new()
        }
    }
}

// ── Private file-system helpers ───────────────────────────────────────────────

fn grok_dir() -> Result<PathBuf> {
    // Allow tests (and future CLI flag) to redirect the global context dir.
    if let Ok(dir) = std::env::var("GROK_GLOBAL_CONTEXT_DIR") {
        let path = PathBuf::from(dir);
        std::fs::create_dir_all(&path)?;
        return Ok(path);
    }
    dirs::home_dir()
        .map(|h| h.join(".grok"))
        .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))
}

/// Deserialise `Vec<MemoryEntry>` from a JSON file.
fn load_json(path: &Path) -> Result<Vec<MemoryEntry>> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    // Gracefully handle an empty file — return an empty vec instead of a
    // parse error so a zero-byte file (e.g. from a partial write) is safe.
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&raw).with_context(|| format!("parsing JSON from {}", path.display()))
}

/// Write `content` to `path` atomically via a `.tmp` sibling file.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");

    fs::write(&tmp, content).with_context(|| format!("writing tmp file {}", tmp.display()))?;

    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn mem(dir: &Path) -> LongTermMemory {
        LongTermMemory::load_or_create_at(dir).unwrap()
    }

    // ── save / load round-trip ────────────────────────────────────────────────

    #[test]
    fn save_and_reload_persists_facts() {
        let dir = tempdir().unwrap();
        {
            let mut m = mem(dir.path());
            m.save_fact("use dark mode", MemorySource::User, vec!["ui".into()])
                .unwrap();
            m.save_fact("prefers Rust", MemorySource::User, vec![])
                .unwrap();
        }

        let m2 = mem(dir.path());
        assert_eq!(m2.len(), 2);
        assert!(m2.all().iter().any(|e| e.fact == "use dark mode"));
    }

    #[test]
    fn empty_store_returns_zero_len() {
        let dir = tempdir().unwrap();
        let m = mem(dir.path());
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
    }

    // ── deduplication ────────────────────────────────────────────────────────

    #[test]
    fn duplicate_fact_is_not_added() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        let id1 = m
            .save_fact("same fact", MemorySource::User, vec![])
            .unwrap();
        let id2 = m
            .save_fact("same fact", MemorySource::User, vec![])
            .unwrap();
        assert_eq!(id1, id2);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn whitespace_is_trimmed_before_dedup() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        let id1 = m
            .save_fact("  same fact  ", MemorySource::User, vec![])
            .unwrap();
        let id2 = m
            .save_fact("same fact", MemorySource::User, vec![])
            .unwrap();
        assert_eq!(id1, id2);
        assert_eq!(m.len(), 1);
    }

    // ── search ────────────────────────────────────────────────────────────────

    #[test]
    fn search_matches_fact_text() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact("user likes dark mode", MemorySource::User, vec![])
            .unwrap();
        m.save_fact("project uses Rust", MemorySource::User, vec![])
            .unwrap();

        let results = m.search("dark");
        assert_eq!(results.len(), 1);
        assert!(results[0].fact.contains("dark"));
    }

    #[test]
    fn search_is_case_insensitive() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact("User likes DARK mode", MemorySource::User, vec![])
            .unwrap();

        assert_eq!(m.search("dark").len(), 1);
        assert_eq!(m.search("DARK").len(), 1);
        assert_eq!(m.search("Dark").len(), 1);
    }

    #[test]
    fn search_matches_tags() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact(
            "uses spaces not tabs",
            MemorySource::User,
            vec!["style".into()],
        )
        .unwrap();

        assert_eq!(m.search("style").len(), 1);
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact("something unrelated", MemorySource::User, vec![])
            .unwrap();
        assert!(m.search("nomatch_xyz").is_empty());
    }

    // ── by_tags ───────────────────────────────────────────────────────────────

    #[test]
    fn by_tags_matches_all_supplied_tags() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact(
            "uses rust and tabs",
            MemorySource::User,
            vec!["rust".into(), "style".into()],
        )
        .unwrap();
        m.save_fact("uses rust only", MemorySource::User, vec!["rust".into()])
            .unwrap();

        let both = m.by_tags(&["rust", "style"]);
        assert_eq!(both.len(), 1);
        assert!(both[0].fact.contains("and tabs"));
    }

    // ── remove ────────────────────────────────────────────────────────────────

    #[test]
    fn remove_deletes_fact_by_id() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        let id = m
            .save_fact("to be removed", MemorySource::User, vec![])
            .unwrap();

        assert!(m.remove(&id).unwrap());
        assert_eq!(m.len(), 0);

        // Should not be in a freshly-loaded store either.
        let m2 = mem(dir.path());
        assert_eq!(m2.len(), 0);
    }

    #[test]
    fn remove_returns_false_for_unknown_id() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        assert!(!m.remove("does-not-exist").unwrap());
    }

    // ── update_tags ───────────────────────────────────────────────────────────

    #[test]
    fn update_tags_replaces_existing_tags() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        let id = m
            .save_fact("some fact", MemorySource::User, vec!["old".into()])
            .unwrap();

        assert!(m.update_tags(&id, vec!["new".into()]).unwrap());
        let entry = m.all().iter().find(|e| e.id == id).unwrap();
        assert_eq!(entry.tags, vec!["new".to_string()]);
    }

    // ── prompt section ────────────────────────────────────────────────────────

    #[test]
    fn to_prompt_section_empty_when_no_facts() {
        let dir = tempdir().unwrap();
        let m = mem(dir.path());
        assert!(m.to_prompt_section_default().is_empty());
    }

    #[test]
    fn to_prompt_section_contains_facts() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact("dark mode preferred", MemorySource::User, vec![])
            .unwrap();
        let section = m.to_prompt_section_default();
        assert!(section.contains("dark mode preferred"));
        assert!(section.contains("Remembered Facts"));
    }

    #[test]
    fn to_prompt_section_respects_limit() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        for i in 0..25 {
            m.save_fact(&format!("fact number {}", i), MemorySource::User, vec![])
                .unwrap();
        }
        // Default limit is MAX_FACTS_IN_PROMPT = 20
        let section = m.to_prompt_section_default();
        let bullet_count = section.matches("- fact number").count();
        assert!(bullet_count <= MAX_FACTS_IN_PROMPT);
    }

    // ── atomic write ─────────────────────────────────────────────────────────

    #[test]
    fn json_and_md_files_are_both_created() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact("test fact", MemorySource::User, vec![])
            .unwrap();

        assert!(dir.path().join(MEMORY_JSON).exists());
        assert!(dir.path().join(MEMORY_MD).exists());
    }

    #[test]
    fn markdown_mirror_is_human_readable() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact("my test fact", MemorySource::User, vec![])
            .unwrap();

        let md = fs::read_to_string(dir.path().join(MEMORY_MD)).unwrap();
        assert!(md.contains("my test fact"));
        assert!(md.contains("# Grok Memory"));
    }

    // ── by_source ─────────────────────────────────────────────────────────────

    #[test]
    fn by_source_filters_correctly() {
        let dir = tempdir().unwrap();
        let mut m = mem(dir.path());
        m.save_fact("user fact", MemorySource::User, vec![])
            .unwrap();
        m.save_fact("inferred fact", MemorySource::Inferred, vec![])
            .unwrap();

        assert_eq!(m.by_source(&MemorySource::User).len(), 1);
        assert_eq!(m.by_source(&MemorySource::Inferred).len(), 1);
        assert_eq!(m.by_source(&MemorySource::System).len(), 0);
    }
}
