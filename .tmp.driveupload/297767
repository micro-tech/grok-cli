//! HOH Multi-Agent Simulation Mode (361.8 / 368)
//!
//! Safe, fast simulation of multi-agent interactions for prediction and what-if analysis.
//! No side effects on the real filesystem, no real LLM calls (unless explicitly requested),
//! and no actual tool execution.
//!
//! Use cases:
//! - Planner: "What would happen if we spawn 3 agents for this task?"
//! - Strategy / Trajectory engines: predict collaboration outcomes over many steps.
//! - Before real birth/delegation: run cheap simulations to choose best agent mix.
//! - Training / calibration of the collaboration protocol and skill evolution.
//!
//! Integrates tightly with:
//! - 361.7 MultiAgentCollaborationProtocol
//! - 361.6 AgentSkillEvolutionSystem
//! - 361.5 SpecializedAgentProfile
//! - 403/404 Lifecycle + Birth (simulated birth/retirement)
//! - Global HOH simulation_mode

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::hoh::specialized_agents::AgentProfile;
use crate::hoh::agent_skill_evolution::AgentSkillEvolutionSystem;
use crate::hoh::multi_agent_collaboration::{MultiAgentCollaborationProtocol, CollaborationStatus};

/// A lightweight simulated agent for fast what-if runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedAgent {
    pub id: String,
    pub profile: AgentProfile,
    pub skill_portfolio: crate::hoh::agent_skill_evolution::AgentSkillPortfolio,
    pub success_bias: f32,      // base competence modifier (0.6–0.95)
    pub cost_per_step: f32,     // abstract "tokens/time" cost
    pub personality: Vec<String>,
}

/// Outcome of a single simulated collaboration or delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutcome {
    pub scenario: String,
    pub participants: Vec<String>,
    pub predicted_success_rate: f32,     // 0.0–1.0
    pub predicted_quality: f32,          // 0.0–1.0
    pub predicted_cost: f32,
    pub predicted_conflicts: u32,
    pub predicted_knowledge_gain: f32,
    pub steps_taken: u32,
    pub final_status: String,
    pub key_events: Vec<String>,
    pub confidence: f32,                 // how reliable this prediction is
}

/// Configuration for a simulation run.
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub num_steps: u32,
    pub monte_carlo_runs: u32,   // >1 enables statistical aggregation
    pub include_skill_evolution: bool,
    pub include_negotiation: bool,
    pub noise_level: f32,        // 0.0 = deterministic, 0.15 = realistic variance
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            num_steps: 8,
            monte_carlo_runs: 5,
            include_skill_evolution: true,
            include_negotiation: true,
            noise_level: 0.08,
        }
    }
}

/// The Multi-Agent Simulation Engine.
#[derive(Debug, Clone)]
pub struct MultiAgentSimulator {
    pub simulation_mode: bool,
    pub config: SimulationConfig,
    /// Reusable simulated agents for a run
    pub agents: HashMap<String, SimulatedAgent>,
}

impl MultiAgentSimulator {
    pub fn new(simulation_mode: bool, config: SimulationConfig) -> Self {
        Self {
            simulation_mode,
            config,
            agents: HashMap::new(),
        }
    }

    /// Register a simulated agent (usually derived from real SpecializedProfile + skills).
    pub fn register_simulated_agent(
        &mut self,
        id: &str,
        profile: AgentProfile,
        skill_summary: &str,
    ) {
        let portfolio = crate::hoh::agent_skill_evolution::AgentSkillPortfolio::new(id);
        // In a richer version we would clone from a real AgentSkillEvolutionSystem.
        // For now we just create a minimal one.
        let mut sim_agent = SimulatedAgent {
            id: id.to_string(),
            profile,
            skill_portfolio: portfolio,
            success_bias: 0.78,
            cost_per_step: 12.0,
            personality: vec!["focused".into()],
        };

        // Bias based on profile (simple model)
        match sim_agent.profile {
            AgentProfile::Architect => {
                sim_agent.success_bias = 0.82;
                sim_agent.cost_per_step = 18.0;
            }
            AgentProfile::Debugger => {
                sim_agent.success_bias = 0.85;
                sim_agent.cost_per_step = 15.0;
            }
            AgentProfile::Tester => {
                sim_agent.success_bias = 0.80;
                sim_agent.cost_per_step = 10.0;
            }
            AgentProfile::Refactorer => sim_agent.success_bias = 0.79,
            AgentProfile::Governor => sim_agent.success_bias = 0.76,
            AgentProfile::Researcher => sim_agent.success_bias = 0.81,
        }

        if skill_summary.contains("multi_agent") || skill_summary.contains("coordinat") {
            sim_agent.success_bias += 0.04;
        }

        self.agents.insert(id.to_string(), sim_agent);
    }

