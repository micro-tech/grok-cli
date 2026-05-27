//! Context budget manager.
//!
//! Helps decide when to switch to delta prompting, compress history,
//! or drop low-value context to stay under token limits.

use crate::context::error::{ContextError, ContextResult};
use crate::context::token_counter::TokenCounter;

/// Simple budget manager.
pub struct ContextBudget {
    max_tokens: u32,
    counter: TokenCounter,
}

impl ContextBudget {
    pub fn new(max_tokens: u32) -> ContextResult<Self> {
        if max_tokens == 0 {
            return Err(ContextError::InvalidTokenCount(0));
        }
        Ok(Self {
            max_tokens,
            counter: TokenCounter::new(),
        })
    }

    pub fn remaining(&self) -> u32 {
        let used = self.counter.total();
        if used >= self.max_tokens {
            0
        } else {
            self.max_tokens - used
        }
    }

    pub fn should_use_delta(&self) -> bool {
        // Use delta prompting when we're past 60% of budget
        self.counter.total() > (self.max_tokens * 60 / 100)
    }

    pub fn record_usage(&self, input: u32, output: u32) -> ContextResult<()> {
        if input > 1_000_000 || output > 1_000_000 {
            return Err(ContextError::InvalidTokenCount(input.max(output)));
        }
        self.counter.add_input(input);
        self.counter.add_output(output);
        Ok(())
    }

    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_creation() {
        let b = ContextBudget::new(1000).unwrap();
        assert_eq!(b.max_tokens(), 1000);
        assert_eq!(b.remaining(), 1000);
    }

    #[test]
    fn test_budget_zero_fails() {
        assert!(ContextBudget::new(0).is_err());
    }

    #[test]
    fn test_record_usage_and_delta() {
        let b = ContextBudget::new(1000).unwrap();
        b.record_usage(700, 100).unwrap();
        assert!(b.should_use_delta());
        assert_eq!(b.remaining(), 200);
    }

    #[test]
    fn test_record_usage_invalid() {
        let b = ContextBudget::new(1000).unwrap();
        assert!(b.record_usage(2_000_000, 0).is_err());
    }
}
