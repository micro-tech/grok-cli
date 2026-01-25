//! Grok API client module for X/Twitter API integration
//!
//! This module provides a robust HTTP client for interacting with Grok AI
//! through the X (Twitter) API, with built-in retry logic, timeout handling,
//! and error recovery for network instability.

pub mod grok;

use anyhow::{Result, anyhow};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

/// API response wrapper for Grok completions
#[derive(Debug, Serialize, Deserialize)]
pub struct GrokResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Error types for Grok API interactions
#[derive(Debug, thiserror::Error)]
pub enum GrokApiError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication failed: Invalid API key")]
    Authentication,

    #[error("Rate limit exceeded. Please try again later")]
    RateLimit,

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Request timeout after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Connection dropped - Starlink network instability detected")]
    NetworkDrop,

    #[error("Maximum retries ({max_retries}) exceeded")]
    MaxRetriesExceeded { max_retries: u32 },
}

// Removed duplicate From implementation - anyhow already provides this via StdError

/// Base HTTP client configuration for robust network handling
pub struct ApiClient {
    client: Client,
    timeout_secs: u64,
    max_retries: u32,
}

impl ApiClient {
    pub fn new(timeout_secs: u64, max_retries: u32) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .user_agent("grok-cli/0.1.0")
            .build()?;

        Ok(Self {
            client,
            timeout_secs,
            max_retries,
        })
    }

    /// Execute HTTP request with retry logic and network drop detection
    pub async fn execute_with_retry<T, F, Fut>(&self, request_fn: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            debug!(
                "Attempting request (attempt {} of {})",
                attempt, self.max_retries
            );

            match request_fn().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!("Request attempt {} failed: {}", attempt, e);
                    last_error = Some(e);

                    // Check if we should retry based on error type
                    if attempt < self.max_retries && should_retry(last_error.as_ref().unwrap()) {
                        let backoff_secs = calculate_backoff(attempt);
                        debug!("Backing off for {} seconds before retry", backoff_secs);
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        continue;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All {} retry attempts failed", self.max_retries)))
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }
}

/// Determine if an error should trigger a retry
fn should_retry(error: &anyhow::Error) -> bool {
    let error_string = error.to_string().to_lowercase();

    // Network-related errors that should be retried
    error_string.contains("connection") ||
    error_string.contains("timeout") ||
    error_string.contains("network") ||
    error_string.contains("dns") ||
    error_string.contains("tcp") ||
    error_string.contains("ssl") ||
    error_string.contains("tls") ||
    // Starlink-specific indicators
    error_string.contains("starlink") ||
    error_string.contains("satellite") ||
    // HTTP status codes that should be retried
    error_string.contains("502") ||  // Bad Gateway
    error_string.contains("503") ||  // Service Unavailable
    error_string.contains("504") ||  // Gateway Timeout
    error_string.contains("520") ||  // Web Server Unknown Error
    error_string.contains("521") ||  // Web Server Is Down
    error_string.contains("522") ||  // Connection Timed Out
    error_string.contains("523") ||  // Origin Is Unreachable
    error_string.contains("524") // A Timeout Occurred
}

/// Calculate exponential backoff with jitter for retries
fn calculate_backoff(attempt: u32) -> u64 {
    use rand::Rng;

    let base_delay = 2_u64.pow(attempt - 1); // Exponential: 1, 2, 4, 8, etc.
    let max_delay = 60; // Cap at 60 seconds
    let jitter = rand::rng().random_range(0..=1000); // Add up to 1 second jitter

    std::cmp::min(base_delay + jitter / 1000, max_delay)
}

/// Detect potential Starlink network drop patterns
pub fn detect_starlink_drop(error: &anyhow::Error) -> bool {
    let error_string = error.to_string().to_lowercase();

    // Patterns that suggest Starlink satellite network drops
    error_string.contains("connection reset") ||
    error_string.contains("broken pipe") ||
    error_string.contains("network unreachable") ||
    (error_string.contains("timeout") && error_string.contains("connect")) ||
    // Multiple rapid timeouts can indicate satellite handoff
    error_string.contains("multiple timeout")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_retry() {
        assert!(should_retry(&anyhow!("Connection timeout")));
        assert!(should_retry(&anyhow!("Network unreachable")));
        assert!(should_retry(&anyhow!("HTTP 502 Bad Gateway")));
        assert!(!should_retry(&anyhow!("Authentication failed")));
        assert!(!should_retry(&anyhow!("Invalid JSON")));
    }

    #[test]
    fn test_calculate_backoff() {
        assert_eq!(calculate_backoff(1), 1);
        let backoff_2 = calculate_backoff(2);
        assert!((2..=3).contains(&backoff_2));

        // Should cap at max_delay
        let backoff_10 = calculate_backoff(10);
        assert!(backoff_10 <= 60);
    }

    #[test]
    fn test_detect_starlink_drop() {
        assert!(detect_starlink_drop(&anyhow!("Connection reset by peer")));
        assert!(detect_starlink_drop(&anyhow!("Network unreachable")));
        assert!(!detect_starlink_drop(&anyhow!("Invalid API key")));
    }
}
