//! HOH Cross-Project Knowledge Transfer (361.11 / 371)
//!
//! Enables transfer of learnings, patterns, and abstractions between different
//! projects using OKF (Open Knowledge Format) and HOH memory mechanisms as the
//! portable transport.
//!
//! Core idea:
//! - Discover patterns in the current project that are likely generalizable.
//! - Package them as portable "transfer candidates" (often as OKF concepts).
//! - Detect when patterns from "other projects" (via OKF bundles, previous
//!   HOH runs, or explicit imports) are applicable here.
//! - Record successful transfers so they influence planning and can be audited.

use crate::hoh::state::HOHError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatternType {
    ArchitecturalPattern,
    AgentBehavior,
    RefactoringHeuristic,
    PromptTechnique,
    EvaluationSignal,
    ToolUsagePattern,
    KnowledgeStructure,
    ProcessImprovement,
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternType::ArchitecturalPattern => write!(f, "architecture"),
            PatternType::AgentBehavior => write!(f, "agent-behavior"),
            PatternType::RefactoringHeuristic => write!(f, "refactoring"),
            PatternType::PromptTechnique => write!(f, "prompt"),
            PatternType::EvaluationSignal => write!(f, "evaluation"),
            PatternType::ToolUsagePattern => write!(f, "tool"),
            PatternType::KnowledgeStructure => write!(f, "knowledge"),
            PatternType::ProcessImprovement => write!(f, "process"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransferablePattern {
    pub id: String,
    pub title: String,
    pub description: String,
    pub pattern_type: PatternType,
    /// Where this pattern was originally observed (project name or "current")
    pub source_project: String,
    /// Evidence from current run (task titles, proposal summaries, scores, etc.)
    pub evidence: Vec<String>,
    /// How portable we think this is (0.0 = very specific, 1.0 = highly general)
    pub portability_score: f32,
    /// Suggested way to apply this in a new project
    pub suggested_application: String,
    /// Tags that help matching (e.g. "multi-agent", "self-refinement", "okf")
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrossProjectTransfer {
    pub id: String,
    pub pattern_id: String,
    pub pattern_title: String,
    pub from_project: String,
    pub to_project: String,
    pub transfer_type: String, // "import", "export", "adaptation"
    pub confidence: f32,
    pub notes: String,
    pub applied_this_cycle: bool,
}

/// Engine responsible for cross-project knowledge transfer (361.11).
#[derive(Debug, Clone)]
pub struct CrossProjectKnowledgeTransfer {
    pub simulation_mode: bool,
    /// Name of the "current" project (for labeling transfers)
    pub current_project: String,
}

impl CrossProjectKnowledgeTransfer {
    pub fn new(simulation_mode: bool, current_project: Option<String>) -> Self {
        Self {
            simulation_mode,
            current_project: current_project.unwrap_or_else(|| "grok-cli".to_string()),
        }
    }

    /// Extract patterns from the current HOH cycle that look transferable.
    /// This is the "export" side of cross-project transfer.
    pub fn extract_transferable_patterns(
        &self,
        goals: &[String],
        architecture_proposals: &[String],
        self_refinement_proposals: &[String],
        creative_ideas: &[crate::hoh::creativity::CreativityIdea],
        refactoring_actions: &[String],
    ) -> Vec<TransferablePattern> {
        let mut patterns = Vec::new();
        let goal_text = goals.join(" ").to_lowercase();

        // Pattern from architecture evolution / self-refinement
        if !architecture_proposals.is_empty() || !self_refinement_proposals.is_empty() {
            let desc = if !self_refinement_proposals.is_empty() {
                "Self-refinement loop that lets HOH propose and apply meta-improvements to its own planner and engines"
            } else {
                "Architecture evolution proposals turned into concrete, high-confidence refactoring actions"
            };

            patterns.push(TransferablePattern {
                id: format!("xproj-arch-{}", uuid::Uuid::new_v4()),
                title: "Self-Improving Architecture Evolution Loop".to_string(),
                description: desc.to_string(),
                pattern_type: PatternType::ArchitecturalPattern,
                source_project: self.current_project.clone(),
                evidence: architecture_proposals
                    .iter()
                    .chain(self_refinement_proposals.iter())
                    .take(3)
                    .cloned()
                    .collect(),
                portability_score: 0.82,
                suggested_application: "Run a similar propose → score → materialize → apply loop in the target project's outer development harness.".to_string(),
                tags: vec!["architecture".into(), "self-improvement".into(), "meta".into()],
            });
        }

        // Patterns coming from the creativity engine
        for idea in creative_ideas.iter().filter(|i| i.overall_score > 0.65) {
            if idea.tags.iter().any(|t| t.contains("creative") || t.contains("novel")) || goal_text.contains("creative") {
                patterns.push(TransferablePattern {
                    id: format!("xproj-idea-{}", uuid::Uuid::new_v4()),
                    title: format!("Creative Idea: {}", idea.title),
                    description: idea.description.clone(),
                    pattern_type: PatternType::ProcessImprovement,
                    source_project: self.current_project.clone(),
                    evidence: vec![format!("score={:.2}", idea.overall_score), idea.provenance.clone()],
                    portability_score: (idea.novelty_score * 0.6 + idea.feasibility_score * 0.4).clamp(0.4, 0.9),
                    suggested_application: "Seed a similar creativity / idea-generation step inside the target's planning phase.".to_string(),
                    tags: idea.tags.clone(),
                });
            }
        }

        // Refactoring heuristics that worked well
        for action in refactoring_actions.iter().take(2) {
            if action.to_lowercase().contains("extract") || action.to_lowercase().contains("split") || action.to_lowercase().contains("trait") {
                patterns.push(TransferablePattern {
                    id: format!("xproj-refactor-{}", uuid::Uuid::new_v4()),
                    title: "High-Confidence Small Refactoring Heuristic".to_string(),
                    description: action.clone(),
                    pattern_type: PatternType::RefactoringHeuristic,
                    source_project: self.current_project.clone(),
                    evidence: vec![action.clone()],
                    portability_score: 0.78,
                    suggested_application: "Apply similar tiny-safe-patch + confidence gating before executing refactors in the target project.".to_string(),
                    tags: vec!["refactoring".into(), "safety".into()],
                });
            }
        }

        // General multi-agent / specialization signal
        if goal_text.contains("agent") || goal_text.contains("multi") || goal_text.contains("specializ") {
            patterns.push(TransferablePattern {
                id: format!("xproj-agent-{}", uuid::Uuid::new_v4()),
                title: "Specialized Agent Profiles + Orchestration".to_string(),
                description: "Different agent profiles (Architect, Debugger, Researcher, etc.) with differentiated risk tolerance, success thresholds, and routing.".to_string(),
                pattern_type: PatternType::AgentBehavior,
                source_project: self.current_project.clone(),
                evidence: goals.iter().filter(|g| g.to_lowercase().contains("agent")).cloned().collect(),
                portability_score: 0.85,
                suggested_application: "Define a small set of role profiles and a lightweight router/orchestrator that selects profiles based on task characteristics.".to_string(),
                tags: vec!["multi-agent".into(), "specialization".into()],
            });
        }

        patterns
    }

    /// Given patterns from other projects (simulated via OKF or passed in),
    /// decide which ones are worth importing/adapting into the current project.
    pub fn find_applicable_transfers(
        &self,
        incoming_patterns: &[TransferablePattern],
        current_goals: &[String],
    ) -> Vec<CrossProjectTransfer> {
        let mut transfers = Vec::new();
        let goal_text = current_goals.join(" ").to_lowercase();

        for pattern in incoming_patterns {
            // Simple relevance heuristic
            let mut relevance = 0.0;

            for tag in &pattern.tags {
                if goal_text.contains(&tag.to_lowercase()) {
                    relevance += 0.35;
                }
            }

            if goal_text.contains(&pattern.pattern_type.to_string()) {
                relevance += 0.4;
            }

            // Boost highly portable patterns
            relevance += pattern.portability_score * 0.3;

            if relevance > 0.55 {
                transfers.push(CrossProjectTransfer {
                    id: format!("xfer-{}", uuid::Uuid::new_v4()),
                    pattern_id: pattern.id.clone(),
                    pattern_title: pattern.title.clone(),
                    from_project: pattern.source_project.clone(),
                    to_project: self.current_project.clone(),
                    transfer_type: "import".to_string(),
                    confidence: relevance.min(0.95),
                    notes: format!(
                        "Pattern '{}' looks relevant (portability {:.2}). Suggested: {}",
                        pattern.title, pattern.portability_score, pattern.suggested_application
                    ),
                    applied_this_cycle: false,
                });
            }
        }

        transfers
    }

    /// Simulate or record that a pattern was successfully transferred and applied.
    /// In a real multi-project setup this would update OKF or a shared knowledge store.
    pub fn record_successful_transfer(
        &self,
        pattern: &TransferablePattern,
        notes: &str,
    ) -> CrossProjectTransfer {
        CrossProjectTransfer {
            id: format!("xfer-applied-{}", uuid::Uuid::new_v4()),
            pattern_id: pattern.id.clone(),
            pattern_title: pattern.title.clone(),
            from_project: pattern.source_project.clone(),
            to_project: self.current_project.clone(),
            transfer_type: "applied".to_string(),
            confidence: 0.9,
            notes: notes.to_string(),
            applied_this_cycle: true,
        }
    }

    /// Convenience: turn the best transferable patterns into OKF-style concept descriptions.
    /// This is the main mechanism for making knowledge portable across projects.
    pub fn patterns_to_okf_concepts(&self, patterns: &[TransferablePattern]) -> Vec<String> {
        patterns
            .iter()
            .map(|p| {
                format!(
                    "---\ntype: HOH Cross-Project Pattern\ntitle: {}\ndescription: {}\ntags: [{}]\nsource_project: {}\nportability: {:.2}\n---\n\n**Pattern Type**: {}\n\n**Suggested Application**:\n{}\n\n**Evidence**:\n- {}",
                    p.title,
                    p.description,
                    p.tags.join(", "),
                    p.source_project,
                    p.portability_score,
                    p.pattern_type,
                    p.suggested_application,
                    p.evidence.join("\n- ")
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::creativity::{CreativityIdea, IdeaSource};

    #[test]
    fn extracts_patterns_in_simulation() {
        let engine = CrossProjectKnowledgeTransfer::new(true, Some("test-project".into()));
        let goals = vec!["Improve multi-agent collaboration".to_string()];
        let ideas = vec![CreativityIdea {
            id: "idea1".into(),
            title: "Agent Personality Genome".into(),
            description: "Evolvable agent traits".into(),
            novelty_score: 0.8,
            feasibility_score: 0.7,
            alignment_score: 0.9,
            overall_score: 0.82,
            source: IdeaSource::Heuristic,
            tags: vec!["agent".into(), "evolution".into()],
            provenance: "multi-agent goals".into(),
        }];

        let patterns = engine.extract_transferable_patterns(&goals, &[], &[], &ideas, &[]);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.pattern_type == PatternType::AgentBehavior));
    }

    #[test]
    fn finds_applicable_transfers() {
        let engine = CrossProjectKnowledgeTransfer::new(true, None);
        let incoming = vec![TransferablePattern {
            id: "p1".into(),
            title: "Self-Refinement Loop".into(),
            description: "...".into(),
            pattern_type: PatternType::ProcessImprovement,
            source_project: "other-project".into(),
            evidence: vec![],
            portability_score: 0.9,
            suggested_application: "...".into(),
            tags: vec!["self-improvement".into(), "meta".into()],
        }];

        let goals = vec!["Add self-refinement to HOH".to_string()];
        let transfers = engine.find_applicable_transfers(&incoming, &goals);
        assert!(!transfers.is_empty());
        assert!(transfers[0].confidence > 0.6);
    }
}