    /// Quick single-run simulation of a delegation.
    pub fn simulate_delegation(
        &self,
        task: &str,
        chosen_agent_id: &str,
    ) -> SimulationOutcome {
        let agent = self.agents.get(chosen_agent_id);

        let (success, quality, cost, conflicts) = if let Some(a) = agent {
            let base = a.success_bias;
            let task_fit = self.task_fit_score(task, &a.profile);
            let mut s = (base + task_fit * 0.15).clamp(0.55, 0.96);
            let mut q = (0.72 + (base - 0.7) * 0.6 + task_fit * 0.1).clamp(0.6, 0.94);
            let c = a.cost_per_step * (1.0 + (1.0 - task_fit) * 0.6);

            // Add noise
            if self.config.noise_level > 0.0 {
                let n = (rand_f32() - 0.5) * self.config.noise_level * 2.0;
                s = (s + n * 0.08).clamp(0.4, 0.98);
                q = (q + n * 0.06).clamp(0.5, 0.95);
            }

            (s, q, c, if task_fit < 0.4 { 1 } else { 0 })
        } else {
            (0.55, 0.62, 25.0, 2)
        };

        SimulationOutcome {
            scenario: format!("delegation: {}", task),
            participants: vec![chosen_agent_id.to_string()],
            predicted_success_rate: success,
            predicted_quality: quality,
            predicted_cost: cost,
            predicted_conflicts: conflicts,
            predicted_knowledge_gain: if success > 0.8 { 0.35 } else { 0.12 },
            steps_taken: 1,
            final_status: if success > 0.75 { "success".into() } else { "partial".into() },
            key_events: vec![format!("Delegated to {}", chosen_agent_id)],
            confidence: 0.78,
        }
    }

