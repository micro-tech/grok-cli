//! HOH Autonomous Research Agent (Task 361.12)

use crate::hoh::state::IterationState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery { pub topic: String, pub context: String, pub depth: ResearchDepth }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResearchDepth { Quick, Standard, Deep }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchOutput {
    pub topic: String,
    pub findings: Vec<String>,
    pub knowledge_fragments: Vec<String>,
    pub follow_up_questions: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAgent { pub simulation_mode: bool }

impl ResearchAgent {
    pub fn new(simulation_mode: bool) -> Self { Self { simulation_mode } }

    /// Conduct research on a topic. In simulation mode, returns plausible stub results.
    pub async fn research(&self, query: ResearchQuery) -> ResearchOutput {
        tracing::info!(topic = query.topic, "[HOH ResearchAgent] starting research");

        // Simulation: return structured findings without external LLM call
        let findings = vec![
            format!("Key insight about {}: it involves complex trade-offs.", query.topic),
            format!("Best practice for {}: start with the simplest approach.", query.topic),
            format!("Common pitfall with {}: over-engineering early.", query.topic),
        ];
        let fragments = vec![
            format!("# {}\n{}", query.topic, query.context),
        ];
        let follow_ups = vec![
            format!("What are the performance implications of {}?", query.topic),
            format!("How does {} integrate with existing systems?", query.topic),
        ];

        ResearchOutput {
            topic: query.topic,
            findings,
            knowledge_fragments: fragments,
            follow_up_questions: follow_ups,
            confidence: if self.simulation_mode { 0.6 } else { 0.8 },
        }
    }

    /// Detect knowledge gaps in the iteration state and produce research queries.
    pub fn detect_gaps(&self, state: &IterationState) -> Vec<ResearchQuery> {
        let mut queries = Vec::new();
        if let Some(plan) = &state.plan {
            for goal in &plan.goals {
                let lower = goal.to_lowercase();
                if lower.contains("unknown") || lower.contains("unclear") || lower.contains("research") {
                    queries.push(ResearchQuery {
                        topic: goal.clone(),
                        context: format!("HOH iteration {} goal", state.iteration_id),
                        depth: ResearchDepth::Standard,
                    });
                }
            }
        }
        queries
    }

    /// Inject research outputs into the iteration state as knowledge bundles.
    pub fn inject_findings(&self, state: &mut IterationState, outputs: &[ResearchOutput]) {
        if let Some(plan) = &mut state.plan {
            for output in outputs {
                plan.experiments.push(format!("[Research] {}: {}", output.topic,
                    output.findings.first().cloned().unwrap_or_default()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_research_returns_output() {
        let agent = ResearchAgent::new(true);
        let q = ResearchQuery { topic: "async Rust".to_string(), context: "HOH test".to_string(), depth: ResearchDepth::Quick };
        let out = agent.research(q).await;
        assert!(!out.findings.is_empty());
        assert!(!out.knowledge_fragments.is_empty());
    }
}
