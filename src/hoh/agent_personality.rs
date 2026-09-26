//! HOH Agent Personality Profiles (Task 361.27)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommunicationStyle { Terse, Balanced, Verbose }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonality {
    pub name: String,
    pub risk_tolerance: f32,      // 0.0 (cautious) – 1.0 (bold)
    pub creativity: f32,          // 0.0–1.0
    pub precision: f32,           // 0.0–1.0
    pub collaboration: f32,       // 0.0–1.0
    pub style: CommunicationStyle,
    pub quirks: Vec<String>,
}

impl AgentPersonality {
    pub fn default_profiles() -> Vec<Self> {
        vec![
            Self { name: "Cautious Architect".into(), risk_tolerance: 0.2, creativity: 0.6, precision: 0.9,
                collaboration: 0.7, style: CommunicationStyle::Verbose,
                quirks: vec!["Always asks for tests first".into(), "Prefers well-established patterns".into()] },
            Self { name: "Bold Innovator".into(), risk_tolerance: 0.8, creativity: 0.95, precision: 0.5,
                collaboration: 0.6, style: CommunicationStyle::Terse,
                quirks: vec!["Proposes radical rewrites".into(), "Favors novelty over familiarity".into()] },
            Self { name: "Pragmatic Coder".into(), risk_tolerance: 0.5, creativity: 0.5, precision: 0.75,
                collaboration: 0.8, style: CommunicationStyle::Balanced,
                quirks: vec!["Ships working code fast".into(), "Minimal ceremony".into()] },
            Self { name: "Meticulous Tester".into(), risk_tolerance: 0.15, creativity: 0.4, precision: 0.95,
                collaboration: 0.85, style: CommunicationStyle::Verbose,
                quirks: vec!["Writes tests before code".into(), "Flags any untested path".into()] },
        ]
    }

    pub fn personality_score(&self) -> f32 {
        (self.precision + self.collaboration + (1.0 - self.risk_tolerance.abs() - 0.5).abs()) / 3.0
    }
}

/// Mutate a personality slightly (simulate evolution).
pub fn evolve_personality(p: &AgentPersonality, performance: f32) -> AgentPersonality {
    // rand::random::<f32>() returns 0.0..1.0
    let r = || rand::random::<f32>() * 2.0 - 1.0; // -1..1
    let delta = if performance > 0.7 { 0.05f32 } else { -0.03f32 };
    AgentPersonality {
        name: p.name.clone(),
        risk_tolerance: (p.risk_tolerance + r() * 0.05).clamp(0.0, 1.0),
        creativity:     (p.creativity     + delta + r() * 0.02).clamp(0.0, 1.0),
        precision:      (p.precision      + delta + r() * 0.02).clamp(0.0, 1.0),
        collaboration:  (p.collaboration  + r() * 0.03).clamp(0.0, 1.0),
        style: p.style.clone(),
        quirks: p.quirks.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profiles_non_empty() {
        let profiles = AgentPersonality::default_profiles();
        assert!(profiles.len() >= 3);
        assert!(profiles.iter().all(|p| !p.name.is_empty()));
    }

    #[test]
    fn test_evolve_stays_in_range() {
        let profiles = AgentPersonality::default_profiles();
        for profile in &profiles {
            let evolved = evolve_personality(profile, 0.8);
            assert!(evolved.risk_tolerance >= 0.0 && evolved.risk_tolerance <= 1.0);
            assert!(evolved.creativity >= 0.0 && evolved.creativity <= 1.0);
        }
    }
}
