//! Lightweight belief state for the agent.
//!
//! Stores simple probability estimates and uncertainty for use in
//! prompt shaping and routing decisions.

use std::collections::HashMap;

#[derive(Default)]
pub struct BeliefState {
    beliefs: HashMap<String, f32>,
}

impl BeliefState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, probability: f32) {
        self.beliefs.insert(key.into(), probability.clamp(0.0, 1.0));
    }

    pub fn get(&self, key: &str) -> Option<f32> {
        self.beliefs.get(key).copied()
    }

    pub fn uncertainty(&self) -> f32 {
        if self.beliefs.is_empty() {
            return 1.0;
        }
        // Simple entropy-like measure
        let avg: f32 = self.beliefs.values().sum::<f32>() / self.beliefs.len() as f32;
        1.0 - (avg - 0.5).abs() * 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belief_uncertainty() {
        let mut b = BeliefState::new();
        b.set("task_is_coding", 0.9);
        assert!(b.uncertainty() < 0.3);
    }
}
