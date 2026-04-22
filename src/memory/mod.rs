//! Unified memory module for Grok CLI.
//!
//! This module provides a four-tier memory hierarchy that every AI call site
//! can use instead of the scattered `Vec<Value>`, `Vec<ConversationItem>`, and
//! bare file-append patterns that existed before.
//!
//! ## Memory tiers
//!
//! | Tier | Module | Volatile? | Storage |
//! |---|---|---|---|
//! | **Working** | [`working`] | Session-scoped | Project context files |
//! | **Short-term** | [`short_term`] | In-memory only | — |
//! | **Long-term** | [`long_term`] | Permanent | `~/.grok/memory.json` |
//! | **Episodic** | [`episodic`] | Permanent | `~/.grok/sessions/` |
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//! use grok_cli::memory::MemoryStore;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Boot a full memory store for a new chat session.
//! let mut store = MemoryStore::new_for_session(
//!     "grok-3-mini",
//!     &PathBuf::from("."),
//!     Some("You are a helpful coding assistant."),
//! )?;
//!
//! // The system prompt is automatically enriched with project context
//! // and remembered long-term facts.
//! println!("{}", store.short_term.system_prompt().unwrap_or(""));
//!
//! // Add conversation turns.
//! store.short_term.push("user",      "What is Rust?", None);
//! store.short_term.push("assistant", "A systems language.", Some(8));
//!
//! // Persist a fact to long-term memory.
//! store.remember("user prefers Rust over C++", vec![])?;
//!
//! // Archive the session when done.
//! store.save_episode(None)?;
//! # Ok(())
//! # }
//! ```

// ── Sub-modules ───────────────────────────────────────────────────────────────

pub mod episodic;
pub mod long_term;
pub mod short_term;
pub mod skill_memory;
pub mod tool_memory;
pub mod types;
pub mod working;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use episodic::EpisodicMemory;
pub use long_term::LongTermMemory;
pub use short_term::ShortTermMemory;
pub use skill_memory::{SkillActivationRecord, SkillAffinity, SkillMemory, SkillTrigger};
pub use tool_memory::{SessionToolSummary, ToolCallRecord, ToolMemory, ToolResult};
pub use types::{
    ChatMessage, EpisodeSummary, MemoryEntry, MemoryKind, MemorySource, estimate_tokens,
};
pub use working::WorkingMemory;

// ── MemoryStore ───────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;
use tracing::{debug, warn};

/// The unified memory facade for a single chat session.
///
/// `MemoryStore` wires together all four memory tiers and exposes the
/// high-level operations that command handlers and the interactive loop need:
///
/// - **Boot**: load project context (working), open the long-term store, and
///   build an enriched system prompt injected into short-term memory.
/// - **Chat**: push messages through [`short_term`] which auto-trims when
///   the context window fills up.
/// - **Remember**: persist a user- or AI-supplied fact to [`long_term`].
/// - **Archive**: save the session transcript + metadata to [`episodic`].
///
/// # Example
/// See the [module-level documentation](self) for a full example.
#[derive(Debug)]
pub struct MemoryStore {
    /// Active conversation window — auto-trimming bounded buffer.
    pub short_term: ShortTermMemory,
    /// Persistent fact store — survives across sessions.
    pub long_term: LongTermMemory,
    /// Completed session archive — read/write access.
    pub episodic: EpisodicMemory,
    /// Project context loaded from files — read-only for this session.
    pub working: WorkingMemory,

    // ── Session metadata (used when saving an episode) ────────────────────
    session_id: String,
    model: String,
    started_at: chrono::DateTime<Utc>,
    /// Per-session tool call ledger.
    pub tool_memory: tool_memory::ToolMemory,
    /// Cross-session skill usage store (opened lazily — errors are non-fatal).
    pub skill_memory: Option<skill_memory::SkillMemory>,
}

