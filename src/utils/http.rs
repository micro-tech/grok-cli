//! Centralized HTTP client for all reqwest usage.
//!
//! Provides a single shared `reqwest::Client` with connection pooling,
//! reasonable timeouts tuned for satellite (Starlink) connections,
//! and consistent configuration across the application.
//!
//! Benefits:
//! - Avoids repeated TCP/TLS handshakes (major perf win)
//! - Single place to tune timeouts, pools, headers, etc.
//! - Easier to add middleware / retry in the future (Task 281.3)

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::Client;

/// Default total timeout for HTTP requests (5 minutes).
/// Satellite links can have long handovers; keep this generous.
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Default connect timeout.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 15;

/// Default pool idle timeout (keep connections alive).
const DEFAULT_POOL_IDLE_TIMEOUT_SECS: u64 = 90;

/// Shared reqwest client used everywhere.
///
/// Initialized exactly once (via LazyLock) with connection pooling enabled.
/// All modules should use `get_http_client()` instead of `reqwest::Client::new()`.
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    #[cfg(test)]
    CLIENT_CREATION_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        .pool_idle_timeout(Duration::from_secs(DEFAULT_POOL_IDLE_TIMEOUT_SECS))
        .user_agent(
            "Mozilla/5.0 (compatible; grok-cli/0.2; +https://github.com/grok-cli/grok-cli)",
        )
        // Enable HTTP/2 where possible (good for multiplexing)
        .http2_prior_knowledge()
        .build()
        .expect("FATAL: failed to construct shared reqwest HTTP client")
});

/// Test-only counter used to assert that the shared client is created exactly once.
#[cfg(test)]
pub static CLIENT_CREATION_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Returns a reference to the single shared HTTP client.
///
/// This is the preferred way to perform HTTP requests throughout the crate.
/// The client is configured for long-lived connections and satellite resilience.
pub fn get_http_client() -> &'static Client {
    &HTTP_CLIENT
}

/// Returns a client builder pre-configured with the same sensible defaults
/// as the shared client. Useful when a call site needs to customize
/// (e.g. a much shorter timeout for health checks).
///
/// Callers should still prefer the shared client when possible.
pub fn http_client_builder() -> reqwest::ClientBuilder {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        .pool_idle_timeout(Duration::from_secs(DEFAULT_POOL_IDLE_TIMEOUT_SECS))
        .user_agent(
            "Mozilla/5.0 (compatible; grok-cli/0.2; +https://github.com/grok-cli/grok-cli)",
        )
}

/// Convenience helper: perform a GET and return the response text,
/// using the shared client + basic Starlink retry (delegates to network module).
pub async fn get_text_with_retry(url: &str, max_retries: u32) -> anyhow::Result<String> {
    use crate::utils::network::{calculate_retry_delay, detect_network_drop};

    for attempt in 0..=max_retries {
        match get_http_client().get(url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    return resp.text().await.map_err(Into::into);
                } else {
                    return Err(anyhow::anyhow!("HTTP {} for {}", resp.status(), url));
                }
            }
            Err(e) if attempt < max_retries && detect_network_drop(&anyhow::anyhow!("{}", e)) => {
                let delay = calculate_retry_delay(attempt);
                tracing::warn!(attempt = attempt + 1, url = %url, "network drop, retrying in {:?}", delay);
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_client_is_singleton() {
        let _ = get_http_client();
        let c1 = CLIENT_CREATION_COUNT.load(std::sync::atomic::Ordering::Relaxed);

        let _ = get_http_client();
        let c2 = CLIENT_CREATION_COUNT.load(std::sync::atomic::Ordering::Relaxed);

        assert_eq!(c1, c2, "get_http_client must not create multiple clients");
    }

    #[test]
    fn builder_produces_usable_client() {
        let b = http_client_builder();
        // Just ensure it builds without panic
        let _ = b.build().expect("builder should produce valid client");
    }
}
