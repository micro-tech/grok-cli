//! HOH Generative Architecture Designer (Task 402)
//!
//! Builds on top of the Creativity Engine (401) to propose *entirely new system architectures*.
//! Accepts high-level goals and generates structured, actionable architecture designs
//! that include new modules, traits, data models, integration points, and migration paths.
//!
//! These designs are meant to be consumed by:
//! - Patch capture / codegen
//! - Task mutation (new 40x tasks)
//! - ArchitectureEvolutionEngine
//! - Human review or autonomous application (under governance)

use crate::hoh::creativity::{CreativityEngine, CreativityIdea};
use crate::hoh::state::HOHError;
use serde::{Deserialize, Serialize};


/// A complete generative architecture proposal.
/// This is richer than a simple CreativityIdea — it is actionable for implementation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArchitectureDesign {
    pub id: String,
    pub goal: String,
    pub title: String,
    pub description: String,

    /// New top-level or sub modules to create
    pub new_modules: Vec<NewModuleSpec>,

    /// Traits / interfaces to introduce
    pub traits: Vec<String>,

    /// Core data models / structs
    pub data_models: Vec<String>,

    /// Integration points with existing system
    pub integration_points: Vec<String>,

    /// Ordered migration / implementation steps
    pub migration_steps: Vec<String>,

    /// Estimated effort (person-days or relative)
    pub estimated_effort: f32,

    /// Scores (0.0–1.0)
    pub novelty_score: f32,
    pub feasibility_score: f32,
    pub alignment_score: f32,
    pub overall_score: f32,

    /// Where this design came from
    pub source: DesignSource,

    /// References to real existing modules this builds upon
    pub references_existing: Vec<String>,

    /// Short implementation sketch / pseudocode for the core new module
    pub core_sketch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewModuleSpec {
    pub name: String,
    pub path: String,
    pub purpose: String,
    pub key_abstractions: Vec<String>,
    pub sketch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DesignSource {
    #[default]
    Generative,
    StyleTransfer,
    MutationFromIdea,
    PastArchitecture,
    Combined,
}

/// Library of past successful architectural patterns (for few-shot / style transfer).
/// In a fuller implementation this would be loaded from OKF + previous HOH iterations.
const PAST_SUCCESSFUL_ARCHITECTURES: &[&str] = &[
    "HOH layered outer loop (planner → execution → evaluation → improvement)",
    "Multi-agent collaboration with specialized roles + message bus",
    "CreativityEngine + scoring + incorporation loop (401)",
    "ArchitectureEvolutionEngine + self-refinement proposals",
    "TaskListAdapter + dependency graph + autonomous mutation (327)",
    "Patch capture + stub generation + materialization (361.3 C)",
];

/// The Generative Architecture Designer.
#[derive(Debug)]
pub struct GenerativeArchitectureDesigner {
    creativity_engine: CreativityEngine,
    past_architectures: Vec<String>,
    simulation_mode: bool,
}

impl GenerativeArchitectureDesigner {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            creativity_engine: CreativityEngine::new(simulation_mode),
            past_architectures: PAST_SUCCESSFUL_ARCHITECTURES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            simulation_mode,
        }
    }

    /// Main entry point: given goals and recent context (tasks, ideas), generate
    /// one or more complete generative architecture designs.
    pub async fn generate_designs(
        &mut self,
        goals: &[String],
        recent_tasks: &[String],
        creative_ideas: &[CreativityIdea],
        count: usize,
    ) -> Result<Vec<ArchitectureDesign>, HOHError> {
        if goals.is_empty() {
            return Ok(vec![]);
        }

        let mut designs = Vec::new();

        // 1. Seed with high-quality creative ideas that look architectural
        let arch_ideas: Vec<_> = creative_ideas
            .iter()
            .filter(|i| {
                i.overall_score > 0.55
                    && (i.title.to_lowercase().contains("architect")
                        || i.title.to_lowercase().contains("layer")
                        || i.title.to_lowercase().contains("system")
                        || i.title.to_lowercase().contains("memory")
                        || i.title.to_lowercase().contains("agent"))
            })
            .take(3)
            .collect();

        for idea in &arch_ideas {
            if let Some(design) = self.idea_to_architecture_design(idea, goals) {
                designs.push(design);
            }
        }

        // 2. Generate fresh designs from goals using patterns + style transfer
        let generated = self
            .generate_from_goals(goals, recent_tasks, count.saturating_sub(designs.len()))
            .await?;

        designs.extend(generated);

        // 3. Score and rank
        for design in &mut designs {
            self.score_design(design, goals);
        }

        designs.sort_by(|a, b| {
            b.overall_score
                .partial_cmp(&a.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Dedup + limit
        let mut seen = std::collections::HashSet::new();
        designs.retain(|d| seen.insert(d.title.clone()));

        let result = designs.into_iter().take(count).collect::<Vec<_>>();

        if self.simulation_mode && !result.is_empty() {
            tracing::info!(
                "GenerativeArchitectureDesigner: produced {} designs for goals {:?}",
                result.len(),
                goals
            );
        }

        Ok(result)
    }

    /// Turn a high-scoring creative idea into a structured ArchitectureDesign.
    fn idea_to_architecture_design(
        &self,
        idea: &CreativityIdea,
        goals: &[String],
    ) -> Option<ArchitectureDesign> {
        let goal_text = goals.join(" ");

        let new_modules = if idea.title.to_lowercase().contains("memory") {
            vec![NewModuleSpec {
                name: "LongTermMemoryLayer".to_string(),
                path: "src/hoh/memory_layer.rs".to_string(),
                purpose: "Durable, queryable, versioned long-term knowledge store for HOH.".to_string(),
                key_abstractions: vec!["MemoryStore".to_string(), "ConceptGraph".to_string()],
                sketch: "pub trait MemoryStore { fn store(&self, concept: &str, data: Value); fn recall(&self, query: &str) -> Vec<Concept>; }".to_string(),
            }]
        } else if idea.title.to_lowercase().contains("agent") || idea.title.to_lowercase().contains("ecosystem") {
            vec![NewModuleSpec {
                name: "AgentEcosystem".to_string(),
                path: "src/hoh/agent_ecosystem.rs".to_string(),
                purpose: "Lifecycle, birth, retirement, specialization, and collaboration management for the agent population.".to_string(),
                key_abstractions: vec!["AgentProfile".to_string(), "EcosystemManager".to_string()],
                sketch: "pub struct AgentEcosystem { agents: HashMap<String, AgentHandle> }\nimpl AgentEcosystem { pub fn birth(&mut self, role: Role) -> AgentHandle { ... } }".to_string(),
            }]
        } else {
            vec![NewModuleSpec {
                name: format!("{}Module", idea.title.split_whitespace().next().unwrap_or("New")),
                path: format!("src/hoh/{}.rs", idea.title.to_lowercase().replace(' ', "_")),
                purpose: idea.description.clone(),
                key_abstractions: idea.tags.clone(),
                sketch: format!("// Core sketch for {}\n// {}", idea.title, idea.description),
            }]
        };

        let integration_points = vec![
            "planner.rs (selection + scoring)".to_string(),
            "outer_loop.rs (iteration lifecycle)".to_string(),
            "architecture_evolution.rs".to_string(),
            "creativity.rs (idea incorporation)".to_string(),
        ];

        Some(ArchitectureDesign {
            id: format!("design-{}", uuid::Uuid::new_v4()),
            goal: goal_text,
            title: idea.title.clone(),
            description: idea.description.clone(),
            new_modules,
            traits: vec!["ArchitectureComponent".to_string(), "Evolvable".to_string()],
            data_models: vec!["DesignSpec".to_string(), "MigrationPlan".to_string()],
            integration_points,
            migration_steps: vec![
                "1. Define core traits and data models".to_string(),
                "2. Implement minimal viable module".to_string(),
                "3. Wire into HOHPlanner and outer loop".to_string(),
                "4. Add evaluation hooks + telemetry".to_string(),
                "5. Promote via task mutation or patch stub".to_string(),
            ],
            estimated_effort: 6.5,
            novelty_score: idea.novelty_score,
            feasibility_score: idea.feasibility_score,
            alignment_score: idea.alignment_score,
            overall_score: idea.overall_score,
            source: DesignSource::MutationFromIdea,
            references_existing: vec![
                "src/hoh/planner.rs".to_string(),
                "src/hoh/creativity.rs".to_string(),
                "src/hoh/multi_agent.rs".to_string(),
            ],
            core_sketch: idea.description.clone(),
        })
    }

    /// Core generative logic: create designs directly from goals using pattern library + style transfer.
    async fn generate_from_goals(
        &self,
        goals: &[String],
        _recent_tasks: &[String],
        count: usize,
    ) -> Result<Vec<ArchitectureDesign>, HOHError> {
        let mut designs = Vec::new();
        let goal_text = goals.join(" ").to_lowercase();

        // Pattern-based generation for common high-value goals
        if goal_text.contains("multi-agent") || goal_text.contains("collaboration") || goal_text.contains("ecosystem") {
            designs.push(self.make_design(
                "Multi-Agent Ecosystem Manager (402)",
                "A first-class AgentEcosystem that owns birth, specialization, retirement, collaboration graphs, and resource allocation across all agents.",
                goals,
                vec![NewModuleSpec {
                    name: "AgentEcosystem".to_string(),
                    path: "src/hoh/agent_ecosystem.rs".to_string(),
                    purpose: "Central manager for the entire agent population inside HOH.".to_string(),
                    key_abstractions: vec!["AgentProfile".to_string(), "CollaborationGraph".to_string(), "Role".to_string()],
                    sketch: r#"pub struct AgentEcosystem {
    agents: HashMap<AgentId, AgentHandle>,
    graph: CollaborationGraph,
}
impl AgentEcosystem {
    pub async fn birth_specialized(&mut self, role: Role, context: &Context) -> AgentHandle { ... }
    pub fn negotiate(&self, task: &Task) -> Vec<Proposal> { ... }
}"#.to_string(),
                }],
                vec![
                    "src/hoh/multi_agent.rs".to_string(),
                    "src/hoh/specialized_agents.rs".to_string(),
                    "src/agent/mod.rs".to_string(),
                ],
            ));
        }

        if goal_text.contains("memory") || goal_text.contains("long-term") || goal_text.contains("knowledge") {
            designs.push(self.make_design(
                "Persistent Long-Term Memory & Concept Graph Layer",
                "Durable, versioned, queryable memory that survives iterations and can be used for few-shot architecture style transfer and hypothesis generation.",
                goals,
                vec![NewModuleSpec {
                    name: "LongTermMemory".to_string(),
                    path: "src/hoh/long_term_memory.rs".to_string(),
                    purpose: "Cross-iteration persistent knowledge store with concept graphs.".to_string(),
                    key_abstractions: vec!["MemoryStore".to_string(), "Concept".to_string(), "Provenance".to_string()],
                    sketch: r#"pub trait LongTermMemory {
    fn remember(&self, key: &str, value: &Value, provenance: Provenance);
    fn recall(&self, query: &str) -> Vec<Concept>;
    fn evolve_concept(&self, id: ConceptId, new_evidence: &Value);
}"#.to_string(),
                }],
                vec!["src/memory/*".to_string(), "src/hoh/knowledge_injection.rs".to_string()],
            ));
        }

        if goal_text.contains("governance") || goal_text.contains("ethical") || goal_text.contains("constraint") {
            designs.push(self.make_design(
                "Explicit Governance & Constitutional Layer",
                "A dedicated governance module that evaluates every proposal against a living set of principles before planning or execution.",
                goals,
                vec![NewModuleSpec {
                    name: "GovernanceLayer".to_string(),
                    path: "src/hoh/governance.rs".to_string(),
                    purpose: "Constitutional rules engine for long-term autonomous development.".to_string(),
                    key_abstractions: vec!["Constitution".to_string(), "Rule".to_string(), "Violation".to_string()],
                    sketch: r#"pub struct Governance {
    constitution: Constitution,
}
impl Governance {
    pub fn evaluate(&self, proposal: &ArchitectureDesign) -> Result<(), Violation> { ... }
}"#.to_string(),
                }],
                vec!["src/hoh/safety.rs".to_string(), "src/hoh/continual_improvement.rs".to_string()],
            ));
        }

        // Generic "new architecture" design if nothing matched
        if designs.is_empty() && count > 0 {
            designs.push(self.make_design(
                "New Generative Architecture Layer",
                "A fresh architectural component synthesized from current goals and successful past patterns.",
                goals,
                vec![NewModuleSpec {
                    name: "GenerativeCore".to_string(),
                    path: "src/hoh/generative_core.rs".to_string(),
                    purpose: "Core reasoning and generation substrate for novel architecture proposals.".to_string(),
                    key_abstractions: vec!["DesignGenerator".to_string(), "StyleTransfer".to_string()],
                    sketch: "// Synthesized from 402 generative designer + past successful HOH patterns".to_string(),
                }],
                vec!["src/hoh/architecture_evolution.rs".to_string(), "src/hoh/creativity.rs".to_string()],
            ));
        }

        // Apply style transfer flavor from past architectures
        for d in &mut designs {
            if !self.past_architectures.is_empty() {
                d.source = DesignSource::StyleTransfer;
                d.description = format!(
                    "{}\n\nStyle transfer from: {}",
                    d.description,
                    self.past_architectures.first().unwrap()
                );
            }
        }

        Ok(designs.into_iter().take(count).collect())
    }

    fn make_design(
        &self,
        title: &str,
        description: &str,
        goals: &[String],
        modules: Vec<NewModuleSpec>,
        refs: Vec<String>,
    ) -> ArchitectureDesign {
        ArchitectureDesign {
            id: format!("design-{}", uuid::Uuid::new_v4()),
            goal: goals.join("; "),
            title: title.to_string(),
            description: description.to_string(),
            new_modules: modules,
            traits: vec!["Evolvable".to_string(), "Governable".to_string()],
            data_models: vec!["ArchitectureDesign".to_string(), "MigrationPlan".to_string()],
            integration_points: vec![
                "HOHPlanner".to_string(),
                "outer_loop iteration lifecycle".to_string(),
                "ContinualImprovement".to_string(),
            ],
            migration_steps: vec![
                "Define interfaces and data models".to_string(),
                "Implement core module with minimal viable behavior".to_string(),
                "Integrate with planner + creativity engine".to_string(),
                "Add telemetry and evaluation hooks".to_string(),
                "Create corresponding task(s) in task_list.json".to_string(),
            ],
            estimated_effort: 8.0,
            novelty_score: 0.78,
            feasibility_score: 0.72,
            alignment_score: 0.81,
            overall_score: 0.0, // will be scored later
            source: DesignSource::Generative,
            references_existing: refs,
            core_sketch: format!("// Generative design for: {}\n// Goal alignment: {}", title, goals.join(", ")),
        }
    }

    fn score_design(&self, design: &mut ArchitectureDesign, goals: &[String]) {
        // Simple but effective multi-factor scoring (similar to CreativityEngine)
        let goal_text = goals.join(" ").to_lowercase();

        let mut alignment: f32 = 0.6;
        if goal_text.contains(&design.title.to_lowercase()) { alignment += 0.2; }
        if design.references_existing.len() >= 2 { alignment += 0.1; }
        design.alignment_score = alignment.clamp(0.3, 0.95);

        // Novelty boosted if it references new patterns not in the past library
        let mut novelty: f32 = 0.72;
        if design.new_modules.iter().any(|m| m.name.contains("Ecosystem") || m.name.contains("Governance") || m.name.contains("Memory")) {
            novelty += 0.12;
        }
        design.novelty_score = novelty.clamp(0.4, 0.95);

        design.feasibility_score = 0.68; // conservative for brand new architectures

        design.overall_score = (design.novelty_score * 0.35)
            + (design.feasibility_score * 0.30)
            + (design.alignment_score * 0.35);
    }

    /// Convenience: incorporate successful designs back into the creativity corpus
    /// so future generations become more sophisticated.
    pub fn incorporate_designs(&mut self, designs: &[ArchitectureDesign]) {
        for d in designs {
            if d.overall_score > 0.65 {
                self.creativity_engine.incorporate_ideas(&[CreativityIdea {
                    id: d.id.clone(),
                    title: d.title.clone(),
                    description: d.description.clone(),
                    novelty_score: d.novelty_score,
                    feasibility_score: d.feasibility_score,
                    alignment_score: d.alignment_score,
                    overall_score: d.overall_score,
                    source: crate::hoh::creativity::IdeaSource::PatternMining,
                    tags: d.new_modules.iter().map(|m| m.name.clone()).collect(),
                    provenance: format!("Generated by 402 from goal: {}", d.goal),
                }]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::creativity::CreativityIdea;

    #[tokio::test]
    async fn test_generate_designs_basic() {
        let mut designer = GenerativeArchitectureDesigner::new(true);
        let goals = vec!["improve multi-agent collaboration".to_string()];
        let tasks = vec!["402".to_string()];
        let ideas: Vec<CreativityIdea> = vec![];

        let designs = designer
            .generate_designs(&goals, &tasks, &ideas, 2)
            .await
            .unwrap();

        assert!(!designs.is_empty(), "Should generate at least one architecture design");
        let d = &designs[0];
        assert!(!d.new_modules.is_empty());
        assert!(d.integration_points.len() > 1);
        assert!(d.references_existing.iter().any(|r| r.contains("multi_agent") || r.contains("agent")));
        assert!(d.overall_score > 0.5);
    }

    #[tokio::test]
    async fn test_design_references_real_modules() {
        let mut designer = GenerativeArchitectureDesigner::new(true);
        let goals = vec!["add persistent long-term memory layer".to_string()];
        let designs = designer.generate_designs(&goals, &[], &[], 1).await.unwrap();

        assert!(!designs.is_empty());
        let refs = &designs[0].references_existing;
        assert!(refs.iter().any(|r| r.contains("memory") || r.contains("knowledge_injection")));
    }
}
