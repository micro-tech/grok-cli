//! Utility module for initializing GrokClient

use anyhow::Result;
use crate::GrokClient;
use crate::config::RateLimitConfig;

/// Initialize a GrokClient with the provided settings
pub fn initialize_client(api_key: &str, timeout_secs: u64, max_retries: u32, rate_limit_config: RateLimitConfig) -> Result<GrokClient> {
    GrokClient::with_settings(api_key, timeout_secs, max_retries)
        .map(|client| client.with_rate_limits(rate_limit_config))
}
