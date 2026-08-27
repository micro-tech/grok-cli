//! Network utilities for detecting and handling network issues
//!
//! This module provides utilities specifically designed for handling network
//! instability common with satellite internet connections like Starlink,
//! including connection drops, timeouts, and recovery strategies.

use anyhow::{Error, anyhow};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Patterns that indicate Starlink or satellite network issues.
///
/// These are deliberately kept fairly specific to reduce false-positive retries
/// on legitimate errors (e.g. "connection refused" can mean a real service is down
/// or a misconfiguration, not necessarily a transient satellite handover).
///
/// Broader transient errors (generic timeouts, 5xx) are handled by the
/// `RetryPolicy::should_retry` logic and callers.
const STARLINK_ERROR_PATTERNS: &[&str] = &[
    // Strong indicators of abrupt connection drops (common on Starlink handovers)
    "connection reset by peer",
    "connection reset",
    "connection dropped",
    "broken pipe",

    // Routing / reachability that often recovers quickly on satellite
    "network unreachable",
    "no route to host",
    "host is unreachable",

    // Service-side transient unavailability (model provider overloaded / flapping)
    "service temporarily unavailable",
    "the model did not respond",
    "currently unavailable",

    // Explicit "error sending request" from reqwest (very common on drops)
    "error sending request for url",
    "error sending request",

    // Cloudflare / proxy transient codes (often seen in front of xAI)
    "520", "521", "522", "523", "524",
];

/// HTTP status codes that commonly occur during satellite network issues
const SATELLITE_HTTP_ERRORS: &[u16] = &[
    502, // Bad Gateway
    503, // Service Unavailable
    504, // Gateway Timeout
    520, // Web Server Unknown Error (Cloudflare)
    521, // Web Server Is Down (Cloudflare)
    522, // Connection Timed Out (Cloudflare)
    523, // Origin Is Unreachable (Cloudflare)
    524, // A Timeout Occurred (Cloudflare)
];

/// Network drop detection result
#[derive(Debug, Clone)]
pub struct NetworkDropInfo {
    pub is_drop: bool,
    pub confidence: DropConfidence,
    pub suggested_action: SuggestedAction,
    pub retry_delay: Duration,
}

/// Confidence level in network drop detection
#[derive(Debug, Clone, PartialEq)]
pub enum DropConfidence {
    Low,
    Medium,
    High,
}

/// Suggested action to take when network drop is detected
#[derive(Debug, Clone)]
pub enum SuggestedAction {
    Retry,
    RetryWithBackoff,
    WaitAndRetry(Duration),
    CheckConnection,
    Abort,
}

/// Detect if an error indicates a network drop, particularly from Starlink
pub fn detect_network_drop(error: &Error) -> bool {
    let error_string = error.to_string().to_lowercase();

    // Check for direct error patterns
    for pattern in STARLINK_ERROR_PATTERNS {
        if error_string.contains(pattern) {
            debug!("Network drop detected: pattern '{}' found", pattern);
            return true;
        }
    }

    // Check for HTTP status codes
    for &status in SATELLITE_HTTP_ERRORS {
        if error_string.contains(&status.to_string()) {
            debug!("Network drop detected: HTTP status {} found", status);
            return true;
        }
    }

    // Check for reqwest-specific timeout errors
    if error_string.contains("reqwest") && error_string.contains("timeout") {
        debug!("Network drop detected: reqwest timeout");
        return true;
    }

    false
}

/// Analyze network error and provide detailed information
pub fn analyze_network_error(error: &Error) -> NetworkDropInfo {
    let error_string = error.to_string().to_lowercase();
    let mut confidence = DropConfidence::Low;
    let mut suggested_action = SuggestedAction::Retry;
    let mut retry_delay = Duration::from_secs(1);

    // High confidence indicators
    if error_string.contains("connection reset")
        || error_string.contains("broken pipe")
        || error_string.contains("network unreachable")
        || (error_string.contains("service") && error_string.contains("unavailable"))
        || error_string.contains("model did not respond")
    {
        confidence = DropConfidence::High;
        suggested_action = SuggestedAction::WaitAndRetry(Duration::from_secs(5));
        retry_delay = Duration::from_secs(5);
    }
    // Medium confidence indicators
    else if error_string.contains("timeout")
        || error_string.contains("connection refused")
        || SATELLITE_HTTP_ERRORS
            .iter()
            .any(|&status| error_string.contains(&status.to_string()))
    {
        confidence = DropConfidence::Medium;
        suggested_action = SuggestedAction::RetryWithBackoff;
        retry_delay = Duration::from_secs(2);
    }
    // Low confidence - generic network errors
    else if error_string.contains("network") || error_string.contains("dns") {
        confidence = DropConfidence::Low;
        suggested_action = SuggestedAction::Retry;
        retry_delay = Duration::from_secs(1);
    }

    let is_drop = confidence != DropConfidence::Low || detect_network_drop(error);

    NetworkDropInfo {
        is_drop,
        confidence,
        suggested_action,
        retry_delay,
    }
}

