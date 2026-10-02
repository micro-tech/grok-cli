//! HOH Resource Budgeting (Task 297.27)
//!
//! Enforces token, time, and inner-call budgets per HOH iteration.
//! Gracefully degrades when any budget is exhausted.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Describes which resource was exceeded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetExceeded {
    pub resource: String,
    pub used: u64,
    pub limit: u64,
}

impl std::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Budget exceeded: {} used {}/{} limit",
            self.resource, self.used, self.limit
        )
    }
}

impl std::error::Error for BudgetExceeded {}

/// Tracks resource consumption for one HOH iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    // Limits
    pub max_tokens: u64,
    pub max_duration_secs: u64,
    pub max_inner_calls: u32,
    // Usage counters
    pub tokens_used: u64,
    pub elapsed_secs: u64,
    pub calls_made: u32,
    pub started_at: u64,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self::new(100_000, 3_600, 50)
    }
}

impl ResourceBudget {
    pub fn new(max_tokens: u64, max_duration_secs: u64, max_inner_calls: u32) -> Self {
        Self {
            max_tokens,
            max_duration_secs,
            max_inner_calls,
            tokens_used: 0,
            elapsed_secs: 0,
            calls_made: 0,
            started_at: now_secs(),
        }
    }

    /// Record one inner-harness call; error if limit hit.
    pub fn tick_call(&mut self) -> Result<(), BudgetExceeded> {
        self.calls_made += 1;
        if self.calls_made > self.max_inner_calls {
            Err(BudgetExceeded {
                resource: "inner_calls".to_string(),
                used: self.calls_made as u64,
                limit: self.max_inner_calls as u64,
            })
        } else {
            Ok(())
        }
    }

    /// Record token consumption; error if limit hit.
    pub fn tick_tokens(&mut self, n: u64) -> Result<(), BudgetExceeded> {
        self.tokens_used += n;
        if self.tokens_used > self.max_tokens {
            Err(BudgetExceeded {
                resource: "tokens".to_string(),
                used: self.tokens_used,
                limit: self.max_tokens,
            })
        } else {
            Ok(())
        }
    }

    /// Check wall-clock time; error if duration limit exceeded.
    pub fn check_time(&mut self) -> Result<(), BudgetExceeded> {
        self.elapsed_secs = now_secs().saturating_sub(self.started_at);
        if self.elapsed_secs > self.max_duration_secs {
            Err(BudgetExceeded {
                resource: "duration_secs".to_string(),
                used: self.elapsed_secs,
                limit: self.max_duration_secs,
            })
        } else {
            Ok(())
        }
    }

    /// True if any limit has been reached.
    pub fn is_exhausted(&self) -> bool {
        self.tokens_used >= self.max_tokens
            || self.calls_made >= self.max_inner_calls
            || self.elapsed_secs >= self.max_duration_secs
    }

    /// Human-readable usage summary.
    pub fn summary(&self) -> String {
        format!(
            "Tokens: {}/{} | Calls: {}/{} | Time: {}s/{}s",
            self.tokens_used,
            self.max_tokens,
            self.calls_made,
            self.max_inner_calls,
            self.elapsed_secs,
            self.max_duration_secs,
        )
    }

    /// Percentage of the most-constrained resource used (0.0–1.0).
    pub fn pressure(&self) -> f32 {
        let t = self.tokens_used as f32 / self.max_tokens as f32;
        let c = self.calls_made as f32 / self.max_inner_calls as f32;
        let d = self.elapsed_secs as f32 / self.max_duration_secs as f32;
        t.max(c).max(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_not_exhausted_initially() {
        let b = ResourceBudget::default();
        assert!(!b.is_exhausted());
        assert!(b.pressure() < 0.01);
    }

    #[test]
    fn test_tick_call_returns_err_when_limit_hit() {
        let mut b = ResourceBudget::new(1_000, 3_600, 2);
        assert!(b.tick_call().is_ok());
        assert!(b.tick_call().is_ok());
        assert!(b.tick_call().is_err());
    }

    #[test]
    fn test_tick_tokens_errors_when_exceeded() {
        let mut b = ResourceBudget::new(100, 3_600, 50);
        assert!(b.tick_tokens(50).is_ok());
        assert!(b.tick_tokens(60).is_err()); // 110 > 100
    }

    #[test]
    fn test_summary_contains_all_fields() {
        let b = ResourceBudget::default();
        let s = b.summary();
        assert!(s.contains("Tokens"));
        assert!(s.contains("Calls"));
        assert!(s.contains("Time"));
    }
}
