//! HOH Multi-Agent Mode (Task 297.23)
//!
//! Coordinates multiple specialized agents working in parallel or sequence
//! inside a single HOH iteration. Each agent role gets an assigned task slice,
//! produces patches, and reports contributions back to the outer loop.

use crate::hoh::state::{IterationState, PatchSet};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A named agent role with capabilities and task assignments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRole {
    pub name: String,
    pub capabilities: Vec<String>,
    pub assigned_tasks: Vec<String>,
    /// Higher priority = runs first in sequential mode.
    pub priority: u8,
}

impl AgentRole {
    pub fn new(name: impl Into<String>, capabilities: Vec<String>) -> Self {
        Self {
            name: name.into(),
            capabilities,
            assigned_tasks: Vec::new(),
            priority: 5,
        }
    }
}

/// What one agent produced during its turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContribution {
    pub agent_name: String,
    pub tasks_attempted: Vec<String>,
    pub patches_produced: Vec<PatchSet>,
    pub success: bool,
    pub notes: String,
}

/// Orchestrates multiple agent roles for one HOH iteration.
pub struct MultiAgentRunner {
    pub roles: Vec<AgentRole>,
    pub simulation_mode: bool,
}

impl MultiAgentRunner {
    pub fn new(roles: Vec<AgentRole>, simulation_mode: bool) -> Self {
        Self { roles, simulation_mode }
    }

    /// Run all roles and collect their contributions.
    pub async fn run(&mut self, state: &mut IterationState) -> Vec<AgentContribution> {
        // Sort by priority descending
        self.roles.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut contributions = Vec::new();
        for role in &self.roles {
            let contribution = self.run_role(role, state).await;
            // Merge patches into iteration state
            state.patches.extend(contribution.patches_produced.clone());
            contributions.push(contribution);
        }

        let agent_names: Vec<&str> = self.roles.iter().map(|r| r.name.as_str()).collect();
        state.summary = Some(format!(
            "Multi-agent iteration: {} agent(s) contributed — {}",
            self.roles.len(),
            agent_names.join(", ")
        ));

        tracing::info!(
            iteration = state.iteration_id,
            agents = self.roles.len(),
            "[HOH MultiAgent] iteration complete"
        );
        contributions
    }

    async fn run_role(&self, role: &AgentRole, state: &IterationState) -> AgentContribution {
        tracing::info!(
            agent = role.name,
            tasks = role.assigned_tasks.len(),
            "[HOH MultiAgent] starting role"
        );

        if self.simulation_mode || role.assigned_tasks.is_empty() {
            // Simulated contribution
            let patch = PatchSet {
                id: format!("agent-{}-patch-{}", role.name, state.iteration_id),
                files_changed: vec![format!("src/hoh/{}.rs", role.name.to_lowercase())],
                diff_summary: format!(
                    "Simulated contribution from {} for iteration {}",
                    role.name, state.iteration_id
                ),
                source: format!("agent:{}", role.name),
                timestamp: now_secs(),
                intended_content: None,
            };
            AgentContribution {
                agent_name: role.name.clone(),
                tasks_attempted: role.assigned_tasks.clone(),
                patches_produced: vec![patch],
                success: true,
                notes: format!(
                    "Agent '{}' completed {} task(s) in simulation mode",
                    role.name,
                    role.assigned_tasks.len()
                ),
            }
        } else {
            // Real mode: spawn actual agent (future integration point)
            // For now delegate to simulation until real agent spawning is wired
            tracing::warn!(
                agent = role.name,
                "[HOH MultiAgent] real agent spawning not yet wired — using simulation fallback"
            );
            AgentContribution {
                agent_name: role.name.clone(),
                tasks_attempted: role.assigned_tasks.clone(),
                patches_produced: Vec::new(),
                success: false,
                notes: "Real agent spawning pending integration with spawn_agent API".to_string(),
            }
        }
    }
}

/// Assign tasks from a flat list to roles in round-robin fashion.
pub fn assign_tasks_to_roles(roles: &mut Vec<AgentRole>, tasks: &[String]) {
    if roles.is_empty() || tasks.is_empty() {
        return;
    }
    for role in roles.iter_mut() {
        role.assigned_tasks.clear();
    }
    for (i, task) in tasks.iter().enumerate() {
        let idx = i % roles.len();
        roles[idx].assigned_tasks.push(task.clone());
    }
}

/// Convenience function: create a default set of HOH agent roles.
pub fn default_roles() -> Vec<AgentRole> {
    vec![
        AgentRole::new("Architect", vec!["design".to_string(), "planning".to_string()]),
        AgentRole::new("Coder", vec!["implementation".to_string(), "refactor".to_string()]),
        AgentRole::new("Tester", vec!["testing".to_string(), "verification".to_string()]),
    ]
}

/// Top-level entry used by outer_loop: run multi-agent iteration.
pub async fn run_multi_agent_iteration(
    state: &mut IterationState,
    roles: Vec<AgentRole>,
) -> Vec<AgentContribution> {
    let simulation = true; // default to simulation until real spawning is wired
    let mut runner = MultiAgentRunner::new(roles, simulation);
    runner.run(state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_tasks_round_robin() {
        let mut roles = default_roles();
        let tasks: Vec<String> = (1..=7).map(|i| format!("task-{}", i)).collect();
        assign_tasks_to_roles(&mut roles, &tasks);
        let total: usize = roles.iter().map(|r| r.assigned_tasks.len()).sum();
        assert_eq!(total, 7);
        // No role should have more than ceil(7/3) = 3 tasks
        for role in &roles {
            assert!(role.assigned_tasks.len() <= 3);
        }
    }

    #[tokio::test]
    async fn test_run_multi_agent_produces_contributions() {
        let mut state = IterationState::new(1);
        let roles = default_roles();
        let contributions = run_multi_agent_iteration(&mut state, roles).await;
        assert!(!contributions.is_empty());
        assert!(contributions.iter().all(|c| !c.agent_name.is_empty()));
        assert!(!state.patches.is_empty());
    }
}
