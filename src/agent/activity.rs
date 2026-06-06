//! Global channel for sub-agent activity notifications (Task 128).
//!
//! The ACP layer (`GrokAcpAgent`) registers a sender here so that
//! `spawn_agent`, `fork_agent`, `join_agents`, etc. can emit
//! `AgentActivityUpdate` events without needing a direct reference.

use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::acp::protocol::{AgentActivityStatus, AgentActivityUpdate, SessionUpdate};

static ACTIVITY_SENDER: Lazy<
    std::sync::RwLock<Option<Arc<UnboundedSender<SessionUpdate>>>>,
> = Lazy::new(|| std::sync::RwLock::new(None));

/// Register the ACP event sender so agent tools can emit activity updates.
pub fn set_activity_sender(sender: UnboundedSender<SessionUpdate>) {
    let mut guard = ACTIVITY_SENDER.write().unwrap();
    *guard = Some(Arc::new(sender));
}

/// Emit an agent activity update if a sender is registered.
pub fn emit_agent_activity(
    agent_id: impl Into<String>,
    parent_id: Option<String>,
    status: AgentActivityStatus,
    description: impl Into<String>,
) {
    let guard = ACTIVITY_SENDER.read().unwrap();
    if let Some(sender) = guard.as_ref() {
        let update = AgentActivityUpdate {
            agent_id: agent_id.into(),
            parent_id,
            status,
            description: description.into(),
        };
        let _ = sender.send(SessionUpdate::AgentActivity(update));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_emit_does_not_panic_without_sender() {
        // Should be a no-op when no sender is registered
        emit_agent_activity("agent-1", None, AgentActivityStatus::Spawned, "test");
    }
}
