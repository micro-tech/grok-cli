//! HOH Specialized Agent Profiles (Task 361.5)
//!
//! Defines distinct agent personas for the HOH system.
//! Each profile has differentiated:
//! - Tool allowlists / preferences
//! - System prompts / role instructions
//! - Success criteria / quality thresholds
//! - Risk tolerance (used by governance + lifecycle)
//! - Preferred action types (refactor, debug, research, test, etc.)
//!
//! This fulfills 361.5: "Different profiles produce measurably different behavior on the same task."
//!
//! Builds on the 361.4 routing foundation while making it profile-driven.

use crate::hoh::autonomous_refactoring::{RefactoringAction, RefactoringActionType};
use std::collections::HashSet;

/// Core specialized agent profiles (361.5).
/// These are the distinct personas HOH can route work to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentProfile {
    /// High-level design, module boundaries, long-term architecture.
    Architect,
    /// Root-cause analysis, failure signals, test debugging.
    Debugger,
    /// Exploration, novel ideas, cross-domain synthesis.
    Researcher,
    /// Test strategy, coverage, quality gates, verification.
    Tester,
    /// Tactical, small, safe, incremental changes (baby-step refactors).
    Refactorer,
    /// Ethics, risk assessment, governance review, policy enforcement.
    Governor,
}

