//! HOH Agent Skill Evolution (361.6)
//!
//! Allows agents (specialized profiles, born agents, lifecycle-managed agents)
//! to acquire new skills over time, synthesize new capabilities, and retire
//! obsolete ones.
//!
//! Integrates with:
//! - 361.5 SpecializedAgentProfile (profiles can evolve their tool sets)
//! - 404 Agent Birth (newborns get initial skills + evolution potential)
//! - 405 AgentEvolutionSystem (skill mutations are a form of evolution)
//! - 403 Lifecycle (retired skills affect retirement decisions)
//! - Existing skills/ system and agent_tools (spawn with dynamic allowed_tools)
//!
//! Core concepts:
//! - Skill: named capability + description + required tools + success signals
//! - Dynamic loading from catalog + synthesis from creativity/ideas
//! - Mutation: add/remove/adapt skills based on performance feedback
//! - Retirement: low-utility skills are dropped

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::hoh::state::HOHError;
use crate::hoh::specialized_agents::AgentProfile as SpecializedProfile;
use crate::hoh::creativity::CreativityIdea;

/// A single acquirable/evolvable skill for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentSkill {
    pub name: String,
    pub description: String,
    /// Tools this skill enables (names from the tool registry)
    pub enabled_tools: Vec<String>,
    /// Keywords that match tasks this skill is good for
    pub task_keywords: Vec<String>,
    /// Historical success rate when this skill was used (0.0-1.0)
    pub demonstrated_success: f32,
    /// How many times this skill contributed to a successful outcome
    pub usage_count: u32,
    pub first_acquired: u64,
    #[serde(default)]
    pub last_used: Option<u64>,
}

impl AgentSkill {
    pub fn new(name: &str, description: &str, tools: Vec<String>, keywords: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            enabled_tools: tools,
            task_keywords: keywords,
            demonstrated_success: 0.6,
            usage_count: 0,
            first_acquired: chrono::Utc::now().timestamp() as u64,
            last_used: None,
        }
    }

    pub fn record_use(&mut self, success: bool, quality: f32) {
        self.usage_count += 1;
        self.last_used = Some(chrono::Utc::now().timestamp() as u64);

        // Simple exponential moving average
        let outcome = if success { quality } else { quality * 0.6 };
        self.demonstrated_success = 0.7 * self.demonstrated_success + 0.3 * outcome;
    }

    pub fn utility_score(&self) -> f32 {
        // Higher usage + higher success = higher utility
        let usage_factor = (self.usage_count as f32 / 10.0).min(1.0);
        self.demonstrated_success * (0.4 + 0.6 * usage_factor)
    }
}

/// Per-agent skill portfolio.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentSkillPortfolio {
    pub agent_id: String,
    pub skills: HashMap<String, AgentSkill>,
    pub retired_skills: Vec<String>,
}

