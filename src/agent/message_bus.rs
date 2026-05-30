//! In-memory agent message bus (Task 127)
//!
//! Provides fast, in-process messaging between agents as a complement
//! (and eventual replacement) for the file-based send_message system.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Simple in-memory message bus for inter-agent communication.
pub struct AgentMessageBus {
    channels: RwLock<HashMap<String, Vec<AgentMessage>>>,
}

impl Default for AgentMessageBus {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentMessageBus {
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
        }
    }

    /// Send a message to a target (agent ID or channel).
    pub async fn send(&self, from: &str, to: &str, content: &str) -> String {
        let msg = AgentMessage {
            from: from.to_string(),
            to: to.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
        };

        let mut channels = self.channels.write().await;
        channels
            .entry(to.to_string())
            .or_default()
            .push(msg);

        format!("Message delivered to '{}' (in-memory)", to)
    }

    /// Receive all messages for a given target.
    pub async fn receive(&self, target: &str) -> Vec<AgentMessage> {
        let channels = self.channels.read().await;
        channels.get(target).cloned().unwrap_or_default()
    }

    /// Clear messages for a target (useful after processing).
    pub async fn clear(&self, target: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(target);
    }
}

/// Global shared message bus instance.
pub static MESSAGE_BUS: once_cell::sync::Lazy<Arc<AgentMessageBus>> =
    once_cell::sync::Lazy::new(|| Arc::new(AgentMessageBus::new()));