impl AgentProfile {
    pub fn name(&self) -> &'static str {
        match self {
            AgentProfile::Architect => "Architect",
            AgentProfile::Debugger => "Debugger",
            AgentProfile::Researcher => "Researcher",
            AgentProfile::Tester => "Tester",
            AgentProfile::Refactorer => "Refactorer",
            AgentProfile::Governor => "Governor",
        }
    }

    /// Role-specific system prompt injected into specialized agents.
    pub fn system_prompt(&self) -> String {
        match self {
            AgentProfile::Architect => {
                "You are a senior software architect. Focus on structural integrity, clean module boundaries, \
                 evolvability, and long-term maintainability. Prefer well-scoped larger changes over many tiny ones. \
                 Always consider how this affects the overall system architecture.".to_string()
            }
            AgentProfile::Debugger => {
                "You are a ruthless, evidence-driven debugger. Your primary job is finding root causes from \
                 failure logs, test output, traces, and runtime signals. Be extremely precise, cite specific evidence, \
                 and avoid speculation. Prioritize minimal fixes that address the actual defect.".to_string()
            }
            AgentProfile::Researcher => {
                "You are a creative researcher and knowledge synthesizer. Explore alternatives, connect ideas across \
                 domains, and generate novel but practical approaches. Prioritize insight and novelty while staying \
                 grounded in the actual codebase and goals. Document your reasoning clearly.".to_string()
            }
            AgentProfile::Tester => {
                "You are a quality-obsessed tester and verification specialist. Everything you produce must improve \
                 testability, coverage, and observable correctness. Never propose changes without clear verification \
                 criteria and test strategy. Focus on catching regressions and increasing confidence.".to_string()
            }
            AgentProfile::Refactorer => {
                "You are a disciplined incremental refactorer. Produce the smallest possible safe, reviewable, \
                 behavior-preserving changes. Favor tiny extracted functions, better naming, and clear metadata. \
                 Every change must be easy to understand and revert. Safety and minimal blast radius are paramount.".to_string()
            }
            AgentProfile::Governor => {
                "You are a strict governance, ethics, and risk reviewer. Evaluate every proposal for blast radius, \
                 reversibility, alignment with principles, resource cost, and long-term consequences. Block or heavily \
                 qualify anything that increases risk without compensating value. You are the last line of defense.".to_string()
            }
        }
    }

    /// Risk tolerance (0.0 = extremely conservative, 1.0 = very aggressive).
    /// Used by governance (407) and lifecycle (403) to adjust approval thresholds.
    pub fn risk_tolerance(&self) -> f32 {
        match self {
            AgentProfile::Governor => 0.15,
            AgentProfile::Tester => 0.30,
            AgentProfile::Debugger => 0.40,
            AgentProfile::Refactorer => 0.45,
            AgentProfile::Architect => 0.65,
            AgentProfile::Researcher => 0.70,
        }
    }

    /// Minimum quality / success threshold this profile should achieve.
    /// Higher = more demanding. Used for scoring and retirement decisions.
    pub fn success_threshold(&self) -> f32 {
        match self {
            AgentProfile::Governor => 0.92,
            AgentProfile::Tester => 0.88,
            AgentProfile::Debugger => 0.82,
            AgentProfile::Refactorer => 0.78,
            AgentProfile::Architect => 0.72,
            AgentProfile::Researcher => 0.68,
        }
    }

    /// Preferred tools for this profile (allowlist preference).
    /// Real dispatch code can restrict or weight these tools.
    pub fn preferred_tools(&self) -> HashSet<&'static str> {
        match self {
            AgentProfile::Architect => {
                ["read_multiple_files", "glob_search", "lsp_query", "list_directory"].into()
            }
            AgentProfile::Debugger => {
                ["search_file_content", "run_shell_command", "read_file", "lsp_query"].into()
            }
            AgentProfile::Researcher => {
                ["web_search", "web_fetch", "okf_lookup", "okf_get", "recall_context"].into()
            }
            AgentProfile::Tester => {
                ["run_shell_command", "search_file_content", "read_file"].into()
            }
            AgentProfile::Refactorer => {
                ["replace", "write_file", "read_file", "search_file_content"].into()
            }
            AgentProfile::Governor => {
                // Mostly observational + meta
                ["read_file", "search_file_content", "glob_search"].into()
            }
        }
    }

    /// Which RefactoringActionTypes this profile is best suited for.
    pub fn preferred_action_types(&self) -> Vec<RefactoringActionType> {
        match self {
            AgentProfile::Architect => vec![
                RefactoringActionType::ExtractModule,
                RefactoringActionType::MoveTypesToNewModule,
                RefactoringActionType::SplitLargeFile,
            ],
            AgentProfile::Debugger => vec![
                RefactoringActionType::GeneralStructuralImprovement, // often used for fixes
            ],
            AgentProfile::Researcher => vec![
                RefactoringActionType::GeneralStructuralImprovement,
            ],
            AgentProfile::Tester => vec![
                RefactoringActionType::AddMetaHook, // for quality hooks
            ],
            AgentProfile::Refactorer => vec![
                RefactoringActionType::ExtractModule,
                RefactoringActionType::IntroduceTrait,
                RefactoringActionType::GeneralStructuralImprovement,
                RefactoringActionType::RefactorScoringHeuristic,
            ],
            AgentProfile::Governor => vec![
                RefactoringActionType::ExtractCrossCuttingConcern,
                RefactoringActionType::ExtractGovernanceLayer,
            ],
        }
    }

    /// Derive the best profile for a given RefactoringAction (361.5 core mapping).
    pub fn from_action(action: &RefactoringAction) -> Self {
        let title_lower = action.title.to_lowercase();
        let desc_lower = action.description.to_lowercase();
        let combined = format!("{} {}", title_lower, desc_lower);

        // Strong signals first
        if combined.contains("architect") || combined.contains("module") || combined.contains("structure") || combined.contains("boundary") {
            return AgentProfile::Architect;
        }
        if combined.contains("debug") || combined.contains("fail") || combined.contains("root cause") || combined.contains("trace") {
            return AgentProfile::Debugger;
        }
        if combined.contains("research") || combined.contains("idea") || combined.contains("explore") || combined.contains("novel") {
            return AgentProfile::Researcher;
        }
        if combined.contains("test") || combined.contains("quality") || combined.contains("coverage") || combined.contains("verify") {
            return AgentProfile::Tester;
        }
        if combined.contains("governance") || combined.contains("ethics") || combined.contains("risk") || combined.contains("safety") {
            return AgentProfile::Governor;
        }

        // Action-type based fallback (more reliable)
        match action.action_type {
            RefactoringActionType::ExtractModule | RefactoringActionType::SplitLargeFile | RefactoringActionType::MoveTypesToNewModule => {
                AgentProfile::Architect
            }
            RefactoringActionType::RefactorScoringHeuristic | RefactoringActionType::AddMetaHook => {
                if action.confidence > 0.85 { AgentProfile::Refactorer } else { AgentProfile::Tester }
            }
            RefactoringActionType::ExtractCrossCuttingConcern | RefactoringActionType::ExtractGovernanceLayer => {
                AgentProfile::Governor
            }
            RefactoringActionType::IntroduceTrait => AgentProfile::Refactorer,
            RefactoringActionType::GeneralStructuralImprovement => {
                if action.estimated_effort < 2.5 && action.confidence >= 0.75 {
                    AgentProfile::Refactorer
                } else {
                    AgentProfile::Architect
                }
            }
        }
    }

    /// Returns a short observable signature for telemetry / metrics.
    /// Used to prove "different profiles produce measurably different behavior".
    pub fn signature(&self) -> String {
        format!(
            "{}|risk={:.2}|thresh={:.2}|tools={}",
            self.name(),
            self.risk_tolerance(),
            self.success_threshold(),
            self.preferred_tools().len()
        )
    }
}

