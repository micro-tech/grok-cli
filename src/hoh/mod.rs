//! Harness-of-Harness (HOH) - Multi-Day Autonomous Development Outer Loop
//!
//! This module implements the outer orchestration layer for long-running,
//! self-improving development cycles. HOH plans, delegates to the inner
//! Grok-CLI harness, captures patches, evaluates, and iterates.

pub mod outer_loop;
pub mod state;
pub mod planner;
pub mod simulation;
pub mod patch_capture;
pub mod patch_applier;
pub mod persistence;
pub mod testing;
pub mod helix;
pub mod continual_improvement;
pub mod tool_control;
pub mod iteration_folders;
pub mod timeline;
pub mod safety;
pub mod recovery;
pub mod multi_agent;
pub mod cli;
pub mod tasklist_adapter;
pub mod task_dependency_graph;
pub mod task_mutation;
pub mod task_selection;
pub mod task_prioritization;
pub mod okf_tasklist_sync;
pub mod evolution_engine;
pub mod task_completion_tracker;
pub mod architecture_evolution;
pub mod autonomous_refactoring;
pub mod specialized_agents;
pub mod creativity;
pub mod generative_designer;
pub mod agent_lifecycle;
pub mod agent_evolution;
pub mod agent_skill_evolution;
pub mod multi_agent_collaboration;
pub mod multi_agent_simulation;
pub mod multi_agent_orchestrator;
pub mod multi_domain;
pub mod governance;
pub mod ethics;
pub mod meta_planning;
pub mod meta_evaluation;
pub mod long_term_strategy;
pub mod cross_project_knowledge;
pub mod multi_project_orchestrator;

pub use outer_loop::HOHManager;
pub use state::{IterationState, PatchSet};
pub use evolution_engine::TaskEvolutionEngine;
pub use tasklist_adapter::TaskListAdapter;
pub use task_selection::{TaskSelectionEngine, SelectedTask, SelectionConfig};
pub use task_prioritization::{TaskPrioritizationModel, PrioritizationWeights, PrioritizationContext, PrioritySignals, RankedTask};
pub use okf_tasklist_sync::{OkfTaskListSyncer, OkfSyncProposal, proposals_to_mutations};
pub use architecture_evolution::{ArchitectureEvolutionEngine, ArchitectureProposal, ArchitectureChangeType};
pub use autonomous_refactoring::{AutonomousRefactoringEngine, RefactoringAction, RefactoringActionType};
pub use specialized_agents::{AgentProfile as SpecializedAgentProfile, choose_profile_for_action, execute_with_specialized_agent};
pub use creativity::{CreativityEngine, CreativityIdea, IdeaSource};
pub use generative_designer::{GenerativeArchitectureDesigner, ArchitectureDesign, DesignSource};
pub use agent_lifecycle::{
    AgentLifecycleManager, ManagedAgent, AgentMetrics, LifecycleEvent,
    RetirementDecision, RetirementAction, EcosystemHealth,
};
pub mod agent_birth;
pub use agent_birth::{AgentBirthSystem, AgentProfile, BirthEvent};
pub use agent_evolution::{AgentEvolutionSystem, EvolutionEvent};
pub use agent_skill_evolution::{AgentSkillEvolutionSystem, AgentSkill, AgentSkillPortfolio};
pub use multi_agent_collaboration::{
    MultiAgentCollaborationProtocol, CollaborationMessage, CollaborationIntent,
    SharedBlackboard, CollaborationSession, CollaborationStatus,
};
pub use multi_agent_simulation::{
    MultiAgentSimulator, SimulatedAgent, SimulationOutcome, SimulationConfig,
};
pub use multi_agent_orchestrator::{
    MultiAgentOrchestrator, MultiAgentRequest, OrchestrationResult,
};
pub use multi_agent::{AgentRole, run_multi_agent_iteration};
pub use multi_domain::{MultiDomainReasoner, DomainOutput};
pub use governance::GovernanceEngine;
pub use ethics::EthicsEngine;
pub use meta_planning::{MetaPlanningEngine, MetaPlan};
pub use meta_evaluation::{MetaEvaluationEngine, MetaEvaluation};
pub use long_term_strategy::{LongTermStrategyEngine, StrategicGoal, Milestone, StrategyStatus};
pub use cross_project_knowledge::{
    CrossProjectKnowledgeTransfer, TransferablePattern, CrossProjectTransfer, PatternType,
};
pub use multi_project_orchestrator::{
    MultiProjectOrchestrator, MultiProjectRequest, MultiProjectOrchestrationResult,
    ProjectRef, CrossProjectDependency, ResourceAllocation,
};

/// Top-level HOH configuration and entry points.
#[derive(Debug, Clone, Default)]
pub struct HOHConfig {
    pub simulation_mode: bool,
    pub max_iterations_per_day: u32,
    pub autonomy_level: AutonomyLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutonomyLevel {
    Observe,
    Propose,
    #[default]
    ExecuteWithApproval,
    FullAuto,
}
