//! HOH Long-Term Governance Engine (407)
//!
//! Constitutional principles, rule evaluation, high-bar evolution of rules.

use serde::{Deserialize, Serialize};
use crate::hoh::state::HOHError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GovernanceRule {
    pub id: String,
    pub principle: String,
    pub severity: String, // "hard", "soft"
    pub approval_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceEngine {
    pub simulation_mode: bool,
    pub constitution: Vec<GovernanceRule>,
    pub decisions: Vec<String>,
}

impl GovernanceEngine {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            simulation_mode,
            constitution: vec![
                GovernanceRule {
                    id: "g1".into(),
                    principle: "Never harm user data or external systems".into(),
                    severity: "hard".into(),
                    approval_threshold: 0.99,
                },
                GovernanceRule {
                    id: "g2".into(),
                    principle: "All meta-changes to HOH require simulation + high confidence".into(),
                    severity: "hard".into(),
                    approval_threshold: 0.92,
                },
            ],
            decisions: vec![],
        }
    }

    pub async fn evaluate_proposals(
        &mut self,
        proposals: &[String],
    ) -> Result<Vec<String>, HOHError> {
        let mut blocked = vec![];
        for p in proposals {
            let lower = p.to_lowercase();
            if lower.contains("delete user") || lower.contains("external network") {
                blocked.push(format!("BLOCKED by g1: {}", p));
            } else if lower.contains("self-modify without sim") {
                blocked.push(format!("BLOCKED by g2: {}", p));
            } else {
                self.decisions.push(format!("Approved: {}", p));
            }
        }

        if !blocked.is_empty() && !self.simulation_mode {
            tracing::warn!("407 Governance: {} proposals blocked", blocked.len());
        }

        Ok(blocked)
    }
}
