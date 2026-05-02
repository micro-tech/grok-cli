//! Shared types used across all memory tiers.
//!
//! Every memory module imports from here so that types are defined once and
//! flow consistently through the whole system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Chat message (short-term building block)
// ─────────────────────────────────────────────────────────────────────────────

/// A single turn in an active conversation.
///
/// This is the fundamental unit of [`crate::memory::short_term::ShortTermMemory`].
/// It mirrors the OpenAI / xAI message shape so it can be serialised directly
/// for API calls via [`to_api_value`](ChatMessage::to_api_value).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// `"system"`, `"user"`, `"assistant"`, or `"tool"`
    pub role: String,
    /// Plain-text content of the message.
    pub content: String,
    /// Wall-clock time the message was added to memory.
    pub timestamp: DateTime<Utc>,
    /// Estimated or measured token count for this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,
    /// Raw JSON tool-call array (`null` for non-tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
    /// `tool_call_id` — only present on `role = "tool"` reply messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// Construct a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    /// Construct a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Construct an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    /// Construct a tool-result message.
    pub fn tool(content: impl Into<String>, call_id: impl Into<String>) -> Self {
        let mut m = Self::new("tool", content);
        m.tool_call_id = Some(call_id.into());
        m
    }

    fn new(role: &str, content: impl Into<String>) -> Self {
        Self {
            role: role.to_string(),
            content: content.into(),
            timestamp: Utc::now(),
            tokens_used: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Serialise to the JSON object shape the Grok / OpenAI API expects.
    ///
    /// ```json
    /// { "role": "user", "content": "Hello!" }
    /// ```
    pub fn to_api_value(&self) -> serde_json::Value {
        let mut map = serde_json::json!({
            "role": self.role,
            "content": self.content,
        });

        if let Some(tc) = &self.tool_calls {
            map["tool_calls"] = tc.clone();
        }
        if let Some(id) = &self.tool_call_id {
            map["tool_call_id"] = serde_json::Value::String(id.clone());
        }

        map
    }

    /// Rough token estimate: 1 token ≈ 4 chars (good enough for trimming).
    pub fn estimated_tokens(&self) -> u32 {
        self.tokens_used
            .unwrap_or_else(|| estimate_tokens(&self.content))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Long-term memory entry
// ─────────────────────────────────────────────────────────────────────────────

/// A single persisted fact in long-term memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEntry {
    /// Unique ID — `uuid v4` string.
    pub id: String,
    /// The fact text, e.g. `"User prefers dark mode"`.
    pub fact: String,
    /// Optional keyword tags for search / filtering.
    pub tags: Vec<String>,
    /// When the fact was first recorded.
    pub created_at: DateTime<Utc>,
    /// Where the fact came from.
    pub source: MemorySource,
    /// Optional relevance score (0.0 – 1.0); populated at query time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance: Option<f32>,
}

impl MemoryEntry {
    /// Create a new entry with the current timestamp and a generated UUID.
    pub fn new(fact: impl Into<String>, source: MemorySource) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            fact: fact.into(),
            tags: Vec::new(),
            created_at: Utc::now(),
            source,
            relevance: None,
        }
    }

    /// Builder: attach tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Format for injection into a system prompt section.
    pub fn to_prompt_line(&self) -> String {
        if self.tags.is_empty() {
            format!("- {}", self.fact)
        } else {
            format!("- {} [{}]", self.fact, self.tags.join(", "))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Where a long-term memory came from
// ─────────────────────────────────────────────────────────────────────────────

/// The origin of a [`MemoryEntry`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    /// Explicitly saved by the user via the `save_memory` tool.
    User,
    /// Inferred and auto-saved by the AI during a session.
    Inferred,
    /// Loaded from a project context file (`.grok/context.md`, `GEMINI.md`, …).
    System,
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Inferred => write!(f, "inferred"),
            Self::System => write!(f, "system"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Episodic memory — past session summary
// ─────────────────────────────────────────────────────────────────────────────

/// A high-level record of a completed conversation session.
///
/// Stored at `~/.grok/sessions/<session_id>/episode.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSummary {
    /// Matches `InteractiveSession::session_id`.
    pub session_id: String,
    /// When the session started.
    pub started_at: DateTime<Utc>,
    /// When the session ended (or was saved).
    pub ended_at: DateTime<Utc>,
    /// Model used in this session.
    pub model: String,
    /// Total number of conversation turns.
    pub message_count: usize,
    /// Cumulative token usage for the session.
    pub total_tokens: u32,
    /// Auto-generated or user-assigned human-readable title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Short bullet-point facts extracted from the session
    /// (populated by the AI on session end, or left empty).
    #[serde(default)]
    pub key_facts: Vec<String>,
    /// Working directory when the session ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
}

impl EpisodeSummary {
    /// Create a minimal episode record from session metadata.
    pub fn new(
        session_id: impl Into<String>,
        model: impl Into<String>,
        started_at: DateTime<Utc>,
        message_count: usize,
        total_tokens: u32,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            started_at,
            ended_at: Utc::now(),
            model: model.into(),
            message_count,
            total_tokens,
            title: None,
            key_facts: Vec::new(),
            working_dir: None,
        }
    }

    /// One-line description for listing in the `/history` command.
    pub fn display_line(&self) -> String {
        let title = self.title.as_deref().unwrap_or("(untitled)");
        format!(
            "[{}] {} — {} turns, {} tokens ({})",
            &self.session_id[..8.min(self.session_id.len())],
            title,
            self.message_count,
            self.total_tokens,
            self.started_at.format("%Y-%m-%d %H:%M"),
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Memory kind tag
// ─────────────────────────────────────────────────────────────────────────────

/// Which tier of the memory hierarchy an item belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// Active conversation window — volatile, bounded.
    ShortTerm,
    /// Persisted user/AI facts — permanent, queryable.
    LongTerm,
    /// Completed session records — append-only archive.
    Episodic,
    /// Project context loaded from files — read-only for the session.
    Working,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShortTerm => write!(f, "short-term"),
            Self::LongTerm => write!(f, "long-term"),
            Self::Episodic => write!(f, "episodic"),
            Self::Working => write!(f, "working"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Rough token count: 1 token ≈ 4 characters.
///
/// Good enough for trimming decisions; avoids pulling in a full tokeniser.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4).max(1) as u32
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_system_role() {
        let m = ChatMessage::system("You are helpful.");
        assert_eq!(m.role, "system");
        assert_eq!(m.content, "You are helpful.");
        assert!(m.tool_calls.is_none());
    }

    #[test]
    fn chat_message_to_api_value_basic() {
        let m = ChatMessage::user("Hello!");
        let v = m.to_api_value();
        assert_eq!(v["role"], "user");
        assert_eq!(v["content"], "Hello!");
    }

    #[test]
    fn chat_message_to_api_value_tool() {
        let m = ChatMessage::tool("result text", "call-123");
        let v = m.to_api_value();
        assert_eq!(v["role"], "tool");
        assert_eq!(v["tool_call_id"], "call-123");
    }

    #[test]
    fn memory_entry_new_has_uuid() {
        let e = MemoryEntry::new("user prefers dark mode", MemorySource::User);
        assert!(!e.id.is_empty());
        assert_eq!(e.fact, "user prefers dark mode");
        assert_eq!(e.source, MemorySource::User);
    }

    #[test]
    fn memory_entry_prompt_line_with_tags() {
        let e = MemoryEntry::new("use tabs not spaces", MemorySource::User)
            .with_tags(vec!["style".into(), "rust".into()]);
        assert_eq!(e.to_prompt_line(), "- use tabs not spaces [style, rust]");
    }

    #[test]
    fn memory_entry_prompt_line_no_tags() {
        let e = MemoryEntry::new("dark mode preferred", MemorySource::User);
        assert_eq!(e.to_prompt_line(), "- dark mode preferred");
    }

    #[test]
    fn episode_summary_display_line() {
        let ep = EpisodeSummary::new("abc123def456", "grok-3-mini", Utc::now(), 10, 512);
        let line = ep.display_line();
        assert!(line.contains("abc123de"));
        assert!(line.contains("10 turns"));
        assert!(line.contains("512 tokens"));
    }

    #[test]
    fn estimate_tokens_non_zero() {
        assert!(estimate_tokens("hello world") > 0);
        assert!(estimate_tokens("") == 1); // clamped to minimum 1
    }

    #[test]
    fn memory_kind_display() {
        assert_eq!(MemoryKind::ShortTerm.to_string(), "short-term");
        assert_eq!(MemoryKind::LongTerm.to_string(), "long-term");
        assert_eq!(MemoryKind::Episodic.to_string(), "episodic");
        assert_eq!(MemoryKind::Working.to_string(), "working");
    }

    #[test]
    fn memory_source_display() {
        assert_eq!(MemorySource::User.to_string(), "user");
        assert_eq!(MemorySource::Inferred.to_string(), "inferred");
        assert_eq!(MemorySource::System.to_string(), "system");
    }

    #[test]
    fn chat_message_estimated_tokens_uses_stored_if_present() {
        let mut m = ChatMessage::user("hi");
        m.tokens_used = Some(42);
        assert_eq!(m.estimated_tokens(), 42);
    }

    #[test]
    fn chat_message_estimated_tokens_falls_back_to_estimate() {
        let m = ChatMessage::user("hello world, this is a longer string");
        // No tokens_used set — falls back to len/4
        assert!(m.estimated_tokens() > 0);
        assert!(m.tokens_used.is_none());
    }
}
