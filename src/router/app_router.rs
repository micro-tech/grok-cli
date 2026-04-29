//! Application-level router.
//!
//! [`AppRouter`] is a thin, cloneable wrapper around [`CpuRouter`] that
//! exposes the same async method signatures as [`crate::GrokClient`].
//!
//! All existing call sites can swap `GrokClient` → `AppRouter` with only
//! import and constructor changes — the method bodies stay identical because
//! the signatures are a drop-in match.
//!
//! # Why bother?
//!
//! - **Single dispatch point**: every AI call in the app flows through
//!   `CpuRouter`, so future backends (Ollama, Gemini, …) or routing policies
//!   (A/B, cost cap, Starlink-aware fallback) slot in without touching call
//!   sites.
//! - **Retry + back-off lives here**: `GrokBackend` handles exponential
//!   back-off and Starlink-resilient retries so individual commands don't have
//!   to think about it.
//! - **Cloneable via `Arc`**: the inner `CpuRouter` is ref-counted so
//!   `AppRouter` can be cheaply cloned and sent across async tasks.

use std::sync::Arc;

use anyhow::Result;
use grok_api::MessageContent;
use serde_json::Value;

use crate::MessageWithFinishReason;
use crate::router::backends::GrokBackend;
use crate::router::{CpuRouter, RouterRequest};

// ─────────────────────────────────────────────────────────────────────────────

/// A cloneable, application-level AI router.
///
/// Build one with [`AppRouter::new`] and use it wherever the app previously
/// used [`crate::GrokClient`].
///
/// ```rust,no_run
/// use grok_cli::router::AppRouter;
///
/// # async fn example() -> anyhow::Result<()> {
/// let router = AppRouter::new("xai-...", 30)?;
///
/// // One-shot chat
/// let reply = router
///     .chat_completion("Hello!", None, 0.7, 1024, "grok-3-mini")
///     .await?;
/// println!("{reply}");
///
/// // Multi-turn with tools
/// let history = vec![serde_json::json!({"role": "user", "content": "Hi"})];
/// let resp = router
///     .chat_completion_with_history(&history, 0.7, 1024, "grok-3-mini", None)
///     .await?;
/// println!("{:?}", resp.message.content);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct AppRouter {
    inner: Arc<CpuRouter>,
}

impl AppRouter {
    /// Build a router backed by the Grok API.
    ///
    /// - `api_key`     – xAI API key (must be non-empty).
    /// - `timeout_secs`– per-request HTTP timeout; use `Config::timeout_secs`
    ///                   to honour the user's config value.
    ///
    /// Returns an error if the key is empty or the underlying HTTP client
    /// cannot be constructed.
    pub fn new(api_key: &str, timeout_secs: u64) -> Result<Self> {
        let backend = GrokBackend::new_with_timeout(api_key, timeout_secs)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self {
            inner: Arc::new(CpuRouter::new(vec![Box::new(backend)])),
        })
    }

    // ── GrokClient-compatible methods ────────────────────────────────────────

    /// One-shot chat.
    ///
    /// Builds a minimal two-message thread (`system` + `user`) internally and
    /// returns the text of the assistant reply.
    ///
    /// Mirrors [`crate::GrokClient::chat_completion`].
    pub async fn chat_completion(
        &self,
        message: &str,
        system_prompt: Option<&str>,
        temperature: f32,
        max_tokens: u32,
        model: &str,
    ) -> Result<String> {
        let mut messages: Vec<Value> = Vec::new();

        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": message
        }));

        let mwfr = self
            .chat_completion_with_history(&messages, temperature, max_tokens, model, None)
            .await?;

        let text = mwfr
            .message
            .content
            .map(|c| match c {
                MessageContent::Text(t) => t,
                _ => String::new(),
            })
            .unwrap_or_default();

        Ok(text)
    }

    /// Multi-turn chat with full conversation history and optional tools.
    ///
    /// - `messages` — raw JSON values (role / content / tool_calls objects),
    ///   exactly as assembled by the existing command handlers.
    /// - `tools`    — raw JSON tool definitions in OpenAI function-calling
    ///   format, as returned by
    ///   [`crate::acp::tools::get_available_tool_definitions`].
    ///
    /// Mirrors [`crate::GrokClient::chat_completion_with_history`].
    pub async fn chat_completion_with_history(
        &self,
        messages: &[Value],
        temperature: f32,
        max_tokens: u32,
        model: &str,
        tools: Option<Vec<Value>>,
    ) -> Result<MessageWithFinishReason> {
        // Pass messages as raw JSON so that fields like `tool_call_id` are
        // preserved through the full pipeline.  Typed deserialization was
        // silently stripping that field, breaking multi-turn tool calls.
        let mut req = RouterRequest::new(model, messages.to_vec())
            .with_temperature(temperature)
            .with_max_tokens(max_tokens);

        if let Some(raw_tools) = tools {
            req = req.with_json_tools(raw_tools);
        }

        let resp = self
            .inner
            .route(&req)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(resp.into_message_with_finish_reason())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_api_key() {
        let err = AppRouter::new("", 30);
        assert!(err.is_err(), "should reject an empty API key");
    }

    #[test]
    fn accepts_non_empty_key() {
        // Only tests construction — no real API call is made.
        let result = AppRouter::new("xai-placeholder-key", 30);
        assert!(result.is_ok());
    }

    #[test]
    fn clone_shares_inner_arc() {
        let router = AppRouter::new("xai-placeholder-key", 30).unwrap();
        let clone = router.clone();
        // Both point at the same CpuRouter allocation.
        assert!(Arc::ptr_eq(&router.inner, &clone.inner));
    }
}
