//! HOH Multi-Domain Reasoning Pipeline (406)
//!
//! Enables coordinated reasoning across architecture, testing, research, docs, etc.

use serde::{Deserialize, Serialize};
use crate::hoh::state::HOHError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainOutput {
    pub domain: String, // "architecture", "testing", "research", "docs"
    pub proposals: Vec<String>,
    pub constraints_considered: Vec<String>,
    pub cross_domain_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDomainReasoner {
    pub simulation_mode: bool,
}

impl MultiDomainReasoner {
    pub fn new(simulation_mode: bool) -> Self {
        Self { simulation_mode }
    }

    /// Produce coordinated outputs across domains for the current goals + selected work.
    pub async fn reason_across_domains(
        &self,
        goals: &[String],
        selected_task_titles: &[String],
    ) -> Result<Vec<DomainOutput>, HOHError> {
        let mut outputs = vec![];

        let goal_text = goals.join(" ").to_lowercase();

        // Architecture domain
        let mut arch = DomainOutput {
            domain: "architecture".to_string(),
            proposals: vec![],
            constraints_considered: vec!["safety".to_string(), "modularity".to_string()],
            cross_domain_notes: vec![],
        };
        if goal_text.contains("architecture") || goal_text.contains("evolve") {
            arch.proposals.push("Introduce dedicated meta layer for HOH self-improvement".to_string());
        }
        outputs.push(arch);

        // Testing domain
        let mut test = DomainOutput {
            domain: "testing".to_string(),
            proposals: vec!["Add property-based tests for evolution mutations".to_string()],
            constraints_considered: vec!["coverage".to_string()],
            cross_domain_notes: vec!["Must validate architecture changes".to_string()],
        };
        if selected_task_titles.iter().any(|t| t.to_lowercase().contains("test")) {
            test.proposals.push("Prioritize regression detection for 361.x changes".to_string());
        }
        outputs.push(test);

        // Research / knowledge domain
        if goal_text.contains("knowledge") || goal_text.contains("okf") {
            outputs.push(DomainOutput {
                domain: "research".to_string(),
                proposals: vec!["Fuse 405 evolution signals into OKF patterns".to_string()],
                constraints_considered: vec!["provenance".to_string()],
                cross_domain_notes: vec!["Share with governance".to_string()],
            });
        }

        if !self.simulation_mode {
            tracing::info!("406: Produced coordinated outputs for {} domains", outputs.len());
        }

        Ok(outputs)
    }
}
