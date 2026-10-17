use crate::config::RateLimitConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct UsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub request_count: u64,
    pub last_request_time: Option<u64>, // Unix timestamp in seconds

    // We store timestamps as u64 (Unix timestamp) for serialization
    pub request_history: Vec<(u64, u32)>, // (Timestamp, TokenCount)
}

impl UsageStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load usage stats from disk
    pub fn load() -> Result<Self> {
        let path = get_usage_stats_path()?;
        if path.exists() {
            let json = fs::read_to_string(&path)?;
            let stats: UsageStats = serde_json::from_str(&json)?;
            Ok(stats)
        } else {
            Ok(Self::default())
        }
    }

    /// Save usage stats to disk
    pub fn save(&self) -> Result<()> {
        let path = get_usage_stats_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Checks if the next request of estimated `tokens` size is allowed
    pub fn check_limit(
        &mut self,
        config: &RateLimitConfig,
        estimated_tokens: u32,
    ) -> Result<(), String> {
        self.clean_old_history(Duration::from_secs(60));

        let current_tokens: u32 = self.request_history.iter().map(|(_, tokens)| *tokens).sum();
        let current_requests = self.request_history.len() as u32;

        if current_requests >= config.max_requests_per_minute {
            return Err("Requests per minute".to_string());
        }

        // Only enforce the token budget when there are already requests recorded in
        // this window.  If the window is empty (first call of the minute) we always
        // allow the request, even if the estimated size exceeds the per-minute cap.
        // This prevents blocking large-but-valid first requests in long conversations.
        if current_requests > 0
            && current_tokens.saturating_add(estimated_tokens) > config.max_tokens_per_minute
        {
            return Err("Tokens per minute".to_string());
        }

        Ok(())
    }

    /// Call this AFTER a successful API call to record actual usage
    pub fn record_usage(&mut self, input_tokens: u32, output_tokens: u32) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let total = input_tokens + output_tokens;

        self.total_input_tokens += input_tokens as u64;
        self.total_output_tokens += output_tokens as u64;
        self.request_count += 1;
        self.last_request_time = Some(now);
        self.request_history.push((now, total));

        // Auto-save after update
        if let Err(e) = self.save() {
            warn!("Failed to save usage stats: {}. Stats will not persist.", e);
        }
    }

    fn clean_old_history(&mut self, window: Duration) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window_secs = window.as_secs();

        self.request_history
            .retain(|(time, _)| now.saturating_sub(*time) < window_secs);
    }
}

fn get_usage_stats_path() -> Result<PathBuf> {
    // Use canonical data dir for usage stats (grok_data_dir already handles home dir fallback)
    let _ = dirs::home_dir(); // retained only for potential future error handling; value unused
    Ok(crate::config::grok_data_dir().join("usage_stats.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    // std::thread was previously used for timing tests; currently unused.

    #[test]
    fn test_clean_old_history() {
        let mut stats = UsageStats::default();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Add an old record (61 seconds ago)
        stats.request_history.push((now - 61, 100));
        // Add a recent record (10 seconds ago)
        stats.request_history.push((now - 10, 50));

        stats.clean_old_history(Duration::from_secs(60));

        assert_eq!(stats.request_history.len(), 1);
        assert_eq!(stats.request_history[0].1, 50);
    }

    #[test]
    fn test_check_limit_requests() {
        let config = RateLimitConfig {
            max_requests_per_minute: 2,
            max_tokens_per_minute: 1000,
        };
        let mut stats = UsageStats::default();

        assert!(stats.check_limit(&config, 10).is_ok());
        stats.record_usage(5, 5); // 1st request

        assert!(stats.check_limit(&config, 10).is_ok());
        stats.record_usage(5, 5); // 2nd request

        // 3rd request should fail
        assert!(stats.check_limit(&config, 10).is_err());
    }

    #[test]
    fn test_check_limit_tokens() {
        let config = RateLimitConfig {
            max_requests_per_minute: 10,
            max_tokens_per_minute: 100,
        };
        let mut stats = UsageStats::default();

        assert!(stats.check_limit(&config, 50).is_ok());
        stats.record_usage(50, 0);

        assert!(stats.check_limit(&config, 50).is_ok());
        stats.record_usage(50, 0);

        // 101st token should fail
        assert!(stats.check_limit(&config, 1).is_err());
    }

    #[test]
    fn test_first_request_always_allowed_even_if_large() {
        // A single large request (estimated_tokens > max_tokens_per_minute) must
        // be allowed when no prior requests exist in the current window.
        // This prevents blocking valid first calls from long conversation histories.
        let config = RateLimitConfig {
            max_requests_per_minute: 10,
            max_tokens_per_minute: 100,
        };
        let mut stats = UsageStats::default();

        // estimated_tokens (200) > max_tokens_per_minute (100) — still OK on first call
        assert!(stats.check_limit(&config, 200).is_ok());
    }
}
