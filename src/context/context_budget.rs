//! Context budget manager.
//!
//! Helps decide when to switch to delta prompting, compress history,
//! or drop low-value context to stay under token limits.

use crate::context::token_counter::TokenCounter;

/// Simple budget manager.
pub struct ContextBudget {
    max_tokens: u32,
    counter: TokenCounter,
}

impl ContextBudget {
    pub fn new(max_tokens: u32) -> Self {
        Self {
            max_tokens,
            counter: TokenCounter::new(),
        }
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

    pub fn record_usage(&self, input: u32, output: u32) {
        self.counter.add_input(input);
        self.counter.add_output(output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget() {
        let b = ContextBudget::new(1000);
        b.record_usage(700, 100);
        assert!(b.should_use_delta());
        assert_eq!(b.remaining(), 200);
    }
}
