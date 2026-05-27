//! Token usage tracking for cost and context management.

use std::sync::atomic::{AtomicU32, Ordering};

/// Simple atomic token counter.
pub struct TokenCounter {
    input: AtomicU32,
    output: AtomicU32,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter {
    pub fn new() -> Self {
        Self {
            input: AtomicU32::new(0),
            output: AtomicU32::new(0),
        }
    }

    pub fn add_input(&self, tokens: u32) {
        self.input.fetch_add(tokens, Ordering::Relaxed);
    }

    pub fn add_output(&self, tokens: u32) {
        self.output.fetch_add(tokens, Ordering::Relaxed);
    }

    pub fn total(&self) -> u32 {
        self.input.load(Ordering::Relaxed) + self.output.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.input.store(0, Ordering::Relaxed);
        self.output.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let c = TokenCounter::new();
        c.add_input(100);
        c.add_output(50);
        assert_eq!(c.total(), 150);
    }
}