impl MemoryStore {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a fully initialised memory store for a new chat session.
    ///
    /// Steps performed:
    /// 1. Load project context from `project_dir` into [`WorkingMemory`].
    /// 2. Open (or create) the [`LongTermMemory`] store.
    /// 3. Open the [`EpisodicMemory`] archive.
    /// 4. Create an empty [`ShortTermMemory`] buffer.
    /// 5. Build an enriched system prompt by combining `base_system_prompt`,
    ///    the working context, and the top remembered long-term facts, then
    ///    push it as the system message.
    ///
    /// Returns a ready-to-use store; failures in optional steps (e.g. no
    /// context file found, empty long-term store) are silently skipped so
    /// the session always starts.
    pub fn new_for_session(
        model: &str,
        project_dir: &Path,
        base_system_prompt: Option<&str>,
    ) -> Result<Self> {
        let session_id = generate_session_id();
        let started_at = Utc::now();

        // ── Working memory ────────────────────────────────────────────────────
        let mut working = WorkingMemory::load_for_project(project_dir).unwrap_or_else(|e| {
            warn!("MemoryStore: could not load project context: {e}");
            WorkingMemory::empty()
        });

        // Discard context that fails validation (too large, empty, …).
        working.clear_if_invalid();

        // ── Long-term memory ──────────────────────────────────────────────────
        let long_term = LongTermMemory::load_or_create().unwrap_or_else(|e| {
            warn!("MemoryStore: could not open long-term store: {e}");
            // Fall back to an empty in-memory store so the session works.
            LongTermMemory::load_or_create_at(std::env::temp_dir())
                .expect("temp dir must be writable")
        });

        // ── Episodic memory ───────────────────────────────────────────────────
        let episodic = EpisodicMemory::new().unwrap_or_else(|e| {
            warn!("MemoryStore: could not open episodic store: {e}");
            EpisodicMemory::with_dir(std::env::temp_dir().join("grok_sessions_fallback"))
        });

        // ── Short-term memory + system prompt ─────────────────────────────────
        let mut short_term = ShortTermMemory::new();

        let system_prompt = build_system_prompt(base_system_prompt, &working, &long_term);
        if !system_prompt.trim().is_empty() {
            short_term.push_system(&system_prompt);
        }

        debug!(
            session_id = %session_id,
            model = %model,
            has_context = working.has_context(),
            long_term_facts = long_term.len(),
            "MemoryStore: session initialised"
        );

        // ── Tool memory ───────────────────────────────────────────────────────
        let tool_mem = tool_memory::ToolMemory::new(&session_id);

        // ── Skill memory ──────────────────────────────────────────────────────
        let skill_mem = skill_memory::SkillMemory::load_or_create()
            .ok()
            .or_else(|| {
                warn!("MemoryStore: could not open skill memory store — running without it");
                None
            });

        debug!(
            session_id = %session_id,
            model = %model,
            has_context = working.has_context(),
            long_term_facts = long_term.len(),
            "MemoryStore: session initialised"
        );

        Ok(Self {
            short_term,
            long_term,
            episodic,
            working,
            session_id,
            model: model.to_string(),
            started_at,
            tool_memory: tool_mem,
            skill_memory: skill_mem,
        })
    }

    /// Create a minimal store with only short-term memory active.
    ///
    /// Useful for unit tests and for command handlers that only need the
    /// message buffer (e.g. `grok code explain`).
    ///
    /// Each call gets its own isolated temp subdirectory so parallel test
    /// runs never collide on the same files.
    pub fn minimal() -> Self {
        let uid = uuid::Uuid::new_v4().to_string();
        let base = std::env::temp_dir().join(format!("grok_minimal_{}", &uid[..8]));
        let sessions = base.join("sessions");
        let _ = std::fs::create_dir_all(&base);
        let _ = std::fs::create_dir_all(&sessions);

        let sid = generate_session_id();
        Self {
            short_term: ShortTermMemory::new(),
            long_term: LongTermMemory::load_or_create_at(&base)
                .expect("temp dir must be writable for minimal store"),
            episodic: EpisodicMemory::with_dir(sessions),
            working: WorkingMemory::empty(),
            tool_memory: tool_memory::ToolMemory::new(&sid),
            skill_memory: None,
            session_id: sid,
            model: "unknown".to_string(),
            started_at: Utc::now(),
        }
    }

    // ── Session metadata ──────────────────────────────────────────────────────

    /// The unique ID for the current session.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// The model name used in this session.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// When this session was started.
    pub fn started_at(&self) -> chrono::DateTime<Utc> {
        self.started_at
    }

    // ── High-level operations ─────────────────────────────────────────────────

    /// Persist a fact to long-term memory.
    ///
    /// A thin convenience wrapper around [`LongTermMemory::save_fact`].
    /// Returns the UUID of the stored entry.
    ///
    /// Duplicate facts are silently deduplicated; the existing ID is returned.
    pub fn remember(&mut self, fact: &str, tags: Vec<String>) -> Result<String> {
        self.long_term.save_fact(fact, MemorySource::User, tags)
    }

    /// Persist a fact that was *inferred* by the AI (not explicitly given by
    /// the user).
    pub fn remember_inferred(&mut self, fact: &str, tags: Vec<String>) -> Result<String> {
        self.long_term.save_fact(fact, MemorySource::Inferred, tags)
    }

