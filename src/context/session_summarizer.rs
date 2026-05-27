//! Session context summarizer.
//!
//! Produces compact summaries of recent turns for use with delta prompting.

use std::collections::VecDeque;

#[derive(Default)]
pub struct SessionSummarizer {
    turns: VecDeque<String>,
    max_turns: usize,
}

impl SessionSummarizer {
    pub fn new(max_turns: usize) -> Self {
        Self {
            turns: VecDeque::new(),
            max_turns,
        }
    }

    pub fn add_turn(&mut self, text: String) {
        if self.turns.len() >= self.max_turns {
            self.turns.pop_front();
        }
        self.turns.push_back(text);
    }

    /// Returns a compact summary string.
    pub fn summarize(&self) -> String {
        if self.turns.is_empty() {
            return String::new();
        }
        // Very simple concatenation with truncation
        let joined = self.turns.iter().cloned().collect::<Vec<_>>().join(" | ");
        if joined.len() > 400 {
            format!("{}…", &joined[..397])
        } else {
            joined
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarizer() {
        let mut s = SessionSummarizer::new(3);
        s.add_turn("first".into());
        s.add_turn("second".into());
        assert!(s.summarize().contains("first"));
    }
}
