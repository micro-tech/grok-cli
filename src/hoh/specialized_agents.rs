//! Specialized Sub-Agents for 361.4
//!
//! Seeds the 361.4 vision: routing high-value RefactoringActions (and future proposals)
//! to focused, narrow-purpose sub-agents instead of one monolithic executor.
//!
//! Each agent type can have its own heuristics, prompts, validation rules, and
//! preferred patch strategies.

use crate::hoh::autonomous_refactoring::{RefactoringAction, RefactoringActionType};

/// 361.4: Agent roles / personas that can be routed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubAgentRole {
    /// Handles module extraction, file splitting, moving types
    ModuleExtractor,
    /// Specializes in changing scoring, heuristics, prioritization logic
    HeuristicTuner,
    /// Adds meta-evaluation hooks, feedback loops, self-observation
    MetaHookInstaller,
    /// Governance, safety, cross-cutting concerns (budget, approval, etc.)
    GovernanceLayer,
    /// General structural cleanup and opportunistic improvements
    GeneralRefactorer,
    /// Future: test-strategy enhancer, doc writer, etc.
    QualityEnhancer,
}

impl SubAgentRole {
    pub fn name(&self) -> &'static str {
        match self {
            SubAgentRole::ModuleExtractor => "ModuleExtractor3614",
            SubAgentRole::HeuristicTuner => "HeuristicTuner3614",
            SubAgentRole::MetaHookInstaller => "MetaHookInstaller3614",
            SubAgentRole::GovernanceLayer => "GovernanceLayer3614",
            SubAgentRole::GeneralRefactorer => "GeneralRefactorer3614",
            SubAgentRole::QualityEnhancer => "QualityEnhancer3614",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SubAgentRole::ModuleExtractor => "Extracts focused modules and improves physical architecture boundaries",
            SubAgentRole::HeuristicTuner => "Tunes scoring functions, prioritization signals, and decision heuristics",
            SubAgentRole::MetaHookInstaller => "Installs feedback loops so HOH can observe and improve its own behavior",
            SubAgentRole::GovernanceLayer => "Extracts and strengthens safety, budget, approval, and policy enforcement",
            SubAgentRole::GeneralRefactorer => "Cleans up structure, reduces duplication, improves readability",
            SubAgentRole::QualityEnhancer => "Improves test coverage, documentation, and acceptance criteria",
        }
    }
}

/// 361.4 core routing function.
/// Given a RefactoringAction, decide which specialized sub-agent should own it.
pub fn route_refactoring_action(action: &RefactoringAction) -> SubAgentRole {
    match action.action_type {
        RefactoringActionType::ExtractModule
        | RefactoringActionType::SplitLargeFile
        | RefactoringActionType::MoveTypesToNewModule => SubAgentRole::ModuleExtractor,

        RefactoringActionType::RefactorScoringHeuristic => SubAgentRole::HeuristicTuner,

        RefactoringActionType::AddMetaHook => SubAgentRole::MetaHookInstaller,

        RefactoringActionType::ExtractCrossCuttingConcern
        | RefactoringActionType::ExtractGovernanceLayer => SubAgentRole::GovernanceLayer,

        RefactoringActionType::IntroduceTrait
        | RefactoringActionType::GeneralStructuralImprovement => SubAgentRole::GeneralRefactorer,
    }
}

/// Convenience: get both role and a suggested system prompt flavor for the sub-agent.
pub fn route_with_prompt_hint(action: &RefactoringAction) -> (SubAgentRole, String) {
    let role = route_refactoring_action(action);
    let hint = match role {
        SubAgentRole::ModuleExtractor => {
            "You are a precise module extraction specialist. Focus on clean boundaries, minimal public surface, and preserving behavior."
        }
        SubAgentRole::HeuristicTuner => {
            "You are a scoring and decision-heuristic expert. Improve signal quality without introducing bias or instability."
        }
        SubAgentRole::MetaHookInstaller => {
            "You install observability and self-improvement hooks. Make HOH more aware of its own proposals and outcomes."
        }
        SubAgentRole::GovernanceLayer => {
            "You strengthen the governance and safety layer. All changes must be auditable and respect budgets/approvals."
        }
        _ => "You are a careful structural refactorer. Prioritize clarity, testability, and minimal diff size.",
    };
    (role, hint.to_string())
}

/// Stub for a future specialized agent executor.
/// In 361.4+ this would spawn a focused inner agent with the role-specific prompt + action.
pub async fn execute_with_specialized_agent(action: &RefactoringAction) -> Result<String, crate::hoh::state::HOHError> {
    let (role, hint) = route_with_prompt_hint(action);
    tracing::info!(
        "361.4: Routing refactoring '{}' to specialized sub-agent {} — {}",
        action.title,
        role.name(),
        hint
    );

    // Placeholder: real impl would call inner_harness or spawn_agent with constrained tools + role prompt
    Ok(format!(
        "361.4 [{}]: Would execute '{}' using specialized agent (confidence {:.2})",
        role.name(),
        action.title,
        action.confidence
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::autonomous_refactoring::{RefactoringAction, RefactoringActionType};

    fn sample_action(action_type: RefactoringActionType) -> RefactoringAction {
        RefactoringAction {
            id: "test".to_string(),
            source_proposal_id: "p1".to_string(),
            action_type,
            title: "Test action".to_string(),
            description: "desc".to_string(),
            target_files: vec!["src/foo.rs".to_string()],
            suggested_changes: vec![],
            new_tasks: vec![],
            estimated_effort: 2.0,
            confidence: 0.8,
        }
    }

    #[test]
    fn test_routing_extract_module() {
        let a = sample_action(RefactoringActionType::ExtractModule);
        assert_eq!(route_refactoring_action(&a), SubAgentRole::ModuleExtractor);
    }

    #[test]
    fn test_routing_heuristic() {
        let a = sample_action(RefactoringActionType::RefactorScoringHeuristic);
        assert_eq!(route_refactoring_action(&a), SubAgentRole::HeuristicTuner);
    }
}
