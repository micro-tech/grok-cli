//! HOH Agent Birth System (Task 404)
//!
//! Spawns new specialized agents when HOH detects emerging needs or opportunities.
//!
//! Integrates tightly with:
//! - 403 Agent Lifecycle (reuses retired/hibernated slots via available_slots())
//! - 401 Creativity + 402 Generative Designs (opportunity signals)
//! - 361.4 Specialized routing (births for high-value roles)
//! - Real spawn_agent infrastructure (tools/agent_tools)
//!
//! Supports:
//! - Permanent agents (tracked in lifecycle)
//! - Ephemeral / temporary agents (one-shot, lighter footprint)
//! - Knowledge bootstrapping from OKF + recent iteration history

use crate::hoh::agent_lifecycle::AgentLifecycleManager;
use crate::hoh::creativity::CreativityIdea;
use crate::hoh::generative_designer::ArchitectureDesign;
use crate::hoh::state::HOHError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Profile for a newly born agent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentProfile {
    pub role: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub personality_traits: Vec<String>,
    pub max_tokens: u32,
    pub trusted_dirs: Vec<String>,
    pub is_ephemeral: bool,
    pub bootstrap_knowledge: Vec<String>,
}

/// Result of a birth attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthEvent {
    pub agent_id: String,
    pub role: String,
    pub reason: String,
    pub profile_summary: String,
    pub used_retired_slot: bool,
    pub spawned_successfully: bool,
    pub message: String,
    pub at: u64,
}

/// Signals that can trigger agent birth.
#[derive(Debug, Clone, Default)]
pub struct BirthSignals {
    pub skill_gaps: Vec<String>,
    pub opportunity_from_creativity: Vec<String>,
    pub workload_pressure: bool,
    pub failed_iterations: usize,
    pub architecture_needs: Vec<String>,
}

/// The Agent Birth System.
#[derive(Debug, Clone)]
pub struct AgentBirthSystem {
    pub simulation_mode: bool,
    pub max_births_per_cycle: usize,
}

