//! HOH Architecture Evolution Engine (361.1 + 361.2 foundation)
//!
//! Core engine for proposing and applying high-level architectural changes.
//! This enables HOH to evolve its own structure, extract layers, introduce new abstractions,
//! and perform self-refinement at the architecture level.
//!
//! Part of the 361 Advanced Autonomy batch.

use crate::hoh::state::{HOHError, PatchSet};
use crate::hoh::tasklist_adapter::TaskListAdapter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A proposed architectural change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureProposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub change_type: ArchitectureChangeType,
    pub target_modules: Vec<String>,
    pub rationale: String,
    pub estimated_impact: f32, // 0.0 - 1.0
    pub risk_level: RiskLevel,
    pub proposed_patches: Vec<PatchSet>,
    pub new_tasks: Vec<u64>, // IDs of tasks this would create
}

/// Types of architectural evolution supported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArchitectureChangeType {
    LayerExtraction,        // e.g. extract a new hoh/ layer or tool/ sublayer
    ModuleSplit,            // Split a large module into focused submodules
    TraitIntroduction,      // Introduce traits for better decoupling
    DependencyInversion,    // Invert dependencies (e.g. via traits or events)
    NewAbstraction,         // Introduce a new core abstraction (e.g. AgentEcosystem)
    SelfRefinement,         // Changes that improve HOH's own internals
    CrossCuttingConcern,    // Logging, telemetry, governance extraction
}

/// Risk assessment for architectural proposals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// The Architecture Evolution Engine.
#[derive(Debug)]
pub struct ArchitectureEvolutionEngine {
    adapter: TaskListAdapter,
    simulation_mode: bool,
}

impl ArchitectureEvolutionEngine {
    pub fn new(adapter: TaskListAdapter, simulation_mode: bool) -> Self {
        Self {
            adapter,
            simulation_mode,
        }
    }

    /// Analyze the current codebase (via task list + known structure) and propose architectural evolutions.
    pub async fn propose_evolutions(&self) -> Result<Vec<ArchitectureProposal>, HOHError> {
        let list = self.adapter.load().await?;
        let mut proposals = Vec::new();

        // 1. Detect large monolithic modules from task list signals + known structure
        let large_modules = self.detect_large_modules(&list);

        for (module, size_signal) in &large_modules {
            if *size_signal > 1500 {
                proposals.push(ArchitectureProposal {
                    id: format!("arch-{}", uuid::Uuid::new_v4()),
                    title: format!("Split {} into focused submodules", module),
                    description: format!(
                        "Module {} shows signs of growing too large (signal: {}). Extract domain-specific concerns.",
                        module, size_signal
                    ),
                    change_type: ArchitectureChangeType::ModuleSplit,
                    target_modules: vec![module.clone()],
                    rationale: "Improves maintainability, testability, and allows independent evolution of concerns.".to_string(),
                    estimated_impact: 0.75,
                    risk_level: RiskLevel::Medium,
                    proposed_patches: vec![], // Real patch generation would be done by inner harness
                    new_tasks: vec![361, 3611], // Example: point to 361.x tasks
                });
            }
        }

        // 2. Propose new abstraction layers when HOH is evolving (361 batch)
        if list.tasks.iter().any(|t| t.title.contains("Architecture") || t.title.contains("Self-Refinement")) {
            proposals.push(ArchitectureProposal {
                id: format!("arch-{}", uuid::Uuid::new_v4()),
                title: "Introduce ArchitectureEvolutionEngine as first-class layer".to_string(),
                description: "Extract architecture reasoning and proposal generation into its own dedicated module with clear interfaces.".to_string(),
                change_type: ArchitectureChangeType::LayerExtraction,
                target_modules: vec!["hoh/".to_string()],
                rationale: "361 batch requires dedicated architectural reasoning. Prevents planner and evolution_engine from becoming god objects.".to_string(),
                estimated_impact: 0.82,
                risk_level: RiskLevel::Low,
                proposed_patches: vec![],
                new_tasks: vec![3611],
            });
        }

        // 3. Self-refinement opportunities (361.2) - now more sophisticated
        if self.detect_self_refinement_opportunities(&list) {
            proposals.push(ArchitectureProposal {
                id: format!("arch-{}", uuid::Uuid::new_v4()),
                title: "Add meta-planning and meta-evaluation hooks to HOH core".to_string(),
                description: "Extend IterationState and Planner to support recursive self-improvement loops.".to_string(),
                change_type: ArchitectureChangeType::SelfRefinement,
                target_modules: vec!["hoh/planner.rs".to_string(), "hoh/state.rs".to_string()],
                rationale: "Enables HOH to plan improvements to its own planning and evaluation subsystems.".to_string(),
                estimated_impact: 0.9,
                risk_level: RiskLevel::Medium,
                proposed_patches: vec![],
                new_tasks: vec![3612, 3619],
            });
        }

        // 4. Cross-cutting concern extraction (telemetry, governance)
        proposals.push(ArchitectureProposal {
            id: format!("arch-{}", uuid::Uuid::new_v4()),
            title: "Extract HOH Governance and Safety as explicit cross-cutting layer".to_string(),
            description: "Move scattered guardrails, budgets, and ethical constraints into a dedicated governance module.".to_string(),
            change_type: ArchitectureChangeType::CrossCuttingConcern,
            target_modules: vec!["hoh/safety.rs".to_string(), "hoh/outer_loop.rs".to_string()],
            rationale: "As autonomy increases (361+), governance must be first-class, auditable, and evolvable independently.".to_string(),
            estimated_impact: 0.68,
            risk_level: RiskLevel::Medium,
            proposed_patches: vec![],
            new_tasks: vec![407, 408],
        });

        // Filter for simulation vs real
        if self.simulation_mode {
            tracing::info!("ArchitectureEvolutionEngine: simulation mode — {} proposals generated (no patches applied)", proposals.len());
        }

        Ok(proposals)
    }

