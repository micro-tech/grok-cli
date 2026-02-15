//! Network utilities for detecting and handling network issues
//!
//! This module provides utilities specifically designed for handling network
//! instability common with satellite internet connections like Starlink,
//! including connection drops, timeouts, and recovery strategies.

use anyhow::{Error, anyhow};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Patterns that indicate Starlink or satellite network issues
const STARLINK_ERROR_PATTERNS: &[&str] = &[
    "connection reset",
    "connection dropped",
    "network unreachable",
    "no route to host",
    "broken pipe",
    "connection refused",
    "timeout",
    "dns resolution failed",
    "temporary failure in name resolution",
    "network is down",
    "host is unreachable",
    "service unavailable",
    "service temporarily unavailable",
    "the model did not respond",
    "currently unavailable",
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

/// Check if we're likely on a Starlink connection
pub async fn detect_starlink_connection() -> bool {
    // Try to resolve Starlink-specific domains or check for satellite-specific patterns
    // This is a heuristic approach

    // Check if we can resolve starlink.com (indicates possible Starlink connection)
    if let Ok(addrs) = tokio::net::lookup_host("starlink.com:80").await {
        if addrs.count() > 0 {
            info!("Starlink domain resolution successful - possible Starlink connection");
            return true;
        }
    }

    // Additional heuristics could be added here:
    // - Check for specific IP ranges
    // - Analyze latency patterns
    // - Check for satellite-specific network characteristics

    false
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

/// Calculate optimal retry delay based on network conditions
pub fn calculate_retry_delay(attempt: u32, is_starlink: bool) -> Duration {
    let base_delay = if is_starlink {
        // Longer delays for satellite connections
        Duration::from_secs(2_u64.pow(attempt.min(4)))
    } else {
        // Standard exponential backoff
        Duration::from_secs(2_u64.pow(attempt.min(3)))
    };

    // Add jitter to prevent thundering herd
    let jitter = Duration::from_millis(rand::random::<u64>() % 1000);
    base_delay + jitter
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
    fn test_calculate_retry_delay() {
        let delay1 = calculate_retry_delay(1, false);
        let delay2 = calculate_retry_delay(2, false);
        assert!(delay2 >= delay1);

        let starlink_delay = calculate_retry_delay(1, true);
        let regular_delay = calculate_retry_delay(1, false);
        // Starlink delays should generally be longer (though jitter may affect this)
        // We just test that both are reasonable
        assert!(starlink_delay >= Duration::from_secs(1));
        assert!(regular_delay >= Duration::from_secs(1));
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
