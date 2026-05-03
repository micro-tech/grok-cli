//! AI-powered conversation compressor.
//!
//! Calls the Grok API with a structured prompt to summarize a batch of
//! conversation messages into a 2–3 sentence summary plus bullet-point
//! key facts.  Applies Starlink-safe retries (3 attempts, 5 s / 10 s / 20 s).

use anyhow::anyhow;
use tracing::{debug, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum number of retry attempts on transient errors.
const MAX_RETRIES: u32 = 3;

/// Delay in seconds between successive retry attempts.
const RETRY_DELAYS: [u64; 3] = [5, 10, 20];

/// Maximum transcript length in characters before truncation.
const MAX_TRANSCRIPT_CHARS: usize = 60_000;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Summarize a batch of conversation messages into a short text summary and
/// a list of bullet-point key facts, using the Grok API.
///
/// Returns `(summary, key_facts)` on success.
///
/// # Short-circuit
///
/// If `messages` is empty the function returns immediately without making any
/// API call, returning a placeholder summary and an empty fact list.
///
/// # Starlink resilience
///
/// Transient errors are retried up to [`MAX_RETRIES`] times with exponential
/// back-off: 5 s → 10 s → 20 s.  If all attempts fail the last error is
/// returned, wrapped with context.
pub async fn compress(
    messages: &[serde_json::Value],
    router: &crate::router::AppRouter,
    model: &str,
) -> anyhow::Result<(String, Vec<String>)> {
    // ── Short-circuit ─────────────────────────────────────────────────────────
    if messages.is_empty() {
        return Ok(("(no messages to summarize)".to_string(), vec![]));
    }

    // ── Build compact transcript ──────────────────────────────────────────────
    let mut parts: Vec<String> = Vec::with_capacity(messages.len());
    for m in messages {
        let role = m["role"].as_str().unwrap_or("unknown");
        let content = m["content"].as_str().unwrap_or("[tool call / result]");
        parts.push(format!("[{}]: {}", role.to_uppercase(), content));
    }
    let raw = parts.join("\n\n");

    // Guard against enormous transcripts — take up to MAX_TRANSCRIPT_CHARS chars.
    let transcript: String = if raw.chars().count() > MAX_TRANSCRIPT_CHARS {
        raw.chars().take(MAX_TRANSCRIPT_CHARS).collect()
    } else {
        raw
    };

    // ── Prompts ───────────────────────────────────────────────────────────────
    let system_prompt = "You are a context summarizer. Your output must follow this EXACT format:\n\
SUMMARY: <2-3 sentences describing what was discussed and accomplished>\n\
FACTS:\n\
- <fact 1>\n\
- <fact 2>\n\
(up to 10 facts)\n\
Do not include any other text.";

    let user_message = format!(
        "Summarize this conversation excerpt:\n\n---\n{}\n---",
        transcript
    );

    let summarizer_messages: Vec<serde_json::Value> = vec![
        serde_json::json!({"role": "system", "content": system_prompt}),
        serde_json::json!({"role": "user",   "content": user_message}),
    ];

    // ── Retry loop ────────────────────────────────────────────────────────────
    let mut last_err: anyhow::Error = anyhow!("compress: no attempts made");

    for attempt in 0..MAX_RETRIES {
        match router
            .chat_completion_with_history(&summarizer_messages, 0.3, 1024, model, None)
            .await
        {
            Ok(resp) => {
                let text = crate::content_to_string(resp.message.content.as_ref());
                let (summary, facts) = parse_summary_response(&text);

                debug!(
                    attempt = attempt + 1,
                    summary_len = summary.len(),
                    facts_count = facts.len(),
                    "ContextCompressor: summarization successful"
                );

                return Ok((summary, facts));
            }
            Err(e) => {
                last_err = e;
                if attempt + 1 < MAX_RETRIES {
                    let delay_secs = RETRY_DELAYS[attempt as usize];
                    warn!(
                        attempt = attempt + 1,
                        max_retries = MAX_RETRIES,
                        delay_secs,
                        error = %last_err,
                        "ContextCompressor: transient error, retrying after delay"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }

    Err(last_err.context(format!(
        "ContextCompressor: all {} attempts exhausted",
        MAX_RETRIES
    )))
}

// ─────────────────────────────────────────────────────────────────────────────
// Response parser
// ─────────────────────────────────────────────────────────────────────────────

/// Parse the structured summarizer response into `(summary, key_facts)`.
///
/// Expected format:
/// ```text
/// SUMMARY: <text describing the conversation>
/// FACTS:
/// - fact one
/// - fact two
/// ```
///
/// If no `SUMMARY:` line is found, falls back to the first 200 characters of
/// the raw text so callers always receive a non-empty string for non-empty input.
fn parse_summary_response(text: &str) -> (String, Vec<String>) {
    let mut summary = String::new();
    let mut facts: Vec<String> = Vec::new();
    let mut in_facts = false;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("SUMMARY:") {
            summary = rest.trim().to_string();
            in_facts = false;
        } else if line.trim() == "FACTS:" {
            in_facts = true;
        } else if in_facts && let Some(rest) = line.strip_prefix("- ") {
            facts.push(rest.trim().to_string());
        }
    }

    // Fallback: if no SUMMARY: was found, take the first 200 chars of the raw text.
    if summary.is_empty() {
        summary = text.chars().take(200).collect();
    }

    (summary, facts)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_gives_fallback() {
        // An unformatted response (no SUMMARY: prefix) should fall back to
        // returning the first 200 chars of the raw text as the summary.
        let unformatted = "The user asked about async Rust and we talked about tokio.";
        let (summary, facts) = parse_summary_response(unformatted);
        assert!(
            !summary.is_empty(),
            "fallback should produce a non-empty summary for non-empty input"
        );
        assert_eq!(
            summary, unformatted,
            "fallback should return the full text when it is under 200 chars"
        );
        assert!(facts.is_empty(), "no facts expected for unformatted input");
    }

    #[test]
    fn parse_well_formed_response() {
        let text = "SUMMARY: We discussed Rust memory safety and ownership.\n\
FACTS:\n\
- Rust uses borrowing\n\
- No garbage collector\n\
- Zero-cost abstractions";

        let (summary, facts) = parse_summary_response(text);

        assert_eq!(
            summary, "We discussed Rust memory safety and ownership.",
            "summary should be the text after 'SUMMARY:'"
        );
        assert_eq!(facts.len(), 3, "should extract three facts");
        assert!(facts.contains(&"Rust uses borrowing".to_string()));
        assert!(facts.contains(&"No garbage collector".to_string()));
        assert!(facts.contains(&"Zero-cost abstractions".to_string()));
    }

    #[tokio::test]
    async fn compress_empty_messages_returns_placeholder() {
        // The empty-input branch short-circuits before any router call, so we
        // can use any non-empty API key — construction succeeds, and the
        // function returns before ever touching the network.
        let router = crate::router::AppRouter::new("xai-placeholder-key-for-test", 30)
            .expect("AppRouter construction should succeed with any non-empty key");

        let result = compress(&[], &router, "grok-3-mini").await;
        assert!(result.is_ok(), "compress([]) should not error");
        let (summary, facts) = result.unwrap();
        assert_eq!(
            summary, "(no messages to summarize)",
            "placeholder summary expected for empty input"
        );
        assert!(facts.is_empty(), "no facts expected for empty input");
    }
}
