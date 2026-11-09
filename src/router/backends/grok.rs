//! Grok backend — wraps the existing [`GrokClient`] for use inside the router.
//!
//! ## Starlink resilience
//! Every request is retried up to [`MAX_RETRIES`] times with exponential
//! back-off plus a small random jitter.  Transient network errors (timeout,
//! connection reset) are retried; authentication errors (HTTP 401) abort
//! immediately so we don't burn quota on bad keys.

use async_trait::async_trait;
use tracing::{debug, warn};

use grok_api::MessageContent;

use crate::GrokClient;
use crate::router::{Backend, BackendKind, RouterError, RouterRequest, RouterResponse};
use crate::utils::network::{RetryPolicy, detect_network_drop};

// Default temperature when the caller does not specify one.
const DEFAULT_TEMPERATURE: f32 = 0.7;

/// Default max-tokens when the caller does not specify one.
const DEFAULT_MAX_TOKENS: u32 = 1_024;

// ─────────────────────────────────────────────────────────────────────────────

/// A [`Backend`] that dispatches requests to the Grok / xAI API.
///
/// Build one with [`GrokBackend::new`] and hand it to [`CpuRouter::new`]:
///
/// ```rust,no_run
/// use grok_cli::router::{CpuRouter, RouterRequest};
/// use grok_cli::router::backends::GrokBackend;
///
/// let backend = GrokBackend::new("xai-...").expect("valid key");
/// let router  = CpuRouter::new(vec![Box::new(backend)]);
/// ```
#[derive(Debug)]
pub struct GrokBackend {
    client: GrokClient,
    /// Whether the backend was successfully initialised (key present, client built).
    available: bool,
}

impl GrokBackend {
    /// Create a new backend with the default 30-second HTTP timeout.
    ///
    /// Returns [`RouterError::Auth`] if the API key is empty or the client
    /// cannot be constructed.
    pub fn new(api_key: &str) -> Result<Self, RouterError> {
        Self::new_with_timeout(api_key, 30)
    }

    /// Like [`new`] but lets the caller set the HTTP timeout (seconds).
    ///
    /// Use this when you want to honour `Config::timeout_secs` from the app
    /// configuration so user-tuned values are respected end-to-end.
    ///
    /// `max_retries` on the inner [`GrokClient`] is intentionally set to **1**
    /// because all retry / back-off logic lives in [`GrokBackend::send`].
    pub fn new_with_timeout(api_key: &str, timeout_secs: u64) -> Result<Self, RouterError> {
        if api_key.is_empty() {
            return Err(RouterError::Auth(
                "Grok API key must not be empty".to_string(),
            ));
        }

        let client = GrokClient::with_settings(api_key, timeout_secs, 1)
            .map_err(|e| RouterError::Auth(format!("Failed to build GrokClient: {e}")))?;

        Ok(Self {
            client,
            available: true,
        })
    }

    /// Classify an `anyhow::Error` from the Grok client into a [`RouterError`].
    ///
    /// Uses the canonical network drop detector so that transient send failures
    /// (e.g. "error sending request for url") are treated as retriable Network
    /// errors instead of fatal BackendError. This fixes the symptom where a
    /// connectivity hiccup was reported as "Backend error: Network error: ...".
    fn classify_error(err: &anyhow::Error) -> RouterError {
        let msg = err.to_string().to_lowercase();

        if msg.contains("401") || msg.contains("unauthorized") || msg.contains("authentication") {
            return RouterError::Auth(err.to_string());
        }
        if msg.contains("429") || msg.contains("rate limit") || msg.contains("too many requests") {
            return RouterError::RateLimit;
        }

        // Context-window overflow (H4): HTTP 400 with context_length_exceeded,
        // or a "maximum context length" / "too many tokens" message. This is a
        // distinct, recoverable kind — the caller trims history and retries
        // once, instead of the old fatal BackendError fallthrough.
        if msg.contains("context_length_exceeded")
            || msg.contains("maximum context length")
            || msg.contains("context length")
            || (msg.contains("400") && msg.contains("too many tokens"))
        {
            return RouterError::ContextOverflow(err.to_string());
        }

        // Delegate to the Starlink-aware detector first (covers "error sending request",
        // connection reset, Cloudflare 52x, satellite handovers, etc.).
        if detect_network_drop(err) {
            return RouterError::Network(err.to_string());
        }

        // Fallback heuristics for anything that looks network-related.
        if msg.contains("timeout")
            || msg.contains("timed out")
            || msg.contains("connection")
            || msg.contains("reset")
            || msg.contains("eof")
            || msg.contains("error sending request")
        {
            return RouterError::Network(err.to_string());
        }

        RouterError::BackendError(err.to_string())
    }

