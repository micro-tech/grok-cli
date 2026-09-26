//! HOH Agent Negotiation System (Task 361.28)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationProposal {
    pub from_agent: String,
    pub to_agent: String,
    pub topic: String,
    pub offer: String,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NegotiationOutcome { Accepted, Rejected, Compromise(String), Stalemate }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationResult {
    pub proposal: NegotiationProposal,
    pub outcome: NegotiationOutcome,
    pub rounds: u32,
    pub notes: String,
}

pub struct NegotiationEngine { pub simulation_mode: bool }

impl NegotiationEngine {
    pub fn new(sim: bool) -> Self { Self { simulation_mode: sim } }

    /// Simulate negotiation between two agents over a topic.
    pub fn negotiate(&self, proposal: NegotiationProposal, max_rounds: u32) -> NegotiationResult {
        tracing::info!(
            from = proposal.from_agent, to = proposal.to_agent,
            topic = proposal.topic, "[HOH Negotiation] starting"
        );

        // Simple simulation logic
        let has_conditions = !proposal.conditions.is_empty();
        let outcome = if has_conditions && max_rounds >= 3 {
            NegotiationOutcome::Compromise(format!(
                "Agents {} and {} agreed: {} (with conditions waived)",
                proposal.from_agent, proposal.to_agent, proposal.offer
            ))
        } else if !has_conditions {
            NegotiationOutcome::Accepted
        } else if max_rounds < 2 {
            NegotiationOutcome::Stalemate
        } else {
            NegotiationOutcome::Rejected
        };

        let notes = match &outcome {
            NegotiationOutcome::Accepted => format!("'{}' offer accepted without conditions", proposal.topic),
            NegotiationOutcome::Rejected => format!("'{}' offer rejected — conditions unmet", proposal.topic),
            NegotiationOutcome::Compromise(c) => c.clone(),
            NegotiationOutcome::Stalemate => format!("Negotiation on '{}' reached stalemate", proposal.topic),
        };

        NegotiationResult { proposal, outcome, rounds: max_rounds.min(3), notes }
    }

    /// Negotiate resource allocation between multiple agents.
    pub fn allocate_resources(
        &self,
        agents: &[String],
        _resource: &str,
        total: f32,
    ) -> Vec<(String, f32)> {
        // Fair split with priority weighting (in sim, just equal split)
        let per_agent = if agents.is_empty() { 0.0 } else { total / agents.len() as f32 };
        agents.iter().map(|a| (a.clone(), per_agent)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(topic: &str, conditions: Vec<&str>) -> NegotiationProposal {
        NegotiationProposal {
            from_agent: "Architect".to_string(),
            to_agent: "Coder".to_string(),
            topic: topic.to_string(),
            offer: "Use trait objects instead of generics".to_string(),
            conditions: conditions.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_no_conditions_accepted() {
        let engine = NegotiationEngine::new(true);
        let result = engine.negotiate(proposal("API design", vec![]), 3);
        assert_eq!(result.outcome, NegotiationOutcome::Accepted);
    }

    #[test]
    fn test_compromise_with_conditions() {
        let engine = NegotiationEngine::new(true);
        let result = engine.negotiate(proposal("Refactor plan", vec!["keep tests green"]), 4);
        assert!(matches!(result.outcome, NegotiationOutcome::Compromise(_)));
    }

    #[test]
    fn test_resource_allocation_sums_to_total() {
        let engine = NegotiationEngine::new(true);
        let agents = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let allocs = engine.allocate_resources(&agents, "tokens", 300.0);
        let total: f32 = allocs.iter().map(|(_, v)| v).sum();
        assert!((total - 300.0).abs() < 0.01);
    }
}
