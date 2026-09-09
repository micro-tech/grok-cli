//! HOH Multi-Agent Orchestrator (361.7 + 361.8 integration)
//!
//! High-level coordinator that combines:
//! - Agent Skill Evolution (361.6)
//! - Collaboration Protocol (361.7)
//! - Multi-Agent Simulation (361.8)
//!
//! This is the practical entry point planners, the outer loop, and birth/lifecycle systems
//! should use when they want "multi-agent behavior".
//!
//! Responsibilities:
//! - Decide when to use real agents vs simulation
//! - Run cheap simulations to choose agent mix before committing
//! - Execute real delegations/handoffs using the protocol
//! - Track skill evolution across the ecosystem
//! - Provide a clean "orchestrate" API for the rest of HOH

use crate::hoh::state::HOHError;
use crate::hoh::specialized_agents::{AgentProfile, choose_profile_for_action};
use crate::hoh::agent_skill_evolution::AgentSkillEvolutionSystem;
use crate::hoh::multi_agent_collaboration::MultiAgentCollaborationProtocol;
use crate::hoh::multi_agent_simulation::{MultiAgentSimulator, SimulationConfig, SimulationOutcome};
use crate::hoh::autonomous_refactoring::RefactoringAction;

/// High-level request for multi-agent work.
#[derive(Debug, Clone)]
pub struct MultiAgentRequest {
    pub task_description: String,
    pub goals: Vec<String>,
    pub suggested_profiles: Vec<AgentProfile>,
    pub use_simulation_first: bool,
    pub max_agents: usize,
}

/// Result of orchestration (whether simulated or real).
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    pub chosen_agents: Vec<String>,
    pub simulation_prediction: Option<SimulationOutcome>,
    pub real_execution_summary: Option<String>,
    pub skill_evolution_events: Vec<String>,
    pub success_estimate: f32,
}

/// The central Multi-Agent Orchestrator for HOH.
pub struct MultiAgentOrchestrator {
    pub simulation_mode: bool,
    pub skill_evolution: AgentSkillEvolutionSystem,
    pub collaboration: MultiAgentCollaborationProtocol,
    pub simulator: MultiAgentSimulator,
}

impl MultiAgentOrchestrator {
    pub fn new(simulation_mode: bool) -> Self {
        let config = SimulationConfig {
            num_steps: 6,
            monte_carlo_runs: 3,
            include_skill_evolution: true,
            include_negotiation: true,
            noise_level: 0.07,
        };

        Self {
            simulation_mode,
            skill_evolution: AgentSkillEvolutionSystem::new(simulation_mode),
            collaboration: MultiAgentCollaborationProtocol::new(simulation_mode),
            simulator: MultiAgentSimulator::new(simulation_mode, config),
        }
    }

    /// Bootstrap an agent into the ecosystem (skill + registry).
    pub fn bootstrap_agent(&mut self, agent_id: &str, profile: AgentProfile, extra_keywords: &[String]) {
        self.skill_evolution.bootstrap_agent(agent_id, &profile, extra_keywords);
        let summary = self.skill_evolution.describe_skills(agent_id);
        self.collaboration.register_agent(agent_id, Some(&profile), &summary);
        self.simulator.register_simulated_agent(agent_id, profile, &summary);
    }

    /// Main entry point: given a task, decide on agents and (optionally) simulate first.
    pub async fn orchestrate(
        &mut self,
        request: MultiAgentRequest,
    ) -> Result<OrchestrationResult, HOHError> {
        let mut result = OrchestrationResult {
            chosen_agents: vec![],
            simulation_prediction: None,
            real_execution_summary: None,
            skill_evolution_events: vec![],
            success_estimate: 0.65,
        };

        // 1. Choose candidate profiles (use specialized logic if no explicit list)
        let candidates = if request.suggested_profiles.is_empty() {
            self.infer_profiles_from_task(&request.task_description, &request.goals)
        } else {
            request.suggested_profiles
        };

        // Limit
        let candidates: Vec<_> = candidates.into_iter().take(request.max_agents).collect();

        // 2. Assign IDs (in real system these would come from birth or registry)
        let mut agent_ids = vec![];
        for (i, profile) in candidates.iter().enumerate() {
            let id = format!("agent-{}-{}", profile.name().to_lowercase().replace(" ", "-"), i);
            // Bootstrap if not already known
            if !self.simulator.agents.contains_key(&id) {
                self.bootstrap_agent(&id, profile.clone(), &[]);
            }
            agent_ids.push(id);
        }

        result.chosen_agents = agent_ids.clone();

        // 3. Simulation phase (cheap prediction)
        if request.use_simulation_first && !agent_ids.is_empty() {
            let sim_outcome = self.simulator
                .run_monte_carlo_collaboration(
                    &request.task_description,
                    agent_ids.clone(),
                    Some(&mut self.skill_evolution),
                )
                .await;

            result.simulation_prediction = Some(sim_outcome.clone());
            result.success_estimate = sim_outcome.predicted_success_rate;

            // If simulation looks bad, we could adjust agents here (future enhancement)
            if sim_outcome.predicted_success_rate < 0.55 {
                result.skill_evolution_events.push("Simulation suggested low success – consider different mix".into());
            }
        }

        // 4. Real execution path (or simulated real)
        if !self.simulation_mode && !agent_ids.is_empty() {
            // Use the collaboration protocol for real delegation
            let (lead, _msg) = self.collaboration
                .delegate("orchestrator", &request.task_description, Some(&self.skill_evolution))
                .await?;

            // Evolve skills based on "assumed" positive outcome for now
            // In real flow this would come after the work is done
            let evo_events = self.skill_evolution
                .evolve_skills(&lead, &request.task_description, true, 0.82, &[])
                .await?;

            result.skill_evolution_events.extend(evo_events);
            result.real_execution_summary = Some(format!("Delegated lead work to {}", lead));
        } else {
            // In full simulation we still run the collaboration logic (already done above)
            result.real_execution_summary = Some("Executed entirely in simulation".into());
        }

        // 5. Record a collaboration session for observability
        let _sid = self.collaboration.start_session(&request.task_description, agent_ids.clone());

        Ok(result)
    }