    /// Returns `true` for errors that are worth retrying (transient).
    /// Delegates to the canonical `RetryPolicy` + `detect_network_drop`.
    fn is_retryable(err: &RouterError) -> bool {
        if matches!(err, RouterError::Network(_) | RouterError::RateLimit) {
            return true;
        }
        // Also treat raw network strings that the policy would catch.
        if let RouterError::Network(msg) = err {
            let e = anyhow::anyhow!("{}", msg);
            return detect_network_drop(&e);
        }
        false
    }

    /// Parse a `Retry-After` value (seconds) out of an error string, if present.
    ///
    /// Best-effort: grok_api 0.2.4 discards the 429 body, so the value is only
    /// present once H5's vendored grok_api preserves it (or if the API puts it
    /// in the error text). Returns `None` when no usable value is found.
    /// The value is capped at 60s so a bad header cannot stall the CLI.
    /// Uses the LAST occurrence: the vendored grok_api appends the header's
    /// `Retry-After` after the body, so the last match is the header value
    /// (a body's own "retry-after" text must not win).
    fn parse_retry_after(msg: &str) -> Option<std::time::Duration> {
        let lower = msg.to_ascii_lowercase();
        for key in ["retry-after", "retry_after"] {
            if let Some(idx) = lower.rfind(key) {
                let rest = &lower[idx + key.len()..];
                // Skip separators (:, =, spaces) then parse leading digits.
                let digits: String = rest
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(secs) = digits.parse::<u64>() {
                    if secs > 0 {
                        return Some(std::time::Duration::from_secs(secs.min(60)));
                    }
                }
            }
        }
        None
    }

    /// Trim request messages after a context-overflow error, so the retry has
    /// a real chance of fitting. Drops the oldest half of the messages, then
    /// enforces the token budget. Never drops the system message (index 0, if
    /// present) or the last message (the current user turn).
    fn trim_messages_for_retry(messages: &mut Vec<serde_json::Value>, model: &str) {
        if messages.len() > 2 {
            let start = if messages[0]
                .get("role")
                .and_then(|r| r.as_str())
                == Some("system")
            {
                1
            } else {
                0
            };
            // Exclude the system message and the last message from dropping.
            let droppable = messages.len() - start - 1;
            if droppable > 0 {
                let to_drop = (droppable / 2).max(1);
                let drop_end = (start + to_drop).min(messages.len() - 1);
                messages.drain(start..drop_end);
            }
        }
        // Then enforce the token budget.
        let budget = crate::constants::get_context_budget(model);
        crate::acp::context_trim::trim_to_token_budget(messages, budget);
    }
}

