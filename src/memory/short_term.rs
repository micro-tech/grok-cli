//! Short-term memory — the active conversation window.
//!
//! [`ShortTermMemory`] is the bounded, in-session message buffer that every AI
//! call site previously managed as a raw `Vec<Value>` or `Vec<ConversationItem>`.
//! It replaces all of those ad-hoc vectors with a single, well-typed container
//! that:
//!
//! - Keeps an ordered list of [`ChatMessage`] entries (system, user, assistant,
//!   tool).
//! - Enforces configurable limits on message count **and** estimated token count.
//! - Auto-trims the oldest non-system messages when either limit is exceeded so
//!   the context window never silently overflows.
//! - Exports to `Vec<serde_json::Value>` in OpenAI/xAI wire format so existing
//!   `chat_completion_with_history` call sites compile without any changes.
//!
//! # Example
//! ```rust,no_run
//! use grok_cli::memory::short_term::ShortTermMemory;
//!
//! let mut mem = ShortTermMemory::new();
//! mem.push_system("You are a helpful assistant.");
//! mem.push("user",      "Hello!",   None);
//! mem.push("assistant", "Hi there!", Some(8));
//!
//! let msgs = mem.to_json_messages(); // ready to pass to AppRouter
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::memory::types::ChatMessage;

// ── Defaults ─────────────────────────────────────────────────────────────────

/// Default upper bound on the number of messages kept in the window.
/// System messages are exempt and never count toward this limit.
pub const DEFAULT_MAX_MESSAGES: usize = 50;

/// Default upper bound on the **estimated** token count of non-system messages.
/// Rough rule-of-thumb: 1 token ≈ 4 UTF-8 characters.
pub const DEFAULT_MAX_TOKENS: u32 = 6_000;

/// Minimum messages to always retain after a trim (avoids trimming everything).
const MIN_RETAIN: usize = 4;

// ─────────────────────────────────────────────────────────────────────────────

