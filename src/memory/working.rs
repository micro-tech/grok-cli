//! Working memory — project context injection.
//!
//! [`WorkingMemory`] loads and holds the project-specific context that gets
//! injected into every system prompt.  It is a thin, typed wrapper around
//! [`crate::utils::context`] so the rest of the memory system has a clean
//! interface without duplicating any file-discovery logic.
//!
//! ## What counts as "working memory"?
//!
//! Working memory in cognitive science is the small amount of immediately
//! accessible information you hold in mind while doing a task.  For an AI
//! assistant that means:
//!
//! - The **project context file** (`.grok/context.md`, `GEMINI.md`, …) that
//!   describes conventions, architecture, and guidelines for the current repo.
//! - The **global memory file** (`~/.grok/memory.md`) that contains long-term
//!   facts surfaced at session start (see [`crate::memory::long_term`]).
//! - Any **merged context** when multiple context files are found.
//!
//! This content is read-only for the duration of a session; it does not grow
//! or shrink as the conversation progresses.  Use
//! [`crate::memory::short_term::ShortTermMemory`] for the live conversation
//! window and [`crate::memory::long_term::LongTermMemory`] for persistent
//! user-saved facts.
//!
//! # Example
//! ```rust,no_run
//! use std::path::PathBuf;
//! use grok_cli::memory::working::WorkingMemory;
//!
//! let wm = WorkingMemory::load_for_project(&PathBuf::from(".")).unwrap();
//! if let Some(ctx) = wm.as_str() {
//!     println!("Loaded {} chars of project context", ctx.len());
//! }
//! let system_section = wm.to_prompt_section();
//! ```

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::utils::context::{
    format_context_for_prompt, get_context_file_path, load_and_merge_project_context,
    load_project_context, validate_context,
};

// ─────────────────────────────────────────────────────────────────────────────

/// Project context loaded from on-disk files and held for the session.
///
/// Build one with [`WorkingMemory::load_for_project`] (single best file) or
/// [`WorkingMemory::load_and_merge`] (all context files merged).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkingMemory {
    /// The raw context text, if any was found.
    content: Option<String>,
    /// Canonical path of the primary context file that was loaded.
    source_path: Option<PathBuf>,
    /// `true` when content was assembled from multiple files.
    is_merged: bool,
    /// Project root directory used for file discovery.
    project_root: Option<PathBuf>,
}

impl WorkingMemory {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Return an empty working memory (no context file found / available).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load the single highest-priority context file found under `project_dir`.
    ///
    /// Priority order (first match wins):
    /// `GEMINI.md` → `.gemini.md` → `.claude.md` → `.zed/rules` →
    /// `.grok/context.md` → `.ai/context.md` → `CONTEXT.md` →
    /// `.gemini/context.md` → `.cursor/rules` → `AI_RULES.md` →
    /// `.grok/memory.md` → global `~/.grok/context.md`
    ///
    /// Returns `Ok(WorkingMemory::empty())` when no file is found so callers
    /// do not need to handle a separate "not found" branch.
    pub fn load_for_project<P: AsRef<Path>>(project_dir: P) -> Result<Self> {
        let dir = project_dir.as_ref();
        let source_path = get_context_file_path(dir);
        let root = Some(dir.to_path_buf());

        match load_project_context(dir)? {
            Some(raw) => {
                debug!(
                    path = ?source_path,
                    bytes = raw.len(),
                    "WorkingMemory: loaded project context"
                );
                Ok(Self {
                    content: Some(raw),
                    source_path,
                    is_merged: false,
                    project_root: root,
                })
            }
            None => {
                debug!("WorkingMemory: no context file found in {:?}", dir);
                Ok(Self {
                    content: None,
                    source_path: None,
                    is_merged: false,
                    project_root: root,
                })
            }
        }
    }

    /// Load **all** context files found under `project_dir` and merge them.
    ///
    /// Useful when you want every applicable context file (project rules,
    /// global memory, etc.) combined into a single block.  Deduplication is
    /// handled internally so the same file is never included twice.
    pub fn load_and_merge<P: AsRef<Path>>(project_dir: P) -> Result<Self> {
        let dir = project_dir.as_ref();
        let root = Some(dir.to_path_buf());

        match load_and_merge_project_context(dir)? {
            Some(raw) => {
                debug!(
                    bytes = raw.len(),
                    "WorkingMemory: merged multiple context files"
                );
                Ok(Self {
                    content: Some(raw),
                    source_path: None, // multiple sources — no single path
                    is_merged: true,
                    project_root: root,
                })
            }
            None => {
                debug!("WorkingMemory: no context files found in {:?}", dir);
                Ok(Self {
                    content: None,
                    source_path: None,
                    is_merged: false,
                    project_root: root,
                })
            }
        }
    }