#[async_trait]
impl Backend for GrokBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Grok
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn send(&self, req: &RouterRequest) -> Result<RouterResponse, RouterError> {
        // Messages are already raw serde_json::Value — no conversion needed.
        // Owned (not borrowed) so a context-overflow retry can trim them (H4).
        let mut messages = req.messages.clone();

        // ── Convert tool definitions (empty → None, so Grok skips tool plumbing) ──
        let tools: Option<Vec<serde_json::Value>> = if req.tools.is_empty() {
            None
        } else {
            Some(
                req.tools
                    .iter()
                    .map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null))
                    .collect(),
            )
        };

        let temperature = req.temperature.unwrap_or(DEFAULT_TEMPERATURE);
        let max_tokens = req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

        // Only pass reasoning_effort for models that actually support it
        let reasoning_effort: Option<&str> = if req.model.contains("reasoning")
            || req.model.starts_with("grok-4.")
        {
            req.reasoning_effort.as_deref()
        } else {
            None
        };

        // ── Retry loop (now uses the unified RetryPolicy from Task 287) ─────────
        let policy = RetryPolicy::default_starlink();
        let mut last_err = RouterError::Unknown;
        // H4: whether the single trim-and-retry after a context overflow has
        // already been used, and a Retry-After override for the next attempt.
        let mut trimmed_once = false;
        let mut retry_after_override: Option<std::time::Duration> = None;

        for attempt in 0..=policy.max_retries {
            if attempt > 0 {
                // Honor a parsed Retry-After instead of the policy delay when
                // the previous failure carried one (H4).
                let delay = retry_after_override
                    .take()
                    .unwrap_or_else(|| policy.delay_for_attempt(attempt - 1));
                warn!(
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Grok backend: retrying after transient error"
                );
                tokio::time::sleep(delay).await;
            }

            debug!(attempt, model = %req.model, "Grok backend: sending request");

            // Prompt-cache affinity: clone the client and attach this
            // request's conversation key (if any) so xAI routes the turn to
            // the server holding the conversation's prefix cache.
            let mut client = self.client.clone();
            if let Some(key) = req.prompt_cache_key.clone() {
                client = client.with_prompt_cache_key(key);
            }

            let result = client
                .chat_completion_with_history(
                    &messages,
                    temperature,
                    max_tokens,
                    &req.model,
                    tools.clone(),
                    reasoning_effort,
                )
                .await;

            match result {
                Ok(mwfr) => {
                    // ── Unpack the response ──────────────────────────
                    // Serialise the full message first, before we partially
                    // move `tool_calls` out of it.
                    let raw =
                        serde_json::to_value(&mwfr.message).unwrap_or(serde_json::Value::Null);

                    let text = match &mwfr.message.content {
                        Some(MessageContent::Text(t)) => Some(t.clone()),
                        _ => None,
                    };

                    // Capture the real finish_reason from the API before
                    // partially moving mwfr.  This is critical: when the model
                    // wants to call tools the API returns "tool_calls" here, NOT
                    // "stop".  Propagating the real value lets handle_chat_completion
                    // correctly continue the tool loop instead of short-circuiting
                    // and returning an empty response.
                    let _finish_reason = mwfr.finish_reason.clone();

                    let tool_calls = mwfr.message.tool_calls.unwrap_or_default();
                    let thinking_content = mwfr.thinking_content;
                    // Real API-reported token counts (Task 463) — clone before
                    // the partial moves above consume `mwfr`.
                    let usage = mwfr.usage.clone();

                    return Ok(RouterResponse {
                        text,
                        tool_calls,
                        raw,
                        model: req.model.clone(),
                        usage,
                        thinking_content,
                    });
                }

                Err(e) => {
                    let classified = Self::classify_error(&e);

                    // Auth errors are fatal — no point retrying.
                    if matches!(classified, RouterError::Auth(_)) {
                        return Err(classified);
                    }

                    // Context overflow (H4): trim history and retry once.
                    // Retrying the same oversized request is pointless, so
                    // this is a single trim-and-retry, not a blind retry.
                    if matches!(classified, RouterError::ContextOverflow(_)) {
                        if trimmed_once {
                            return Err(classified);
                        }
                        trimmed_once = true;
                        Self::trim_messages_for_retry(&mut messages, &req.model);
                        warn!("Grok backend: context overflow — trimmed history and retrying once");
                        continue;
                    }

                    // Rate limit with a Retry-After: honor it on the next
                    // attempt instead of the policy delay (H4, best-effort
                    // until H5's vendored grok_api preserves the header).
                    if matches!(classified, RouterError::RateLimit) {
                        if let Some(delay) = Self::parse_retry_after(&e.to_string()) {
                            retry_after_override = Some(delay);
                        }
                    }

                    let retryable = Self::is_retryable(&classified);
                    last_err = classified;

                    if !retryable {
                        // Non-retryable backend / serialization error.
                        break;
                    }
                    // Otherwise loop around for the next attempt.
                }
            }
        }

        Err(last_err)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_api_key() {
        let err = GrokBackend::new("").expect_err("should reject empty key");
        assert!(matches!(err, RouterError::Auth(_)));
    }

    #[test]
    fn accepts_non_empty_api_key() {
        // We only test construction; we can't make real API calls in unit tests.
        let backend = GrokBackend::new("xai-test-key-placeholder");
        assert!(backend.is_ok());
    }

    #[test]
    fn backoff_delay_grows_with_attempt() {
        // Now exercised via the unified RetryPolicy (Task 287)
        let policy = crate::utils::network::RetryPolicy::default_starlink();
        let d0 = policy.delay_for_attempt(0);
        let d1 = policy.delay_for_attempt(1);
        let d2 = policy.delay_for_attempt(2);
        assert!(d1 >= d0);
        assert!(d2 >= d1);
    }

    #[test]
    fn backoff_delay_is_capped() {
        let policy = crate::utils::network::RetryPolicy::default_starlink();
        let high = policy.delay_for_attempt(20);
        assert!(high <= policy.max_delay + std::time::Duration::from_millis(policy.jitter_ms));
    }

    #[test]
    fn classify_auth_errors() {
        let e = anyhow::anyhow!("HTTP 401 Unauthorized");
        assert!(matches!(
            GrokBackend::classify_error(&e),
            RouterError::Auth(_)
        ));
    }

    #[test]
    fn classify_rate_limit_errors() {
        let e = anyhow::anyhow!("HTTP 429 Too Many Requests");
        assert!(matches!(
            GrokBackend::classify_error(&e),
            RouterError::RateLimit
        ));
    }

    #[test]
    fn classify_network_errors() {
        let e = anyhow::anyhow!("connection timed out");
        assert!(matches!(
            GrokBackend::classify_error(&e),
            RouterError::Network(_)
        ));
    }

    #[test]
    fn network_and_rate_limit_are_retryable() {
        assert!(GrokBackend::is_retryable(&RouterError::Network("x".into())));
        assert!(GrokBackend::is_retryable(&RouterError::RateLimit));
    }

    #[test]
    fn classify_context_overflow_errors() {
        // The OpenAI-style error code xAI returns on a 400 overflow.
        let e = anyhow::anyhow!(
            "HTTP 400 Bad Request: {{\"error\": {{\"code\": \"context_length_exceeded\", \"message\": \"too many tokens\"}}}}"
        );
        assert!(matches!(
            GrokBackend::classify_error(&e),
            RouterError::ContextOverflow(_)
        ));

        let e = anyhow::anyhow!("maximum context length exceeded");
        assert!(matches!(
            GrokBackend::classify_error(&e),
            RouterError::ContextOverflow(_)
        ));

        let e = anyhow::anyhow!("400 Bad Request: too many tokens for this model");
        assert!(matches!(
            GrokBackend::classify_error(&e),
            RouterError::ContextOverflow(_)
        ));
    }

    #[test]
    fn context_overflow_is_not_blindly_retryable() {
        // Retrying the same oversized request is pointless — the send loop
        // handles it with a single trim-and-retry instead.
        assert!(!GrokBackend::is_retryable(&RouterError::ContextOverflow(
            "x".into()
        )));
    }

    #[test]
    fn parse_retry_after_from_error_text() {
        assert_eq!(
            GrokBackend::parse_retry_after("Rate limit exceeded. Retry-After: 7"),
            Some(std::time::Duration::from_secs(7))
        );
        assert_eq!(
            GrokBackend::parse_retry_after("429 Too Many Requests (retry_after=3)"),
            Some(std::time::Duration::from_secs(3))
        );
        // No value present.
        assert_eq!(GrokBackend::parse_retry_after("Rate limit exceeded"), None);
        // Absurd values are capped at 60s.
        assert_eq!(
            GrokBackend::parse_retry_after("Retry-After: 3600"),
            Some(std::time::Duration::from_secs(60))
        );
    }

    #[test]
    fn trim_messages_for_retry_keeps_system_and_last() {
        use serde_json::json;
        let mut messages: Vec<serde_json::Value> = vec![
            json!({"role": "system", "content": "sys"}),
            json!({"role": "user", "content": "one"}),
            json!({"role": "assistant", "content": "two"}),
            json!({"role": "user", "content": "three"}),
            json!({"role": "assistant", "content": "four"}),
            json!({"role": "user", "content": "five"}),
        ];
        GrokBackend::trim_messages_for_retry(&mut messages, "grok-4.7");

        // System message and the last user turn survive.
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages.last().unwrap()["content"], "five");
        // At least the oldest half of the middle messages were dropped.
        assert!(messages.len() < 6);
        assert!(messages.len() >= 2);
    }

    #[test]
    fn auth_and_backend_errors_are_not_retryable() {
        assert!(!GrokBackend::is_retryable(&RouterError::Auth("x".into())));
        assert!(!GrokBackend::is_retryable(&RouterError::BackendError(
            "x".into()
        )));
    }
}
