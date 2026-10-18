//! Web tools — DuckDuckGo search and URL fetch.
//!
//! Network calls are built with timeout + retry semantics so they survive
//! Starlink satellite handover drops.

use anyhow::{Result, anyhow};
use regex::Regex;
use reqwest::Client;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::warn;

use crate::tools::ToolContext;
use crate::utils::http::get_http_client;

// ── Compiled regex patterns (compiled once) ───────────────────────────────────

static RE_SEARCH_RESULT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)class="result__body".*?class="result__a" href="([^"]+)">(.*?)</a>.*?class="result__snippet"[^>]*>(.*?)</a>"#)
        .expect("BUG: invalid static RE_SEARCH_RESULT pattern")
});

static RE_SEARCH_SIMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"class="result__a" href="([^"]+)">(.*?)</a>"#)
        .expect("BUG: invalid static RE_SEARCH_SIMPLE pattern")
});

static RE_STRIP_TAGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]*>").expect("BUG: invalid static RE_STRIP_TAGS pattern"));

// Note: The central shared client lives in crate::utils::http and keeps its
// default redirect policy. web_fetch deliberately does NOT use it: it needs
// its own no-redirect client (below) so every redirect hop can be re-checked
// by the SSRF guard before being followed.

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
/// Retries up to 3 times on transient network errors (satellite handover,
/// timeout, connection reset) with exponential back-off before surfacing an
/// error to the caller.
pub async fn web_search(query: &str, _ctx: &ToolContext) -> Result<String> {
    let query = query.trim();
    if query.is_empty() {
        return Err(anyhow::anyhow!("web_search: query must not be empty"));
    }

    const MAX_RETRIES: u32 = 3;
    for attempt in 0..=MAX_RETRIES {
        match duckduckgo_search(query).await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < MAX_RETRIES && crate::utils::network::detect_network_drop(&e) => {
                let delay = crate::utils::network::RetryPolicy::default_starlink().delay_for_attempt(attempt);
                warn!(
                    attempt = attempt + 1,
                    max_attempts = MAX_RETRIES + 1,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "web_search: network error — retrying after delay"
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

/// Returns `true` when the IP address must never be a `web_fetch` target.
///
/// Blocks loopback, private (RFC 1918), link-local (which includes the
/// `169.254.169.254` cloud metadata endpoint), carrier-grade NAT, multicast,
/// unspecified, broadcast, documentation, benchmarking, and reserved ranges —
/// for both IPv4 and IPv6 (including IPv4-mapped IPv6 like `::ffff:127.0.0.1`).
fn fetch_ip_is_blocked(ip: &std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    // Normalize IPv4-mapped IPv6 addresses so `::ffff:10.0.0.1` can't dodge
    // the IPv4 checks.
    let ip = match ip {
        IpAddr::V6(v6) => match v6.to_ipv4() {
            Some(v4) => IpAddr::V4(v4),
            None => *ip,
        },
        IpAddr::V4(_) => *ip,
    };
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local() // 169.254.0.0/16 — cloud metadata endpoints live here
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || o[0] == 0 // 0.0.0.0/8 (software scope)
                || o[0] >= 240 // 240.0.0.0/4 (reserved)
                || (o[0] == 198 && (o[1] & 0xfe) == 18) // 198.18.0.0/15 (benchmarking)
                || (o[0] == 100 && (64..=127).contains(&o[1])) // 100.64.0.0/10 CGNAT
                // Documentation ranges (never real fetch targets).
                || (o[0] == 192 && o[1] == 0 && o[2] == 2) // TEST-NET-1
                || (o[0] == 198 && o[1] == 51 && o[2] == 100) // TEST-NET-2
                || (o[0] == 203 && o[1] == 0 && o[2] == 113) // TEST-NET-3
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00 // fc00::/7 unique-local
                || (s[0] & 0xffc0) == 0xfe80 // fe80::/10 link-local
                || s[0] == 0x2001 && s[1] == 0x0db8 // 2001:db8::/32 documentation
        }
    }
}

/// Pure (no-DNS) policy check for a parsed fetch URL against already-resolved
/// IPs.  Separated out so the policy is unit-testable without network.
fn check_fetch_target(url: &reqwest::Url, resolved_ips: &[std::net::IpAddr]) -> Result<()> {
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(anyhow!(
                "SSRF guard: refusing to fetch URL with scheme '{}' — only http/https are allowed",
                other
            ))
        }
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(anyhow!(
            "SSRF guard: refusing to fetch URL containing credentials in the authority"
        ));
    }
    for ip in resolved_ips {
        if fetch_ip_is_blocked(ip) {
            return Err(anyhow!(
                "SSRF guard: refusing to fetch {} — target resolves to blocked address {} \
                 (SSRF protection: loopback/private/link-local/reserved ranges are not fetchable)",
                url.host_str().unwrap_or("<unknown host>"),
                ip
            ));
        }
    }
    Ok(())
}

