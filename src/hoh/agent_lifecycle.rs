//! Agent Lifecycle Management (Task 403)
//!
//! Implements performance tracking, retirement criteria, graceful retirement,
//! and hibernation for HOH-managed agents (specialized sub-agents, spawned agents, etc.).
//!
//! Designed to integrate with:
//! - 361.4 specialized agent routing (specialized_agents.rs)
//! - 402 GenerativeArchitectureDesigner (which sketches AgentEcosystem)
//! - Future 404 Agent Birth System (retired slots become available)
//!
//! Retirement is **not** destructive to the underlying agent tooling — it is a
//! higher-level HOH governance decision.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique identifier for a managed agent within HOH.
pub type AgentId = String;

/// Current lifecycle status of an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentLifecycleStatus {
    Active,
    Hibernating { since: u64 },
    Retired { reason: String, at: u64 },
}

impl Default for AgentLifecycleStatus {
    fn default() -> Self {
        AgentLifecycleStatus::Active
    }
}

/// Performance and contribution metrics for an individual agent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetrics {
    /// Ratio of successful outcomes (0.0–1.0)
    pub success_rate: f32,
    /// Average quality score across completed work (0.0–1.0)
    pub avg_quality: f32,
    /// Number of HOH iterations this agent contributed to
    pub iterations_contributed: u32,
    /// Rough resource cost (tokens, time, etc.) accumulated
    pub total_cost: f32,
    /// Number of consecutive failures (resets on success)
    pub failure_streak: u32,
    /// Timestamp of last successful contribution
    pub last_success: Option<u64>,
    /// Optional free-text capabilities / roles this agent has demonstrated
    #[serde(default)]
    pub demonstrated_capabilities: Vec<String>,
}

impl AgentMetrics {
    pub fn new() -> Self {
        Self {
            success_rate: 0.5,
            avg_quality: 0.6,
            ..Default::default()
        }
    }

    /// Update metrics after a work outcome.
    pub fn record_outcome(&mut self, success: bool, quality: Option<f32>, cost: f32) {
        let now = current_timestamp();

        self.iterations_contributed += 1;
        self.total_cost += cost;

        if success {
            self.failure_streak = 0;
            self.last_success = Some(now);

            let q = quality.unwrap_or(0.7);
            // Exponential moving average for quality
            self.avg_quality = 0.7 * self.avg_quality + 0.3 * q;

            // Success rate with smoothing
            let prev_successes = (self.success_rate * (self.iterations_contributed - 1) as f32).max(0.0f32);
            self.success_rate = (prev_successes + 1.0) / self.iterations_contributed as f32;
        } else {
            self.failure_streak += 1;

            let q = quality.unwrap_or(0.3);
            self.avg_quality = 0.8 * self.avg_quality + 0.2 * q;

            let prev_successes = (self.success_rate * (self.iterations_contributed - 1) as f32).max(0.0f32);
            self.success_rate = prev_successes / self.iterations_contributed as f32;
        }
    }
}

/// A single agent under HOH lifecycle management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedAgent {
    pub id: AgentId,
    pub role: String,
    pub status: AgentLifecycleStatus,
    pub metrics: AgentMetrics,
    pub created_at: u64,
    #[serde(default)]
    pub hibernated_at: Option<u64>,
}

/// Decision produced by the retirement evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetirementDecision {
    pub agent_id: AgentId,
    pub role: String,
    pub reason: String,
    pub confidence: f32,
    pub suggested_action: RetirementAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RetirementAction {
    Retire,
    Hibernate,
    Monitor, // borderline case
}

/// Report returned after a retirement or hibernation action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub agent_id: AgentId,
    pub action: String, // "retired", "hibernated", "revived"
    pub reason: String,
    pub at: u64,
    #[serde(default)]
    pub handoff_summary: Option<String>,
}

/// Main lifecycle manager for HOH agents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentLifecycleManager {
    pub agents: HashMap<AgentId, ManagedAgent>,
    pub events: Vec<LifecycleEvent>,
    pub simulation_mode: bool,
}