impl AgentSkillPortfolio {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            skills: HashMap::new(),
            retired_skills: vec![],
        }
    }

    pub fn add_skill(&mut self, skill: AgentSkill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    pub fn remove_skill(&mut self, name: &str) -> Option<AgentSkill> {
        if let Some(s) = self.skills.remove(name) {
            self.retired_skills.push(name.to_string());
            Some(s)
        } else {
            None
        }
    }

    pub fn get_effective_tools(&self) -> HashSet<String> {
        self.skills
            .values()
            .flat_map(|s| s.enabled_tools.iter().cloned())
            .collect()
    }

    pub fn describe_skills(&self, _agent_id: &str) -> String {
        if self.skills.is_empty() {
            return "base-profile".to_string();
        }
        let skills: Vec<String> = self.skills.values().map(|s| s.name.clone()).collect();
        skills.join(",")
    }

    pub fn best_matching_skill(&self, task_text: &str) -> Option<&AgentSkill> {
        let lower = task_text.to_lowercase();
        self.skills
            .values()
            .filter(|s| {
                s.task_keywords.iter().any(|k| lower.contains(&k.to_lowercase()))
                    || lower.contains(&s.name.to_lowercase())
            })
            .max_by(|a, b| {
                a.utility_score()
                    .partial_cmp(&b.utility_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

/// The main Agent Skill Evolution System (361.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillEvolutionSystem {
    pub simulation_mode: bool,
    pub portfolios: HashMap<String, AgentSkillPortfolio>,
    pub global_skill_catalog: Vec<AgentSkill>, // seeds + synthesized
}

impl AgentSkillEvolutionSystem {
    pub fn new(simulation_mode: bool) -> Self {
        let mut sys = Self {
            simulation_mode,
            portfolios: HashMap::new(),
            global_skill_catalog: vec![],
        };
        sys.seed_initial_skills();
        sys
    }

    fn seed_initial_skills(&mut self) {
        self.global_skill_catalog = vec![
            AgentSkill::new(
                "architecture_design",
                "Propose and evaluate high-level module structures and layerings",
                vec!["read_file".into(), "glob_search".into()],
                vec!["architecture".into(), "module".into(), "layer".into(), "design".into()],
            ),
            AgentSkill::new(
                "debugging_diagnosis",
                "Root-cause analysis from logs, tests, and stack traces",
                vec!["search_file_content".into(), "run_shell_command".into()],
                vec!["bug".into(), "fail".into(), "error".into(), "debug".into()],
            ),
            AgentSkill::new(
                "test_strategy",
                "Design and generate high-quality tests (unit, property, integration)",
                vec!["run_shell_command".into()],
                vec!["test".into(), "coverage".into(), "verify".into()],
            ),
            AgentSkill::new(
                "safe_refactoring",
                "Small, low-risk structural improvements with good tests",
                vec!["read_file".into(), "search_file_content".into()],
                vec!["refactor".into(), "clean".into(), "extract".into()],
            ),
            AgentSkill::new(
                "multi_agent_coordination",
                "Delegate, negotiate, and synchronize work across specialized agents",
                vec!["spawn_agent".into(), "send_message".into(), "join_agents".into(), "list_agents".into()],
                vec!["multi-agent".into(), "collaborate".into(), "delegate".into(), "handoff".into()],
            ),
            AgentSkill::new(
                "knowledge_synthesis",
                "Extract, index, and reuse cross-iteration knowledge (OKF + history)",
                vec!["okf_lookup".into(), "okf_get".into(), "recall_context".into()],
                vec!["memory".into(), "knowledge".into(), "okf".into()],
            ),
        ];
    }

    /// Give an agent its initial skills (usually from profile + birth).
    pub fn bootstrap_agent(&mut self, agent_id: &str, profile: &SpecializedProfile, extra_keywords: &[String]) {
        let mut portfolio = AgentSkillPortfolio::new(agent_id);

        // Map profile to core skills
        let core_names: Vec<&str> = match profile {
            SpecializedProfile::Architect => vec!["architecture_design", "safe_refactoring"],
            SpecializedProfile::Debugger => vec!["debugging_diagnosis"],
            SpecializedProfile::Researcher => vec!["knowledge_synthesis"],
            SpecializedProfile::Tester => vec!["test_strategy"],
            SpecializedProfile::Refactorer => vec!["safe_refactoring"],
            SpecializedProfile::Governor => vec!["safe_refactoring", "test_strategy"],
        };

        for name in core_names {
            if let Some(seed) = self.global_skill_catalog.iter().find(|s| s.name == name) {
                let mut skill = seed.clone();
                // Personalize a bit
                if !extra_keywords.is_empty() {
                    skill.task_keywords.extend(extra_keywords.iter().cloned());
                }
                portfolio.add_skill(skill);
            }
        }

        // Always give coordination potential (361.7 synergy)
        if let Some(coord) = self.global_skill_catalog.iter().find(|s| s.name == "multi_agent_coordination") {
            portfolio.add_skill(coord.clone());
        }

        self.portfolios.insert(agent_id.to_string(), portfolio);
    }

    /// Synthesize a brand new skill from a creativity idea or design.
    pub fn synthesize_skill_from_idea(&mut self, idea: &CreativityIdea) -> Option<AgentSkill> {
        if idea.overall_score < 0.55 {
            return None;
        }

        let name = format!("synthesized_{}", idea.id.replace(|c: char| !c.is_alphanumeric(), "_").to_lowercase());
        if self.global_skill_catalog.iter().any(|s| s.name == name) {
            return None;
        }

        let tools = if idea.title.to_lowercase().contains("agent") || idea.title.to_lowercase().contains("collaborat") {
            vec!["spawn_agent".into(), "send_message".into(), "join_agents".into()]
        } else if idea.title.to_lowercase().contains("test") {
            vec!["run_shell_command".into()]
        } else {
            vec!["read_file".into(), "search_file_content".into()]
        };

        let skill = AgentSkill::new(
            &name,
            &format!("Synthesized from creativity: {}", idea.title),
            tools,
            idea.tags.clone(),
        );

        self.global_skill_catalog.push(skill.clone());
        Some(skill)
    }

    /// Evolve skills for agents based on recent performance signals.
    /// This is the core 361.6 loop: success → reinforce, failure → mutate or retire.
    pub async fn evolve_skills(
        &mut self,
        agent_id: &str,
        task_description: &str,
        success: bool,
        quality: f32,
        creativity_ideas: &[CreativityIdea],
    ) -> Result<Vec<String>, HOHError> {
        let mut changes = vec![];

        // Synthesize first (if needed) to avoid double mutable borrow on self
        let maybe_new_skill = if success && quality > 0.75 {
            let fallback_idea = creativity_ideas.first().cloned().unwrap_or_else(|| CreativityIdea {
                id: "fallback".into(),
                title: "general improvement".into(),
                description: "synthesized fallback".into(),
                novelty_score: 0.5,
                feasibility_score: 0.7,
                alignment_score: 0.6,
                overall_score: 0.6,
                source: crate::hoh::creativity::IdeaSource::Heuristic,
                tags: vec!["fallback".into()],
                provenance: "skill-evolution-fallback".into(),
            });
            self.synthesize_skill_from_idea(&fallback_idea)
        } else {
            None
        };

        let portfolio = self.portfolios.entry(agent_id.to_string()).or_insert_with(|| AgentSkillPortfolio::new(agent_id));

        // 1. Find or create matching skill for this task
        let matching = portfolio.best_matching_skill(task_description).cloned();
        if let Some(mut skill) = matching {
            skill.record_use(success, quality);
            changes.push(format!("Reinforced skill '{}' (utility now {:.2})", skill.name, skill.utility_score()));
            portfolio.skills.insert(skill.name.clone(), skill);
        } else if let Some(new_skill) = maybe_new_skill {
            portfolio.add_skill(new_skill.clone());
            changes.push(format!("Acquired new synthesized skill: {}", new_skill.name));
        } else if success && quality > 0.75 {
            // Fall back to a global catalog skill
            if let Some(seed) = self.global_skill_catalog.first() {
                let mut s = seed.clone();
                s.task_keywords.push(task_description.split_whitespace().take(3).collect::<Vec<_>>().join(" "));
                portfolio.add_skill(s.clone());
                changes.push(format!("Acquired skill from catalog: {}", s.name));
            }
        }

        // 2. Retire very low-utility skills (skill retirement)
        let to_retire: Vec<String> = portfolio
            .skills
            .values()
            .filter(|s| s.usage_count >= 4 && s.utility_score() < 0.35)
            .map(|s| s.name.clone())
            .collect();

        for name in to_retire {
            if let Some(retired) = portfolio.remove_skill(&name) {
                changes.push(format!("Retired low-utility skill '{}'", retired.name));
            }
        }

        // 3. Occasionally promote a high-performing skill into the global catalog
        for skill in portfolio.skills.values() {
            if skill.usage_count >= 5 && skill.demonstrated_success > 0.88 && !self.global_skill_catalog.iter().any(|g| g.name == skill.name) {
                self.global_skill_catalog.push(skill.clone());
                changes.push(format!("Promoted '{}' to global skill catalog", skill.name));
            }
        }

        if !changes.is_empty() && !self.simulation_mode {
            tracing::info!("361.6: Skill evolution for {} produced: {:?}", agent_id, changes);
        }

        Ok(changes)
    }

    /// Get the current effective tool set for an agent (used when spawning/configuring sub-agents).
    pub fn get_agent_tools(&self, agent_id: &str) -> HashSet<String> {
        self.portfolios
            .get(agent_id)
            .map(|p| p.get_effective_tools())
            .unwrap_or_default()
    }

    /// Return a human-readable summary of an agent's current skills.
    pub fn describe_skills(&self, agent_id: &str) -> String {
        match self.portfolios.get(agent_id) {
            Some(p) if !p.skills.is_empty() => {
                let skills: Vec<String> = p.skills.values().map(|s| {
                    format!("{} (util={:.2}, uses={})", s.name, s.utility_score(), s.usage_count)
                }).collect();
                format!("Skills: {}", skills.join(", "))
            }
            _ => "No evolved skills yet (using base profile)".to_string(),
        }
    }

    /// Public helper so simulation/orchestrator code can get a simple keyword summary.
    pub fn skill_summary(&self, agent_id: &str) -> String {
        self.describe_skills(agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::specialized_agents::AgentProfile;

    #[test]
    fn test_bootstrap_and_tool_derivation() {
        let mut sys = AgentSkillEvolutionSystem::new(true);
        sys.bootstrap_agent("arch-1", &AgentProfile::Architect, &[]);

        let tools = sys.get_agent_tools("arch-1");
        assert!(tools.contains("read_file"));
        assert!(tools.contains("spawn_agent")); // coordination is always seeded
    }

    #[tokio::test]
    async fn test_skill_evolution_and_retirement() {
        let mut sys = AgentSkillEvolutionSystem::new(true);
        sys.bootstrap_agent("test-agent", &AgentProfile::Debugger, &[]);

        // Simulate repeated poor performance on a skill
        for _ in 0..6 {
            let _ = sys.evolve_skills("test-agent", "fix nasty bug in parser", false, 0.3, &[]).await;
        }

        let portfolio = sys.portfolios.get("test-agent").unwrap();
        // Should have retired something or not added bad skills
        assert!(portfolio.skills.values().all(|s| s.utility_score() > 0.2) || portfolio.retired_skills.len() > 0);
    }

    #[test]
    fn test_synthesis_from_idea() {
        let mut sys = AgentSkillEvolutionSystem::new(true);
        let idea = CreativityIdea {
            id: "idea-xyz".into(),
            title: "New multi-agent collaboration pattern".into(),
            description: "...".into(),
            novelty_score: 0.8,
            feasibility_score: 0.7,
            alignment_score: 0.75,
            overall_score: 0.78,
            source: crate::hoh::creativity::IdeaSource::Combination,
            tags: vec!["collaboration".into()],
            provenance: "test".into(),
        };

        let synthesized = sys.synthesize_skill_from_idea(&idea);
        assert!(synthesized.is_some());
        let s = synthesized.unwrap();
        assert!(s.name.contains("synthesized"));
        assert!(s.enabled_tools.contains(&"send_message".to_string()));
    }
}