//! Per-session context archive: persists summarized conversation chunks to disk
//! so that old history is not lost when the context window fills up.
//!
//! ## Storage layout
//! ```text
//! ~/.grok-cli/sessions/{session_id}/
//!   archives/
//!     index.json        ← lightweight chunk registry
//!     chunk_001.json    ← raw messages + AI summary
//!     chunk_002.json
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Lightweight registry entry stored in `archives/index.json`.
///
/// Contains only the data needed to list and search chunks without loading
/// each full `chunk_NNN.json` from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// Chunk identifier — matches the `NNN` in `chunk_NNN.json`.
    pub chunk_id: u32,
    /// Wall-clock time the chunk was created.
    pub created_at: DateTime<Utc>,
    /// Number of messages compressed into this chunk.
    pub message_count: usize,
    /// Estimated tokens reclaimed by replacing the raw messages with this chunk.
    pub estimated_tokens_saved: usize,
    /// First 80 chars of the AI summary, truncated with "…" if needed.
    pub summary_preview: String,
}

/// Lightweight index written to `archives/index.json`.
///
/// Loaded once on construction; updated in memory and on disk each time a
/// chunk is saved via [`ContextArchive::save_chunk`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArchiveIndex {
    /// Session this archive belongs to.
    pub session_id: String,
    /// Ordered list of chunk metadata entries.
    pub chunks: Vec<ChunkMeta>,
    /// Running total of tokens saved across all chunks.
    pub total_tokens_archived: usize,
}

/// A complete archived chunk stored at `archives/chunk_{id:03}.json`.
///
/// Includes the raw messages that were compressed, the AI-generated summary,
/// and extracted key facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub chunk_id: u32,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub message_count: usize,
    pub estimated_tokens_saved: usize,
    pub summary: String,
    pub key_facts: Vec<String>,
    pub raw_messages: Vec<serde_json::Value>,
}

