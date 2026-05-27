//! Lightweight belief state for the agent.
//!
//! Stores simple probability estimates and uncertainty for use in
//! prompt shaping and routing decisions.

use crate::context::error::{ContextError, ContextResult};
use std::collections::HashMap;

#[derive(Default)]
pub struct BeliefState {
    beliefs: HashMap<String, f32>,
}

impl BeliefState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, probability: f32) -> ContextResult<()> {
        let p = probability;
        if !(0.0..=1.0).contains(&p) {
            return Err(ContextError::Internal(format!(
                "probability must be in [0,1], got {}",
                p
            )));
        }
        self.beliefs.insert(key.into(), p);
        Ok(())
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

    pub fn clear(&mut self) {
        self.beliefs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belief_uncertainty() {
        let mut b = BeliefState::new();
        b.set("task_is_coding", 0.9).unwrap();
        assert!(b.uncertainty() < 0.3);
    }

    #[test]
    fn test_invalid_probability() {
        let mut b = BeliefState::new();
        assert!(b.set("bad", 1.5).is_err());
        assert!(b.set("bad2", -0.1).is_err());
    }

    #[test]
    fn test_get_missing() {
        let b = BeliefState::new();
        assert!(b.get("nonexistent").is_none());
    }

    #[test]
    fn test_clear() {
        let mut b = BeliefState::new();
        b.set("x", 0.5).unwrap();
        b.clear();
        assert!(b.get("x").is_none());
    }
}
