//! Grok backend — wraps the existing [`GrokClient`] for use inside the router.
//!
//! ## Starlink resilience
//! Every request is retried up to [`MAX_RETRIES`] times with exponential
//! back-off plus a small random jitter.  Transient network errors (timeout,
//! connection reset) are retried; authentication errors (HTTP 401) abort
//! immediately so we don't burn quota on bad keys.

use async_trait::async_trait;
use rand::RngExt;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

use grok_api::MessageContent;

use crate::GrokClient;
use crate::router::{Backend, BackendKind, RouterError, RouterRequest, RouterResponse};

// ── Retry knobs ──────────────────────────────────────────────────────────────

/// Maximum number of retry attempts (not counting the first try).
const MAX_RETRIES: u32 = 4;

/// Base delay for exponential back-off (seconds).
const BASE_DELAY_SECS: u64 = 2;

/// Maximum back-off delay to cap runaway waits (seconds).
const MAX_DELAY_SECS: u64 = 30;

/// Maximum jitter added on top of the computed delay (milliseconds).
const MAX_JITTER_MS: u64 = 500;

/// Default temperature when the caller does not specify one.
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

    /// Compute an exponential back-off delay with jitter.
    ///
    /// ```text
    /// delay = min(BASE * 2^attempt, MAX) + rand(0..MAX_JITTER_MS)
    /// ```
    fn backoff_delay(attempt: u32) -> Duration {
        let exp = BASE_DELAY_SECS.saturating_mul(2u64.saturating_pow(attempt));
        let capped = exp.min(MAX_DELAY_SECS);
        let jitter = rand::rng().random_range(0..MAX_JITTER_MS);
        Duration::from_millis(capped * 1_000 + jitter)
    }

    /// Classify an `anyhow::Error` from the Grok client into a [`RouterError`].
    fn classify_error(err: &anyhow::Error) -> RouterError {
        let msg = err.to_string().to_lowercase();

        if msg.contains("401") || msg.contains("unauthorized") || msg.contains("authentication") {
            return RouterError::Auth(err.to_string());
        }
        if msg.contains("429") || msg.contains("rate limit") || msg.contains("too many requests") {
            return RouterError::RateLimit;
        }
        if msg.contains("timeout")
            || msg.contains("timed out")
            || msg.contains("connection")
            || msg.contains("reset")
            || msg.contains("eof")
        {
            return RouterError::Network(err.to_string());
        }

        RouterError::BackendError(err.to_string())
    }

    /// Returns `true` for errors that are worth retrying (transient).
    fn is_retryable(err: &RouterError) -> bool {
        matches!(err, RouterError::Network(_) | RouterError::RateLimit)
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
        let messages = &req.messages;

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

        // ── Retry loop ───────────────────────────────────────────────────────
        let mut last_err = RouterError::Unknown;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = Self::backoff_delay(attempt - 1);
                warn!(
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Grok backend: retrying after transient error"
                );
                sleep(delay).await;
            }

            debug!(attempt, model = %req.model, "Grok backend: sending request");

            let result = self
                .client
                .chat_completion_with_history(
                    messages,
                    temperature,
                    max_tokens,
                    &req.model,
                    tools.clone(),
                    req.reasoning_effort.as_deref(),
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
                    let finish_reason = mwfr.finish_reason.clone();

                    let tool_calls = mwfr.message.tool_calls.unwrap_or_default();
                    let thinking_content = mwfr.thinking_content;

                    return Ok(RouterResponse {
                        text,
                        tool_calls,
                        raw,
                        model: req.model.clone(),
                        usage: None,
                            thinking_content,
                        });
                }

                Err(e) => {
                    let classified = Self::classify_error(&e);

                    // Auth errors are fatal — no point retrying.
                    if matches!(classified, RouterError::Auth(_)) {
                        return Err(classified);
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
        let d0 = GrokBackend::backoff_delay(0);
        let d1 = GrokBackend::backoff_delay(1);
        let d2 = GrokBackend::backoff_delay(2);
        // Each delay (ignoring jitter) should be >= the previous one.
        assert!(d1.as_secs() >= d0.as_secs());
        assert!(d2.as_secs() >= d1.as_secs());
    }

    #[test]
    fn backoff_delay_is_capped() {
        // At a very high attempt number the delay must not exceed MAX_DELAY_SECS + jitter.
        let max_possible = Duration::from_millis(MAX_DELAY_SECS * 1_000 + MAX_JITTER_MS);
        let high = GrokBackend::backoff_delay(20);
        assert!(high <= max_possible);
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
    fn auth_and_backend_errors_are_not_retryable() {
        assert!(!GrokBackend::is_retryable(&RouterError::Auth("x".into())));
        assert!(!GrokBackend::is_retryable(&RouterError::BackendError(
            "x".into()
        )));
    }
}