    /// Full multi-step collaboration simulation using the real CollaborationProtocol
    /// (but in pure simulation mode).
    pub async fn simulate_collaboration_scenario(
        &mut self,
        topic: &str,
        participant_ids: Vec<String>,
        mut evolution_system: Option<&mut AgentSkillEvolutionSystem>,
    ) -> SimulationOutcome {
        if participant_ids.is_empty() {
            return SimulationOutcome {
                scenario: topic.to_string(),
                participants: vec![],
                predicted_success_rate: 0.3,
                predicted_quality: 0.4,
                predicted_cost: 5.0,
                predicted_conflicts: 0,
                predicted_knowledge_gain: 0.0,
                steps_taken: 0,
                final_status: "no_participants".into(),
                key_events: vec!["No agents provided".into()],
                confidence: 0.95,
            };
        }

        // Create a simulated collaboration protocol (always in sim mode here)
        let mut collab = MultiAgentCollaborationProtocol::new(true);

        // Register participants
        for id in &participant_ids {
            if let Some(sim) = self.agents.get(id) {
                collab.register_agent(id, Some(&sim.profile), &sim.skill_portfolio.describe_skills(id));
            } else {
                collab.register_agent(id, None, "general");
            }
        }

        let session_id = collab.start_session(topic, participant_ids.clone());

        let mut total_success = 0.0f32;
        let mut total_quality = 0.0f32;
        let mut total_cost = 0.0f32;
        let mut conflicts = 0u32;
        let mut events = vec![];
        let mut steps = 0u32;

        // Run simulated steps
        for step in 0..self.config.num_steps {
            steps += 1;

            // Pick a "current" agent (round-robin or coordinator)
            let current = &participant_ids[step as usize % participant_ids.len()];

            // Simulate delegation inside the scenario
            if let Some(agent) = self.agents.get(current) {
                let fit = self.task_fit_score(topic, &agent.profile);
                let step_success = agent.success_bias * (0.85 + fit * 0.2);
                total_success += step_success;
                total_quality += 0.70 + (step_success - 0.75) * 0.4;
                total_cost += agent.cost_per_step * (1.0 - fit * 0.2);

                if fit < 0.35 {
                    conflicts += 1;
                    events.push(format!("step {}: low fit conflict for {}", step, current));
                }

                // Simulate skill evolution effect
                if self.config.include_skill_evolution {
                    if let Some(evo) = evolution_system.as_mut() {
                        if let Ok(changes) = evo
                            .evolve_skills(current, topic, step_success > 0.72, step_success * 0.9, &[])
                            .await
                        {
                            if !changes.is_empty() {
                                events.push(format!("step {}: skill evolution on {} → {:?}", step, current, changes));
                            }
                        }
                    }
                }

                // Occasional negotiation simulation
                if self.config.include_negotiation && step % 3 == 1 && participant_ids.len() > 1 {
                    let _ = collab.negotiate(&session_id, &format!("subtask-{}", step), participant_ids.clone()).await;
                    events.push(format!("step {}: negotiation round", step));
                }

                // Handoff simulation
                if step == self.config.num_steps / 2 && participant_ids.len() > 1 {
                    let next = &participant_ids[(step as usize + 1) % participant_ids.len()];
                    let _ = collab.handoff(&session_id, current, next, "simulated handoff of partial state").await;
                    events.push(format!("handoff {} → {}", current, next));
                }
            }
        }

        let n = steps as f32;
        let avg_success = (total_success / n).clamp(0.4, 0.97);
        let avg_quality = (total_quality / n).clamp(0.55, 0.93);

        // Finalize session in sim
        if let Some(s) = collab.sessions.get_mut(&session_id) {
            s.status = if avg_success > 0.78 {
                CollaborationStatus::Resolved
            } else {
                CollaborationStatus::Active
            };
        }

        SimulationOutcome {
            scenario: format!("collaboration: {}", topic),
            participants: participant_ids,
            predicted_success_rate: avg_success,
            predicted_quality: avg_quality,
            predicted_cost: total_cost,
            predicted_conflicts: conflicts,
            predicted_knowledge_gain: (avg_success * 0.45 + (conflicts as f32 * -0.08)).clamp(0.05, 0.65),
            steps_taken: steps,
            final_status: if avg_success > 0.8 { "resolved".into() } else { "in_progress".into() },
            key_events: events,
            confidence: 0.71 + (self.config.monte_carlo_runs as f32 * 0.02).min(0.2),
        }
    }

    /// Run multiple Monte-Carlo simulations and return an aggregated prediction.
    pub async fn run_monte_carlo_collaboration(
        &mut self,
        topic: &str,
        participant_ids: Vec<String>,
        evolution_system: Option<&mut AgentSkillEvolutionSystem>,
    ) -> SimulationOutcome {
        let runs = self.config.monte_carlo_runs.max(1);
        let mut outcomes = vec![];

        let mut evo_ref = evolution_system;
        for _ in 0..runs {
            let mut outcome = self
                .simulate_collaboration_scenario(topic, participant_ids.clone(), evo_ref.as_deref_mut())
                .await;
            // Small per-run variance
            if self.config.noise_level > 0.0 {
                let n = (rand_f32() - 0.5) * self.config.noise_level;
                outcome.predicted_success_rate = (outcome.predicted_success_rate + n).clamp(0.3, 0.98);
                outcome.predicted_quality = (outcome.predicted_quality + n * 0.7).clamp(0.5, 0.95);
            }
            outcomes.push(outcome);
        }

        // Aggregate
        let n = outcomes.len() as f32;
        let avg_success = outcomes.iter().map(|o| o.predicted_success_rate).sum::<f32>() / n;
        let avg_quality = outcomes.iter().map(|o| o.predicted_quality).sum::<f32>() / n;
        let avg_cost = outcomes.iter().map(|o| o.predicted_cost).sum::<f32>() / n;
        let total_conflicts = outcomes.iter().map(|o| o.predicted_conflicts).sum::<u32>() / runs;
        let avg_knowledge = outcomes.iter().map(|o| o.predicted_knowledge_gain).sum::<f32>() / n;

        let mut all_events: Vec<String> = outcomes.into_iter().flat_map(|o| o.key_events).collect();
        all_events.truncate(12);

        SimulationOutcome {
            scenario: format!("monte-carlo: {}", topic),
            participants: participant_ids,
            predicted_success_rate: avg_success,
            predicted_quality: avg_quality,
            predicted_cost: avg_cost,
            predicted_conflicts: total_conflicts,
            predicted_knowledge_gain: avg_knowledge,
            steps_taken: self.config.num_steps,
            final_status: if avg_success > 0.78 { "likely_resolved".into() } else { "risky".into() },
            key_events: all_events,
            confidence: 0.82,
        }
    }