/// Perform a network connectivity test
pub async fn test_connectivity(timeout: Duration) -> Result<Duration, Error> {
    let start = Instant::now();

    // Test connectivity to multiple reliable endpoints
    let test_hosts = vec!["google.com:80", "cloudflare.com:80", "github.com:80"];

    for host in test_hosts {
        match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(host)).await {
            Ok(Ok(_stream)) => {
                let elapsed = start.elapsed();
                info!("Connectivity test successful to {} in {:?}", host, elapsed);
                return Ok(elapsed);
            }
            Ok(Err(e)) => {
                warn!("Failed to connect to {}: {}", host, e);
                continue;
            }
            Err(_) => {
                warn!("Timeout connecting to {}", host);
                continue;
            }
        }
    }

    Err(anyhow!("All connectivity tests failed"))
}

/// Calculate optimal retry delay based on network conditions.
///
/// This uses a satellite-friendly exponential backoff (longer tail than
/// standard clients) because Starlink and similar connections can experience
/// 20-60s handovers. Callers should use this for any transient network error.
///
/// Deprecated in favor of `RetryPolicy::delay_for_attempt`. Kept for
/// backward compatibility during the unification (Task 287).
#[deprecated(since = "0.3.0", note = "Use RetryPolicy::delay_for_attempt instead")]
pub fn calculate_retry_delay(attempt: u32) -> Duration {
    RetryPolicy::default_starlink().delay_for_attempt(attempt)
}

/// Convenience: create a policy from the current config (or sensible defaults).
pub fn default_retry_policy() -> RetryPolicy {
    // We try to read a live config if possible; otherwise fall back.
    // Most call sites that have a &Config should prefer RetryPolicy::from_config(&cfg.network).
    RetryPolicy::default_starlink()
}

/// Unified retry policy for transient network / Starlink errors.
///
/// Central place for:
/// - max retries
/// - base / max delay
/// - jitter
/// - retriable error classification
///
/// Created from `NetworkConfig` (recommended) or with explicit values for tests.
///
/// Example:
/// ```ignore
/// let policy = RetryPolicy::from_config(&config.network);
/// if policy.should_retry(attempt, &err) {
///     let delay = policy.delay_for_attempt(attempt);
///     tokio::time::sleep(delay).await;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of *retry* attempts (not counting the initial try).
    pub max_retries: u32,
    /// Base delay before first retry.
    pub base_delay: Duration,
    /// Hard cap on any computed delay.
    pub max_delay: Duration,
    /// Random jitter range in milliseconds (0 .. jitter_ms).
    pub jitter_ms: u64,
    /// When true, use longer tail backoff suitable for satellite links.
    pub starlink_mode: bool,
}

impl RetryPolicy {
    /// Construct from the hierarchical `NetworkConfig`.
    pub fn from_config(cfg: &crate::config::NetworkConfig) -> Self {
        Self {
            max_retries: cfg.max_retries,
            base_delay: Duration::from_secs(cfg.base_retry_delay.max(1)),
            max_delay: Duration::from_secs(cfg.max_retry_delay.max(5)),
            jitter_ms: cfg.jitter_ms,
            starlink_mode: cfg.starlink_optimizations,
        }
    }

    /// Conservative "Starlink-friendly" defaults (used when no config is available).
    pub fn default_starlink() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(60),
            jitter_ms: 500,
            starlink_mode: true,
        }
    }

    /// Simple defaults for unit tests (deterministic, short delays).
    pub fn for_tests() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            jitter_ms: 0,
            starlink_mode: false,
        }
    }

    /// Returns true if we should attempt a retry for this error on this attempt number.
    ///
    /// `attempt` is the *current* failure count (0 = first try just failed).
    pub fn should_retry(&self, attempt: u32, error: &anyhow::Error) -> bool {
        if attempt >= self.max_retries {
            return false;
        }
        // Use the existing detector + a few extra transient patterns.
        if detect_network_drop(error) {
            return true;
        }
        let msg = error.to_string().to_lowercase();
        msg.contains("timeout")
            || msg.contains("timed out")
            || msg.contains("reset")
            || msg.contains("connection")
            || msg.contains("503")
            || msg.contains("502")
            || msg.contains("504")
            || msg.contains("service unavailable")
    }

    /// Compute the delay before the *next* attempt (attempt is 0-based failure count).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp_factor = if self.starlink_mode {
            // Longer tail for satellite handovers (2^attempt but capped)
            1u64 << (attempt.min(5))
        } else {
            1u64 << (attempt.min(3))
        };

        let base = self.base_delay.as_secs().saturating_mul(exp_factor);
        let capped = base.min(self.max_delay.as_secs());

        let jitter = if self.jitter_ms > 0 {
            rand::random::<u64>() % (self.jitter_ms + 1)
        } else {
            0
        };

        Duration::from_millis(capped * 1000 + jitter)
    }

    /// Convenience: sleep using this policy's delay for the given attempt.
    pub async fn sleep_for_attempt(&self, attempt: u32) {
        let d = self.delay_for_attempt(attempt);
        if d > Duration::from_millis(5) {
            tokio::time::sleep(d).await;
        }
    }
}