/// A bounded, auto-trimming conversation message buffer.
///
/// The buffer distinguishes between **system** messages (pinned at index 0,
/// never trimmed) and **conversational** messages (user / assistant / tool).
/// When either [`max_messages`] or [`max_tokens`] is exceeded, the oldest
/// conversational messages are dropped until both constraints are satisfied or
/// [`MIN_RETAIN`] messages remain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemory {
    messages: Vec<ChatMessage>,
    /// Maximum number of non-system messages to retain.
    max_messages: usize,
    /// Maximum estimated token budget for non-system messages.
    max_tokens: u32,
    /// Running total of estimated tokens across all non-system messages.
    total_tokens: u32,
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortTermMemory {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Create a buffer with the default limits
    /// ([`DEFAULT_MAX_MESSAGES`] messages, [`DEFAULT_MAX_TOKENS`] tokens).
    pub fn new() -> Self {
        Self::with_limits(DEFAULT_MAX_MESSAGES, DEFAULT_MAX_TOKENS)
    }

    /// Create a buffer with explicit limits.
    ///
    /// `max_messages` — maximum non-system messages before trimming begins.
    /// `max_tokens`   — maximum estimated tokens before trimming begins.
    pub fn with_limits(max_messages: usize, max_tokens: u32) -> Self {
        Self {
            messages: Vec::new(),
            max_messages: max_messages.max(MIN_RETAIN),
            max_tokens,
            total_tokens: 0,
        }
    }

    // ── Mutation ─────────────────────────────────────────────────────────────

    /// Push a new message and auto-trim if needed.
    ///
    /// `tokens_used` — if the caller knows the real token count from an API
    /// response it can supply it here; otherwise the buffer estimates from
    /// the content length.
    pub fn push(&mut self, role: &str, content: &str, tokens_used: Option<u32>) {
        let tokens = tokens_used.unwrap_or_else(|| Self::estimate_tokens(content));

        let msg = ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens_used: Some(tokens),
            tool_calls: None,
            tool_call_id: None,
        };

        // System messages are pinned; do not count toward the budget.
        if role != "system" {
            self.total_tokens = self.total_tokens.saturating_add(tokens);
        }

        self.messages.push(msg);

        if role != "system" {
            self.trim();
        }
    }

    /// Convenience wrapper for pushing (or replacing) the system message.
    ///
    /// There should only ever be one system message and it must live at index 0.
    /// Calling this when a system message already exists **replaces** it.
    pub fn push_system(&mut self, content: &str) {
        let msg = ChatMessage {
            role: "system".to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens_used: None,
            tool_calls: None,
            tool_call_id: None,
        };

        if let Some(existing) = self.messages.first_mut() {
            if existing.role == "system" {
                *existing = msg;
                return;
            }
        }

        // No system message yet — insert at the front.
        self.messages.insert(0, msg);
    }

    /// Push a tool-result message (role = `"tool"`).
    ///
    /// `tool_call_id` is stored in the `tool_calls` JSON field so the
    /// serialised format matches the OpenAI wire protocol.
    pub fn push_tool_result(&mut self, tool_call_id: &str, content: &str) {
        let tokens = Self::estimate_tokens(content);
        self.total_tokens = self.total_tokens.saturating_add(tokens);

        let msg = ChatMessage {
            role: "tool".to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens_used: Some(tokens),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        };
        self.messages.push(msg);
        self.trim();
    }

    /// Clear all messages **including** the system message.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.total_tokens = 0;
    }

    /// Clear conversational messages but keep the system message (if present).
    ///
    /// This mirrors the `/clear` slash command behaviour.
    pub fn clear_keep_system(&mut self) {
        let system = self
            .messages
            .first()
            .filter(|m| m.role == "system")
            .cloned();

        self.messages.clear();
        self.total_tokens = 0;

        if let Some(sys) = system {
            self.messages.push(sys);
        }
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    /// Immutable view of all messages in chronological order.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Number of messages in the buffer (includes the system message).
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// `true` if there are no messages at all.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Estimated token count across **non-system** messages.
    pub fn estimated_tokens(&self) -> u32 {
        self.total_tokens
    }

    /// Number of non-system (conversational) messages.
    pub fn conversational_len(&self) -> usize {
        self.messages.iter().filter(|m| m.role != "system").count()
    }

    /// Return the current system prompt text, if one has been set.
    pub fn system_prompt(&self) -> Option<&str> {
        self.messages
            .first()
            .filter(|m| m.role == "system")
            .map(|m| m.content.as_str())
    }

    /// Timestamp of the most recent message, or `None` if empty.
    pub fn last_updated(&self) -> Option<DateTime<Utc>> {
        self.messages.last().map(|m| m.timestamp)
    }

    // ── Export ───────────────────────────────────────────────────────────────

    /// Serialise the buffer to the `Vec<serde_json::Value>` format expected by
    /// [`crate::router::AppRouter::chat_completion_with_history`] and all legacy
    /// `GrokClient::chat_completion_with_history` call sites.
    ///
    /// Each value is an object with at minimum `"role"` and `"content"` keys.
    /// Tool messages additionally carry a `"tool_call_id"` field.
    pub fn to_json_messages(&self) -> Vec<serde_json::Value> {
        self.messages
            .iter()
            .map(|m| {
                let mut obj = serde_json::json!({
                    "role":    m.role,
                    "content": m.content,
                });

                // tool_call_id is present on role="tool" result messages
                if let Some(id) = &m.tool_call_id {
                    obj["tool_call_id"] = serde_json::Value::String(id.clone());
                }

                // tool_calls is present on role="assistant" messages that
                // request one or more tool invocations
                if let Some(tc) = &m.tool_calls {
                    obj["tool_calls"] = tc.clone();
                }

                obj
            })
            .collect()
    }

    /// Clone the most recent `n` conversational (non-system) messages,
    /// prepended with the system message if one exists.
    ///
    /// Useful for sliding-window summarisation or truncated context injection.
    pub fn recent(&self, n: usize) -> Vec<ChatMessage> {
        let system = self
            .messages
            .first()
            .filter(|m| m.role == "system")
            .cloned();

        let conv: Vec<ChatMessage> = self
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let mut out = Vec::with_capacity(conv.len() + 1);
        if let Some(sys) = system {
            out.push(sys);
        }
        out.extend(conv);
        out
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Trim the oldest non-system messages until both `max_messages` and
    /// `max_tokens` constraints are satisfied, while always keeping at least
    /// [`MIN_RETAIN`] conversational messages.
    fn trim(&mut self) {
        loop {
            let conv_count = self.conversational_len();

            let over_messages = conv_count > self.max_messages;
            let over_tokens = self.total_tokens > self.max_tokens;

            if (!over_messages && !over_tokens) || conv_count <= MIN_RETAIN {
                break;
            }

            // Find and remove the oldest non-system message.
            if let Some(pos) = self.messages.iter().position(|m| m.role != "system") {
                let removed = self.messages.remove(pos);
                let removed_tokens = removed
                    .tokens_used
                    .unwrap_or_else(|| Self::estimate_tokens(&removed.content));
                self.total_tokens = self.total_tokens.saturating_sub(removed_tokens);

                debug!(
                    role = %removed.role,
                    removed_tokens,
                    remaining_tokens = self.total_tokens,
                    remaining_messages = self.messages.len(),
                    "ShortTermMemory: trimmed oldest message"
                );
            } else {
                break; // only system messages left
            }
        }
    }

    /// Rough token estimate: 1 token ≈ 4 UTF-8 bytes.
    fn estimate_tokens(text: &str) -> u32 {
        ((text.len() as f32) / 4.0).ceil() as u32
    }
}

// ── Conversion helpers ────────────────────────────────────────────────────────

impl From<ShortTermMemory> for Vec<serde_json::Value> {
    fn from(mem: ShortTermMemory) -> Self {
        mem.to_json_messages()
    }
}

impl<'a> From<&'a ShortTermMemory> for Vec<serde_json::Value> {
    fn from(mem: &'a ShortTermMemory) -> Self {
        mem.to_json_messages()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mem() -> ShortTermMemory {
        ShortTermMemory::with_limits(10, 10_000)
    }

    // ── Basic push / len ────────────────────────────────────────────────────

    #[test]
    fn push_increases_len() {
        let mut m = make_mem();
        assert_eq!(m.len(), 0);
        m.push("user", "hello", None);
        assert_eq!(m.len(), 1);
        m.push("assistant", "hi", None);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn system_message_lands_at_index_zero() {
        let mut m = make_mem();
        m.push("user", "first", None);
        m.push_system("be helpful");
        assert_eq!(m.messages()[0].role, "system");
    }

    #[test]
    fn push_system_replaces_existing() {
        let mut m = make_mem();
        m.push_system("v1");
        m.push_system("v2");
        assert_eq!(
            m.messages()
                .iter()
                .filter(|msg| msg.role == "system")
                .count(),
            1
        );
        assert_eq!(m.system_prompt(), Some("v2"));
    }

    // ── Clear variants ──────────────────────────────────────────────────────

    #[test]
    fn clear_removes_everything() {
        let mut m = make_mem();
        m.push_system("sys");
        m.push("user", "hi", None);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.estimated_tokens(), 0);
    }

    #[test]
    fn clear_keep_system_retains_system_message() {
        let mut m = make_mem();
        m.push_system("keep me");
        m.push("user", "a", None);
        m.push("assistant", "b", None);
        m.clear_keep_system();
        assert_eq!(m.len(), 1);
        assert_eq!(m.system_prompt(), Some("keep me"));
        assert_eq!(m.estimated_tokens(), 0);
    }

    // ── Token tracking ──────────────────────────────────────────────────────

    #[test]
    fn token_tracking_uses_provided_value() {
        let mut m = make_mem();
        m.push("user", "hello world", Some(42));
        assert_eq!(m.estimated_tokens(), 42);
    }

    #[test]
    fn token_tracking_estimates_when_none() {
        let mut m = make_mem();
        let content = "abcdefghijklmnop"; // 16 chars → ceil(16/4) = 4 tokens
        m.push("user", content, None);
        assert_eq!(m.estimated_tokens(), 4);
    }

    #[test]
    fn system_message_does_not_count_toward_token_budget() {
        let mut m = make_mem();
        m.push_system("a very long system prompt that should not be counted");
        assert_eq!(m.estimated_tokens(), 0);
    }

    // ── Auto-trim by message count ──────────────────────────────────────────

    #[test]
    fn trims_oldest_when_message_limit_exceeded() {
        // limit: 5 messages, very high token budget
        let mut m = ShortTermMemory::with_limits(5, 1_000_000);
        for i in 0..8u8 {
            m.push("user", &format!("msg {}", i), Some(1));
        }
        assert!(m.conversational_len() <= 5);
    }

    #[test]
    fn system_message_is_never_trimmed() {
        let mut m = ShortTermMemory::with_limits(5, 1_000_000);
        m.push_system("pinned");
        for i in 0..10u8 {
            m.push("user", &format!("msg {}", i), Some(1));
        }
        assert!(
            m.messages()
                .first()
                .map(|msg| msg.role == "system")
                .unwrap_or(false)
        );
    }

    // ── Auto-trim by token budget ───────────────────────────────────────────

    #[test]
    fn trims_when_token_budget_exceeded() {
        // very low token budget: 20 tokens
        let mut m = ShortTermMemory::with_limits(1_000, 20);
        // each message is 10 tokens
        for i in 0..5u8 {
            m.push("user", &format!("msg {}", i), Some(10));
        }
        assert!(m.estimated_tokens() <= 20 || m.conversational_len() <= MIN_RETAIN);
    }

    // ── JSON export ─────────────────────────────────────────────────────────

    #[test]
    fn to_json_messages_has_correct_roles() {
        let mut m = make_mem();
        m.push_system("sys");
        m.push("user", "hello", None);
        m.push("assistant", "hi", None);

        let json = m.to_json_messages();
        assert_eq!(json.len(), 3);
        assert_eq!(json[0]["role"], "system");
        assert_eq!(json[1]["role"], "user");
        assert_eq!(json[2]["role"], "assistant");
    }

    #[test]
    fn to_json_messages_includes_content() {
        let mut m = make_mem();
        m.push("user", "test content", None);
        let json = m.to_json_messages();
        assert_eq!(json[0]["content"], "test content");
    }

    #[test]
    fn from_ref_conversion_works() {
        let mut m = make_mem();
        m.push("user", "hi", None);
        let v: Vec<serde_json::Value> = (&m).into();
        assert_eq!(v.len(), 1);
    }

    // ── recent() ────────────────────────────────────────────────────────────

    #[test]
    fn recent_returns_last_n_with_system() {
        let mut m = make_mem();
        m.push_system("sys");
        for i in 0..6u8 {
            m.push("user", &i.to_string(), None);
        }
        let r = m.recent(3);
        // system + 3 conversational
        assert_eq!(r.len(), 4);
        assert_eq!(r[0].role, "system");
    }

    #[test]
    fn recent_without_system_message() {
        let mut m = make_mem();
        for i in 0..5u8 {
            m.push("user", &i.to_string(), None);
        }
        let r = m.recent(2);
        assert_eq!(r.len(), 2);
    }

    // ── conversational_len ──────────────────────────────────────────────────

    #[test]
    fn conversational_len_excludes_system() {
        let mut m = make_mem();
        m.push_system("sys");
        m.push("user", "a", None);
        m.push("assistant", "b", None);
        assert_eq!(m.conversational_len(), 2);
    }

    // ── tool result ─────────────────────────────────────────────────────────

    #[test]
    fn push_tool_result_adds_tool_call_id_to_json() {
        let mut m = make_mem();
        m.push_tool_result("call-123", "result text");
        let json = m.to_json_messages();
        assert_eq!(json[0]["role"], "tool");
        assert_eq!(json[0]["tool_call_id"], "call-123");
    }
}