    /// 361.2: Self-Refinement Loop
    /// Analyzes recent performance (via task list signals + self-knowledge) and proposes
    /// concrete improvements to HOH's own internals (scoring, mutation rules, heuristics).
    /// This is the "meta" layer that lets HOH improve its own planning and evolution logic.
    pub async fn propose_self_refinements(&self) -> Result<Vec<ArchitectureProposal>, HOHError> {
        let list = self.adapter.load().await?;
        let mut proposals = Vec::new();

        // Analyze signals from the current task list and known HOH components
        let high_detail_tasks = list.tasks.iter().filter(|t| t.details.len() > 500).count();
        let high_prio_without_tests = list.tasks.iter()
            .filter(|t| t.priority == "high" && t.test_strategy.trim().is_empty())
            .count();
        let many_pending = list.tasks.iter().filter(|t| t.status == "pending").count();

        // Self-refinement proposal 1: Improve scoring in planner
        if high_prio_without_tests > 2 {
            proposals.push(ArchitectureProposal {
                id: format!("selfref-{}", uuid::Uuid::new_v4()),
                title: "Increase weight of test_strategy signal in planner scoring (361.2)".to_string(),
                description: "High-priority tasks without test strategies are common. Boost the +4.0 bonus for clear test_strategy and add penalty for missing ones on high-prio work.".to_string(),
                change_type: ArchitectureChangeType::SelfRefinement,
                target_modules: vec!["hoh/planner.rs".to_string()],
                rationale: "Current scoring rewards test_strategy but high-prio tasks are still slipping through without them. Stronger signal will produce better plans.".to_string(),
                estimated_impact: 0.78,
                risk_level: RiskLevel::Low,
                proposed_patches: vec![],
                new_tasks: vec![3612],
            });
        }

        // Self-refinement proposal 2: Evolve mutation heuristics
        if many_pending > 30 {
            proposals.push(ArchitectureProposal {
                id: format!("selfref-{}", uuid::Uuid::new_v4()),
                title: "Add staleness detection to TaskEvolutionEngine".to_string(),
                description: "Large number of pending tasks detected. Introduce age-based deferral and auto-prioritization for stale high-value work.".to_string(),
                change_type: ArchitectureChangeType::SelfRefinement,
                target_modules: vec!["hoh/evolution_engine.rs".to_string()],
                rationale: "Task list can accumulate stale items. Self-refinement of the evolution engine will keep the backlog healthy.".to_string(),
                estimated_impact: 0.65,
                risk_level: RiskLevel::Low,
                proposed_patches: vec![],
                new_tasks: vec![3612],
            });
        }

        // Self-refinement proposal 3: Meta-evaluation feedback loop
        if high_detail_tasks > 5 {
            proposals.push(ArchitectureProposal {
                id: format!("selfref-{}", uuid::Uuid::new_v4()),
                title: "Feed architecture proposals back into planner scoring (meta loop)".to_string(),
                description: "When high-impact architecture proposals exist, give bonus score to tasks that implement or test those proposals.".to_string(),
                change_type: ArchitectureChangeType::SelfRefinement,
                target_modules: vec!["hoh/planner.rs".to_string(), "hoh/state.rs".to_string()],
                rationale: "Closes the loop: architecture evolution should influence what the planner selects next iteration.".to_string(),
                estimated_impact: 0.85,
                risk_level: RiskLevel::Medium,
                proposed_patches: vec![],
                new_tasks: vec![3612, 3613],
            });
        }

        // Self-refinement proposal 4: General autonomy increase
        proposals.push(ArchitectureProposal {
            id: format!("selfref-{}", uuid::Uuid::new_v4()),
            title: "Raise default autonomy_level when recent meta scores are high".to_string(),
            description: "Track meta_improvement_score across iterations. When consistently > 0.7, suggest moving from ExecuteWithApproval toward FullAuto for 361+ tasks.".to_string(),
            change_type: ArchitectureChangeType::SelfRefinement,
            target_modules: vec!["hoh/mod.rs".to_string(), "hoh/outer_loop.rs".to_string()],
            rationale: "True self-improvement requires HOH to gradually trust its own proposals more.".to_string(),
            estimated_impact: 0.72,
            risk_level: RiskLevel::High, // safety critical
            proposed_patches: vec![],
            new_tasks: vec![3612],
        });

        if self.simulation_mode && !proposals.is_empty() {
            tracing::info!("SelfRefinement: generated {} meta-improvement proposals", proposals.len());
        }

        Ok(proposals)
    }