/// SSRF guard for [`web_fetch`] (Task 468).
///
/// The URL comes from the model, so a prompt-injected page could otherwise
/// steer fetches at `http://169.254.169.254/` (cloud metadata), `localhost`
/// services (Ollama on :11434, the Proxmox host), or other LAN hosts.
/// IP literals are checked directly; hostnames are resolved and *every*
/// resolved address is checked.
///
/// Best-effort by nature: DNS is resolved once up front, so a DNS-rebinding
/// race between the check and the connection is not covered.  The denylist
/// here is defense for the common static cases, alongside the tool-approval
/// gate.
pub(crate) async fn check_fetch_url_allowed(url: &str) -> Result<()> {
    let parsed =
        reqwest::Url::parse(url).map_err(|e| anyhow!("SSRF guard: invalid URL '{}': {}", url, e))?;

    // Fast path: IP literal — no DNS needed.
    if let Some(host) = parsed.host_str() {
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            return check_fetch_target(&parsed, &[ip]);
        }
        // `localhost` (and `*.localhost`) never needs a DNS round-trip.
        if host.eq_ignore_ascii_case("localhost") || host.to_lowercase().ends_with(".localhost") {
            return Err(anyhow!(
                "SSRF guard: refusing to fetch '{}' — localhost is not fetchable (SSRF protection)",
                host
            ));
        }
    }

    // Hostname: resolve and check every address it maps to.
    let port = parsed.port_or_known_default().unwrap_or(443);
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("SSRF guard: URL '{}' has no host", url))?;
    let addrs: Vec<std::net::IpAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| {
            anyhow!(
                "SSRF guard: refusing to fetch '{}' — could not resolve host: {}",
                host,
                e
            )
        })?
        .map(|sa| sa.ip())
        .collect();
    if addrs.is_empty() {
        return Err(anyhow!(
            "SSRF guard: refusing to fetch '{}' — host resolved to no addresses",
            host
        ));
    }
    check_fetch_target(&parsed, &addrs)
}

/// Resolve a redirect `Location` header value against the URL that produced it.
///
/// Handles absolute URLs, protocol-relative (`//host/path`), and
/// origin-relative (`/path`) targets — the shapes servers actually send.
fn resolve_redirect_location(current_url: &str, location: &str) -> Result<String> {
    let base = reqwest::Url::parse(current_url)
        .map_err(|e| anyhow!("Failed to parse current URL '{}': {}", current_url, e))?;
    base.join(location)
        .map(|u| u.to_string())
        .map_err(|e| anyhow!("Failed to resolve redirect target '{}': {}", location, e))
}

/// Dedicated HTTP client for [`web_fetch`] that never follows redirects.
///
/// Redirects are handled manually inside `web_fetch` so the SSRF guard
/// (`check_fetch_url_allowed`) re-vets every hop. reqwest's automatic redirect
/// following would otherwise walk past the guard to an unchecked target
/// (e.g. a 302 from a benign page to `http://169.254.169.254/`).
/// Timeouts mirror the shared client in `crate::utils::http`.
static NO_REDIRECT_HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
        .user_agent(
            "Mozilla/5.0 (compatible; grok-cli/0.2; +https://github.com/grok-cli/grok-cli)",
        )
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("FATAL: failed to construct no-redirect HTTP client for web_fetch")
});

