//! Episodic memory — a record of past conversation sessions.
//!
//! [`EpisodicMemory`] stores one [`EpisodeSummary`] per completed session in
//! `~/.grok-cli/sessions/<session_id>/episode.json`.  Full conversation transcripts
//! are stored alongside as `transcript.json` when available.
//!
//! # Design
//!
//! ```text
//!  ~/.grok-cli/ (system) / .grok/ (project)
//!  └── sessions/
//!      ├── abc123/
//!      │   ├── episode.json      ← EpisodeSummary (metadata + key facts)
//!      │   └── transcript.json   ← Vec<ChatMessage> (full conversation)
//!      └── def456/
//!          ├── episode.json
//!          └── transcript.json
//! ```
//!
//! # Starlink resilience
//!
//! All file I/O uses atomic write-then-rename so a dropped connection during a
//! save cannot corrupt an existing episode.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow}; // `anyhow` macro kept and used below (user preference: use rather than remove)
use chrono::{DateTime, Utc};
use tracing::{debug, warn};

use crate::memory::types::{ChatMessage, EpisodeSummary};

// ─────────────────────────────────────────────────────────────────────────────
// Storage paths
// ─────────────────────────────────────────────────────────────────────────────

/// File name for the metadata record inside a session directory.
const EPISODE_FILE: &str = "episode.json";

/// File name for the full conversation transcript inside a session directory.
const TRANSCRIPT_FILE: &str = "transcript.json";

// ─────────────────────────────────────────────────────────────────────────────

/// Manages the on-disk archive of completed conversation sessions.
///
/// Each session occupies its own subdirectory under `~/.grok-cli/sessions/` (system).
/// [`EpisodicMemory`] keeps a lightweight in-memory index of known episodes
/// so repeated calls to [`list`] do not re-scan the filesystem every time.
#[derive(Debug)]
pub struct EpisodicMemory {
    /// Root directory: `~/.grok-cli/sessions/` (system)
    sessions_dir: PathBuf,
    /// Cached episode index — loaded lazily on first access.
    index: Option<Vec<EpisodeSummary>>,
}

impl EpisodicMemory {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Open (or create) the episodic memory store backed by the default
    /// sessions directory (typically `~/.grok-cli/sessions/` or the platform
    /// config dir under `grok-cli/sessions`).
    ///
    /// The sessions directory is created on demand when first saving an episode.
    pub fn new() -> Result<Self> {
        // Validate we can determine a home directory (using the `anyhow!` macro
        // explicitly so the import stays live — user preference: use rather than remove).
        let _home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot determine home directory for episodic memory"))?;

        // Delegate to the canonical helper from config.
        let sessions_dir = crate::config::grok_config_dir().join("sessions");

        Ok(Self {
            sessions_dir,
            index: None,
        })
    }

