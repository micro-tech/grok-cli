//! Retry logic for handling transient network errors

use crate::error::{Error, Result};
use rand::RngExt;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,

    /// Base delay for exponential backoff (in milliseconds)
    pub base_delay_ms: u64,

    /// Maximum delay between retries (in milliseconds)
    pub max_delay_ms: u64,

    /// Whether to add jitter to backoff delays
    pub use_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 60000,
            use_jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Set base delay in milliseconds
    pub fn with_base_delay_ms(mut self, ms: u64) -> Self {
        self.base_delay_ms = ms;
        self
    }

    /// Set maximum delay in milliseconds
    pub fn with_max_delay_ms(mut self, ms: u64) -> Self {
        self.max_delay_ms = ms;
        self
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, use_jitter: bool) -> Self {
        self.use_jitter = use_jitter;
        self
    }

    /// Create a [`RetryConfig`] optimised for Starlink satellite connections.
    ///
    /// Starlink experiences periodic blackout gaps of 15–60 seconds during
    /// beam and satellite handoffs.  These settings back off aggressively so
    /// requests survive those gaps without giving up too early.
    ///
    /// | Setting | Starlink | Default |
    /// |---|---|---|
    /// | `max_retries` | 7 | 3 |
    /// | `base_delay_ms` | 2 000 | 1 000 |
    /// | `max_delay_ms` | 120 000 | 60 000 |
    /// | `use_jitter` | true | true |
    ///
    /// Applied automatically by [`crate::GrokClientBuilder::build`] when the
    /// `starlink` Cargo feature is enabled.
    #[cfg(feature = "starlink")]
    pub fn for_starlink() -> Self {
        Self {
            max_retries: 7,
            base_delay_ms: 2_000,
            max_delay_ms: 120_000,
            use_jitter: true,
        }
    }
}

/// Execute a function with retry logic
pub async fn retry_async<F, Fut, T>(config: &RetryConfig, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 1..=config.max_retries {
        debug!("Retry attempt {} of {}", attempt, config.max_retries);

        let outcome = operation().await;
        match outcome {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!("Attempt {} failed: {}", attempt, e);

                // Check if error is retryable
                if !e.is_retryable() {
                    debug!("Error is not retryable, failing immediately");
                    return Err(e);
                }

                last_error = Some(e);

                // Don't sleep after the last attempt
                if attempt < config.max_retries {
                    let delay = calculate_backoff(attempt, config);
                    debug!("Backing off for {} ms", delay.as_millis());
                    sleep(delay).await;
                }
            }
        }
    }

    // All retries exhausted
    Err(last_error.unwrap_or(Error::MaxRetriesExceeded(config.max_retries)))
}

/// Calculate exponential backoff delay with optional jitter
fn calculate_backoff(attempt: u32, config: &RetryConfig) -> Duration {
    // Exponential backoff: base_delay * 2^(attempt-1)
    let base_delay = config.base_delay_ms;
    let exponential_delay = base_delay.saturating_mul(2u64.saturating_pow(attempt - 1));

    // Cap at max delay
    let capped_delay = exponential_delay.min(config.max_delay_ms);

    // Add jitter if enabled (0-25% of the delay)
    let final_delay = if config.use_jitter {
        let jitter = rand::rng().random_range(0..=(capped_delay / 4));
        capped_delay + jitter
    } else {
        capped_delay
    };

    Duration::from_millis(final_delay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new(5)
            .with_base_delay_ms(500)
            .with_max_delay_ms(30000)
            .with_jitter(false);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 30000);
        assert!(!config.use_jitter);
    }

    #[test]
    fn test_calculate_backoff_no_jitter() {
        let config = RetryConfig::new(3)
            .with_base_delay_ms(1000)
            .with_max_delay_ms(60000)
            .with_jitter(false);

        let delay1 = calculate_backoff(1, &config);
        assert_eq!(delay1, Duration::from_millis(1000));

        let delay2 = calculate_backoff(2, &config);
        assert_eq!(delay2, Duration::from_millis(2000));

        let delay3 = calculate_backoff(3, &config);
        assert_eq!(delay3, Duration::from_millis(4000));
    }

    #[test]
    fn test_calculate_backoff_with_cap() {
        let config = RetryConfig::new(10)
            .with_base_delay_ms(1000)
            .with_max_delay_ms(5000)
            .with_jitter(false);

        // Should cap at max_delay_ms
        let delay = calculate_backoff(10, &config);
        assert_eq!(delay, Duration::from_millis(5000));
    }

    #[tokio::test]
    async fn test_retry_succeeds_on_first_attempt() {
        let config = RetryConfig::new(3);
        let call_count = AtomicUsize::new(0);

        let result = retry_async(&config, || async {
            call_count.fetch_add(1, Ordering::SeqCst);
            Ok::<_, Error>(42)
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_on_second_attempt() {
        let config = RetryConfig::new(3).with_base_delay_ms(10);
        let call_count = AtomicUsize::new(0);

        let result = retry_async(&config, || async {
            let count = call_count.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                Err(Error::Network("connection failed".to_string()))
            } else {
                Ok::<_, Error>(42)
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_fails_non_retryable() {
        let config = RetryConfig::new(3);
        let call_count = AtomicUsize::new(0);

        let result = retry_async(&config, || async {
            call_count.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(Error::Authentication)
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1); // Should not retry
    }

    #[tokio::test]
    async fn test_retry_exhausts_attempts() {
        let config = RetryConfig::new(2).with_base_delay_ms(10);
        let call_count = AtomicUsize::new(0);

        let result = retry_async(&config, || async {
            call_count.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(Error::Network("persistent failure".to_string()))
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 2); // All attempts used
    }
}
