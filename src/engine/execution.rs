//! Execution logic for plan steps, including multi-agent delegation.
//!
//! This module provides the runtime handlers that turn a [`StepAction`]
//! into real work (tool calls, model calls, memory queries, and
//! sub-agent delegation).

use crate::engine::state::{PlanStep, StepAction};
use anyhow::Result;

/// Execute a `DelegateToSubAgent` step by spawning a focused sub-agent.
///
/// This is the core implementation for Task 131 multi-agent orchestration.
/// When the reasoning engine reaches a `DelegateToSubAgent` step, this
/// function calls the existing `spawn_agent` tool infrastructure and
/// returns the sub-agent's result.
pub async fn execute_delegate_to_sub_agent(step: &PlanStep) -> Result<String> {
    if let StepAction::DelegateToSubAgent { task, .. } = &step.action {
        // Use the existing spawn_agent tool from the agent tools layer.
        // This reuses all the AgentManager registration, retry logic,
        // and result tracking that was built in Task 127.
        let result = crate::tools::agent_tools::spawn_agent(task, "", 2048).await?;
        Ok(result)
    } else {
        Err(anyhow::anyhow!(
            "execute_delegate_to_sub_agent called with non-delegation step"
        ))
    }
}