    /// Infer reasonable profiles from task text + goals (lightweight router).
    fn infer_profiles_from_task(&self, task: &str, goals: &[String]) -> Vec<AgentProfile> {
        let combined = format!("{} {}", task, goals.join(" ")).to_lowercase();
        let mut profiles = vec![];

        if combined.contains("arch") || combined.contains("design") || combined.contains("layer") {
            profiles.push(AgentProfile::Architect);
        }
        if combined.contains("test") || combined.contains("verify") || combined.contains("coverage") {
            profiles.push(AgentProfile::Tester);
        }
        if combined.contains("bug") || combined.contains("debug") || combined.contains("fail") {
            profiles.push(AgentProfile::Debugger);
        }
        if combined.contains("refactor") || combined.contains("clean") || combined.contains("extract") {
            profiles.push(AgentProfile::Refactorer);
        }
        if combined.contains("multi") || combined.contains("collaborat") || combined.contains("agent") {
            profiles.push(AgentProfile::Governor); // or a coordinator role
        }
        if combined.contains("research") || combined.contains("knowledge") || combined.contains("okf") {
            profiles.push(AgentProfile::Researcher);
        }

        if profiles.is_empty() {
            profiles.push(AgentProfile::Refactorer);
        }

        // Always add a second agent for collaboration flavor when possible
        if profiles.len() == 1 && combined.contains("complex") {
            profiles.push(AgentProfile::Tester);
        }

        profiles
    }

    /// Convenience: orchestrate a refactoring action using the specialized profile system.
    pub async fn orchestrate_refactoring_action(
        &mut self,
        action: &RefactoringAction, // or a generic task struct
    ) -> Result<OrchestrationResult, HOHError> {
        let profile = choose_profile_for_action(action);
        let req = MultiAgentRequest {
            task_description: format!("{}: {}", action.title, action.description),
            goals: vec![],
            suggested_profiles: vec![profile],
            use_simulation_first: true,
            max_agents: 2,
        };
        self.orchestrate(req).await
    }

    /// Get current ecosystem health summary (useful for planner / governance).
    pub fn ecosystem_summary(&self) -> String {
        let active = self.simulator.agents.len();
        let skills = self.skill_evolution.portfolios.len();
        format!(
            "Multi-agent ecosystem: {} simulated agents, {} skill portfolios tracked. Collaboration sessions: {}",
            active,
            skills,
            self.collaboration.sessions.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::specialized_agents::AgentProfile;

    #[tokio::test]
    async fn test_orchestrator_basic_flow() {
        let mut orch = MultiAgentOrchestrator::new(true);

        let req = MultiAgentRequest {
            task_description: "Improve multi-agent collaboration and add new architecture layer".to_string(),
            goals: vec!["better agents".into()],
            suggested_profiles: vec![AgentProfile::Architect, AgentProfile::Tester],
            use_simulation_first: true,
            max_agents: 3,
        };

        let res = orch.orchestrate(req).await.unwrap();
        assert!(!res.chosen_agents.is_empty());
        assert!(res.simulation_prediction.is_some());
        assert!(res.success_estimate > 0.5);
    }

    #[test]
    fn test_infer_profiles() {
        let orch = MultiAgentOrchestrator::new(true);
        let profiles = orch.infer_profiles_from_task("debug failing tests and refactor module", &[]);
        assert!(profiles.iter().any(|p| matches!(p, AgentProfile::Debugger)));
        assert!(profiles.iter().any(|p| matches!(p, AgentProfile::Tester | AgentProfile::Refactorer)));
    }
}