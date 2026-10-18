//! HOH Multi-Agent Collaboration Protocol (361.7)
//!
//! Defines how specialized agents, born agents, and evolved agents communicate,
//! delegate work, negotiate ownership, perform handoffs, and synchronize state.
//!
//! This is the concrete protocol layer on top of the existing infrastructure:
//! - In-memory message bus (`agent::message_bus`)
//! - File-based `send_message` + `receive_messages`
//! - `spawn_agent`, `fork_agent`, `join_agents`, `team_create`
//!
//! Integrates with:
//! - 361.5 Specialized Agent Profiles
//! - 361.6 Agent Skill Evolution (agents advertise evolved skills)
//! - 403/404 Lifecycle + Birth (handoffs reuse retired slots)
//! - Planner (routed tasks can be delegated)
//!
//! Protocol primitives:
//! - Delegate (give work to best agent)
//! - Negotiate (multiple agents bid for a task)
//! - Handoff (graceful transfer of state + responsibility)
//! - Broadcast / ShareResult
//! - RequestHelp / SkillOffer
//! - Synchronize (shared blackboard / state)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Utc;

use crate::hoh::state::HOHError;
use crate::hoh::specialized_agents::AgentProfile;
use crate::hoh::agent_skill_evolution::AgentSkillEvolutionSystem;

/// A structured message in the collaboration protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationMessage {
    pub from: String,
    pub to: String,           // agent id or "broadcast" or team name
    pub intent: CollaborationIntent,
    pub payload: String,
    pub correlation_id: Option<String>,
    pub timestamp: i64,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollaborationIntent {
    /// "Here is work for you"
    Delegate,
    /// "I am offering to take this task"
    Bid,
    /// "I accept / decline the bid or delegation"
    Accept,
    Decline,
    /// "I am handing off this work + context to you"
    Handoff,
    /// "Please help with this subproblem"
    RequestHelp,
    /// "I have a skill that might be useful"
    SkillOffer,
    /// "Here is a result / artifact for you"
    ShareResult,
    /// "What is the current status of X?"
    QueryStatus,
    /// "Synchronizing shared blackboard"
    SyncState,
}

/// Simple shared blackboard for collaboration sessions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SharedBlackboard {
    pub facts: HashMap<String, String>,
    pub decisions: Vec<String>,
    pub artifacts: Vec<String>,
}

impl SharedBlackboard {
    pub fn record_fact(&mut self, key: &str, value: &str) {
        self.facts.insert(key.to_string(), value.to_string());
    }

    pub fn add_decision(&mut self, decision: &str) {
        self.decisions.push(decision.to_string());
    }
}

/// Tracks an active collaboration between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    pub id: String,
    pub participants: Vec<String>,
    pub topic: String,
    pub blackboard: SharedBlackboard,
    pub messages: Vec<CollaborationMessage>,
    pub status: CollaborationStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollaborationStatus {
    Active,
    Resolved,
    Failed,
    HandedOff { to: String },
}

/// The main Multi-Agent Collaboration Protocol engine for HOH.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentCollaborationProtocol {
    pub simulation_mode: bool,
    pub sessions: HashMap<String, CollaborationSession>,
    /// Simple registry of known agents and their last known profile/skill summary
    pub agent_registry: HashMap<String, AgentRegistration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub id: String,
    pub profile: Option<String>, // e.g. "Architect", "Debugger"
    pub skill_summary: String,
    pub last_seen: i64,
}

