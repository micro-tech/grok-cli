//! Lightweight Handoff / Collaboration Tracking (Task 419).
//!
//! Records transfers of responsibility between agents, roles, or steps
//! within a single ACP session.
//!
//! Stored as a simple Vec in SessionData for zero-config, session-scoped
//! observability. Exposed via `/handoffs` and `get_handoff_log()`.
//!
//! Future: can be promoted to OKF or persisted.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single handoff event: work moved from one actor/step to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffEvent {
    /// Who / what initiated the handoff (e.g. "main", "planner", "agent-abc123")
    pub from: String,
    /// Recipient (role, agent id, "implementer", etc.)
    pub to: String,
    /// Short summary of what context was passed (can be empty).
    pub context_summary: String,
    /// Why the handoff happened / what decision was made.
    pub decision: String,
    /// When it occurred (UTC).
    pub timestamp: DateTime<Utc>,
}

impl HandoffEvent {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        context_summary: impl Into<String>,
        decision: impl Into<String>,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            context_summary: context_summary.into(),
            decision: decision.into(),
            timestamp: Utc::now(),
        }
    }

    /// Returns true if this handoff carried essentially no context.
    pub fn is_empty_context(&self) -> bool {
        self.context_summary.trim().is_empty()
    }
}

/// Simple analysis result for a collection of handoffs.
#[derive(Debug, Clone, Default)]
pub struct HandoffAnalysis {
    pub total: usize,
    pub empty_context_count: usize,
    pub role_switches: usize,
    pub agent_to_agent: usize,
    pub most_common_from_to: Option<(String, String, usize)>,
}

impl HandoffAnalysis {
    pub fn from_events(events: &[HandoffEvent]) -> Self {
        let total = events.len();
        let empty_context_count = events.iter().filter(|e| e.is_empty_context()).count();

        let mut role_switches = 0;
        let mut agent_to_agent = 0;

        for e in events {
            let from_l = e.from.to_lowercase();
            let to_l = e.to.to_lowercase();

            if from_l.contains("role") || to_l.contains("role") || from_l == "planner" || to_l == "planner" {
                role_switches += 1;
            }
            if from_l.starts_with("agent-") || to_l.starts_with("agent-") {
                agent_to_agent += 1;
            }
        }

        // Very naive most-common pair
        let mut pair_counts: std::collections::HashMap<(String, String), usize> = std::collections::HashMap::new();
        for e in events {
            *pair_counts.entry((e.from.clone(), e.to.clone())).or_insert(0) += 1;
        }

        let most_common_from_to = pair_counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|((f, t), c)| (f, t, c));

        Self {
            total,
            empty_context_count,
            role_switches,
            agent_to_agent,
            most_common_from_to,
        }
    }

    pub fn to_summary(&self) -> String {
        let mut s = format!(
            "Handoffs: {} total | {} empty-context",
            self.total, self.empty_context_count
        );
        if self.role_switches > 0 {
            s.push_str(&format!(" | {} role-related", self.role_switches));
        }
        if self.agent_to_agent > 0 {
            s.push_str(&format!(" | {} agent↔agent", self.agent_to_agent));
        }
        if let Some((f, t, c)) = &self.most_common_from_to {
            s.push_str(&format!(" | most common: {} → {} ({}×)", f, t, c));
        }
        if self.empty_context_count > 0 {
            s.push_str("  ⚠️  Some handoffs carried no context summary — risk of lost information.");
        }
        s
    }
}