impl AgentBirthSystem {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            simulation_mode,
            max_births_per_cycle: 3,
        }
    }

    /// Main entry point: analyze context and attempt births.
    /// Returns birth events (some may be "planned" only in simulation).
    pub async fn detect_and_birth(
        &self,
        goals: &[String],
        recent_tasks: &[u64],
        creative_ideas: &[CreativityIdea],
        architecture_designs: &[ArchitectureDesign],
        lifecycle: &mut AgentLifecycleManager,
        recent_failures: usize,
    ) -> Result<Vec<BirthEvent>, HOHError> {
        let signals = self.analyze_needs(
            goals,
            recent_tasks,
            creative_ideas,
            architecture_designs,
            recent_failures,
        );

        if signals.skill_gaps.is_empty()
            && signals.opportunity_from_creativity.is_empty()
            && !signals.workload_pressure
            && signals.architecture_needs.is_empty()
        {
            return Ok(vec![]);
        }

        let mut births = Vec::new();
        let available_slots = lifecycle.available_slots();
        let mut used_slots = HashSet::new();

        let candidate_roles = self.synthesize_roles(&signals, goals, creative_ideas);

        for role in candidate_roles.into_iter().take(self.max_births_per_cycle) {
            if births.len() >= self.max_births_per_cycle {
                break;
            }

            // Prefer reusing a retired slot (403 synergy)
            let (agent_id, used_retired) = if let Some(slot) = available_slots
                .iter()
                .find(|s| !used_slots.contains(*s))
            {
                used_slots.insert(slot.clone());
                (slot.clone(), true)
            } else {
                // Fresh birth
                let fresh_id = format!(
                    "born-{}-{}",
                    role.to_lowercase().replace(|c: char| !c.is_alphanumeric(), ""),
                    chrono::Utc::now().timestamp() % 100000
                );
                (fresh_id, false)
            };

            let profile = self.generate_profile(&role, goals, &signals);

            // Bootstrap knowledge
            let bootstrap = self.bootstrap_knowledge(&role, creative_ideas, architecture_designs);
            let mut final_profile = profile.clone();
            final_profile.bootstrap_knowledge = bootstrap;

            // Attempt actual spawn (or simulate)
            let spawn_result = self
                .attempt_spawn(&agent_id, &role, &final_profile)
                .await;

            // Register in lifecycle (always, even in sim)
            lifecycle.register_agent(&agent_id, &role);

            // Record a positive initial outcome signal (optimistic bootstrap)
            lifecycle.record_outcome(&agent_id, true, Some(0.75), 20.0);

            let event = BirthEvent {
                agent_id: agent_id.clone(),
                role: role.clone(),
                reason: self.describe_reason(&signals, &role),
                profile_summary: format!(
                    "role={}, tools={}, ephemeral={}",
                    role,
                    final_profile.allowed_tools.len(),
                    final_profile.is_ephemeral
                ),
                used_retired_slot: used_retired,
                spawned_successfully: spawn_result.is_ok(),
                message: spawn_result.unwrap_or_else(|e| format!("Simulated birth: {}", e)),
                at: chrono::Utc::now().timestamp() as u64,
            };

            births.push(event.clone());

            tracing::info!(
                agent_id = %agent_id,
                role = %role,
                reused = used_retired,
                "HOH (404): Agent born — {}",
                event.reason
            );
        }

        Ok(births)
    }

    /// Analyze the current state for birth triggers.
    fn analyze_needs(
        &self,
        goals: &[String],
        recent_tasks: &[u64],
        creative_ideas: &[CreativityIdea],
        architecture_designs: &[ArchitectureDesign],
        recent_failures: usize,
    ) -> BirthSignals {
        let mut skill_gaps = Vec::new();
        let mut opportunities = Vec::new();
        let mut arch_needs = Vec::new();

        let goal_text = goals.join(" ").to_lowercase();

        // 1. Skill gaps from goals and tasks
        if goal_text.contains("multi-agent") || goal_text.contains("collaboration") {
            skill_gaps.push("multi_agent_coordinator".to_string());
        }
        if goal_text.contains("memory") || goal_text.contains("knowledge") {
            skill_gaps.push("long_term_memory_specialist".to_string());
        }
        if goal_text.contains("governance") || goal_text.contains("safety") {
            skill_gaps.push("governance_and_ethics".to_string());
        }
        if goal_text.contains("test") || goal_text.contains("quality") {
            skill_gaps.push("test_engineer".to_string());
        }
        if goal_text.contains("refactor") || goal_text.contains("architecture") {
            skill_gaps.push("autonomous_refactorer".to_string());
        }

        // From high-priority recent tasks
        for &tid in recent_tasks {
            if tid >= 400 && tid < 430 {
                skill_gaps.push(format!("task_{}_specialist", tid));
            }
        }

        // 2. Creativity opportunities
        for idea in creative_ideas {
            if idea.overall_score > 0.6
                && (idea.title.to_lowercase().contains("agent")
                    || idea.title.to_lowercase().contains("specialist")
                    || idea.title.to_lowercase().contains("ecosystem"))
            {
                opportunities.push(idea.title.clone());
            }
        }

        // 3. Architecture designs that explicitly need new agents
        for design in architecture_designs {
            if design.new_modules.iter().any(|m| {
                m.name.to_lowercase().contains("agent")
                    || m.purpose.to_lowercase().contains("birth")
                    || m.purpose.to_lowercase().contains("ecosystem")
            }) {
                arch_needs.push(design.title.clone());
            }
        }

        // 4. Workload / failure pressure
        let workload_pressure = recent_failures >= 2 || recent_tasks.len() > 6;

        BirthSignals {
            skill_gaps,
            opportunity_from_creativity: opportunities,
            workload_pressure,
            failed_iterations: recent_failures,
            architecture_needs: arch_needs,
        }
    }

    /// Turn signals into concrete role names to birth.
    fn synthesize_roles(
        &self,
        signals: &BirthSignals,
        goals: &[String],
        _ideas: &[CreativityIdea],
    ) -> Vec<String> {
        let mut roles = Vec::new();
        let goal_text = goals.join(" ").to_lowercase();

        // Priority order
        if signals.skill_gaps.contains(&"multi_agent_coordinator".to_string())
            || goal_text.contains("multi-agent")
        {
            roles.push("MultiAgentCoordinator".to_string());
        }
        if signals.skill_gaps.contains(&"long_term_memory_specialist".to_string()) {
            roles.push("LongTermMemorySpecialist".to_string());
        }
        if !signals.architecture_needs.is_empty() {
            roles.push("ArchitectureSynthesizer".to_string());
        }
        if signals.workload_pressure || signals.failed_iterations > 1 {
            roles.push("FailureRecoverySpecialist".to_string());
        }
        if signals.opportunity_from_creativity.len() > 1 {
            roles.push("CreativeIdeator".to_string());
        }
        if goal_text.contains("test") {
            roles.push("TestGenerationSpecialist".to_string());
        }

        // Always add at least one if we have any signal
        if roles.is_empty() && (!signals.skill_gaps.is_empty() || signals.workload_pressure) {
            roles.push("EmergentSpecialist".to_string());
        }

        // Dedup
        roles.sort();
        roles.dedup();
        roles
    }

    /// Generate a rich agent profile for the role.
    fn generate_profile(&self, role: &str, _goals: &[String], signals: &BirthSignals) -> AgentProfile {
        let is_ephemeral = role.contains("Creative") || role.contains("Recovery") || signals.failed_iterations > 3;

        let (prompt, tools, traits) = match role {
            "MultiAgentCoordinator" => (
                "You are an expert multi-agent coordinator. You analyze workloads, assign tasks to the best specialists, mediate conflicts, and maintain ecosystem health. Be decisive and concise.".to_string(),
                vec!["spawn_agent".to_string(), "send_message".to_string(), "list_agents".to_string(), "join_agents".to_string()],
                vec!["decisive".to_string(), "orchestrator".to_string(), "fair".to_string()],
            ),
            "LongTermMemorySpecialist" => (
                "You specialize in long-term knowledge management for autonomous systems. You synthesize, index, and retrieve cross-iteration knowledge from OKF and HOH history.".to_string(),
                vec!["okf_lookup".to_string(), "okf_get".to_string(), "recall_context".to_string()],
                vec!["synthesizer".to_string(), "archivist".to_string(), "precise".to_string()],
            ),
            "ArchitectureSynthesizer" => (
                "You are a senior software architect. Given goals and current state, you propose coherent, evolvable architectures and migration paths.".to_string(),
                vec!["read_file".to_string(), "glob_search".to_string()],
                vec!["visionary".to_string(), "pragmatic".to_string(), "system_thinker".to_string()],
            ),
            "FailureRecoverySpecialist" => (
                "You are a debugging and recovery expert. When iterations fail or stall, you diagnose root causes, propose minimal recovery patches, and restore momentum.".to_string(),
                vec!["run_shell_command".to_string(), "search_file_content".to_string()],
                vec!["resilient".to_string(), "diagnostic".to_string(), "calm".to_string()],
            ),
            "CreativeIdeator" => (
                "You are a high-novelty idea generator. Produce surprising but useful concepts that go beyond obvious next steps.".to_string(),
                vec![],
                vec!["imaginative".to_string(), "bold".to_string(), "playful".to_string()],
            ),
            "TestGenerationSpecialist" => (
                "You excel at writing high-quality tests, property-based tests, and test strategies that catch real regressions.".to_string(),
                vec!["run_shell_command".to_string()],
                vec!["thorough".to_string(), "skeptical".to_string()],
            ),
            _ => (
                format!("You are a specialized {} agent for the HOH autonomous development system. Focus on your role and contribute high-value work.", role),
                vec!["read_file".to_string(), "search_file_content".to_string()],
                vec!["focused".to_string(), "reliable".to_string()],
            ),
        };

        AgentProfile {
            role: role.to_string(),
            system_prompt: prompt,
            allowed_tools: tools,
            personality_traits: traits,
            max_tokens: if is_ephemeral { 1024 } else { 4096 },
            trusted_dirs: vec![".".to_string()],
            is_ephemeral,
            bootstrap_knowledge: vec![],
        }
    }

    /// Pull relevant knowledge for bootstrapping.
    fn bootstrap_knowledge(
        &self,
        role: &str,
        ideas: &[CreativityIdea],
        designs: &[ArchitectureDesign],
    ) -> Vec<String> {
        let mut knowledge = vec![
            "HOH outer loop philosophy: plan → execute → evaluate → improve".to_string(),
            "Always respect safety and governance layers".to_string(),
        ];

        // Pull from recent creative ideas
        for idea in ideas.iter().take(2) {
            if idea.overall_score > 0.55 {
                knowledge.push(format!("Recent idea: {} — {}", idea.title, idea.description));
            }
        }

        // Pull from architecture designs
        for d in designs.iter().take(1) {
            knowledge.push(format!("Architecture context: {}", d.title));
        }

        if role.contains("Memory") {
            knowledge.push("Prioritize durable, queryable, versioned storage".to_string());
        }
        if role.contains("Coordinator") {
            knowledge.push("Use send_message and spawn_agent for collaboration".to_string());
        }

        knowledge
    }

    fn describe_reason(&self, signals: &BirthSignals, role: &str) -> String {
        if signals.architecture_needs.iter().any(|n| n.contains(&role.replace("Specialist", ""))) {
            return format!("Architecture need from 402 design for role {}", role);
        }
        if signals.opportunity_from_creativity.iter().any(|o| o.contains(&role.replace("Specialist", ""))) {
            return format!("Creative opportunity (401) for {}", role);
        }
        if signals.workload_pressure {
            return format!("Workload / recovery pressure ({} failures)", signals.failed_iterations);
        }
        if !signals.skill_gaps.is_empty() {
            return format!("Skill gap detected: {}", signals.skill_gaps.join(", "));
        }
        format!("Emergent need for {}", role)
    }

    /// Attempt to actually spawn the agent using the real tool infrastructure.
    async fn attempt_spawn(&self, agent_id: &str, role: &str, profile: &AgentProfile) -> Result<String, String> {
        if self.simulation_mode {
            return Ok(format!(
                "SIMULATED: Would spawn {} ({}) with {} tools. Prompt length: {}",
                agent_id, role, profile.allowed_tools.len(), profile.system_prompt.len()
            ));
        }

        // Real spawn path — craft a SubAgentConfig-like task
        // We use the public spawn_agent for simplicity and safety.
        let task = format!(
            "You are a freshly born HOH agent with role: {}. \n\nPersonality: {:?}\n\nBootstrap knowledge:\n{}\n\nBegin by acknowledging your birth and stating how you will contribute to the current HOH goals.",
            role,
            profile.personality_traits,
            profile.bootstrap_knowledge.join("\n- ")
        );

        // Use the existing spawn_agent (lightweight path)
        match crate::tools::agent_tools::spawn_agent(&task, "", profile.max_tokens).await {
            Ok(result) => {
                Ok(format!("Real spawn succeeded for {}: {}", agent_id, result.chars().take(120).collect::<String>()))
            }
            Err(e) => {
                // Still count as "born" in lifecycle even if spawn had issues
                Err(format!("Spawn attempt failed (still registered in lifecycle): {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::agent_lifecycle::AgentLifecycleManager;
    use crate::hoh::creativity::CreativityIdea;

    #[tokio::test]
    async fn test_birth_detects_multi_agent_need() {
        let mut lifecycle = AgentLifecycleManager::new(true);
        let system = AgentBirthSystem::new(true);

        let goals = vec!["improve multi-agent collaboration".to_string()];
        let ideas = vec![CreativityIdea {
            id: "c1".into(),
            title: "Agent Ecosystem Birth System".into(),
            description: "...".into(),
            overall_score: 0.78,
            ..Default::default()
        }];

        let events = system
            .detect_and_birth(&goals, &[404], &ideas, &[], &mut lifecycle, 0)
            .await
            .unwrap();

        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.role.contains("MultiAgent") || e.role.contains("Coordinator")));
    }

    #[tokio::test]
    async fn test_reuses_retired_slot() {
        let mut lifecycle = AgentLifecycleManager::new(true);
        lifecycle.register_agent("old-123", "ObsoleteRole");
        let _ = lifecycle.retire_agent("old-123", "test retirement for 404");

        let system = AgentBirthSystem::new(true);
        let goals = vec!["add long-term memory".to_string()];

        let events = system
            .detect_and_birth(&goals, &[], &[], &[], &mut lifecycle, 1)
            .await
            .unwrap();

        assert!(!events.is_empty());
        let ev = &events[0];
        assert!(ev.used_retired_slot);
        assert!(ev.agent_id.contains("old") || ev.agent_id == "old-123"); // may reuse id or similar
    }

    #[tokio::test]
    async fn test_generates_rich_profile() {
        let system = AgentBirthSystem::new(true);
        let signals = BirthSignals {
            skill_gaps: vec!["multi_agent_coordinator".to_string()],
            ..Default::default()
        };

        let profile = system.generate_profile("MultiAgentCoordinator", &["test".to_string()], &signals);

        assert!(!profile.system_prompt.is_empty());
        assert!(profile.allowed_tools.contains(&"spawn_agent".to_string()));
        assert!(!profile.personality_traits.is_empty());
        assert!(!profile.is_ephemeral);
    }
}