    /// Record a completed tool call in the session's tool ledger.
    ///
    /// This is the central place all command handlers should call after every
    /// tool execution so the AI and loop-detection layer can see what happened.
    pub fn record_tool_call(
        &mut self,
        tool_name: &str,
        args: serde_json::Value,
        result: tool_memory::ToolResult,
        duration_ms: Option<u64>,
    ) {
        self.tool_memory
            .record_call(tool_name, args, result, duration_ms);
    }

    /// Return `true` when the same `(tool, args)` pair has failed at least
    /// `threshold` times this session — used to break infinite retry loops.
    pub fn is_tool_loop(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        threshold: usize,
    ) -> bool {
        self.tool_memory.failed_recently(tool_name, args, threshold)
    }

    /// Activate a skill and record it in skill memory for the current project.
    ///
    /// `project_hash` should come from
    /// [`skill_memory::project_hash_for_path`].
    pub fn activate_skill(
        &mut self,
        skill_name: &str,
        trigger: skill_memory::SkillTrigger,
        project_hash: &str,
    ) {
        if let Some(sm) = &mut self.skill_memory {
            if let Err(e) =
                sm.record_activation(skill_name, trigger, project_hash, &self.session_id)
            {
                warn!("MemoryStore: could not record skill activation — {e}");
            }
        }
    }

    /// Record whether a skill was helpful this session.
    pub fn skill_outcome(&mut self, skill_name: &str, was_helpful: bool) {
        if let Some(sm) = &mut self.skill_memory {
            if let Err(e) = sm.record_outcome(skill_name, &self.session_id, was_helpful, None) {
                warn!("MemoryStore: could not record skill outcome — {e}");
            }
        }
    }

