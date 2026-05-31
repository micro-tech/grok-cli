//! Cost-aware routing layer.
//!
//! Selects model, compression level, and token budget based on request type.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostProfile {
    Cheap,
    Balanced,
    HighPrecision,
}

#[derive(Debug, Clone)]
pub struct CostRouter {
    pub default_profile: CostProfile,
}

impl Default for CostRouter {
    fn default() -> Self {
        Self {
            default_profile: CostProfile::Balanced,
        }
    }
}

impl CostRouter {
    pub fn new(default: CostProfile) -> Self {
        Self {
            default_profile: default,
        }
    }

    /// Classify a request and return the appropriate cost profile.
    pub fn classify_request(&self, prompt: &str, has_complex_tools: bool) -> CostProfile {
        if prompt.len() > 4000 || has_complex_tools {
            CostProfile::HighPrecision
        } else if prompt.len() < 800 {
            CostProfile::Cheap
        } else {
            self.default_profile
        }
    }

    /// Returns a suggested token budget for the given profile.
    pub fn token_budget(&self, profile: CostProfile) -> usize {
        match profile {
            CostProfile::Cheap => 4_000,
            CostProfile::Balanced => 12_000,
            CostProfile::HighPrecision => 32_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification() {
        let router = CostRouter::default();
        assert_eq!(
            router.classify_request("short", false),
            CostProfile::Cheap
        );
        assert_eq!(
            router.classify_request(&"x".repeat(5000), true),
            CostProfile::HighPrecision
        );
    }
}