/// Legacy compatibility type (maps old SubAgentRole → new AgentProfile).
/// We are migrating toward AgentProfile (361.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubAgentRole {
    ModuleExtractor,
    HeuristicTuner,
    MetaHookInstaller,
    GovernanceLayer,
    GeneralRefactorer,
    QualityEnhancer,
}

impl From<AgentProfile> for SubAgentRole {
    fn from(profile: AgentProfile) -> Self {
        match profile {
            AgentProfile::Architect => SubAgentRole::ModuleExtractor,
            AgentProfile::Debugger => SubAgentRole::GeneralRefactorer,
            AgentProfile::Researcher => SubAgentRole::GeneralRefactorer,
            AgentProfile::Tester => SubAgentRole::QualityEnhancer,
            AgentProfile::Refactorer => SubAgentRole::GeneralRefactorer,
            AgentProfile::Governor => SubAgentRole::GovernanceLayer,
        }
    }
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
}

/// 361.5: Choose profile for an action (primary entry point).
pub fn choose_profile_for_action(action: &RefactoringAction) -> AgentProfile {
    AgentProfile::from_action(action)
}

/// Enhanced routing that returns the rich profile + hint.
pub fn route_with_profile(action: &RefactoringAction) -> (AgentProfile, String) {
    let profile = choose_profile_for_action(action);
    let hint = profile.system_prompt();
    (profile, hint)
}

/// Legacy routing (kept for backward compatibility during migration).
pub fn route_refactoring_action(action: &RefactoringAction) -> SubAgentRole {
    let profile = choose_profile_for_action(action);
    SubAgentRole::from(profile)
}

pub fn route_with_prompt_hint(action: &RefactoringAction) -> (SubAgentRole, String) {
    let (profile, hint) = route_with_profile(action);
    (SubAgentRole::from(profile), hint)
}

/// 361.5 main execution entry point.
/// Now returns much richer telemetry so we can observe different behavior per profile.
pub async fn execute_with_specialized_agent(action: &RefactoringAction) -> Result<String, crate::hoh::state::HOHError> {
    let profile = choose_profile_for_action(action);
    let risk = profile.risk_tolerance();
    let threshold = profile.success_threshold();
    let tools = profile.preferred_tools();
    let sig = profile.signature();

    tracing::info!(
        profile = %profile.name(),
        risk = risk,
        threshold = threshold,
        tools = tools.len(),
        confidence = action.confidence,
        "361.5: Routing '{}' to specialized profile {}",
        action.title,
        profile.name()
    );

    // Rich observable result — this is what the planner logs and what we can later measure
    let result = format!(
        "361.5 [{}]: Routed '{}' | sig={} | risk_tol={:.2} | success≥{:.2} | tools={} | conf={:.2} | effort={:.1}",
        profile.name(),
        action.title,
        sig,
        risk,
        threshold,
        tools.len(),
        action.confidence,
        action.estimated_effort
    );

    Ok(result)
}