    /// Construct directly from a pre-loaded string (e.g. from a unit test or
    /// in-memory template).
    pub fn from_content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            source_path: None,
            is_merged: false,
            project_root: None,
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// `true` when context content is present.
    pub fn has_context(&self) -> bool {
        self.content
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    }

    /// Raw context text, or `None` if no context was loaded.
    pub fn as_str(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Canonical path of the context file that was loaded.
    ///
    /// `None` when no file was found, or when the content was merged from
    /// multiple files.
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// `true` when the content was assembled from multiple context files.
    pub fn is_merged(&self) -> bool {
        self.is_merged
    }

    /// Estimated byte size of the loaded context.
    pub fn byte_len(&self) -> usize {
        self.content.as_ref().map(|s| s.len()).unwrap_or(0)
    }

    /// Rough token estimate (1 token ≈ 4 chars).
    pub fn estimated_tokens(&self) -> u32 {
        (self.byte_len() / 4).max(if self.has_context() { 1 } else { 0 }) as u32
    }

    // ── Prompt integration ────────────────────────────────────────────────────

    /// Format the context as a system-prompt section.
    ///
    /// Returns an empty string when no context is loaded so callers can
    /// unconditionally append it to their system prompt without a guard.
    ///
    /// Output format:
    /// ```text
    /// ## Project Context
    ///
    /// The following context has been loaded from the project:
    ///
    /// <content>
    ///
    /// ---
    /// ```
    pub fn to_prompt_section(&self) -> String {
        match &self.content {
            Some(raw) if !raw.trim().is_empty() => format_context_for_prompt(raw),
            _ => String::new(),
        }
    }

    /// Validate that the loaded context is suitable for prompt injection.
    ///
    /// Returns `Err` when the context is empty or exceeds the token limit.
    /// Use this before injecting into a system prompt if you want to surface
    /// validation errors to the user.
    pub fn validate(&self) -> Result<()> {
        match &self.content {
            Some(raw) => validate_context(raw),
            None => Err(anyhow::anyhow!("No context loaded")),
        }
    }

    /// Display string for the `/context` command or startup banner.
    ///
    /// Shows the source path (or `"merged"`) and token estimate.
    pub fn display_summary(&self) -> String {
        if !self.has_context() {
            return "No project context loaded.".to_string();
        }

        let source = if self.is_merged {
            "merged from multiple files".to_string()
        } else {
            self.source_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown source".to_string())
        };

        format!(
            "Context: {} (~{} tokens) — {}",
            self.byte_len(),
            self.estimated_tokens(),
            source,
        )
    }

    // ── Mutation ──────────────────────────────────────────────────────────────

    /// Reload context from the original project directory.
    ///
    /// Useful when the user edits their context file mid-session and wants to
    /// pick up the changes without restarting (`/reload-context`).
    ///
    /// Returns `Ok(true)` if new content was loaded, `Ok(false)` if nothing
    /// changed.
    pub fn reload(&mut self) -> Result<bool> {
        let Some(root) = self.project_root.clone() else {
            return Ok(false);
        };

        let reloaded = if self.is_merged {
            Self::load_and_merge(&root)?
        } else {
            Self::load_for_project(&root)?
        };

        let changed = reloaded.content != self.content;
        if changed {
            *self = reloaded;
        }
        Ok(changed)
    }

    /// Append extra content to the current context (e.g. injecting skill
    /// definitions or per-session rules on top of the project context).
    ///
    /// Appended text is separated from the existing content by a blank line.
    pub fn append(&mut self, extra: &str) {
        if extra.trim().is_empty() {
            return;
        }

        match &mut self.content {
            Some(existing) => {
                existing.push_str("\n\n");
                existing.push_str(extra);
            }
            None => {
                self.content = Some(extra.to_string());
            }
        }
    }

    /// Replace the entire context with new content (e.g. a synthesised
    /// summary of multiple sources).
    pub fn set_content(&mut self, content: impl Into<String>) {
        let s = content.into();
        if s.trim().is_empty() {
            self.content = None;
        } else {
            self.content = Some(s);
        }
    }

    /// Log a warning and clear the context when validation fails.
    ///
    /// Called internally when context is too large or otherwise unusable so
    /// the session can still start rather than hard-erroring.
    pub fn clear_if_invalid(&mut self) {
        if let Err(e) = self.validate() {
            warn!("WorkingMemory: context failed validation, clearing — {}", e);
            self.content = None;
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_project(content: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        // Place a .git marker so find_project_root stops here.
        fs::create_dir(dir.path().join(".git")).unwrap();
        let grok = dir.path().join(".grok");
        fs::create_dir(&grok).unwrap();
        fs::write(grok.join("context.md"), content).unwrap();
        dir
    }

    #[test]
    fn empty_has_no_context() {
        let wm = WorkingMemory::empty();
        assert!(!wm.has_context());
        assert!(wm.as_str().is_none());
        assert_eq!(wm.to_prompt_section(), "");
    }

    #[test]
    fn load_for_project_finds_grok_context() {
        let dir = make_project("# My Project\nUse Rust 2024.");
        let wm = WorkingMemory::load_for_project(dir.path()).unwrap();
        assert!(wm.has_context());
        assert!(wm.as_str().unwrap().contains("My Project"));
    }

    #[test]
    fn load_for_project_missing_returns_empty() {
        let dir = tempdir().unwrap();
        // .git marker only — no context file
        fs::create_dir(dir.path().join(".git")).unwrap();
        // Point global context dir at an empty temp dir so ~/.grok/context.md
        // or memory.md from the developer's machine doesn't bleed in.
        let empty_global = tempdir().unwrap();
        unsafe { std::env::set_var("GROK_GLOBAL_CONTEXT_DIR", empty_global.path()) };
        let wm = WorkingMemory::load_for_project(dir.path()).unwrap();
        unsafe { std::env::remove_var("GROK_GLOBAL_CONTEXT_DIR") };
        assert!(!wm.has_context());
    }

    #[test]
    fn from_content_has_context() {
        let wm = WorkingMemory::from_content("# Rules\n- Use tabs");
        assert!(wm.has_context());
        assert_eq!(wm.as_str(), Some("# Rules\n- Use tabs"));
    }

    #[test]
    fn to_prompt_section_contains_header() {
        let wm = WorkingMemory::from_content("some rules");
        let section = wm.to_prompt_section();
        assert!(section.contains("Project Context"));
        assert!(section.contains("some rules"));
    }

    #[test]
    fn to_prompt_section_empty_when_no_context() {
        let wm = WorkingMemory::empty();
        assert_eq!(wm.to_prompt_section(), "");
    }

    #[test]
    fn estimated_tokens_proportional_to_length() {
        let wm = WorkingMemory::from_content("abcd"); // 4 chars → 1 token
        assert_eq!(wm.estimated_tokens(), 1);
    }

    #[test]
    fn append_adds_content() {
        let mut wm = WorkingMemory::from_content("base");
        wm.append("extra");
        let c = wm.as_str().unwrap();
        assert!(c.contains("base"));
        assert!(c.contains("extra"));
    }

    #[test]
    fn append_to_empty_sets_content() {
        let mut wm = WorkingMemory::empty();
        wm.append("new content");
        assert!(wm.has_context());
    }

    #[test]
    fn append_blank_is_noop() {
        let mut wm = WorkingMemory::from_content("base");
        wm.append("   \n  ");
        assert_eq!(wm.as_str(), Some("base"));
    }

    #[test]
    fn set_content_replaces() {
        let mut wm = WorkingMemory::from_content("old");
        wm.set_content("new");
        assert_eq!(wm.as_str(), Some("new"));
    }

    #[test]
    fn set_content_blank_clears() {
        let mut wm = WorkingMemory::from_content("something");
        wm.set_content("   ");
        assert!(!wm.has_context());
    }

    #[test]
    fn display_summary_when_empty() {
        let wm = WorkingMemory::empty();
        assert_eq!(wm.display_summary(), "No project context loaded.");
    }

    #[test]
    fn display_summary_when_loaded() {
        let dir = make_project("# Context\nHello world context.");
        let wm = WorkingMemory::load_for_project(dir.path()).unwrap();
        let s = wm.display_summary();
        assert!(s.contains("Context:"));
        assert!(s.contains("tokens"));
    }

    #[test]
    fn validate_fails_on_empty_content() {
        let wm = WorkingMemory::empty();
        assert!(wm.validate().is_err());
    }

    #[test]
    fn clear_if_invalid_removes_empty_content() {
        let mut wm = WorkingMemory::empty();
        // validate will fail → nothing to clear, but should not panic
        wm.clear_if_invalid();
        assert!(!wm.has_context());
    }

    #[test]
    fn reload_returns_false_without_project_root() {
        let mut wm = WorkingMemory::from_content("static");
        let changed = wm.reload().unwrap();
        assert!(!changed);
    }

    #[test]
    fn validate_passes_for_real_content() {
        let wm = WorkingMemory::from_content("# Project\nThis is valid context.");
        assert!(wm.validate().is_ok());
    }
}