impl ContextChunk {
    /// Build a [`ChunkMeta`] for the index.
    ///
    /// The `summary_preview` is the first 80 *characters* of [`summary`](Self::summary),
    /// followed by `"…"` if the summary was longer.
    pub fn meta(&self) -> ChunkMeta {
        let char_count = self.summary.chars().count();
        let preview = if char_count > 80 {
            let truncated: String = self.summary.chars().take(80).collect();
            format!("{}…", truncated)
        } else {
            self.summary.clone()
        };

        ChunkMeta {
            chunk_id: self.chunk_id,
            created_at: self.created_at,
            message_count: self.message_count,
            estimated_tokens_saved: self.estimated_tokens_saved,
            summary_preview: preview,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ContextArchive
// ─────────────────────────────────────────────────────────────────────────────

/// Manages the on-disk archive of compressed conversation chunks for a session.
///
/// Chunks are stored under `~/.grok/sessions/{session_id}/archives/`.
/// All writes are atomic (write to `.tmp`, then `rename`) so a Starlink drop
/// cannot corrupt an existing chunk or the index.
#[derive(Debug)]
pub struct ContextArchive {
    /// Absolute path to the `archives/` directory for this session.
    pub archive_dir: PathBuf,
    /// In-memory index, kept in sync with `index.json` after every save.
    pub index: ArchiveIndex,
}

impl ContextArchive {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Open (or create) the archive for a session using the default
    /// `~/.grok/sessions/{session_id}/archives/` location.
    pub fn for_session(session_id: &str) -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot determine home directory for context archive"))?;
        let sessions_dir = crate::config::grok_config_dir().join("sessions");
        Self::with_sessions_dir(session_id, &sessions_dir)
    }

    /// Open with an explicit sessions directory.
    ///
    /// Useful in tests or when the caller needs a non-default location.
    pub fn with_sessions_dir(session_id: &str, sessions_dir: &Path) -> Result<Self> {
        let archive_dir = sessions_dir.join(session_id).join("archives");
        fs::create_dir_all(&archive_dir).with_context(|| {
            format!(
                "Failed to create archive directory: {}",
                archive_dir.display()
            )
        })?;

        let index = Self::load_index(&archive_dir, session_id)?;

        Ok(Self { archive_dir, index })
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Persist `chunk` to `chunk_{id:03}.json` atomically and update `index.json`.
    ///
    /// On success the in-memory index is updated so subsequent calls to
    /// [`next_chunk_id`](Self::next_chunk_id) and [`list_chunks`](Self::list_chunks)
    /// reflect the newly saved chunk immediately.
    pub fn save_chunk(&mut self, chunk: &ContextChunk) -> Result<()> {
        let chunk_path = self
            .archive_dir
            .join(format!("chunk_{:03}.json", chunk.chunk_id));

        let json =
            serde_json::to_string_pretty(chunk).context("Failed to serialise ContextChunk")?;
        atomic_write(&chunk_path, &json)?;

        // Update in-memory index.
        let meta = chunk.meta();
        self.index.total_tokens_archived += chunk.estimated_tokens_saved;
        self.index.chunks.push(meta);

        // Persist the updated index atomically.
        let index_path = self.archive_dir.join("index.json");
        let index_json = serde_json::to_string_pretty(&self.index)
            .context("Failed to serialise ArchiveIndex")?;
        atomic_write(&index_path, &index_json)?;

        debug!(
            chunk_id = chunk.chunk_id,
            session_id = %chunk.session_id,
            tokens_saved = chunk.estimated_tokens_saved,
            "ContextArchive: chunk saved"
        );

        Ok(())
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// Load a previously saved chunk by its ID.
    ///
    /// Returns `Ok(None)` if the chunk file does not exist.
    pub fn load_chunk(&self, chunk_id: u32) -> Result<Option<ContextChunk>> {
        let chunk_path = self.archive_dir.join(format!("chunk_{:03}.json", chunk_id));

        if !chunk_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&chunk_path)
            .with_context(|| format!("Failed to read chunk file: {}", chunk_path.display()))?;

        let chunk: ContextChunk =
            serde_json::from_str(&json).context("Failed to deserialise ContextChunk")?;

        Ok(Some(chunk))
    }

    /// Returns the ordered slice of known chunk metadata from the in-memory index.
    pub fn list_chunks(&self) -> &[ChunkMeta] {
        &self.index.chunks
    }

    /// Next available chunk ID: `max(existing) + 1`, or `1` for an empty archive.
    pub fn next_chunk_id(&self) -> u32 {
        self.index
            .chunks
            .iter()
            .map(|m| m.chunk_id)
            .max()
            .map(|max_id| max_id + 1)
            .unwrap_or(1)
    }

    /// Total tokens reclaimed across all archived chunks.
    pub fn total_tokens_archived(&self) -> usize {
        self.index.total_tokens_archived
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Load `index.json` from `archive_dir`, or return a fresh default index.
    fn load_index(archive_dir: &Path, session_id: &str) -> Result<ArchiveIndex> {
        let index_path = archive_dir.join("index.json");
        if !index_path.exists() {
            return Ok(ArchiveIndex {
                session_id: session_id.to_string(),
                ..Default::default()
            });
        }

        let json = fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read index file: {}", index_path.display()))?;

        let index: ArchiveIndex =
            serde_json::from_str(&json).context("Failed to deserialise ArchiveIndex")?;

        Ok(index)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Atomic write helper
// ─────────────────────────────────────────────────────────────────────────────

/// Write `content` to `path` atomically via a sibling `.tmp` file.
///
/// Steps:
/// 1. Write to `<path>.tmp`
/// 2. `rename(<path>.tmp, <path>)` — atomic on all major platforms
///
/// This ensures that a Starlink drop or process kill cannot leave a
/// half-written file at the target path.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");

    fs::write(&tmp, content)
        .with_context(|| format!("Failed to write temp file: {}", tmp.display()))?;

    fs::rename(&tmp, path)
        .with_context(|| format!("Failed to rename {} -> {}", tmp.display(), path.display()))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Build a test chunk with the given ID and summary text.
    fn make_chunk(id: u32, summary: &str) -> ContextChunk {
        ContextChunk {
            chunk_id: id,
            session_id: "test-session".to_string(),
            created_at: Utc::now(),
            message_count: 5,
            estimated_tokens_saved: 200,
            summary: summary.to_string(),
            key_facts: vec!["fact one".to_string(), "fact two".to_string()],
            raw_messages: vec![serde_json::json!({"role": "user", "content": "hello"})],
        }
    }

    #[test]
    fn chunk_meta_preview_truncated() {
        // A summary of 100 'a' chars should be cut to 80 + "…"
        let long_summary = "a".repeat(100);
        let chunk = make_chunk(1, &long_summary);
        let meta = chunk.meta();

        assert!(
            meta.summary_preview.ends_with('…'),
            "truncated summary should end with ellipsis"
        );
        // Strip the ellipsis and check that exactly 80 ASCII bytes remain.
        let text_part = meta.summary_preview.trim_end_matches('…');
        assert_eq!(text_part.len(), 80, "truncated portion should be 80 chars");
    }

    #[test]
    fn save_and_load_chunk_roundtrip() {
        let dir = tempdir().unwrap();
        let mut archive = ContextArchive::with_sessions_dir("test-session", dir.path()).unwrap();

        let chunk = make_chunk(1, "Discussion about Rust memory safety.");
        archive.save_chunk(&chunk).unwrap();

        let loaded = archive.load_chunk(1).unwrap();
        assert!(loaded.is_some(), "chunk 1 should exist after save");
        let loaded = loaded.unwrap();

        assert_eq!(loaded.chunk_id, 1);
        assert_eq!(loaded.session_id, "test-session");
        assert_eq!(loaded.summary, "Discussion about Rust memory safety.");
        assert_eq!(loaded.key_facts.len(), 2);
        assert_eq!(loaded.message_count, 5);
        assert_eq!(loaded.estimated_tokens_saved, 200);
    }

    #[test]
    fn next_chunk_id_starts_at_one() {
        let dir = tempdir().unwrap();
        let archive = ContextArchive::with_sessions_dir("test-session", dir.path()).unwrap();
        assert_eq!(archive.next_chunk_id(), 1, "empty archive should return 1");
    }

    #[test]
    fn next_chunk_id_increments() {
        let dir = tempdir().unwrap();
        let mut archive = ContextArchive::with_sessions_dir("test-session", dir.path()).unwrap();

        let chunk = make_chunk(1, "First chunk summary.");
        archive.save_chunk(&chunk).unwrap();

        assert_eq!(
            archive.next_chunk_id(),
            2,
            "after saving chunk_id=1, next should be 2"
        );
    }

    #[test]
    fn list_chunks_empty() {
        let dir = tempdir().unwrap();
        let archive = ContextArchive::with_sessions_dir("test-session", dir.path()).unwrap();
        assert!(
            archive.list_chunks().is_empty(),
            "fresh archive should have no chunks"
        );
    }
}
