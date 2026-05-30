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