/// Network health monitor for continuous connection quality assessment
pub struct NetworkHealthMonitor {
    consecutive_failures: u32,
    last_success: Option<Instant>,
    total_requests: u64,
    failed_requests: u64,
}

impl NetworkHealthMonitor {
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            last_success: None,
            total_requests: 0,
            failed_requests: 0,
        }
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.last_success = Some(Instant::now());
        self.total_requests += 1;
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.total_requests += 1;
        self.failed_requests += 1;
    }

    pub fn health_score(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }

        let success_rate =
            (self.total_requests - self.failed_requests) as f64 / self.total_requests as f64;

        // Penalize consecutive failures
        let consecutive_penalty = (self.consecutive_failures as f64 * 0.1).min(0.5);

        (success_rate - consecutive_penalty).max(0.0)
    }

    pub fn should_increase_timeout(&self) -> bool {
        self.consecutive_failures >= 3 || self.health_score() < 0.5
    }

    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.total_requests = 0;
        self.failed_requests = 0;
        self.last_success = None;
    }
}

impl Default for NetworkHealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_network_drop() {
        assert!(detect_network_drop(&anyhow!("Connection reset by peer")));
        assert!(detect_network_drop(&anyhow!("Network unreachable")));
        assert!(detect_network_drop(&anyhow!("HTTP 502 Bad Gateway")));
        assert!(detect_network_drop(&anyhow!(
            "Service temporarily unavailable"
        )));
        assert!(detect_network_drop(&anyhow!(
            "The model did not respond to this request"
        )));
        assert!(detect_network_drop(&anyhow!(
            "Network error: error sending request for url (https://api.x.ai/v1/chat/completions)"
        )));
        assert!(detect_network_drop(&anyhow!(
            "error sending request for url"
        )));
        assert!(!detect_network_drop(&anyhow!("Invalid API key")));
        assert!(!detect_network_drop(&anyhow!("JSON parsing error")));
    }

    #[test]
    fn test_analyze_network_error() {
        let reset_error = anyhow!("Connection reset by peer");
        let analysis = analyze_network_error(&reset_error);
        assert!(analysis.is_drop);
        assert_eq!(analysis.confidence, DropConfidence::High);

        let timeout_error = anyhow!("Request timeout");
        let analysis = analyze_network_error(&timeout_error);
        assert!(analysis.is_drop);
        assert_eq!(analysis.confidence, DropConfidence::Medium);

        let service_error =
            anyhow!("Service temporarily unavailable. The model did not respond to this request.");
        let analysis = analyze_network_error(&service_error);
        assert!(analysis.is_drop);
        assert_eq!(analysis.confidence, DropConfidence::High);
    }

    #[test]
    fn test_calculate_retry_delay_delegates() {
        // Should delegate to RetryPolicy (even if deprecated)
        let delay1 = calculate_retry_delay(1);
        let delay2 = calculate_retry_delay(2);
        assert!(delay2 >= delay1);
        assert!(delay1 >= Duration::from_millis(1000)); // base 2s + jitter
    }

    #[test]
    fn test_retry_policy_defaults() {
        let p = RetryPolicy::default_starlink();
        assert!(p.max_retries >= 3);
        assert!(p.starlink_mode);
        assert!(p.base_delay >= Duration::from_secs(1));
    }

    #[test]
    fn test_retry_policy_for_tests_is_deterministic() {
        let p = RetryPolicy::for_tests();
        assert_eq!(p.jitter_ms, 0);
        let d0 = p.delay_for_attempt(0);
        let d1 = p.delay_for_attempt(1);
        // With 0 jitter and small base, delays should be predictable
        assert!(d1 > d0);
    }

    #[test]
    fn test_retry_policy_should_retry() {
        let p = RetryPolicy::for_tests();

        assert!(p.should_retry(0, &anyhow!("connection reset by peer")));
        assert!(p.should_retry(0, &anyhow!("timeout")));
        assert!(p.should_retry(0, &anyhow!("HTTP 503")));
        assert!(!p.should_retry(0, &anyhow!("invalid api key")));

        // Respects max_retries
        assert!(!p.should_retry(p.max_retries, &anyhow!("timeout")));
    }

    #[test]
    fn test_network_health_monitor() {
        let mut monitor = NetworkHealthMonitor::new();
        assert_eq!(monitor.health_score(), 1.0);

        monitor.record_success();
        assert_eq!(monitor.health_score(), 1.0);

        monitor.record_failure();
        assert!(monitor.health_score() < 1.0);
        assert!(monitor.health_score() > 0.0);

        // Multiple consecutive failures
        monitor.record_failure();
        monitor.record_failure();
        assert!(monitor.should_increase_timeout());
    }
}