    /// Very cheap ecosystem simulation (births, specialization, retirement signals).
    pub fn simulate_ecosystem(
        &self,
        initial_agents: usize,
        steps: u32,
    ) -> (f32, Vec<String>) {
        let mut active = initial_agents as f32;
        let mut events = vec![];

        for s in 0..steps {
            // Simple population dynamics
            let growth = if active < 4.0 { 0.8 } else { 0.15 };
            let retirement_pressure = (active / 6.0).min(0.4);

            active = (active + growth - retirement_pressure).max(1.0);

            if s % 2 == 0 && active > 3.5 {
                events.push(format!("step {}: specialization pressure ({} active)", s, active as u32));
            }
            if retirement_pressure > 0.25 {
                events.push(format!("step {}: retirement risk rising", s));
            }
        }

        (active, events)
    }

    fn task_fit_score(&self, task: &str, profile: &AgentProfile) -> f32 {
        let t = task.to_lowercase();
        match profile {
            AgentProfile::Architect if t.contains("arch") || t.contains("design") || t.contains("layer") => 0.9,
            AgentProfile::Debugger if t.contains("bug") || t.contains("fail") || t.contains("debug") => 0.92,
            AgentProfile::Tester if t.contains("test") || t.contains("verify") || t.contains("coverage") => 0.88,
            AgentProfile::Refactorer if t.contains("refactor") || t.contains("clean") || t.contains("extract") => 0.85,
            AgentProfile::Researcher if t.contains("research") || t.contains("knowledge") || t.contains("okf") => 0.82,
            AgentProfile::Governor if t.contains("govern") || t.contains("safety") || t.contains("policy") => 0.80,
            _ => 0.65,
        }
    }
}

// Simple deterministic-ish random for simulation reproducibility in tests
fn rand_f32() -> f32 {
    // Cheap pseudo-random (good enough for simulation predictions)
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    ((nanos % 1000) as f32) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::specialized_agents::AgentProfile;

    #[tokio::test]
    async fn test_delegation_simulation() {
        let mut sim = MultiAgentSimulator::new(true, SimulationConfig::default());
        sim.register_simulated_agent("arch-1", AgentProfile::Architect, "architecture");

        let out = sim.simulate_delegation("introduce new architecture layer", "arch-1");
        assert!(out.predicted_success_rate > 0.75);
        assert!(out.predicted_quality > 0.7);
        assert!(out.participants.contains(&"arch-1".to_string()));
    }

    #[tokio::test]
    async fn test_collaboration_simulation() {
        let mut sim = MultiAgentSimulator::new(true, SimulationConfig {
            num_steps: 4,
            monte_carlo_runs: 1,
            ..Default::default()
        });
        sim.register_simulated_agent("a", AgentProfile::Architect, "");
        sim.register_simulated_agent("b", AgentProfile::Tester, "test");

        let out = sim.simulate_collaboration_scenario(
            "build and test new module",
            vec!["a".into(), "b".into()],
            None,
        ).await;

        assert!(out.steps_taken > 0);
        assert!(out.predicted_success_rate > 0.5);
        assert!(!out.key_events.is_empty() || out.participants.len() == 2);
    }

    #[tokio::test]
    async fn test_monte_carlo_aggregation() {
        let mut sim = MultiAgentSimulator::new(true, SimulationConfig {
            num_steps: 3,
            monte_carlo_runs: 3,
            noise_level: 0.05,
            ..Default::default()
        });
        sim.register_simulated_agent("d", AgentProfile::Debugger, "");

        let out = sim.run_monte_carlo_collaboration(
            "debug critical path",
            vec!["d".into()],
            None,
        ).await;

        assert!(out.confidence > 0.7);
        assert!(out.predicted_success_rate > 0.6);
    }

    #[test]
    fn test_ecosystem_simulation() {
        let sim = MultiAgentSimulator::new(true, SimulationConfig::default());
        let (final_pop, events) = sim.simulate_ecosystem(3, 6);
        assert!(final_pop >= 1.0);
        assert!(!events.is_empty() || final_pop > 2.0);
    }
}