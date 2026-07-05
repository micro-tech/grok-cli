//! Session-only Rules
//!
//! Temporary rules that exist only for the current session.
//! These are added via slash commands and injected into the prompt.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Maximum number of session rules allowed
const MAX_SESSION_RULES: usize = 20;

/// A single session-only rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRule {
    pub id: u32,
    pub content: String,
}

/// Manages temporary session rules
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRules {
    rules: VecDeque<SessionRule>,
    next_id: u32,
}

impl SessionRules {
    pub fn new() -> Self {
        Self {
            rules: VecDeque::new(),
            next_id: 1,
        }
    }

    /// Add a new rule (returns the assigned ID)
    pub fn add(&mut self, content: String) -> u32 {
        // Enforce max limit (remove oldest if needed)
        while self.rules.len() >= MAX_SESSION_RULES {
            self.rules.pop_front();
        }

        let id = self.next_id;
        self.next_id += 1;

        self.rules.push_back(SessionRule { id, content });
        id
    }

    /// Remove a rule by ID
    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < before
    }

    /// Clear all session rules
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// List all current rules
    pub fn list(&self) -> Vec<&SessionRule> {
        self.rules.iter().collect()
    }

    /// Format all rules for prompt injection
    pub fn format_for_prompt(&self) -> String {
        if self.rules.is_empty() {
            return String::new();
        }

        let mut output = String::from("\n\n## Session Rules (Temporary)\n\n");

        for rule in &self.rules {
            output.push_str(&format!("- {}\n", rule.content.trim()));
        }

        output
    }

    /// Returns true if there are any active session rules
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}