    /// Open with an explicit sessions directory — useful in tests.
    pub fn with_dir(sessions_dir: PathBuf) -> Self {
        Self {
            sessions_dir,
            index: None,
        }
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Record the end of a session by persisting its [`EpisodeSummary`] and
    /// optionally its full [`ChatMessage`] transcript.
    ///
    /// Uses an atomic write-then-rename so a crash or Starlink drop mid-save
    /// cannot leave a half-written file.
    pub fn save(
        &mut self,
        summary: &EpisodeSummary,
        transcript: Option<&[ChatMessage]>,
    ) -> Result<PathBuf> {
        let session_dir = self.session_dir(&summary.session_id);
        fs::create_dir_all(&session_dir).with_context(|| {
            format!(
                "Failed to create session directory: {}",
                session_dir.display()
            )
        })?;

        // ── Write episode metadata ────────────────────────────────────────────
        let episode_json =
            serde_json::to_string_pretty(summary).context("Failed to serialise EpisodeSummary")?;
        atomic_write(&session_dir.join(EPISODE_FILE), &episode_json)?;

        // ── Write transcript (optional) ───────────────────────────────────────
        if let Some(msgs) = transcript {
            let transcript_json =
                serde_json::to_string_pretty(msgs).context("Failed to serialise transcript")?;
            atomic_write(&session_dir.join(TRANSCRIPT_FILE), &transcript_json)?;
        }

        // Invalidate the cached index so the next list() re-reads from disk.
        self.index = None;

        debug!(
            session_id = %summary.session_id,
            path = %session_dir.display(),
            "EpisodicMemory: episode saved"
        );

        Ok(session_dir)
    }

    /// Update only the metadata for an existing episode (e.g. add key_facts
    /// after AI summarisation).  A no-op if the session does not exist.
    pub fn update_summary(&mut self, summary: &EpisodeSummary) -> Result<()> {
        let episode_file = self.session_dir(&summary.session_id).join(EPISODE_FILE);
        if !episode_file.exists() {
            return Ok(());
        }

        let json = serde_json::to_string_pretty(summary)
            .context("Failed to serialise updated EpisodeSummary")?;
        atomic_write(&episode_file, &json)?;
        self.index = None;
        Ok(())
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// Load the [`EpisodeSummary`] for a specific session.
    ///
    /// Returns `None` if the session does not exist on disk.
    pub fn load(&self, session_id: &str) -> Result<Option<EpisodeSummary>> {
        let episode_file = self.session_dir(session_id).join(EPISODE_FILE);
        if !episode_file.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(&episode_file)
            .with_context(|| format!("Failed to read episode file: {}", episode_file.display()))?;

        let summary: EpisodeSummary =
            serde_json::from_str(&raw).context("Failed to deserialise EpisodeSummary")?;

        Ok(Some(summary))
    }

    /// Load the full [`ChatMessage`] transcript for a session.
    ///
    /// Returns `None` if no transcript was saved.
    pub fn load_transcript(&self, session_id: &str) -> Result<Option<Vec<ChatMessage>>> {
        let transcript_file = self.session_dir(session_id).join(TRANSCRIPT_FILE);
        if !transcript_file.exists() {
            return Ok(None);
        }

        let raw = fs::read_to_string(&transcript_file)
            .with_context(|| format!("Failed to read transcript: {}", transcript_file.display()))?;

        let msgs: Vec<ChatMessage> =
            serde_json::from_str(&raw).context("Failed to deserialise transcript")?;

        Ok(Some(msgs))
    }

    /// Return all known episode summaries sorted by `started_at` descending
    /// (most recent first).
    ///
    /// Results are cached in memory; call [`refresh`] to force a re-scan.
    pub fn list(&mut self) -> Result<Vec<EpisodeSummary>> {
        if let Some(ref cached) = self.index {
            return Ok(cached.clone());
        }
        self.refresh()
    }

    /// Force a full re-scan of the sessions directory and return the updated
    /// list sorted most-recent first.
    pub fn refresh(&mut self) -> Result<Vec<EpisodeSummary>> {
        let mut summaries = Vec::new();

        if !self.sessions_dir.exists() {
            self.index = Some(Vec::new());
            return Ok(Vec::new());
        }

        for entry in fs::read_dir(&self.sessions_dir).with_context(|| {
            format!(
                "Failed to read sessions directory: {}",
                self.sessions_dir.display()
            )
        })? {
            let entry = entry.context("Failed to read directory entry")?;
            let episode_file = entry.path().join(EPISODE_FILE);

            if !episode_file.is_file() {
                continue;
            }

            match fs::read_to_string(&episode_file)
                .ok()
                .and_then(|s| serde_json::from_str::<EpisodeSummary>(&s).ok())
            {
                Some(summary) => summaries.push(summary),
                None => warn!(
                    path = %episode_file.display(),
                    "EpisodicMemory: skipped unreadable episode file"
                ),
            }
        }

        // Most recent first.
        summaries.sort_by_key(|s| std::cmp::Reverse(s.started_at));
        self.index = Some(summaries.clone());
        Ok(summaries)
    }

    /// Return up to `n` most-recent episode summaries.
    pub fn recent(&mut self, n: usize) -> Result<Vec<EpisodeSummary>> {
        let all = self.list()?;
        Ok(all.into_iter().take(n).collect())
    }

    /// Check whether a session with the given ID has been recorded.
    pub fn exists(&self, session_id: &str) -> bool {
        self.session_dir(session_id).join(EPISODE_FILE).exists()
    }

    /// Delete the episode and transcript for a session.
    ///
    /// Returns `Ok(false)` if the session did not exist.
    pub fn delete(&mut self, session_id: &str) -> Result<bool> {
        let dir = self.session_dir(session_id);
        if !dir.exists() {
            return Ok(false);
        }
        fs::remove_dir_all(&dir)
            .with_context(|| format!("Failed to delete session directory: {}", dir.display()))?;
        self.index = None;
        Ok(true)
    }

    // ── Context injection ─────────────────────────────────────────────────────

    /// Build a short paragraph summarising recent episodes for injection into a
    /// system prompt.  Only episodes that have `key_facts` populated are
    /// included.
    ///
    /// Returns `None` if there are no usable episodes.
    pub fn to_prompt_context(&mut self, max_episodes: usize) -> Result<Option<String>> {
        let recent = self.recent(max_episodes)?;
        let with_facts: Vec<&EpisodeSummary> = recent
            .iter()
            .filter(|ep| !ep.key_facts.is_empty())
            .collect();

        if with_facts.is_empty() {
            return Ok(None);
        }

        let mut out = String::from("## Recent session context\n\n");
        for ep in with_facts {
            let title = ep.title.as_deref().unwrap_or("session");
            out.push_str(&format!(
                "**{}** ({})\n",
                title,
                ep.started_at.format("%Y-%m-%d")
            ));
            for fact in &ep.key_facts {
                out.push_str(&format!("- {}\n", fact));
            }
            out.push('\n');
        }

        Ok(Some(out))
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id)
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
fn atomic_write(path: &std::path::Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");

    fs::write(&tmp, content)
        .with_context(|| format!("Failed to write temp file: {}", tmp.display()))?;

    fs::rename(&tmp, path)
        .with_context(|| format!("Failed to rename {} → {}", tmp.display(), path.display()))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Backward-compatibility re-exports for utils/session.rs callers
// ─────────────────────────────────────────────────────────────────────────────

/// Thin wrapper: save a session using [`EpisodicMemory`].
///
/// Keeps `utils::session::save_session` working after the memory migration.
pub fn save_episode_from_session(
    session_id: &str,
    model: &str,
    started_at: DateTime<Utc>,
    message_count: usize,
    total_tokens: u32,
    transcript: Option<&[ChatMessage]>,
) -> Result<PathBuf> {
    let summary = EpisodeSummary::new(session_id, model, started_at, message_count, total_tokens);
    let mut store = EpisodicMemory::new()?;
    store.save(&summary, transcript)
}

/// Thin wrapper: list all saved session IDs.
///
/// Keeps `utils::session::list_sessions` working after the memory migration.
pub fn list_episode_ids() -> Result<Vec<String>> {
    let mut store = EpisodicMemory::new()?;
    Ok(store.list()?.into_iter().map(|ep| ep.session_id).collect())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_store() -> (EpisodicMemory, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = EpisodicMemory::with_dir(dir.path().to_path_buf());
        (store, dir)
    }

    fn episode(id: &str) -> EpisodeSummary {
        EpisodeSummary::new(id, "grok-3-mini", Utc::now(), 5, 200)
    }

    // ── Save / load round-trip ────────────────────────────────────────────────

    #[test]
    fn save_and_load_episode() {
        let (mut store, _dir) = make_store();
        let ep = episode("sess-001");
        store.save(&ep, None).unwrap();

        let loaded = store.load("sess-001").unwrap().unwrap();
        assert_eq!(loaded.session_id, "sess-001");
        assert_eq!(loaded.model, "grok-3-mini");
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let (store, _dir) = make_store();
        assert!(store.load("nope").unwrap().is_none());
    }

    // ── Transcript ────────────────────────────────────────────────────────────

    #[test]
    fn save_and_load_transcript() {
        let (mut store, _dir) = make_store();
        let ep = episode("sess-002");
        let transcript = vec![
            ChatMessage::user("hello"),
            ChatMessage::assistant("hi there"),
        ];
        store.save(&ep, Some(&transcript)).unwrap();

        let loaded = store.load_transcript("sess-002").unwrap().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].role, "user");
        assert_eq!(loaded[1].content, "hi there");
    }

    #[test]
    fn load_transcript_nonexistent_returns_none() {
        let (store, _dir) = make_store();
        assert!(store.load_transcript("nope").unwrap().is_none());
    }

    // ── List / refresh ────────────────────────────────────────────────────────

    #[test]
    fn list_returns_all_episodes() {
        let (mut store, _dir) = make_store();
        store.save(&episode("a"), None).unwrap();
        store.save(&episode("b"), None).unwrap();
        store.save(&episode("c"), None).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn list_is_sorted_most_recent_first() {
        let (mut store, _dir) = make_store();

        let mut ep1 = episode("old");
        ep1.started_at = Utc::now() - chrono::Duration::hours(2);

        let mut ep2 = episode("new");
        ep2.started_at = Utc::now();

        store.save(&ep1, None).unwrap();
        store.save(&ep2, None).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list[0].session_id, "new");
        assert_eq!(list[1].session_id, "old");
    }

    #[test]
    fn empty_sessions_dir_returns_empty_list() {
        let (mut store, _dir) = make_store();
        let list = store.list().unwrap();
        assert!(list.is_empty());
    }

    // ── recent() ─────────────────────────────────────────────────────────────

    #[test]
    fn recent_limits_count() {
        let (mut store, _dir) = make_store();
        for i in 0..5u8 {
            store.save(&episode(&format!("sess-{}", i)), None).unwrap();
        }
        let r = store.recent(3).unwrap();
        assert_eq!(r.len(), 3);
    }

    // ── exists / delete ───────────────────────────────────────────────────────

    #[test]
    fn exists_returns_true_after_save() {
        let (mut store, _dir) = make_store();
        store.save(&episode("x"), None).unwrap();
        assert!(store.exists("x"));
        assert!(!store.exists("y"));
    }

    #[test]
    fn delete_removes_session_dir() {
        let (mut store, _dir) = make_store();
        store.save(&episode("del-me"), None).unwrap();
        assert!(store.exists("del-me"));

        let removed = store.delete("del-me").unwrap();
        assert!(removed);
        assert!(!store.exists("del-me"));
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let (mut store, _dir) = make_store();
        assert!(!store.delete("nope").unwrap());
    }

    // ── update_summary ────────────────────────────────────────────────────────

    #[test]
    fn update_summary_persists_key_facts() {
        let (mut store, _dir) = make_store();
        store.save(&episode("upd"), None).unwrap();

        let mut updated = store.load("upd").unwrap().unwrap();
        updated.key_facts = vec!["fact 1".into(), "fact 2".into()];
        store.update_summary(&updated).unwrap();

        let reloaded = store.load("upd").unwrap().unwrap();
        assert_eq!(reloaded.key_facts, vec!["fact 1", "fact 2"]);
    }

    // ── to_prompt_context ─────────────────────────────────────────────────────

    #[test]
    fn prompt_context_is_none_when_no_key_facts() {
        let (mut store, _dir) = make_store();
        store.save(&episode("no-facts"), None).unwrap();
        assert!(store.to_prompt_context(5).unwrap().is_none());
    }

    #[test]
    fn prompt_context_includes_key_facts() {
        let (mut store, _dir) = make_store();
        let mut ep = episode("with-facts");
        ep.key_facts = vec!["user prefers Rust".into()];
        store.save(&ep, None).unwrap();
        // update so key_facts are persisted
        store.update_summary(&ep).unwrap();

        let ctx = store.to_prompt_context(5).unwrap();
        assert!(ctx.is_some());
        let text = ctx.unwrap();
        assert!(text.contains("user prefers Rust"));
    }

    // ── atomic_write ─────────────────────────────────────────────────────────

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        atomic_write(&path, r#"{"ok":true}"#).unwrap();
        assert!(path.exists());
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        atomic_write(&path, "v1").unwrap();
        atomic_write(&path, "v2").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "v2");
    }
}
