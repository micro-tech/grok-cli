//! Tool memory — session-scoped tool-call ledger.
//!
//! [`ToolMemory`] records every tool invocation that happens during a chat
//! session so that the AI and the loop-detection layer can reason about:
//!
//! - **What happened** — full audit trail of tool calls, arguments, results,
//!   and wall-clock durations.
//! - **What failed** — detect when the same `(tool, args)` pair keeps failing
//!   so the caller can break infinite retry loops before they burn tokens.
//! - **What to tell the model** — `to_prompt_section()` serialises the recent
//!   call history into a compact Markdown block for system-prompt injection.
//!
//! ## Persistence
//!
//! Tool calls are **in-memory only** for the current session by default.
//! Call [`ToolMemory::flush_to_disk`] to append a session summary to
//! `~/.grok/tool_history.json` when the session ends.  This persistent log
//! is append-only and never blocks the hot path.
//!
//! ## Starlink resilience
//!
//! `flush_to_disk` uses an atomic write-then-rename pattern so a connection
//! drop mid-write never corrupts the on-disk history.
//!
//! # Example
//!
//! ```rust,no_run
//! use grok_cli::memory::tool_memory::{ToolMemory, ToolResult};
//! use serde_json::json;
//!
//! let mut tm = ToolMemory::new("session-42");
//!
//! tm.record_call("read_file", json!({"path": "src/main.rs"}),
//!     ToolResult::Success("fn main() { }".into()), Some(12));
//!
//! tm.record_call("shell", json!({"command": "cargo build"}),
//!     ToolResult::Error("linker not found".into()), Some(340));
//!
//! // Check before retrying a failing call
//! if tm.failed_recently("shell", &json!({"command": "cargo build"}), 3) {
//!     eprintln!("Same shell call failed 3 times — stopping");
//! }
//!
//! println!("{}", tm.to_prompt_section(5));
//! ```

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of recent calls shown in the prompt section.
const MAX_PROMPT_CALLS: usize = 10;

/// Filename for the persistent append-only tool history.
const TOOL_HISTORY_FILE: &str = "tool_history.json";

// ─────────────────────────────────────────────────────────────────────────────

/// The outcome of a single tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum ToolResult {
    /// The tool ran and returned a (possibly truncated) output string.
    Success(String),
    /// The tool ran but returned an error message.
    Error(String),
    /// The user denied permission before the tool ran.
    Denied,
    /// The tool call timed out.
    Timeout,
}

impl ToolResult {
    /// `true` when this is not a [`ToolResult::Success`].
    pub fn is_failure(&self) -> bool {
        !matches!(self, ToolResult::Success(_))
    }

    /// Short display label used in the prompt section.
    pub fn label(&self) -> &'static str {
        match self {
            ToolResult::Success(_) => "✓",
            ToolResult::Error(_) => "✗",
            ToolResult::Denied => "⊘",
            ToolResult::Timeout => "⏱",
        }
    }

    /// The output text if successful, `None` otherwise.
    pub fn output(&self) -> Option<&str> {
        match self {
            ToolResult::Success(s) => Some(s),
            _ => None,
        }
    }

    /// The error message if failed, `None` otherwise.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            ToolResult::Error(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for ToolResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolResult::Success(s) => write!(f, "ok: {}", truncate(s, 80)),
            ToolResult::Error(e) => write!(f, "error: {}", truncate(e, 80)),
            ToolResult::Denied => write!(f, "denied"),
            ToolResult::Timeout => write!(f, "timeout"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// A single recorded tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// Unique call ID — `<session_id>-<sequence_number>`.
    pub id: String,
    /// Session that made this call.
    pub session_id: String,
    /// Name of the tool (e.g. `"read_file"`, `"shell"`, `"save_memory"`).
    pub tool_name: String,
    /// Arguments passed to the tool, as a JSON object.
    pub args: serde_json::Value,
    /// What happened when the tool ran.
    pub result: ToolResult,
    /// Wall-clock time the call was made.
    pub timestamp: DateTime<Utc>,
    /// How long the tool took to run, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// The `tool_call_id` supplied by the model (for round-trip matching).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_call_id: Option<String>,
}

