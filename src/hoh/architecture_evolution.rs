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

    /// Analyze the current codebase (via task list + known structure + OKF signals) and propose architectural evolutions.
    /// This is the core of 361.1: HOH Architecture Evolution Engine.
    pub async fn propose_evolutions(&self) -> Result<Vec<ArchitectureProposal>, HOHError> {
        let list = self.adapter.load().await?;
        let mut proposals = Vec::new();

        // 1. Detect large monolithic modules from task list signals + known structure
        let large_modules = self.detect_large_modules(&list);

        for (module, size_signal) in &large_modules {
            if *size_signal > 1500 {
                let patch_stub = self.create_patch_stub_for_split(module);
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
                    proposed_patches: vec![patch_stub],
                    new_tasks: vec![361, 3611],
                });
            }
        }

        // 2. Propose new abstraction layers when HOH is evolving (361 batch)
        if list.tasks.iter().any(|t| t.title.contains("Architecture") || t.title.contains("Self-Refinement") || t.title.contains("361")) {
            let patch = self.create_layer_extraction_patch("hoh/architecture");
            proposals.push(ArchitectureProposal {
                id: format!("arch-{}", uuid::Uuid::new_v4()),
                title: "Introduce ArchitectureEvolutionEngine as first-class layer".to_string(),
                description: "Extract architecture reasoning and proposal generation into its own dedicated module with clear interfaces.".to_string(),
                change_type: ArchitectureChangeType::LayerExtraction,
                target_modules: vec!["hoh/".to_string()],
                rationale: "361 batch requires dedicated architectural reasoning. Prevents planner and evolution_engine from becoming god objects.".to_string(),
                estimated_impact: 0.82,
                risk_level: RiskLevel::Low,
                proposed_patches: vec![patch],
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

        // 5. Trait introduction / dependency inversion opportunities (new for richer 361.1)
        if list.tasks.iter().any(|t| t.details.to_lowercase().contains("agent") || t.details.to_lowercase().contains("profile")) {
            let patch = self.create_trait_introduction_patch("specialized_agents");
            proposals.push(ArchitectureProposal {
                id: format!("arch-{}", uuid::Uuid::new_v4()),
                title: "Introduce AgentProfile trait for better decoupling of specialized agents".to_string(),
                description: "Define a clear trait boundary so different agent profiles (architect, researcher, debugger) can be swapped and tested independently.".to_string(),
                change_type: ArchitectureChangeType::TraitIntroduction,
                target_modules: vec!["hoh/specialized_agents.rs".to_string()],
                rationale: "Supports 361.5 specialized profiles and future agent evolution (361.6) with clean contracts.".to_string(),
                estimated_impact: 0.71,
                risk_level: RiskLevel::Low,
                proposed_patches: vec![patch],
                new_tasks: vec![3615],
            });
        }

        // 6. New core abstraction (e.g. ecosystem or strategy)
        if list.tasks.iter().any(|t| t.title.to_lowercase().contains("long-term") || t.title.to_lowercase().contains("strategy")) {
            proposals.push(ArchitectureProposal {
                id: format!("arch-{}", uuid::Uuid::new_v4()),
                title: "Introduce LongTermStrategy as first-class HOH abstraction".to_string(),
                description: "Promote long-horizon strategic goals (361.9) into a reusable abstraction that can be queried by planner, meta-planner and governance.".to_string(),
                change_type: ArchitectureChangeType::NewAbstraction,
                target_modules: vec!["hoh/long_term_strategy.rs".to_string(), "hoh/state.rs".to_string()],
                rationale: "Gives HOH coherent direction across many iterations instead of purely reactive planning.".to_string(),
                estimated_impact: 0.79,
                risk_level: RiskLevel::Low,
                proposed_patches: vec![],
                new_tasks: vec![3619],
            });
        }

        // Filter for simulation vs real
        if self.simulation_mode {
            tracing::info!("ArchitectureEvolutionEngine (361.1): simulation mode — {} proposals generated", proposals.len());
        }

        Ok(proposals)
    }

    /// 361.2: Self-Refinement Loop (core implementation)
    /// Analyzes recent performance (via task list signals + self-knowledge) and proposes
    /// concrete improvements to HOH's own internals (scoring, mutation rules, heuristics).
    /// This is the "meta" layer that lets HOH improve its own planning and evolution logic.
    ///
    /// Strong safety: proposals are only suggestions; application requires simulation + high confidence.
    /// Tracks signals that can be used to compute a meta-improvement rate over time.
    pub async fn propose_self_refinements(&self) -> Result<Vec<ArchitectureProposal>, HOHError> {
        let list = self.adapter.load().await?;
        let mut proposals = Vec::new();

        // Analyze signals from the current task list and known HOH components
        let high_detail_tasks = list.tasks.iter().filter(|t| t.details.len() > 500).count();
        let high_prio_without_tests = list.tasks.iter()
            .filter(|t| t.priority == "high" && t.test_strategy.trim().is_empty())
            .count();
        let many_pending = list.tasks.iter().filter(|t| t.status == "pending").count();
        let stale_high_prio = list.tasks.iter()
            .filter(|t| t.priority == "high" && t.status == "pending")
            .count();

        // Self-refinement proposal 1: Improve scoring in planner (most common & safe win)
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

        // Self-refinement proposal 3: Meta-evaluation feedback loop (close the architecture → planning loop)
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

        // Self-refinement proposal 4: Stale high-prio recovery
        if stale_high_prio > 5 {
            proposals.push(ArchitectureProposal {
                id: format!("selfref-{}", uuid::Uuid::new_v4()),
                title: "Add automatic re-prioritization of long-stale high-priority tasks".to_string(),
                description: "Detect high-priority tasks that have been pending for many iterations and either split them, add recovery subtasks, or temporarily boost their score.".to_string(),
                change_type: ArchitectureChangeType::SelfRefinement,
                target_modules: vec!["hoh/planner.rs".to_string(), "hoh/task_mutation.rs".to_string()],
                rationale: "Prevents important work from being starved by newer items.".to_string(),
                estimated_impact: 0.70,
                risk_level: RiskLevel::Low,
                proposed_patches: vec![],
                new_tasks: vec![3612],
            });
        }

        // Self-refinement proposal 5: General autonomy increase (gated, high risk)
        // Only propose when we have evidence of good recent behavior
        if high_detail_tasks > 8 && high_prio_without_tests < 3 {
            proposals.push(ArchitectureProposal {
                id: format!("selfref-{}", uuid::Uuid::new_v4()),
                title: "Raise default autonomy_level when recent meta scores are high".to_string(),
                description: "Track meta_improvement_score across iterations. When consistently > 0.7, suggest moving from ExecuteWithApproval toward FullAuto for 361+ tasks.".to_string(),
                change_type: ArchitectureChangeType::SelfRefinement,
                target_modules: vec!["hoh/mod.rs".to_string(), "hoh/outer_loop.rs".to_string()],
                rationale: "True self-improvement requires HOH to gradually trust its own proposals more. Only when quality signals are strong.".to_string(),
                estimated_impact: 0.72,
                risk_level: RiskLevel::High, // safety critical
                proposed_patches: vec![],
                new_tasks: vec![3612],
            });
        }

        if self.simulation_mode && !proposals.is_empty() {
            tracing::info!("SelfRefinement (361.2): generated {} meta-improvement proposals for HOH internals", proposals.len());
        }

        Ok(proposals)
    }

    /// Compute a lightweight meta-improvement signal from the current state.
    /// This can be tracked over iterations to decide when to increase autonomy.
    pub fn compute_meta_improvement_score(&self, recent_proposal_count: usize, recent_success_rate: f32) -> f32 {
        // Simple composite: more good proposals + higher success = higher meta score
        let proposal_factor = (recent_proposal_count as f32 / 6.0).min(1.0);
        let success_factor = recent_success_rate.clamp(0.0, 1.0);
        (proposal_factor * 0.4 + success_factor * 0.6).clamp(0.0, 1.0)
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

    // === Helper methods for generating safe PatchSet stubs (used by 361.1 proposals) ===

    fn create_patch_stub_for_split(&self, module: &str) -> PatchSet {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        PatchSet {
            id: format!("arch-split-{}", uuid::Uuid::new_v4()),
            files_changed: vec![format!("src/hoh/{}.rs", module), format!("src/hoh/{}_mod.rs", module)],
            diff_summary: format!(
                "ARCH-SPLIT: Extract domain concerns from {} into focused submodules.\n\
                 - Create src/hoh/{}/mod.rs\n\
                 - Move related types into submodules\n\
                 - Update imports and re-exports\n\
                 (tiny safe stub — real diff generated by autonomous refactoring)",
                module, module
            ),
            source: "architecture_evolution".to_string(),
            timestamp: now,
            intended_content: None,
        }
    }

    fn create_layer_extraction_patch(&self, layer: &str) -> PatchSet {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        PatchSet {
            id: format!("arch-layer-{}", uuid::Uuid::new_v4()),
            files_changed: vec![format!("src/hoh/{}.rs", layer.replace('/', "_"))],
            diff_summary: format!(
                "ARCH-LAYER: Introduce dedicated {} layer.\n\
                 - New module with clear public API\n\
                 - Move architecture reasoning logic here\n\
                 - Update hoh/mod.rs exports\n\
                 (simulation-safe stub for 361.1)",
                layer
            ),
            source: "architecture_evolution".to_string(),
            timestamp: now,
            intended_content: None,
        }
    }

    fn create_trait_introduction_patch(&self, target: &str) -> PatchSet {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        PatchSet {
            id: format!("arch-trait-{}", uuid::Uuid::new_v4()),
            files_changed: vec![format!("src/hoh/{}.rs", target)],
            diff_summary: format!(
                "ARCH-TRAIT: Introduce trait boundary for {}.\n\
                 - Define {} trait with core methods\n\
                 - Implement for existing profiles\n\
                 - Update call sites to use trait objects / generics\n\
                 Enables 361.5 specialization and 361.6 evolution.",
                target, target
            ),
            source: "architecture_evolution".to_string(),
            timestamp: now,
            intended_content: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::tasklist_adapter::TaskListAdapter;


    #[tokio::test]
    async fn test_propose_evolutions_in_simulation() {
        // Use a temp dir + simulation=false so that load() works reliably
        // and we can seed a list that triggers proposals.
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let adapter = TaskListAdapter::new(dir.path(), false);

        // Seed a task list that will trigger proposals
        let mut list = crate::hoh::tasklist_adapter::TaskList::default();
        list.tasks.push(crate::hoh::tasklist_adapter::Task {
            id: 1,
            title: "Architecture Self-Refinement work".to_string(),
            details: "Improve planner and evolution_engine for 361 batch".to_string(),
            priority: "high".to_string(),
            status: "pending".to_string(),
            test_strategy: "Run cargo test and verify proposals are generated.".to_string(),
            ..Default::default()
        });
        adapter.save(&list).await.unwrap();

        let engine = ArchitectureEvolutionEngine::new(adapter, true);

        let proposals = engine.propose_evolutions().await.unwrap();
        assert!(!proposals.is_empty(), "Should generate at least some architectural proposals");

        // At least one should be a module split or layer extraction or self-refinement
        let has_structural = proposals.iter().any(|p| {
            matches!(p.change_type, ArchitectureChangeType::ModuleSplit | ArchitectureChangeType::LayerExtraction | ArchitectureChangeType::SelfRefinement)
        });
        assert!(has_structural);
    }
}