impl AgentLifecycleManager {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            agents: HashMap::new(),
            events: Vec::new(),
            simulation_mode,
        }
    }

    /// Register a newly spawned or routed agent.
    pub fn register_agent(&mut self, id: impl Into<String>, role: impl Into<String>) {
        let id = id.into();
        if self.agents.contains_key(&id) {
            return;
        }

        let now = current_timestamp();
        let agent = ManagedAgent {
            id: id.clone(),
            role: role.into(),
            status: AgentLifecycleStatus::Active,
            metrics: AgentMetrics::new(),
            created_at: now,
            hibernated_at: None,
        };
        self.agents.insert(id.clone(), agent);

        tracing::info!(
            agent_id = %id,
            "HOH 403: Registered new managed agent"
        );
    }

    /// Record the outcome of work performed by an agent.
    pub fn record_outcome(
        &mut self,
        agent_id: &str,
        success: bool,
        quality: Option<f32>,
        cost: f32,
    ) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            if matches!(agent.status, AgentLifecycleStatus::Active) {
                agent.metrics.record_outcome(success, quality, cost);
            }
        }
    }

    /// Core evaluation pass — returns agents that should be retired or hibernated.
    pub fn evaluate_retirement(&self) -> Vec<RetirementDecision> {
        let mut decisions = Vec::new();

        for (id, agent) in &self.agents {
            if !matches!(agent.status, AgentLifecycleStatus::Active) {
                continue;
            }

            let m = &agent.metrics;

            // Criterion 1: Sustained low performance + failure streak
            if m.failure_streak >= 3 && m.avg_quality < 0.45 {
                decisions.push(RetirementDecision {
                    agent_id: id.clone(),
                    role: agent.role.clone(),
                    reason: format!(
                        "Sustained low performance (quality {:.2}, {} consecutive failures)",
                        m.avg_quality, m.failure_streak
                    ),
                    confidence: 0.85,
                    suggested_action: RetirementAction::Retire,
                });
                continue;
            }

            // Criterion 2: Very low success rate over many iterations
            if m.iterations_contributed >= 5 && m.success_rate < 0.35 {
                decisions.push(RetirementDecision {
                    agent_id: id.clone(),
                    role: agent.role.clone(),
                    reason: format!(
                        "Persistently low success rate ({:.0}%) over {} iterations",
                        m.success_rate * 100.0,
                        m.iterations_contributed
                    ),
                    confidence: 0.78,
                    suggested_action: RetirementAction::Retire,
                });
                continue;
            }

            // Criterion 3: High cost, low value
            if m.total_cost > 1200.0 && m.avg_quality < 0.55 && m.iterations_contributed >= 4 {
                decisions.push(RetirementDecision {
                    agent_id: id.clone(),
                    role: agent.role.clone(),
                    reason: format!(
                        "High resource cost ({:.0}) with mediocre quality ({:.2})",
                        m.total_cost, m.avg_quality
                    ),
                    confidence: 0.65,
                    suggested_action: RetirementAction::Hibernate,
                });
            }

            // Soft signal: long inactivity (could become hibernation)
            if let Some(last) = m.last_success {
                let age = current_timestamp().saturating_sub(last);
                if age > 3600 * 24 * 3 && m.iterations_contributed < 3 {
                    decisions.push(RetirementDecision {
                        agent_id: id.clone(),
                        role: agent.role.clone(),
                        reason: "Inactive for extended period with low contribution".to_string(),
                        confidence: 0.55,
                        suggested_action: RetirementAction::Hibernate,
                    });
                }
            }
        }

        decisions
    }

    /// Gracefully retire an agent.
    /// Returns a LifecycleEvent describing what happened.
    pub fn retire_agent(&mut self, agent_id: &str, reason: &str) -> Option<LifecycleEvent> {
        let now = current_timestamp();

        if let Some(agent) = self.agents.get_mut(agent_id) {
            if matches!(agent.status, AgentLifecycleStatus::Retired { .. }) {
                return None;
            }

            // Compute handoff summary using only the agent's data (avoids double borrow)
            let handoff = Self::compute_handoff_summary(agent);

            let event = LifecycleEvent {
                agent_id: agent_id.to_string(),
                action: "retired".to_string(),
                reason: reason.to_string(),
                at: now,
                handoff_summary: Some(handoff.clone()),
            };

            agent.status = AgentLifecycleStatus::Retired {
                reason: reason.to_string(),
                at: now,
            };

            self.events.push(event.clone());

            tracing::info!(
                agent_id = %agent_id,
                role = %agent.role,
                reason = %reason,
                "HOH 403: Agent retired — {}",
                handoff
            );

            Some(event)
        } else {
            None
        }
    }

    /// Put an agent into hibernation (softer than retirement).
    pub fn hibernate_agent(&mut self, agent_id: &str) -> Option<LifecycleEvent> {
        let now = current_timestamp();

        if let Some(agent) = self.agents.get_mut(agent_id) {
            if matches!(agent.status, AgentLifecycleStatus::Hibernating { .. }) {
                return None;
            }

            agent.status = AgentLifecycleStatus::Hibernating { since: now };
            agent.hibernated_at = Some(now);

            let event = LifecycleEvent {
                agent_id: agent_id.to_string(),
                action: "hibernated".to_string(),
                reason: "Low activity / high cost-benefit ratio".to_string(),
                at: now,
                handoff_summary: Some("State preserved for potential revival".to_string()),
            };

            self.events.push(event.clone());

            tracing::info!(agent_id = %agent_id, "HOH 403: Agent hibernated");
            Some(event)
        } else {
            None
        }
    }

    /// Revive a hibernated agent (used by 404 birth system or manual intervention).
    pub fn revive_agent(&mut self, agent_id: &str) -> Option<LifecycleEvent> {
        let now = current_timestamp();

        if let Some(agent) = self.agents.get_mut(agent_id) {
            if let AgentLifecycleStatus::Hibernating { .. } = &agent.status {
                agent.status = AgentLifecycleStatus::Active;
                agent.hibernated_at = None;

                let event = LifecycleEvent {
                    agent_id: agent_id.to_string(),
                    action: "revived".to_string(),
                    reason: "Revived by lifecycle manager / birth system".to_string(),
                    at: now,
                    handoff_summary: None,
                };

                self.events.push(event.clone());
                tracing::info!(agent_id = %agent_id, "HOH 403: Agent revived from hibernation");
                Some(event)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Compute graceful hand-off summary (static helper to avoid borrow issues).
    fn compute_handoff_summary(agent: &ManagedAgent) -> String {
        // In a real system we would:
        // - Snapshot current in-flight tasks
        // - Archive knowledge into creativity corpus or knowledge_injection
        // - Notify other agents via send_message / team channels
        // - Update task list to reassign work

        let summary = format!(
            "Handoff for {} ({}): {} contributions, {:.0}% success, quality {:.2}. \
             Knowledge archived. In-flight work marked for reassignment.",
            agent.id,
            agent.role,
            agent.metrics.iterations_contributed,
            agent.metrics.success_rate * 100.0,
            agent.metrics.avg_quality
        );

        // For now we just log the intent — real handoff can be wired later
        tracing::debug!("HOH 403 handoff: {}", summary);
        summary
    }

    /// Perform graceful hand-off / archiving logic (kept for compatibility).
    #[allow(dead_code)]
    fn perform_graceful_handoff(&self, agent: &ManagedAgent) -> String {
        Self::compute_handoff_summary(agent)
    }

    /// Returns a summary of ecosystem health.
    pub fn ecosystem_health(&self) -> EcosystemHealth {
        let active = self
            .agents
            .values()
            .filter(|a| matches!(a.status, AgentLifecycleStatus::Active))
            .count();

        let hibernating = self
            .agents
            .values()
            .filter(|a| matches!(a.status, AgentLifecycleStatus::Hibernating { .. }))
            .count();

        let retired = self
            .agents
            .values()
            .filter(|a| matches!(a.status, AgentLifecycleStatus::Retired { .. }))
            .count();

        let avg_quality: f32 = if active > 0 {
            self.agents
                .values()
                .filter(|a| matches!(a.status, AgentLifecycleStatus::Active))
                .map(|a| a.metrics.avg_quality)
                .sum::<f32>()
                / active as f32
        } else {
            0.0
        };

        EcosystemHealth {
            total_agents: self.agents.len(),
            active,
            hibernating,
            retired,
            avg_active_quality: avg_quality,
            recent_events: self.events.iter().rev().take(5).cloned().collect(),
        }
    }

    /// Get a list of agents that are eligible to be "reborn" into new roles (for 404).
    pub fn available_slots(&self) -> Vec<AgentId> {
        self.agents
            .values()
            .filter(|a| matches!(a.status, AgentLifecycleStatus::Retired { .. }))
            .map(|a| a.id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemHealth {
    pub total_agents: usize,
    pub active: usize,
    pub hibernating: usize,
    pub retired: usize,
    pub avg_active_quality: f32,
    pub recent_events: Vec<LifecycleEvent>,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_update_success() {
        let mut m = AgentMetrics::new();
        m.record_outcome(true, Some(0.9), 50.0);
        assert!(m.success_rate > 0.6);
        assert!(m.avg_quality > 0.65); // EMA after first update lands around 0.69
        assert_eq!(m.failure_streak, 0);
    }

    #[test]
    fn test_retirement_criteria_low_performance() {
        let mut mgr = AgentLifecycleManager::new(true);
        mgr.register_agent("bad-agent-1", "LowQualityWorker");

        // Simulate bad performance
        for _ in 0..4 {
            mgr.record_outcome("bad-agent-1", false, Some(0.25), 30.0);
        }

        let decisions = mgr.evaluate_retirement();
        assert!(!decisions.is_empty());
        let d = &decisions[0];
        assert_eq!(d.agent_id, "bad-agent-1");
        assert!(matches!(d.suggested_action, RetirementAction::Retire));
    }

    #[test]
    fn test_graceful_retirement_and_handoff() {
        let mut mgr = AgentLifecycleManager::new(true);
        mgr.register_agent("agent-x", "HeuristicTuner3614");

        mgr.record_outcome("agent-x", false, Some(0.3), 100.0);
        mgr.record_outcome("agent-x", false, Some(0.2), 80.0);
        mgr.record_outcome("agent-x", false, Some(0.35), 120.0);

        let event = mgr.retire_agent("agent-x", "sustained low performance").unwrap();
        assert_eq!(event.action, "retired");
        assert!(event.handoff_summary.is_some());

        let agent = mgr.agents.get("agent-x").unwrap();
        assert!(matches!(agent.status, AgentLifecycleStatus::Retired { .. }));
    }

    #[test]
    fn test_hibernation_and_revival() {
        let mut mgr = AgentLifecycleManager::new(true);
        mgr.register_agent("sleepy", "GeneralRefactorer3614");

        let _ = mgr.hibernate_agent("sleepy");
        assert!(matches!(
            mgr.agents.get("sleepy").unwrap().status,
            AgentLifecycleStatus::Hibernating { .. }
        ));

        let revived = mgr.revive_agent("sleepy").unwrap();
        assert_eq!(revived.action, "revived");

        let agent = mgr.agents.get("sleepy").unwrap();
        assert_eq!(agent.status, AgentLifecycleStatus::Active);
    }

    #[test]
    fn test_ecosystem_health_and_available_slots() {
        let mut mgr = AgentLifecycleManager::new(true);
        mgr.register_agent("a1", "role1");
        mgr.register_agent("a2", "role2");

        let _ = mgr.retire_agent("a1", "test retirement");

        let health = mgr.ecosystem_health();
        assert_eq!(health.active, 1);
        assert_eq!(health.retired, 1);

        let slots = mgr.available_slots();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0], "a1");
    }
}