    /// Get suggested skills for the current project based on past history.
    ///
    /// Returns skill names ordered by affinity score descending.
    pub fn suggested_skills(&self, project_hash: &str, min_score: f32) -> Vec<String> {
        self.skill_memory
            .as_ref()
            .map(|sm| {
                sm.suggested_skills(project_hash, min_score)
                    .into_iter()
                    .map(|(n, _)| n.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Archive the current session as an episode in [`EpisodicMemory`].
    ///
    /// - `title` — optional human-readable session title. When `None` the
    ///   episode is stored without a title.
    ///
    /// The full short-term transcript is saved alongside the episode summary.
    /// Also flushes the tool call history to disk.
    pub fn save_episode(&mut self, title: Option<&str>) -> Result<PathBuf> {
        let messages: Vec<ChatMessage> = self.short_term.messages().to_vec();
        let message_count = messages.len();
        let total_tokens = self.short_term.estimated_tokens();

        let mut summary = EpisodeSummary::new(
            &self.session_id,
            &self.model,
            self.started_at,
            message_count,
            total_tokens,
        );

        summary.title = title.map(|t| t.to_string());

        if let Some(wd) = std::env::current_dir()
            .ok()
            .map(|p| p.display().to_string())
        {
            summary.working_dir = Some(wd);
        }

        let path = self.episodic.save(&summary, Some(&messages))?;

        // Flush tool call history to disk alongside the episode.
        if let Err(e) = self.tool_memory.flush_to_disk() {
            warn!("MemoryStore: could not flush tool history — {e}");
        }

        debug!(
            session_id = %self.session_id,
            message_count,
            total_tokens,
            path = %path.display(),
            tool_calls = self.tool_memory.len(),
            "MemoryStore: episode saved"
        );

        Ok(path)
    }

    /// Reload the working context from disk (honours `/reload-context`).
    ///
    /// Returns `Ok(true)` when the context changed, `Ok(false)` if it is
    /// unchanged.  When the context changes the system prompt in short-term
    /// memory is automatically rebuilt and replaced.
    pub fn reload_context(&mut self) -> Result<bool> {
        let changed = self.working.reload()?;

        if changed {
            let new_prompt = build_system_prompt(None, &self.working, &self.long_term);
            if !new_prompt.trim().is_empty() {
                self.short_term.push_system(&new_prompt);
            }
            debug!("MemoryStore: working context reloaded — system prompt updated");
        }

        Ok(changed)
    }

    /// Build and return a fresh system prompt string from the current working
    /// context and long-term facts without pushing it into short-term memory.
    ///
    /// Useful when you need the system prompt text for logging or display.
    pub fn build_system_prompt(&self) -> String {
        build_system_prompt(None, &self.working, &self.long_term)
    }

    /// Return a brief one-liner describing memory usage — suitable for the
    /// `/context` slash command or the session footer.
    ///
    /// Example output:
    /// ```text
    /// Short-term: 12 msgs (~840 tokens) | Long-term: 5 facts | Working: 1.2 KB
    /// ```
    pub fn status_line(&self) -> String {
        format!(
            "Short-term: {} msgs (~{} tokens) | Long-term: {} facts | Tools: {} calls | Working: {} bytes",
            self.short_term.len(),
            self.short_term.estimated_tokens(),
            self.long_term.len(),
            self.tool_memory.len(),
            self.working.byte_len(),
        )
    }

    /// Retrieve recent episodes for injection into the system prompt.
    ///
    /// Returns a formatted Markdown section, or an empty string when no
    /// episodes with key facts exist.
    pub fn recent_episode_context(&mut self, max_episodes: usize) -> String {
        self.episodic
            .to_prompt_context(max_episodes)
            .unwrap_or_else(|e| {
                warn!("MemoryStore: could not load episodic context: {e}");
                None
            })
            .unwrap_or_default()
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Assemble a system prompt from all available memory sources.
///
/// Order of sections:
/// 1. `base`          — caller-supplied base instruction (e.g. "You are a
///                       helpful assistant.")
/// 2. Working context — project rules, conventions, etc.
/// 3. Long-term facts — user-remembered facts, most recent first.
fn build_system_prompt(
    base: Option<&str>,
    working: &WorkingMemory,
    long_term: &LongTermMemory,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(b) = base {
        let trimmed = b.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }

    let working_section = working.to_prompt_section();
    if !working_section.trim().is_empty() {
        parts.push(working_section);
    }

    let facts_section = long_term.to_prompt_section_default();
    if !facts_section.trim().is_empty() {
        parts.push(facts_section);
    }

    parts.join("\n\n")
}

/// Generate a short random session ID.
///
/// Format: `<timestamp_secs>-<4_random_hex_bytes>`, e.g. `1714000000-a3f2b1c9`.
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Use uuid v4 for the random part so we don't need a separate rand call.
    let uid = uuid::Uuid::new_v4();
    let bytes = uid.as_bytes();
    format!(
        "{}-{:02x}{:02x}{:02x}{:02x}",
        secs, bytes[0], bytes[1], bytes[2], bytes[3]
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    fn project_with_context(content: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        let grok = dir.path().join(".grok");
        fs::create_dir(&grok).unwrap();
        fs::write(grok.join("context.md"), content).unwrap();
        dir
    }

    // ── session_id / model ────────────────────────────────────────────────────

    #[test]
    fn session_id_is_non_empty() {
        let id = generate_session_id();
        assert!(!id.is_empty());
        assert!(id.contains('-'));
    }

    #[test]
    fn minimal_store_has_correct_model() {
        let store = MemoryStore::minimal();
        assert_eq!(store.model(), "unknown");
        assert!(!store.session_id().is_empty());
    }

    // ── system prompt assembly ────────────────────────────────────────────────

    #[test]
    fn build_system_prompt_base_only() {
        let wm = WorkingMemory::empty();
        let lt = LongTermMemory::load_or_create_at(tempdir().unwrap().path()).unwrap();
        let prompt = build_system_prompt(Some("You are helpful."), &wm, &lt);
        assert_eq!(prompt, "You are helpful.");
    }

    #[test]
    fn build_system_prompt_includes_working_context() {
        let wm = WorkingMemory::from_content("# Rules\nUse Rust 2024.");
        let lt = LongTermMemory::load_or_create_at(tempdir().unwrap().path()).unwrap();
        let prompt = build_system_prompt(Some("base"), &wm, &lt);
        assert!(prompt.contains("Use Rust 2024."));
        assert!(prompt.contains("base"));
    }

    #[test]
    fn build_system_prompt_includes_long_term_facts() {
        let dir = tempdir().unwrap();
        let mut lt = LongTermMemory::load_or_create_at(dir.path()).unwrap();
        lt.save_fact("user likes dark mode", MemorySource::User, vec![])
            .unwrap();
        let wm = WorkingMemory::empty();
        let prompt = build_system_prompt(None, &wm, &lt);
        assert!(prompt.contains("dark mode"));
        assert!(prompt.contains("Remembered Facts"));
    }

    #[test]
    fn build_system_prompt_empty_when_nothing() {
        let wm = WorkingMemory::empty();
        let lt = LongTermMemory::load_or_create_at(tempdir().unwrap().path()).unwrap();
        let prompt = build_system_prompt(None, &wm, &lt);
        assert!(prompt.trim().is_empty());
    }

    // ── MemoryStore::new_for_session ──────────────────────────────────────────

    #[test]
    #[serial]
    fn new_for_session_with_context_injects_system_prompt() {
        let dir = project_with_context("# Project\nAlways use Rust.");
        let store =
            MemoryStore::new_for_session("grok-3-mini", dir.path(), Some("You are helpful."))
                .unwrap();

        let sys = store.short_term.system_prompt().unwrap_or("");
        assert!(sys.contains("You are helpful."));
        assert!(sys.contains("Always use Rust."));
    }

    #[test]
    #[serial]
    fn new_for_session_no_context_still_builds() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        // Isolate from any real ~/.grok context on the developer's machine.
        let empty_global = tempdir().unwrap();
        unsafe { std::env::set_var("GROK_GLOBAL_CONTEXT_DIR", empty_global.path()) };

        let store =
            MemoryStore::new_for_session("grok-3-mini", dir.path(), Some("base prompt")).unwrap();

        unsafe { std::env::remove_var("GROK_GLOBAL_CONTEXT_DIR") };
        assert_eq!(store.model(), "grok-3-mini");
        // System prompt should at minimum contain the base.
        let sys = store.short_term.system_prompt().unwrap_or("");
        assert!(sys.contains("base prompt"));
    }

    #[test]
    #[serial]
    fn new_for_session_no_prompt_no_context_has_no_system_message() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();

        // Point BOTH context dirs at empty temp dirs so nothing from the
        // developer's real ~/.grok installation bleeds into this assertion.
        //
        // GROK_GLOBAL_CONTEXT_DIR  — isolates WorkingMemory (context.md / memory.md)
        // GROK_LONG_TERM_MEMORY_DIR — isolates LongTermMemory (memory.json)
        //   Without this second override, LongTermMemory::load_or_create() would
        //   read ~/.grok/memory.json and inject any saved facts into the system
        //   prompt, causing the is_none() assertion to fail on a developer's machine.
        let empty_global = tempdir().unwrap();
        let empty_lt = tempdir().unwrap();
        unsafe {
            std::env::set_var("GROK_GLOBAL_CONTEXT_DIR", empty_global.path());
            std::env::set_var("GROK_LONG_TERM_MEMORY_DIR", empty_lt.path());
        }

        let store = MemoryStore::new_for_session("grok-3-mini", dir.path(), None).unwrap();

        unsafe {
            std::env::remove_var("GROK_GLOBAL_CONTEXT_DIR");
            std::env::remove_var("GROK_LONG_TERM_MEMORY_DIR");
        }
        assert!(store.short_term.system_prompt().is_none());
    }

    // ── remember ─────────────────────────────────────────────────────────────

    #[test]
    fn remember_stores_fact_in_long_term() {
        let mut store = MemoryStore::minimal();
        store.remember("prefers dark mode", vec![]).unwrap();
        assert_eq!(store.long_term.len(), 1);
        assert_eq!(store.long_term.search("dark").len(), 1);
    }

    #[test]
    fn remember_inferred_uses_inferred_source() {
        let mut store = MemoryStore::minimal();
        store
            .remember_inferred("user seems to prefer tabs", vec![])
            .unwrap();
        let facts = store.long_term.by_source(&MemorySource::Inferred);
        assert_eq!(facts.len(), 1);
    }

    // ── status_line ───────────────────────────────────────────────────────────

    #[test]
    fn status_line_contains_key_metrics() {
        let mut store = MemoryStore::minimal();
        store.short_term.push("user", "hello", Some(5));
        let line = store.status_line();
        assert!(line.contains("Short-term"));
        assert!(line.contains("Long-term"));
        assert!(line.contains("Working"));
    }

    // ── save_episode ──────────────────────────────────────────────────────────

    #[test]
    fn save_episode_creates_files_in_temp() {
        // EpisodicMemory::minimal() uses temp dirs; just check it returns Ok.
        let mut store = MemoryStore::minimal();
        store.short_term.push("user", "hello", None);
        store.short_term.push("assistant", "hi", None);
        let result = store.save_episode(Some("test session"));
        assert!(result.is_ok(), "save_episode failed: {:?}", result);
    }

    // ── build_system_prompt method ────────────────────────────────────────────

    #[test]
    fn build_system_prompt_method_reflects_current_state() {
        let mut store = MemoryStore::minimal();
        store.remember("user is a Rust expert", vec![]).unwrap();
        let prompt = store.build_system_prompt();
        // long-term facts were just added but working/base are empty
        assert!(prompt.contains("Rust expert"));
    }
}