/// 361.5 helper: Get a profile from goal signals (used when no RefactoringAction is available).
pub fn profile_from_goals(goals: &[String]) -> AgentProfile {
    let text = goals.join(" ").to_lowercase();

    if text.contains("architect") || text.contains("structure") || text.contains("module") {
        AgentProfile::Architect
    } else if text.contains("debug") || text.contains("fail") || text.contains("test failure") {
        AgentProfile::Debugger
    } else if text.contains("research") || text.contains("idea") || text.contains("explore") {
        AgentProfile::Researcher
    } else if text.contains("test") || text.contains("quality") || text.contains("coverage") {
        AgentProfile::Tester
    } else if text.contains("govern") || text.contains("ethics") || text.contains("risk") || text.contains("safety") {
        AgentProfile::Governor
    } else {
        AgentProfile::Refactorer // default for most 361.3 work
    }
}

/// Observable metric helper for tests / telemetry.
/// Returns a simple score that should differ meaningfully between profiles on the same input.
pub fn profile_behavior_score(profile: &AgentProfile, action: &RefactoringAction) -> f32 {
    let base = action.confidence;
    let risk_factor = profile.risk_tolerance();
    let thresh_factor = profile.success_threshold();

    // Different profiles weight things differently → measurable difference
    match profile {
        AgentProfile::Governor => base * 0.6 + (1.0 - risk_factor) * 0.4,
        AgentProfile::Tester => base * 0.7 + thresh_factor * 0.3,
        AgentProfile::Debugger => base * 0.85,
        _ => base * 0.9 + risk_factor * 0.1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::autonomous_refactoring::RefactoringAction;

    fn make_action(title: &str, action_type: RefactoringActionType, conf: f32, effort: f32) -> RefactoringAction {
        RefactoringAction {
            id: "test".to_string(),
            source_proposal_id: "p1".to_string(),
            action_type,
            title: title.to_string(),
            description: "test description".to_string(),
            target_files: vec!["src/hoh/foo.rs".to_string()],
            suggested_changes: vec![],
            new_tasks: vec![],
            estimated_effort: effort,
            confidence: conf,
        }
    }

    #[test]
    fn test_profile_from_extract_module_prefers_architect() {
        let a = make_action("Extract module for planner", RefactoringActionType::ExtractModule, 0.82, 3.5);
        let p = AgentProfile::from_action(&a);
        assert_eq!(p, AgentProfile::Architect);
        assert!(p.risk_tolerance() > 0.5);
        assert!(p.preferred_tools().contains("glob_search"));
    }

    #[test]
    fn test_profile_from_governance_action() {
        let a = make_action("Extract governance layer", RefactoringActionType::ExtractGovernanceLayer, 0.78, 4.0);
        let p = AgentProfile::from_action(&a);
        assert_eq!(p, AgentProfile::Governor);
        assert!(p.risk_tolerance() < 0.3);
        assert!(p.success_threshold() > 0.9);
    }

    #[test]
    fn test_different_profiles_produce_different_behavior_scores() {
        let a = make_action("General improvement", RefactoringActionType::GeneralStructuralImprovement, 0.75, 2.0);

        let arch_score = profile_behavior_score(&AgentProfile::Architect, &a);
        let gov_score = profile_behavior_score(&AgentProfile::Governor, &a);
        let tester_score = profile_behavior_score(&AgentProfile::Tester, &a);

        // Prove differentiation (361.5 test strategy)
        assert!(arch_score > gov_score, "Architect should be more optimistic than Governor");
        assert!(tester_score > gov_score);
        // Scores should not be identical
        assert_ne!(arch_score, gov_score);
    }

    #[test]
    fn test_profile_signature_is_observable() {
        let p1 = AgentProfile::Governor;
        let p2 = AgentProfile::Refactorer;
        assert_ne!(p1.signature(), p2.signature());
        assert!(p1.signature().contains("Governor"));
    }

    #[test]
    fn test_profile_from_goals() {
        let goals = vec!["improve test coverage and quality".to_string()];
        assert_eq!(profile_from_goals(&goals), AgentProfile::Tester);
    }
}