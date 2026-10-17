//! HOH Architecture Simulation (Task 361.22)

use crate::hoh::architecture_evolution::{ArchitectureChangeType, ArchitectureProposal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchSimResult {
    pub proposal_id: String,
    pub coupling_delta: f32,
    pub complexity_delta: f32,
    pub maintainability_delta: f32,
    pub predicted_risk: f32,
    pub recommendation: SimRecommendation,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SimRecommendation { Apply, ApplyWithCaution, Defer, Reject }

/// Simulate the effects of an architecture proposal before applying it.
pub fn simulate_proposal(proposal: &ArchitectureProposal) -> ArchSimResult {
    let mut notes = Vec::new();
    let desc = format!("{:?} {}", proposal.change_type, proposal.description).to_lowercase();

    let coupling_delta = match proposal.change_type {
        ArchitectureChangeType::LayerExtraction
        | ArchitectureChangeType::ModuleSplit
        | ArchitectureChangeType::DependencyInversion => {
            notes.push("Extraction/split typically reduces coupling".to_string());
            -0.1
        }
        ArchitectureChangeType::NewAbstraction => {
            notes.push("New abstraction may initially increase coupling before reducing it".to_string());
            0.02
        }
        _ => 0.0,
    };

    let complexity_delta = match proposal.change_type {
        ArchitectureChangeType::SelfRefinement => {
            notes.push("Self-refinement reduces internal complexity".to_string());
            -0.05
        }
        ArchitectureChangeType::LayerExtraction | ArchitectureChangeType::NewAbstraction => {
            notes.push("Adding layers increases complexity initially".to_string());
            0.08
        }
        _ => 0.0,
    };

    let maintainability_delta = -complexity_delta - coupling_delta * 0.5;

    let affected_count = proposal.target_modules.len();
    let breaking = desc.contains("breaking");
    let predicted_risk = ((affected_count as f32 * 0.05) + if breaking { 0.3 } else { 0.0 })
        .clamp(0.0, 1.0);

    let recommendation = if predicted_risk > 0.6 {
        SimRecommendation::Reject
    } else if predicted_risk > 0.35 || coupling_delta > 0.0 {
        SimRecommendation::ApplyWithCaution
    } else if maintainability_delta > 0.0 {
        SimRecommendation::Apply
    } else {
        SimRecommendation::Defer
    };

    ArchSimResult {
        proposal_id: proposal.id.clone(),
        coupling_delta, complexity_delta, maintainability_delta, predicted_risk,
        recommendation, notes,
    }
}

pub fn simulate_all(proposals: &[ArchitectureProposal]) -> Vec<ArchSimResult> {
    proposals.iter().map(simulate_proposal).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::architecture_evolution::RiskLevel;


    fn proposal(desc: &str, kind: ArchitectureChangeType, modules: usize) -> ArchitectureProposal {
        ArchitectureProposal {
            id: "test".to_string(),
            title: desc.to_string(),
            description: desc.to_string(),
            change_type: kind,
            target_modules: (0..modules).map(|i| format!("src/mod{}.rs", i)).collect(),
            rationale: String::new(),
            estimated_impact: 0.5,
            risk_level: RiskLevel::Low,
            proposed_patches: vec![],
            new_tasks: vec![],
        }
    }

    #[test]
    fn test_module_split_reduces_coupling() {
        let p = proposal("Split large module", ArchitectureChangeType::ModuleSplit, 1);
        let result = simulate_proposal(&p);
        assert!(result.coupling_delta <= 0.0);
    }

    #[test]
    fn test_many_modules_increases_risk() {
        let p = proposal("Add breaking new layer", ArchitectureChangeType::LayerExtraction, 15);
        let result = simulate_proposal(&p);
        assert!(result.predicted_risk > 0.3);
    }

    #[test]
    fn test_self_refinement_apply_or_caution() {
        let p = proposal("Improve planner heuristics", ArchitectureChangeType::SelfRefinement, 1);
        let result = simulate_proposal(&p);
        assert!(result.recommendation == SimRecommendation::Apply
            || result.recommendation == SimRecommendation::ApplyWithCaution);
    }
}