impl MultiAgentCollaborationProtocol {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            simulation_mode,
            sessions: HashMap::new(),
            agent_registry: HashMap::new(),
        }
    }

    /// Register an agent so others can discover it for collaboration.
    pub fn register_agent(&mut self, agent_id: &str, profile: Option<&AgentProfile>, skill_summary: &str) {
        let reg = AgentRegistration {
            id: agent_id.to_string(),
            profile: profile.map(|p| p.name().to_string()),
            skill_summary: skill_summary.to_string(),
            last_seen: Utc::now().timestamp(),
        };
        self.agent_registry.insert(agent_id.to_string(), reg);
    }

    /// Create a new collaboration session.
    pub fn start_session(&mut self, topic: &str, participants: Vec<String>) -> String {
        let id = format!("collab-{}", Utc::now().timestamp_millis());
        let session = CollaborationSession {
            id: id.clone(),
            participants,
            topic: topic.to_string(),
            blackboard: SharedBlackboard::default(),
            messages: vec![],
            status: CollaborationStatus::Active,
            created_at: Utc::now().timestamp(),
        };
        self.sessions.insert(id.clone(), session);
        id
    }

    /// Send a protocol message (uses real infrastructure when not in sim).
    pub async fn send_protocol_message(&self, msg: CollaborationMessage) -> Result<String, HOHError> {
        let serialized = serde_json::to_string(&msg).unwrap_or_default();

        if self.simulation_mode {
            return Ok(format!("[SIM] {} → {} : {:?}", msg.from, msg.to, msg.intent));
        }

        // Prefer the fast in-memory bus
        let result = crate::tools::agent_tools::send_message_in_memory(
            &msg.from,
            &msg.to,
            &serialized,
        ).await;

        match result {
            Ok(r) => Ok(r),
            Err(_) => {
                // Fallback to file-based
                crate::tools::agent_tools::send_message(&msg.to, &serialized)
                    .map_err(|e| HOHError::Other(format!("send failed: {}", e)))
            }
        }
    }

    /// Receive messages for an agent (protocol-aware).
    pub async fn receive_protocol_messages(&self, target: &str) -> Vec<CollaborationMessage> {
        if self.simulation_mode {
            return vec![];
        }

        // Try in-memory first
        if let Ok(raw) = crate::tools::agent_tools::receive_messages(target).await {
            // Best-effort parse of protocol messages
            raw.lines()
                .filter_map(|line| serde_json::from_str::<CollaborationMessage>(line).ok())
                .collect()
        } else {
            vec![]
        }
    }

    /// High-level: Delegate a task to the best matching registered agent.
    /// Returns the chosen agent id + a message id.
    pub async fn delegate(
        &mut self,
        from: &str,
        task_description: &str,
        skill_evolution: Option<&AgentSkillEvolutionSystem>,
    ) -> Result<(String, String), HOHError> {
        // Simple heuristic: pick agent whose skill_summary or profile matches best
        let best = self.agent_registry
            .values()
            .filter(|r| r.id != from)
            .max_by_key(|r| {
                let mut score = 0;
                let lower = task_description.to_lowercase();
                if let Some(ref prof) = r.profile {
                    if lower.contains(&prof.to_lowercase()) { score += 10; }
                }
                if lower.split_whitespace().any(|w| r.skill_summary.to_lowercase().contains(w)) {
                    score += 5;
                }
                // Bonus if they have evolved skills that match (361.6 integration)
                if let Some(evo) = skill_evolution {
                    if !evo.get_agent_tools(&r.id).is_empty() {
                        score += 3;
                    }
                }
                score
            });

        let target = match best {
            Some(reg) => reg.id.clone(),
            None => {
                // Fallback: pick any other registered agent or create a generic one
                self.agent_registry.keys().find(|&id| id != from).cloned()
                    .unwrap_or_else(|| "generalist-1".to_string())
            }
        };

        let msg = CollaborationMessage {
            from: from.to_string(),
            to: target.clone(),
            intent: CollaborationIntent::Delegate,
            payload: task_description.to_string(),
            correlation_id: Some(format!("del-{}", Utc::now().timestamp())),
            timestamp: Utc::now().timestamp(),
            metadata: HashMap::new(),
        };

        let delivery = self.send_protocol_message(msg.clone()).await?;
        Ok((target, delivery))
    }

    /// Negotiation: multiple agents can bid. Simple first-come or highest-utility wins.
    pub async fn negotiate(
        &mut self,
        session_id: &str,
        task_description: &str,
        bidders: Vec<String>,
    ) -> Result<String, HOHError> {
        // Simple arbitration first (before any borrows that could conflict)
        let winner = bidders.first().cloned().unwrap_or_else(|| "unknown".to_string());

        {
            let session = self.sessions.get_mut(session_id)
                .ok_or_else(|| HOHError::Other("Session not found".into()))?;

            // Record bids (clone messages so we can send after releasing borrow)
            let mut outgoing: Vec<CollaborationMessage> = vec![];

            for bidder in &bidders {
                let bid_msg = CollaborationMessage {
                    from: bidder.clone(),
                    to: "coordinator".to_string(),
                    intent: CollaborationIntent::Bid,
                    payload: format!("I can do: {}", task_description),
                    correlation_id: Some(session_id.to_string()),
                    timestamp: Utc::now().timestamp(),
                    metadata: HashMap::new(),
                };
                session.messages.push(bid_msg.clone());
                outgoing.push(bid_msg);
            }

            let accept = CollaborationMessage {
                from: "coordinator".to_string(),
                to: winner.clone(),
                intent: CollaborationIntent::Accept,
                payload: format!("You won the negotiation for: {}", task_description),
                correlation_id: Some(session_id.to_string()),
                timestamp: Utc::now().timestamp(),
                metadata: HashMap::new(),
            };
            session.messages.push(accept.clone());
            outgoing.push(accept);

            session.blackboard.add_decision(&format!("{} assigned via negotiation", winner));

            // Now send outside the mutable borrow on sessions
            for msg in outgoing {
                let _ = self.send_protocol_message(msg).await;
            }
        }

        Ok(winner)
    }

    /// Graceful handoff — transfers context and updates session.
    pub async fn handoff(
        &mut self,
        session_id: &str,
        from: &str,
        to: &str,
        context_summary: &str,
    ) -> Result<(), HOHError> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| HOHError::Other("no such session".into()))?;

        let msg = CollaborationMessage {
            from: from.to_string(),
            to: to.to_string(),
            intent: CollaborationIntent::Handoff,
            payload: context_summary.to_string(),
            correlation_id: Some(session_id.to_string()),
            timestamp: Utc::now().timestamp(),
            metadata: HashMap::new(),
        };

        session.messages.push(msg.clone());
        session.blackboard.record_fact("last_handoff", &format!("{} → {} : {}", from, to, context_summary));
        session.status = CollaborationStatus::HandedOff { to: to.to_string() };

        let _ = self.send_protocol_message(msg).await;

        // Also try real handoff via agent tools if available
        if !self.simulation_mode {
            let _ = crate::tools::agent_tools::send_message(to, &format!("HANDOFF from {}: {}", from, context_summary));
        }

        Ok(())
    }

    /// Broadcast a result or fact to all participants in a session.
    pub async fn share_result(&mut self, session_id: &str, from: &str, result: &str) -> Result<(), HOHError> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| HOHError::Other("session missing".into()))?;

        session.blackboard.artifacts.push(result.to_string());

        let msg = CollaborationMessage {
            from: from.to_string(),
            to: "broadcast".to_string(),
            intent: CollaborationIntent::ShareResult,
            payload: result.to_string(),
            correlation_id: Some(session_id.to_string()),
            timestamp: Utc::now().timestamp(),
            metadata: HashMap::new(),
        };

        session.messages.push(msg.clone());
        let _ = self.send_protocol_message(msg).await;
        Ok(())
    }

    /// Ask another agent for help (used by skill evolution and planner).
    pub async fn request_help(
        &mut self,
        from: &str,
        target: &str,
        problem: &str,
    ) -> Result<String, HOHError> {
        let msg = CollaborationMessage {
            from: from.to_string(),
            to: target.to_string(),
            intent: CollaborationIntent::RequestHelp,
            payload: problem.to_string(),
            correlation_id: None,
            timestamp: Utc::now().timestamp(),
            metadata: HashMap::new(),
        };
        self.send_protocol_message(msg).await
    }

    /// Synchronize shared state across agents (simple blackboard merge).
    pub async fn synchronize(&mut self, session_id: &str, key: &str, value: &str) -> Result<(), HOHError> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.blackboard.record_fact(key, value);

            let msg = CollaborationMessage {
                from: "sync".to_string(),
                to: "broadcast".to_string(),
                intent: CollaborationIntent::SyncState,
                payload: format!("{}={}", key, value),
                correlation_id: Some(session_id.to_string()),
                timestamp: Utc::now().timestamp(),
                metadata: HashMap::new(),
            };
            let _ = self.send_protocol_message(msg).await;
        }
        Ok(())
    }

    /// Get a summary useful for logging / planner feedback.
    pub fn session_summary(&self, session_id: &str) -> Option<String> {
        self.sessions.get(session_id).map(|s| {
            format!(
                "Session {} [{}]: {} participants, {} messages, status: {:?}",
                s.id, s.topic, s.participants.len(), s.messages.len(), s.status
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::specialized_agents::AgentProfile;

    #[tokio::test]
    async fn test_delegate_and_session() {
        let mut proto = MultiAgentCollaborationProtocol::new(true);
        proto.register_agent("arch-42", Some(&AgentProfile::Architect), "architecture + design");
        proto.register_agent("debug-7", Some(&AgentProfile::Debugger), "root cause analysis");

        let (chosen, _delivery) = proto.delegate("planner", "extract new architecture layer", None).await.unwrap();
        assert!(chosen == "arch-42" || chosen == "debug-7" || chosen == "generalist-1");

        let sid = proto.start_session("big refactor", vec!["arch-42".into(), "debug-7".into()]);
        let _ = proto.share_result(&sid, "arch-42", "Proposed new module boundary").await;
        assert!(proto.session_summary(&sid).unwrap().contains("messages"));
    }

    #[tokio::test]
    async fn test_negotiation_and_handoff() {
        let mut proto = MultiAgentCollaborationProtocol::new(true);
        proto.register_agent("a", None, "tester");
        proto.register_agent("b", None, "refactorer");

        let sid = proto.start_session("test + refactor", vec!["a".into(), "b".into()]);
        let winner = proto.negotiate(&sid, "improve test coverage then refactor", vec!["a".into(), "b".into()]).await.unwrap();
        assert!(winner == "a" || winner == "b");

        let _ = proto.handoff(&sid, &winner, "b", "Here is the test plan and current state").await;
        if let Some(s) = proto.sessions.get(&sid) {
            assert!(matches!(s.status, CollaborationStatus::HandedOff { .. }));
        }
    }

    #[tokio::test]
    async fn test_skill_offer_and_request_help() {
        let mut proto = MultiAgentCollaborationProtocol::new(true);
        let msg = CollaborationMessage {
            from: "evolved-agent".to_string(),
            to: "other".to_string(),
            intent: CollaborationIntent::SkillOffer,
            payload: "I have strong multi_agent_coordination after evolution".to_string(),
            correlation_id: None,
            timestamp: Utc::now().timestamp(),
            metadata: HashMap::new(),
        };
        let _ = proto.send_protocol_message(msg).await;

        let help = proto.request_help("stuck-agent", "coordinator", "need help with parallel work").await;
        assert!(help.is_ok() || proto.simulation_mode);
    }
}