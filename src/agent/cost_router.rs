//! Cost-aware model router.
//!
//! Provides cost profiles and request classification to choose the most
//! economical model that can still handle the request well.

use serde::{Deserialize, Serialize};

/// Cost tier for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CostTier {
    /// Ultra-cheap / fast model (e.g. small Grok variant)
    #[default]
    Low,
    /// Balanced cost/performance
    Medium,
    /// High-capability, higher cost model
    High,
}

/// Profile describing cost characteristics of a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostProfile {
    /// Estimated input tokens
    pub estimated_input_tokens: u32,
    /// Estimated output tokens
    pub estimated_output_tokens: u32,
    /// Complexity score (0.0 = trivial, 1.0 = very complex)
    pub complexity: f32,
    /// Whether the request requires long context or heavy reasoning
    pub requires_long_context: bool,
    /// Preferred cost tier (can be overridden by policy)
    pub preferred_tier: CostTier,
}

impl Default for CostProfile {
    fn default() -> Self {
        Self {
            estimated_input_tokens: 0,
            estimated_output_tokens: 0,
            complexity: 0.5,
            requires_long_context: false,
            preferred_tier: CostTier::Medium,
        }
    }
}

impl CostProfile {
    /// Create a new cost profile with explicit values.
    pub fn new(
        input_tokens: u32,
        output_tokens: u32,
        complexity: f32,
        long_context: bool,
    ) -> Self {
        Self {
            estimated_input_tokens: input_tokens,
            estimated_output_tokens: output_tokens,
            complexity: complexity.clamp(0.0, 1.0),
            requires_long_context: long_context,
            preferred_tier: if complexity > 0.75 {
                CostTier::High
            } else if complexity > 0.4 {
                CostTier::Medium
            } else {
                CostTier::Low
            },
        }
    }

    /// Rough token cost estimate (input + output).
    pub fn total_tokens(&self) -> u32 {
        self.estimated_input_tokens + self.estimated_output_tokens
    }

    /// Returns true if this request should prefer a cheaper model.
    pub fn prefers_cheap_model(&self) -> bool {
        self.complexity < 0.4 && !self.requires_long_context
    }
}

/// Classify a user request into a CostProfile.
/// This is a lightweight heuristic used before sending the request to the model.
pub fn classify_request(prompt: &str, history_len: usize) -> CostProfile {
    let len = prompt.len();
    let word_count = prompt.split_whitespace().count();

    let complexity = if word_count > 120 || history_len > 12 {
        0.85
    } else if word_count > 60 || history_len > 6 {
        0.6
    } else if word_count > 25 {
        0.4
    } else {
        0.25
    };

    let requires_long_context = history_len > 15 || len > 4000;

    CostProfile::new(
        (len as u32 / 3).max(50), // rough token estimate
        300,
        complexity,
        requires_long_context,
    )
}

/// Select the most appropriate cost tier for a given profile.
/// This is the core of the cost-aware router.
pub fn select_tier(profile: &CostProfile, allow_high: bool) -> CostTier {
    if profile.requires_long_context && allow_high {
        return CostTier::High;
    }

    if profile.complexity > 0.75 && allow_high {
        CostTier::High
    } else if profile.complexity > 0.45 {
        CostTier::Medium
    } else {
        CostTier::Low
    }
}

/// High-level entry point: classify a request and return the recommended tier.
pub fn recommend_tier(prompt: &str, history_len: usize, allow_high: bool) -> CostTier {
    let profile = classify_request(prompt, history_len);
    select_tier(&profile, allow_high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_profile_defaults() {
        let p = CostProfile::default();
        assert_eq!(p.preferred_tier, CostTier::Medium);
    }

    #[test]
    fn test_cost_profile_complexity_tier() {
        let cheap = CostProfile::new(200, 50, 0.2, false);
        assert_eq!(cheap.preferred_tier, CostTier::Low);

        let hard = CostProfile::new(4000, 800, 0.9, true);
        assert_eq!(hard.preferred_tier, CostTier::High);
    }

    #[test]
    fn test_classify_request_simple() {
        let profile = classify_request("hello", 0);
        assert!(profile.complexity < 0.5);
        assert!(!profile.requires_long_context);
    }

    #[test]
    fn test_classify_request_complex() {
        let long_prompt = "explain ".repeat(80);
        let profile = classify_request(&long_prompt, 20);
        assert!(profile.complexity > 0.7);
        assert!(profile.requires_long_context);
    }
}