impl ToolCallRecord {
    /// One-line summary for display.
    pub fn summary(&self) -> String {
        let args_str =
            if self.args.is_null() || self.args == serde_json::Value::Object(Default::default()) {
                String::new()
            } else {
                format!(" {}", truncate(&self.args.to_string(), 60))
            };
        let duration = self
            .duration_ms
            .map(|d| format!(" ({} ms)", d))
            .unwrap_or_default();
        format!(
            "{} `{}`{}{}  → {}",
            self.result.label(),
            self.tool_name,
            args_str,
            duration,
            self.result,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Session-scoped ledger of tool invocations.
///
/// All records are kept in memory for the lifetime of the session.
/// Call [`flush_to_disk`](ToolMemory::flush_to_disk) at session end to persist
/// a compact summary to `~/.grok/tool_history.json`.
#[derive(Debug)]
pub struct ToolMemory {
    /// ID of the owning session.
    session_id: String,
    /// Ordered list of all tool calls made this session.
    records: Vec<ToolCallRecord>,
    /// Monotonically increasing call counter for ID generation.
    seq: u64,
}

impl ToolMemory {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create an empty tool memory for the given session.
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            records: Vec::new(),
            seq: 0,
        }
    }

    // ── Recording ─────────────────────────────────────────────────────────────

    /// Record a completed tool call and return a reference to the stored record.
    ///
    /// - `tool_name`    — name of the tool
    /// - `args`         — JSON arguments
    /// - `result`       — outcome
    /// - `duration_ms`  — optional wall-clock duration in milliseconds
    pub fn record_call(
        &mut self,
        tool_name: &str,
        args: serde_json::Value,
        result: ToolResult,
        duration_ms: Option<u64>,
    ) -> &ToolCallRecord {
        self.seq += 1;
        let id = format!(
            "{}-{:06}",
            &self.session_id[..self.session_id.len().min(8)],
            self.seq
        );

        let record = ToolCallRecord {
            id,
            session_id: self.session_id.clone(),
            tool_name: tool_name.to_string(),
            args,
            result,
            timestamp: Utc::now(),
            duration_ms,
            model_call_id: None,
        };

        debug!(
            tool = %tool_name,
            result = %record.result.label(),
            "ToolMemory: recorded call"
        );

        self.records.push(record);
        self.records.last().unwrap()
    }

    /// Record a call and also store the model's `tool_call_id`.
    pub fn record_call_with_id(
        &mut self,
        tool_name: &str,
        args: serde_json::Value,
        result: ToolResult,
        duration_ms: Option<u64>,
        model_call_id: &str,
    ) -> &ToolCallRecord {
        self.record_call(tool_name, args, result, duration_ms);
        let last = self.records.last_mut().unwrap();
        last.model_call_id = Some(model_call_id.to_string());
        self.records.last().unwrap()
    }

    // ── Loop detection ────────────────────────────────────────────────────────

    /// Return `true` when the same `(tool_name, args)` pair has failed at least
    /// `threshold` times in the current session.
    ///
    /// Used to break infinite tool-retry loops before they exhaust the token
    /// budget.  `args` comparison is done by JSON equality.
    pub fn failed_recently(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        threshold: usize,
    ) -> bool {
        let count = self
            .records
            .iter()
            .filter(|r| r.tool_name == tool_name && &r.args == args && r.result.is_failure())
            .count();
        count >= threshold
    }

    /// Return `true` when *any* call to `tool_name` has failed at least
    /// `threshold` times, regardless of arguments.
    pub fn tool_failed_repeatedly(&self, tool_name: &str, threshold: usize) -> bool {
        let count = self
            .records
            .iter()
            .filter(|r| r.tool_name == tool_name && r.result.is_failure())
            .count();
        count >= threshold
    }

    /// Return the number of consecutive failures at the tail of the record
    /// list for a specific tool.  Resets to 0 on the first success.
    ///
    /// Useful for detecting "stuck" tools where the most recent N calls all
    /// failed.
    pub fn consecutive_failures_at_tail(&self, tool_name: &str) -> usize {
        self.records
            .iter()
            .rev()
            .take_while(|r| r.tool_name == tool_name && r.result.is_failure())
            .count()
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// All recorded calls in chronological order.
    pub fn all_calls(&self) -> &[ToolCallRecord] {
        &self.records
    }

    /// The `n` most-recent calls across all tools.
    pub fn recent_calls(&self, n: usize) -> &[ToolCallRecord] {
        let len = self.records.len();
        &self.records[len.saturating_sub(n)..]
    }

    /// All calls for a specific tool.
    pub fn calls_for_tool(&self, tool_name: &str) -> Vec<&ToolCallRecord> {
        self.records
            .iter()
            .filter(|r| r.tool_name == tool_name)
            .collect()
    }

    /// All failed calls.
    pub fn failed_calls(&self) -> Vec<&ToolCallRecord> {
        self.records
            .iter()
            .filter(|r| r.result.is_failure())
            .collect()
    }

    /// How many times `tool_name` has been invoked this session.
    pub fn call_count(&self, tool_name: &str) -> usize {
        self.records
            .iter()
            .filter(|r| r.tool_name == tool_name)
            .count()
    }

    /// Total number of recorded calls.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// `true` when no calls have been recorded.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The set of distinct tool names invoked this session.
    pub fn tools_used(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for r in &self.records {
            if seen.insert(r.tool_name.clone()) {
                out.push(r.tool_name.clone());
            }
        }
        out
    }

    /// Count successes and failures for a tool as `(successes, failures)`.
    pub fn tool_stats(&self, tool_name: &str) -> (usize, usize) {
        let calls = self.calls_for_tool(tool_name);
        let failures = calls.iter().filter(|r| r.result.is_failure()).count();
        (calls.len() - failures, failures)
    }

    // ── Prompt injection ──────────────────────────────────────────────────────

    /// Build a compact Markdown section listing the `max_calls` most-recent
    /// tool invocations.
    ///
    /// Returns an empty string when no calls have been recorded — safe to
    /// unconditionally append to a system prompt.
    ///
    /// Example output:
    /// ```text
    /// ## Recent tool calls
    ///
    /// - ✓ `read_file` {"path":"src/main.rs"} (12 ms) → ok: fn main() { }
    /// - ✗ `shell` {"command":"cargo build"} (340 ms) → error: linker not found
    /// ```
    pub fn to_prompt_section(&self, max_calls: usize) -> String {
        if self.records.is_empty() {
            return String::new();
        }

        let limit = max_calls.min(MAX_PROMPT_CALLS);
        let recent = self.recent_calls(limit);

        let lines: Vec<String> = recent
            .iter()
            .map(|r| format!("- {}", r.summary()))
            .collect();

        format!("\n\n## Recent tool calls\n\n{}\n", lines.join("\n"))
    }

    /// Convenience wrapper using the default [`MAX_PROMPT_CALLS`] limit.
    pub fn to_prompt_section_default(&self) -> String {
        self.to_prompt_section(MAX_PROMPT_CALLS)
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Append a session summary to `~/.grok/tool_history.json`.
    ///
    /// The file is a JSON array of [`SessionToolSummary`] objects.  Each call
    /// to this method appends one entry for the current session.
    ///
    /// Uses atomic write-then-rename to be safe against Starlink drops.
    pub fn flush_to_disk(&self) -> Result<()> {
        if self.records.is_empty() {
            return Ok(());
        }

        let path = tool_history_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }

        // Load existing history (or start fresh if file doesn't exist / is corrupt).
        let mut history: Vec<SessionToolSummary> = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        history.push(SessionToolSummary::from_memory(self));

        let json = serde_json::to_string_pretty(&history).context("serialising tool history")?;

        atomic_write(&path, &json).context("writing tool_history.json")?;

        debug!(
            session_id = %self.session_id,
            calls = self.records.len(),
            path = %path.display(),
            "ToolMemory: flushed session to disk"
        );

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Session summary (persisted to disk)
// ─────────────────────────────────────────────────────────────────────────────

/// A compact summary of all tool calls made in one session.
///
/// This is the unit stored in `~/.grok/tool_history.json`.  It does not
/// include full argument values (to keep the file small) but records enough
/// metadata to understand what the AI did.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToolSummary {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub total_calls: usize,
    pub successful_calls: usize,
    pub failed_calls: usize,
    pub denied_calls: usize,
    pub tools_used: Vec<String>,
    /// Compact per-call log: `"✓ read_file"`, `"✗ shell"`, …
    pub call_log: Vec<String>,
}

impl SessionToolSummary {
    /// Build a summary from an in-memory [`ToolMemory`].
    pub fn from_memory(tm: &ToolMemory) -> Self {
        let successful_calls = tm
            .records
            .iter()
            .filter(|r| matches!(r.result, ToolResult::Success(_)))
            .count();
        let denied_calls = tm
            .records
            .iter()
            .filter(|r| matches!(r.result, ToolResult::Denied))
            .count();
        let failed_calls = tm.records.len() - successful_calls - denied_calls;

        let call_log: Vec<String> = tm
            .records
            .iter()
            .map(|r| format!("{} {}", r.result.label(), r.tool_name))
            .collect();

        Self {
            session_id: tm.session_id.clone(),
            started_at: tm
                .records
                .first()
                .map(|r| r.timestamp)
                .unwrap_or_else(Utc::now),
            ended_at: Utc::now(),
            total_calls: tm.records.len(),
            successful_calls,
            failed_calls,
            denied_calls,
            tools_used: tm.tools_used(),
            call_log,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Private helpers
// ─────────────────────────────────────────────────────────────────────────────

fn tool_history_path() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".grok").join(TOOL_HISTORY_FILE))
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))
}

/// Truncate a string to `max_chars`, appending `…` if truncated.
fn truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        s
    } else {
        // Find the nearest char boundary.
        let mut boundary = max_chars;
        while !s.is_char_boundary(boundary) && boundary > 0 {
            boundary -= 1;
        }
        &s[..boundary]
    }
}

