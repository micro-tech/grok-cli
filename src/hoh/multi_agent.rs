//! Multi-Agent Mode (Task 297.23)

use crate::hoh::state::IterationState;

#[derive(Debug, Clone)]
pub struct AgentRole {
    pub name: String,
    pub capabilities: Vec<String>,
}

pub async fn run_multi_agent_iteration(state: &mut IterationState, roles: Vec<AgentRole>) {
    for role in roles {
        println!("[HOH MultiAgent] Running role: {}", role.name);
        // TODO: spawn actual agents via spawn_agent / send_message
    }
    state.summary = Some("Multi-agent iteration skeleton executed".to_string());
}