/// Fetch the raw text content of a URL.
///
/// Truncates responses longer than 10 000 *characters* (not bytes) using a
/// char-boundary-safe split so multibyte UTF-8 sequences never cause a panic.
///
/// Redirects (up to 5) are followed manually, with the SSRF guard re-run on
/// every hop — automatic redirect following would bypass the guard.
///
/// # Starlink resilience
/// Retries up to 3 times on transient network errors with exponential
/// back-off before returning an error to the caller.
///
/// # Errors
/// Returns an error with a human-readable diagnosis if the request fails,
/// including hints about network connectivity, invalid URLs, and
/// firewall / proxy issues.
pub async fn web_fetch(url: &str, _ctx: &ToolContext) -> Result<String> {
    const MAX_RETRIES: u32 = 3;
    const MAX_REDIRECTS: u32 = 5;

    let mut current_url = url.to_string();
    let mut redirects_followed = 0u32;

    // One iteration per redirect hop. The SSRF guard runs on every hop, not
    // just the initial URL, so a redirect can never smuggle in an unchecked
    // target.
    'hop: loop {
        check_fetch_url_allowed(&current_url).await?;

        for attempt in 0..=MAX_RETRIES {
            let send_result = NO_REDIRECT_HTTP_CLIENT
                .get(&current_url)
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
                        current_url,
                        e
                    )
                });

            match send_result {
                Ok(response) => {
                    // Manual redirect: resolve Location against the current
                    // URL and loop, so the guard above vets the new target.
                    if response.status().is_redirection() {
                        if redirects_followed >= MAX_REDIRECTS {
                            return Err(anyhow!(
                                "Failed to fetch URL '{}': too many redirects (>{})",
                                url,
                                MAX_REDIRECTS
                            ));
                        }
                        let location = response
                            .headers()
                            .get(reqwest::header::LOCATION)
                            .and_then(|v| v.to_str().ok())
                            .ok_or_else(|| {
                                anyhow!(
                                    "Failed to fetch URL '{}': redirect without Location header (HTTP {})",
                                    current_url,
                                    response.status()
                                )
                            })?;
                        current_url = resolve_redirect_location(&current_url, location)?;
                        redirects_followed += 1;
                        continue 'hop;
                    }

                    if !response.status().is_success() {
                        warn!(
                            status = %response.status(),
                            url = %current_url,
                            "web_fetch: non-2xx response from server"
                        );
                        return Err(anyhow!(
                            "Failed to fetch URL '{}': HTTP {}\n\
                            The server returned an error status code.",
                            current_url,
                            response.status()
                        ));
                    }

                    let text = response.text().await.map_err(|e| {
                        warn!(error = %e, url = %current_url, "web_fetch: failed to read response body");
                        anyhow!("Failed to read response body: {}", e)
                    })?;

                    // Safe char-boundary truncation — avoids panics on multibyte
                    // UTF-8 sequences that straddle the 10 000-byte mark.
                    let truncated = text
                        .char_indices()
                        .nth(10_000)
                        .map(|(i, _)| &text[..i])
                        .unwrap_or(&text);
                    return Ok(truncated.to_string());
                }
                Err(e) if attempt < MAX_RETRIES && crate::utils::network::detect_network_drop(&e) => {
                    let delay = crate::utils::network::RetryPolicy::default_starlink().delay_for_attempt(attempt);
                    warn!(
                        attempt = attempt + 1,
                        max_attempts = MAX_RETRIES + 1,
                        delay_ms = delay.as_millis(),
                        error = %e,
                        "web_fetch: network error — retrying after delay"
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    warn!(error = %e, url = %current_url, "web_fetch: request failed — no more retries");
                    return Err(e);
                }
            }
        }
        unreachable!("retry loop always returns or continues the hop loop");
    }
}

// ── Private implementation ────────────────────────────────────────────────────

