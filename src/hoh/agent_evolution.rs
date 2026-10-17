//! HOH Agent Evolution Engine (405)
//!
//! Allows agents to evolve their capabilities, personalities, and heuristics over time.
//! Uses performance feedback (Helix + internal) as fitness function.
//! Supports gradual mutations and occasional leaps. Maintains lineage.

use serde::{Deserialize, Serialize};
use crate::hoh::state::HOHError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvolutionEvent {
    pub agent_id: String,
    pub mutation_type: String, // "prompt_delta", "trait_shift", "heuristic_tune", "leap"
    pub description: String,
    pub fitness_before: f32,
    pub fitness_after: f32,
    pub lineage: String,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvolutionSystem {
    pub simulation_mode: bool,
    pub evolution_events: Vec<EvolutionEvent>,
}

impl AgentEvolutionSystem {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            simulation_mode,
            evolution_events: vec![],
        }
    }

    /// Run evolution on tracked agents using recent performance signals.
    /// For now, produces synthetic events based on lifecycle signals.
    pub async fn evolve_agents(
        &mut self,
        lifecycle_signals: &[(String, f32, f32)], // (id, success_rate, avg_quality)
    ) -> Result<Vec<EvolutionEvent>, HOHError> {
        let mut events = vec![];

        for (id, success, quality) in lifecycle_signals {
            if *success < 0.6 || *quality < 0.55 {
                // Negative selection or small mutation
                let ev = EvolutionEvent {
                    agent_id: id.clone(),
                    mutation_type: "heuristic_tune".to_string(),
                    description: format!("Lowered risk tolerance after low success ({:.2})", success),
                    fitness_before: *success,
                    fitness_after: (*success + 0.05).min(0.95f32),
                    lineage: format!("{}-v{}", id, chrono::Utc::now().timestamp() % 1000),
                    at: chrono::Utc::now().timestamp() as u64,
                };
                events.push(ev.clone());
                self.evolution_events.push(ev);
            } else if *quality > 0.82 && *success > 0.85 {
                // Positive leap
                let ev = EvolutionEvent {
                    agent_id: id.clone(),
                    mutation_type: "leap".to_string(),
                    description: "Personality shift toward higher creativity (successful pattern)".to_string(),
                    fitness_before: *quality,
                    fitness_after: (*quality + 0.08).min(0.98f32),
                    lineage: format!("{}-leap-{}", id, chrono::Utc::now().timestamp() % 1000),
                    at: chrono::Utc::now().timestamp() as u64,
                };
                events.push(ev.clone());
                self.evolution_events.push(ev);
            }
        }

        if !events.is_empty() && !self.simulation_mode {
            tracing::info!("405: Applied {} agent evolution mutations", events.len());
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evolution_triggers_on_poor_perf() {
        let mut sys = AgentEvolutionSystem::new(true);
        let signals = vec![("agent-1".to_string(), 0.4, 0.5)];
        let events = sys.evolve_agents(&signals).await.unwrap();
        assert!(!events.is_empty());
        assert!(events[0].mutation_type.contains("tune") || events[0].mutation_type.contains("leap"));
    }
}
