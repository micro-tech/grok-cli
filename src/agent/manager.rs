//! AgentManager — tracks and coordinates spawned sub-agents.
//!
//! This is the central registry for multi-agent orchestration (Task 127).
//! It keeps lightweight records of active sub-agents, their status,
//! results, and parent-child relationships.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Status of a sub-agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Record representing a spawned sub-agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgent {
    pub id: String,
    pub parent_id: Option<String>,
    pub task: String,
    pub status: AgentStatus,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub model: Option<String>,
    pub skill: Option<String>,
}

/// Central manager for all sub-agents in the current process/session.
#[derive(Default)]
pub struct AgentManager {
    agents: RwLock<HashMap<String, SubAgent>>,
}

impl AgentManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            agents: RwLock::new(HashMap::new()),
        })
    }

    /// Spawn/register a new sub-agent
    pub async fn spawn(
        &self,
        task: &str,
        parent_id: Option<String>,
        model: Option<String>,
        skill: Option<String>,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        let agent = SubAgent {
            id: id.clone(),
            parent_id,
            task: task.to_string(),
            status: AgentStatus::Running,
            result: None,
            created_at: Utc::now(),
            completed_at: None,
            model,
            skill,
        };

        let mut agents = self.agents.write().await;
        agents.insert(id.clone(), agent);
        id
    }

    /// Mark a sub-agent as completed with a result
    pub async fn complete(&self, id: &str, result: String) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.status = AgentStatus::Completed;
            agent.result = Some(result);
            agent.completed_at = Some(Utc::now());
        }
    }

    /// Mark a sub-agent as failed
    pub async fn fail(&self, id: &str, error: String) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.status = AgentStatus::Failed;
            agent.result = Some(error);
            agent.completed_at = Some(Utc::now());
        }
    }

    /// Cancel a running sub-agent
    pub async fn cancel(&self, id: &str) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            if agent.status == AgentStatus::Running {
                agent.status = AgentStatus::Cancelled;
                agent.completed_at = Some(Utc::now());
            }
        }
    }

    /// Get a sub-agent by ID
    pub async fn get(&self, id: &str) -> Option<SubAgent> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    /// List all sub-agents (optionally filtered by parent)
    pub async fn list(&self, parent_id: Option<&str>) -> Vec<SubAgent> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| match parent_id {
                Some(pid) => a.parent_id.as_deref() == Some(pid),
                None => true,
            })
            .cloned()
            .collect()
    }

    /// Get the result of a completed sub-agent (if ready)
    pub async fn get_result(&self, id: &str) -> Option<String> {
        let agents = self.agents.read().await;
        agents.get(id).and_then(|a| a.result.clone())
    }

    /// Count of currently running agents
    pub async fn running_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.status == AgentStatus::Running)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_creates_agent() {
        let manager = AgentManager::new();
        let id = manager
            .spawn("test task", None, Some("grok-4".into()), None)
            .await;

        let agent = manager.get(&id).await.unwrap();
        assert_eq!(agent.task, "test task");
        assert_eq!(agent.status, AgentStatus::Running);
        assert_eq!(agent.model.as_deref(), Some("grok-4"));
    }

    #[tokio::test]
    async fn test_spawn_with_parent() {
        let manager = AgentManager::new();
        let parent_id = manager.spawn("parent", None, None, None).await;
        let child_id = manager
            .spawn("child", Some(parent_id.clone()), None, None)
            .await;

        let child = manager.get(&child_id).await.unwrap();
        assert_eq!(child.parent_id.as_deref(), Some(parent_id.as_str()));
    }

    #[tokio::test]
    async fn test_complete_updates_status_and_result() {
        let manager = AgentManager::new();
        let id = manager.spawn("task", None, None, None).await;

        manager.complete(&id, "done".into()).await;

        let agent = manager.get(&id).await.unwrap();
        assert_eq!(agent.status, AgentStatus::Completed);
        assert_eq!(agent.result.as_deref(), Some("done"));
        assert!(agent.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_fail_updates_status_and_error() {
        let manager = AgentManager::new();
        let id = manager.spawn("task", None, None, None).await;

        manager.fail(&id, "boom".into()).await;

        let agent = manager.get(&id).await.unwrap();
        assert_eq!(agent.status, AgentStatus::Failed);
        assert_eq!(agent.result.as_deref(), Some("boom"));
    }

    #[tokio::test]
    async fn test_cancel_only_affects_running() {
        let manager = AgentManager::new();
        let id = manager.spawn("task", None, None, None).await;
        manager.complete(&id, "ok".into()).await;

        // Should not change a completed agent
        manager.cancel(&id).await;
        let agent = manager.get(&id).await.unwrap();
        assert_eq!(agent.status, AgentStatus::Completed);
    }

    #[tokio::test]
    async fn test_list_filters_by_parent() {
        let manager = AgentManager::new();
        let p1 = manager.spawn("p1", None, None, None).await;
        let _c1 = manager.spawn("c1", Some(p1.clone()), None, None).await;
        let p2 = manager.spawn("p2", None, None, None).await;
        let _c2 = manager.spawn("c2", Some(p2.clone()), None, None).await;

        let children_of_p1 = manager.list(Some(&p1)).await;
        assert_eq!(children_of_p1.len(), 1);
        assert_eq!(children_of_p1[0].task, "c1");
    }

    #[tokio::test]
    async fn test_running_count() {
        let manager = AgentManager::new();
        let _a1 = manager.spawn("a1", None, None, None).await;
        let a2 = manager.spawn("a2", None, None, None).await;
        manager.complete(&a2, "done".into()).await;

        assert_eq!(manager.running_count().await, 1);
    }
}