    /// Very lightweight heuristic based on task titles/details mentioning modules.
    fn detect_large_modules(&self, list: &crate::hoh::tasklist_adapter::TaskList) -> HashMap<String, usize> {
        let mut signals: HashMap<String, usize> = HashMap::new();

        let known_large_areas = ["planner", "evolution_engine", "outer_loop", "multi_agent", "agent"];

        for task in &list.tasks {
            let text = format!("{} {} {}", task.title, task.description, task.details).to_lowercase();
            for area in &known_large_areas {
                if text.contains(area) {
                    *signals.entry(area.to_string()).or_insert(0) += task.details.len() + task.title.len();
                }
            }
        }

        // Boost some known hotspots
        if let Some(v) = signals.get_mut("planner") { *v += 800; }
        if let Some(v) = signals.get_mut("evolution_engine") { *v += 600; }

        signals
    }

    fn detect_self_refinement_opportunities(&self, list: &crate::hoh::tasklist_adapter::TaskList) -> bool {
        list.tasks.iter().any(|t| {
            t.title.to_lowercase().contains("self") ||
            t.title.to_lowercase().contains("meta") ||
            t.title.to_lowercase().contains("improve hoh") ||
            t.details.to_lowercase().contains("recursive")
        })
    }

    /// Apply a proposal (very high-level — real application goes through patch system + human/approval gate).
    pub async fn apply_proposal(&self, proposal: &ArchitectureProposal) -> Result<(), HOHError> {
        if self.simulation_mode {
            tracing::info!("ArchitectureEvolutionEngine: simulation — would apply proposal '{}'", proposal.title);
            return Ok(());
        }

        // In real mode: create tasks, generate patches via inner harness, etc.
        // For now we just validate the proposal is coherent.
        if proposal.risk_level == RiskLevel::Critical {
            return Err(HOHError::Other("Critical risk proposals require explicit human approval".to_string()));
        }

        tracing::info!("Architecture proposal accepted for further processing: {}", proposal.title);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::tasklist_adapter::TaskListAdapter;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_propose_evolutions_in_simulation() {
        let adapter = TaskListAdapter::new(PathBuf::from("."), true);
        let engine = ArchitectureEvolutionEngine::new(adapter, true);

        let proposals = engine.propose_evolutions().await.unwrap();
        assert!(!proposals.is_empty(), "Should generate at least some architectural proposals");

        // At least one should be a module split or layer extraction
        let has_structural = proposals.iter().any(|p| {
            matches!(p.change_type, ArchitectureChangeType::ModuleSplit | ArchitectureChangeType::LayerExtraction | ArchitectureChangeType::SelfRefinement)
        });
        assert!(has_structural);
    }
}
