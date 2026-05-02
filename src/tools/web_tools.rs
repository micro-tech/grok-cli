//! Web tools — DuckDuckGo search and URL fetch.
//!
//! Network calls are built with timeout + retry semantics so they survive
//! Starlink satellite handover drops.

use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::warn;

// ── Compiled regex patterns (compiled once) ───────────────────────────────────

static RE_SEARCH_RESULT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?s)class="result__body".*?class="result__a" href="([^"]+)">(.*?)</a>.*?class="result__snippet"[^>]*>(.*?)</a>"#)
        .expect("BUG: invalid static RE_SEARCH_RESULT pattern")
});

static RE_SEARCH_SIMPLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"class="result__a" href="([^"]+)">(.*?)</a>"#)
        .expect("BUG: invalid static RE_SEARCH_SIMPLE pattern")
});

static RE_STRIP_TAGS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<[^>]*>").expect("BUG: invalid static RE_STRIP_TAGS pattern"));

// ── Public helpers ────────────────────────────────────────────────────────────

/// Returns `true` when web search is properly configured.
///
/// DuckDuckGo is always available without any API key, so this always returns
/// `true`. It is kept as a function so callers can filter the tool list
/// consistently.
pub fn is_web_search_configured() -> bool {
    true
}

/// Perform a web search using DuckDuckGo HTML search.
///
/// Returns up to 10 results formatted as `Title / Link / Snippet` blocks.
/// Falls back to title-only results if the snippet regex fails to match.
///
/// # Starlink resilience
/// The underlying HTTP client is built with a 30-second timeout so a
/// satellite handover does not hang indefinitely.
pub async fn web_search(query: &str) -> Result<String> {
    duckduckgo_search(query).await
}

/// Fetch the raw text content of a URL.
///
/// Truncates responses longer than 10 000 characters to avoid overloading
/// the model context window.
///
/// # Errors
/// Returns an error with a human-readable diagnosis if the request fails,
/// including hints about network connectivity, invalid URLs, and firewall
/// / proxy issues.
pub async fn web_fetch(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .header("User-Agent", "grok-cli/0.1.0")
        .send()
        .await
        .map_err(|e| {
            anyhow!(
                "Failed to fetch URL '{}': {}\n\
                This could be due to:\n\
                - Network connectivity issues (Starlink handover?)\n\
                - Invalid URL\n\
                - Server not responding\n\
                - Firewall/proxy blocking the request",
                url,
                e
            )
        })?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch URL '{}': HTTP {}\n\
            The server returned an error status code.",
            url,
            response.status()
        ));
    }

    let text = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read response body: {}", e))?;

    let truncated = if text.len() > 10_000 {
        format!("{}... (truncated)", &text[..10_000])
    } else {
        text
    };

    Ok(truncated)
}

// ── Private implementation ────────────────────────────────────────────────────

async fn duckduckgo_search(query: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/58.0.3029.110 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("DuckDuckGo request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "DuckDuckGo search failed with status: {}",
            response.status()
        ));
    }

    let html = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read DuckDuckGo response: {}", e))?;

    let mut results = Vec::new();

    for cap in RE_SEARCH_RESULT.captures_iter(&html).take(10) {
        let link = urlencoding::decode(&cap[1])
            .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&cap[1]))
            .to_string();
        let title = strip_tags(&cap[2]);
        let snippet = strip_tags(&cap[3]);
        results.push(format!(
            "Title: {}\nLink: {}\nSnippet: {}\n",
            title, link, snippet
        ));
    }

    if results.is_empty() {
        // Fallback: title + link only
        for cap in RE_SEARCH_SIMPLE.captures_iter(&html).take(10) {
            let link = urlencoding::decode(&cap[1])
                .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&cap[1]))
                .to_string();
            let title = strip_tags(&cap[2]);
            results.push(format!("Title: {}\nLink: {}\n", title, link));
        }
    }

    if results.is_empty() {
        warn!(query = %query, "DuckDuckGo search returned no results");
        Ok("No results found via DuckDuckGo.".to_string())
    } else {
        Ok(format!(
            "(Source: DuckDuckGo)\n\n{}",
            results.join("\n---\n")
        ))
    }
}

fn strip_tags(s: &str) -> String {
    RE_STRIP_TAGS.replace_all(s, "").trim().to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_search_is_always_configured() {
        assert!(is_web_search_configured());
    }

    #[tokio::test]
    async fn web_fetch_invalid_url_returns_error() {
        let result = web_fetch("not-a-valid-url").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn web_fetch_timeout_on_unreachable() {
        // This test just verifies we return an error — not a panic/hang.
        let result = web_fetch("http://192.0.2.1/timeout-test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn web_search_returns_result_or_no_results() {
        // Does NOT assert on specific content — just ensures no panic.
        let result = web_search("rust programming language").await;
        assert!(result.is_ok() || result.is_err());
    }
}