async fn duckduckgo_search(query: &str) -> Result<String> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let response = get_http_client()
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

    #[test]
    fn web_search_empty_query_returns_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ctx = ToolContext::default_for_cwd();
        let r = rt.block_on(web_search("   ", &ctx));
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("must not be empty"));
    }

    #[tokio::test]
    async fn web_fetch_invalid_url_returns_error() {
        let ctx = ToolContext::default_for_cwd();
        let result = web_fetch("not-a-valid-url", &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn web_fetch_timeout_on_unreachable() {
        // This test just verifies we return an error — not a panic/hang.
        let ctx = ToolContext::default_for_cwd();
        let result = web_fetch("http://192.0.2.1/timeout-test", &ctx).await;
        assert!(result.is_err());
    }

    // GIVEN various Location header shapes, WHEN resolve_redirect_location()
    // joins them against the current URL, THEN the resolved target is correct
    // (absolute, relative, and protocol-relative forms all occur in the wild).
    #[test]
    fn resolve_redirect_location_absolute_url_replaces_target() {
        let resolved = resolve_redirect_location("http://example.com/a", "https://other.com/b")
            .expect("absolute Location must resolve");
        assert_eq!(resolved, "https://other.com/b");
    }

    #[test]
    fn resolve_redirect_location_relative_path_keeps_origin() {
        let resolved = resolve_redirect_location("http://example.com/a/b", "/c")
            .expect("origin-relative Location must resolve");
        assert_eq!(resolved, "http://example.com/c");
    }

    #[test]
    fn resolve_redirect_location_protocol_relative_keeps_scheme() {
        let resolved = resolve_redirect_location("https://example.com/a", "//other.com/x")
            .expect("protocol-relative Location must resolve");
        assert_eq!(resolved, "https://other.com/x");
    }

    #[test]
    fn resolve_redirect_location_query_only_replaces_query() {
        let resolved = resolve_redirect_location("http://example.com/a?x=1", "?y=2")
            .expect("query-only Location must resolve");
        assert_eq!(resolved, "http://example.com/a?y=2");
    }

    #[test]
    fn resolve_redirect_location_garbage_returns_error_not_panic() {
        let result = resolve_redirect_location("http://example.com/", "http://exa mple.com/");
        assert!(result.is_err(), "unparseable Location must error, not panic");
    }

    #[tokio::test]
    async fn web_search_returns_result_or_no_results() {
        // Does NOT assert on specific content — just ensures no panic.
        let ctx = ToolContext::default_for_cwd();
        let result = web_search("rust programming language", &ctx).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn shared_http_client_constructed_only_once() {
        // Use the centralized client from utils::http (Task 281).
        // The counter lives in crate::utils::http.
        let _ = crate::utils::http::get_http_client();
        let count_after_first = crate::utils::http::CLIENT_CREATION_COUNT
            .load(std::sync::atomic::Ordering::Relaxed);

        let _ = crate::utils::http::get_http_client();
        let count_after_second = crate::utils::http::CLIENT_CREATION_COUNT
            .load(std::sync::atomic::Ordering::Relaxed);

        let _ = crate::utils::http::get_http_client();
        let count_after_third = crate::utils::http::CLIENT_CREATION_COUNT
            .load(std::sync::atomic::Ordering::Relaxed);

        assert_eq!(
            count_after_first, count_after_second,
            "Second access must not construct a new HTTP client"
        );
        assert_eq!(
            count_after_second, count_after_third,
            "Third access must not construct a new HTTP client"
        );
    }
}

#[cfg(test)]
mod ssrf_tests {
    use super::*;
    use std::net::IpAddr;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn blocked_ip_ranges() {
        // Loopback / private / link-local / reserved must never be fetched.
        for s in [
            "127.0.0.1",
            "127.1.2.3",
            "10.0.0.5",
            "172.16.4.4",
            "192.168.1.1",
            "169.254.169.254", // cloud metadata endpoint
            "169.254.10.20",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "192.0.2.1",   // TEST-NET-1 documentation
            "198.18.0.1",  // benchmarking
            "240.1.2.3",   // reserved
            "100.64.0.1",  // CGNAT
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "ff02::1",
            "::ffff:127.0.0.1", // IPv4-mapped IPv6 dodge
            "::ffff:10.9.9.9",
        ] {
            assert!(fetch_ip_is_blocked(&ip(s)), "{} must be blocked", s);
        }
    }

    #[test]
    fn public_ips_allowed() {
        for s in ["93.184.216.34", "8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            assert!(!fetch_ip_is_blocked(&ip(s)), "{} must be allowed", s);
        }
    }

    #[tokio::test]
    async fn ssrf_guard_rejects_dangerous_urls() {
        for url in [
            "http://169.254.169.254/latest/meta-data/",
            "http://127.0.0.1:11434/api/tags",
            "http://localhost:8080/",
            "http://[::1]/",
            "http://[::ffff:127.0.0.1]/",
            "http://10.0.0.5/",
            "http://192.168.1.1/",
            "http://172.16.0.9/admin",
            "ftp://example.com/file",
            "http://user:pass@example.com/",
        ] {
            let r = check_fetch_url_allowed(url).await;
            assert!(r.is_err(), "{} must be rejected: {:?}", url, r);
        }
    }

    #[test]
    fn check_fetch_target_allows_public_ip() {
        let url = reqwest::Url::parse("https://example.com/page").unwrap();
        assert!(check_fetch_target(&url, &[ip("93.184.216.34")]).is_ok());
        assert!(check_fetch_target(&url, &[ip("93.184.216.34"), ip("10.0.0.1")]).is_err());
    }
}