/// Atomically write `content` to `path` via a sibling `.tmp` file.
fn atomic_write(path: &std::path::Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content).with_context(|| format!("writing tmp {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_tm() -> ToolMemory {
        ToolMemory::new("test-session-abc123")
    }

    // ── record_call ───────────────────────────────────────────────────────────

    #[test]
    fn record_increases_len() {
        let mut tm = make_tm();
        assert_eq!(tm.len(), 0);
        tm.record_call(
            "read_file",
            json!({"path": "x.rs"}),
            ToolResult::Success("ok".into()),
            None,
        );
        assert_eq!(tm.len(), 1);
    }

    #[test]
    fn record_assigns_sequential_ids() {
        let mut tm = make_tm();
        tm.record_call("a", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("b", json!({}), ToolResult::Success("".into()), None);
        assert_ne!(tm.records[0].id, tm.records[1].id);
        assert!(tm.records[1].id > tm.records[0].id);
    }

    #[test]
    fn record_with_model_id_stores_it() {
        let mut tm = make_tm();
        tm.record_call_with_id(
            "shell",
            json!({}),
            ToolResult::Success("".into()),
            None,
            "call-xyz",
        );
        assert_eq!(tm.records[0].model_call_id.as_deref(), Some("call-xyz"));
    }

    // ── failed_recently ───────────────────────────────────────────────────────

    #[test]
    fn failed_recently_returns_false_below_threshold() {
        let mut tm = make_tm();
        let args = json!({"cmd": "cargo build"});
        tm.record_call(
            "shell",
            args.clone(),
            ToolResult::Error("linker error".into()),
            None,
        );
        assert!(!tm.failed_recently("shell", &args, 3));
    }

    #[test]
    fn failed_recently_returns_true_at_threshold() {
        let mut tm = make_tm();
        let args = json!({"cmd": "cargo build"});
        for _ in 0..3 {
            tm.record_call("shell", args.clone(), ToolResult::Error("err".into()), None);
        }
        assert!(tm.failed_recently("shell", &args, 3));
    }

    #[test]
    fn failed_recently_ignores_different_args() {
        let mut tm = make_tm();
        for i in 0..3 {
            tm.record_call(
                "shell",
                json!({"cmd": format!("cmd {}", i)}),
                ToolResult::Error("err".into()),
                None,
            );
        }
        // None of the exact args match a single key 3 times.
        assert!(!tm.failed_recently("shell", &json!({"cmd": "cmd 0"}), 3));
    }

    #[test]
    fn failed_recently_counts_only_failures() {
        let mut tm = make_tm();
        let args = json!({"cmd": "x"});
        tm.record_call("shell", args.clone(), ToolResult::Success("".into()), None);
        tm.record_call("shell", args.clone(), ToolResult::Success("".into()), None);
        tm.record_call("shell", args.clone(), ToolResult::Success("".into()), None);
        assert!(!tm.failed_recently("shell", &args, 1));
    }

    // ── tool_failed_repeatedly ────────────────────────────────────────────────

    #[test]
    fn tool_failed_repeatedly_ignores_args() {
        let mut tm = make_tm();
        tm.record_call("net", json!({"url": "a"}), ToolResult::Timeout, None);
        tm.record_call("net", json!({"url": "b"}), ToolResult::Timeout, None);
        assert!(tm.tool_failed_repeatedly("net", 2));
        assert!(!tm.tool_failed_repeatedly("net", 3));
    }

    // ── consecutive_failures_at_tail ──────────────────────────────────────────

    #[test]
    fn consecutive_failures_zero_after_success() {
        let mut tm = make_tm();
        tm.record_call("shell", json!({}), ToolResult::Error("e".into()), None);
        tm.record_call("shell", json!({}), ToolResult::Success("ok".into()), None);
        assert_eq!(tm.consecutive_failures_at_tail("shell"), 0);
    }

    #[test]
    fn consecutive_failures_counts_tail_only() {
        let mut tm = make_tm();
        tm.record_call("shell", json!({}), ToolResult::Success("ok".into()), None);
        tm.record_call("shell", json!({}), ToolResult::Error("e1".into()), None);
        tm.record_call("shell", json!({}), ToolResult::Error("e2".into()), None);
        assert_eq!(tm.consecutive_failures_at_tail("shell"), 2);
    }

    // ── queries ───────────────────────────────────────────────────────────────

    #[test]
    fn recent_calls_returns_last_n() {
        let mut tm = make_tm();
        for i in 0..6u8 {
            tm.record_call("t", json!({"i": i}), ToolResult::Success("".into()), None);
        }
        let r = tm.recent_calls(3);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].args["i"], 3);
        assert_eq!(r[2].args["i"], 5);
    }

    #[test]
    fn calls_for_tool_filters_by_name() {
        let mut tm = make_tm();
        tm.record_call("read_file", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("shell", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("read_file", json!({}), ToolResult::Success("".into()), None);
        assert_eq!(tm.calls_for_tool("read_file").len(), 2);
        assert_eq!(tm.calls_for_tool("shell").len(), 1);
    }

    #[test]
    fn call_count_matches_calls_for_tool_len() {
        let mut tm = make_tm();
        tm.record_call("a", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("a", json!({}), ToolResult::Error("e".into()), None);
        assert_eq!(tm.call_count("a"), 2);
        assert_eq!(tm.call_count("b"), 0);
    }

    #[test]
    fn tools_used_is_deduped_in_order() {
        let mut tm = make_tm();
        tm.record_call("b", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("a", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("b", json!({}), ToolResult::Success("".into()), None);
        let used = tm.tools_used();
        assert_eq!(used, vec!["b", "a"]);
    }

    #[test]
    fn tool_stats_counts_correctly() {
        let mut tm = make_tm();
        tm.record_call("t", json!({}), ToolResult::Success("ok".into()), None);
        tm.record_call("t", json!({}), ToolResult::Error("err".into()), None);
        tm.record_call("t", json!({}), ToolResult::Denied, None);
        let (ok, fail) = tm.tool_stats("t");
        assert_eq!(ok, 1);
        assert_eq!(fail, 2); // Error + Denied are both failures
    }

    #[test]
    fn failed_calls_includes_errors_denials_timeouts() {
        let mut tm = make_tm();
        tm.record_call("t", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("t", json!({}), ToolResult::Error("e".into()), None);
        tm.record_call("t", json!({}), ToolResult::Denied, None);
        tm.record_call("t", json!({}), ToolResult::Timeout, None);
        assert_eq!(tm.failed_calls().len(), 3);
    }

    // ── prompt section ────────────────────────────────────────────────────────

    #[test]
    fn to_prompt_section_empty_when_no_calls() {
        let tm = make_tm();
        assert_eq!(tm.to_prompt_section_default(), "");
    }

    #[test]
    fn to_prompt_section_contains_tool_name_and_result() {
        let mut tm = make_tm();
        tm.record_call(
            "read_file",
            json!({"path": "x.rs"}),
            ToolResult::Success("fn main(){}".into()),
            Some(5),
        );
        let section = tm.to_prompt_section_default();
        assert!(section.contains("read_file"));
        assert!(section.contains("Recent tool calls"));
        assert!(section.contains("✓"));
    }

    #[test]
    fn to_prompt_section_respects_max_calls() {
        let mut tm = make_tm();
        for i in 0..15u8 {
            tm.record_call("t", json!({"i": i}), ToolResult::Success("".into()), None);
        }
        // Default cap is MAX_PROMPT_CALLS = 10
        let section = tm.to_prompt_section_default();
        // Count occurrences of the bullet marker
        let bullets = section.matches("- ").count();
        assert!(bullets <= MAX_PROMPT_CALLS, "too many bullets: {}", bullets);
    }

    // ── ToolResult ────────────────────────────────────────────────────────────

    #[test]
    fn tool_result_is_failure() {
        assert!(!ToolResult::Success("ok".into()).is_failure());
        assert!(ToolResult::Error("e".into()).is_failure());
        assert!(ToolResult::Denied.is_failure());
        assert!(ToolResult::Timeout.is_failure());
    }

    #[test]
    fn tool_result_label() {
        assert_eq!(ToolResult::Success("".into()).label(), "✓");
        assert_eq!(ToolResult::Error("".into()).label(), "✗");
        assert_eq!(ToolResult::Denied.label(), "⊘");
        assert_eq!(ToolResult::Timeout.label(), "⏱");
    }

    #[test]
    fn tool_result_output_and_error() {
        let ok = ToolResult::Success("data".into());
        assert_eq!(ok.output(), Some("data"));
        assert!(ok.error_message().is_none());

        let err = ToolResult::Error("oops".into());
        assert!(err.output().is_none());
        assert_eq!(err.error_message(), Some("oops"));
    }

    // ── session summary ───────────────────────────────────────────────────────

    #[test]
    fn session_summary_totals_are_correct() {
        let mut tm = make_tm();
        tm.record_call("a", json!({}), ToolResult::Success("".into()), None);
        tm.record_call("a", json!({}), ToolResult::Error("".into()), None);
        tm.record_call("a", json!({}), ToolResult::Denied, None);
        let summary = SessionToolSummary::from_memory(&tm);
        assert_eq!(summary.total_calls, 3);
        assert_eq!(summary.successful_calls, 1);
        assert_eq!(summary.denied_calls, 1);
        assert_eq!(summary.failed_calls, 1);
    }

    // ── truncate helper ───────────────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 80), "hello");
    }

    #[test]
    fn truncate_long_string_is_cut() {
        let long = "a".repeat(100);
        let t = truncate(&long, 10);
        assert!(t.len() <= 10);
    }
}
