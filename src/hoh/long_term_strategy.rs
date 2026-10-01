//! HOH Long-Term Strategy Engine (361.9)
//!
//! Maintains and evolves long-term strategic goals across many iterations.
//! Provides high-level strategic objectives, milestones, and trade-off decisions
//! that influence short-term planning in a coherent way.

use serde::{Deserialize, Serialize};
use crate::hoh::state::HOHError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum StrategyStatus {
    #[default]
    Active,
    Paused,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Milestone {
    pub name: String,
    pub target_iteration: u64,
    pub achieved: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategicGoal {
    pub id: String,
    pub description: String,
    /// Number of iterations this goal is intended to span (e.g. 20, 50, 100+)
    pub horizon_iterations: u32,
    /// Relative importance (0.0–1.0)
    pub priority: f32,
    pub milestones: Vec<Milestone>,
    /// Key trade-offs being managed
    pub tradeoffs: Vec<String>,
    /// Measurable success criteria
    pub success_criteria: Vec<String>,
    pub status: StrategyStatus,
    pub created_at: u64,
    pub last_reviewed: u64,
    /// How many times this strategy has directly influenced a plan
    #[serde(default)]
    pub influence_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LongTermStrategyEngine {
    pub simulation_mode: bool,
    pub strategies: Vec<StrategicGoal>,
}

impl LongTermStrategyEngine {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            simulation_mode,
            strategies: vec![],
        }
    }

    /// Evolve or generate long-term strategies based on current context.
    /// This is the core of 361.9 — it looks across many iterations.
    pub async fn evolve_strategies(
        &mut self,
        current_goals: &[String],
        recent_evaluations: &[crate::hoh::state::EvaluationReport],
        current_iteration: u64,
    ) -> Result<Vec<StrategicGoal>, HOHError> {
        let mut new_or_updated: Vec<StrategicGoal> = vec![];

        let goal_text = current_goals.join(" ").to_lowercase();

        // === Heuristic strategy generation / evolution ===

        // 1. Core self-improvement strategy (meta)
        if goal_text.contains("self") || goal_text.contains("meta") || goal_text.contains("improve") {
            if !self.strategies.iter().any(|s| s.description.contains("self-improvement") && s.status == StrategyStatus::Active) {
                let strat = StrategicGoal {
                    id: format!("strat-self-improve-{}", current_iteration),
                    description: "Drive continuous, measurable self-improvement of the HOH system itself".to_string(),
                    horizon_iterations: 80,
                    priority: 0.92,
                    milestones: vec![
                        Milestone { name: "Meta-planning loop stable".into(), target_iteration: current_iteration + 15, achieved: false, notes: "".into() },
                        Milestone { name: "Meta-improvement rate > 0.15 sustained".into(), target_iteration: current_iteration + 40, achieved: false, notes: "".into() },
                    ],
                    tradeoffs: vec![
                        "Short-term feature velocity vs long-term capability growth".into(),
                        "Risk of over-optimization / local maxima".into(),
                    ],
                    success_criteria: vec![
                        "Meta-evaluation health trending upward".into(),
                        "Number of successful self-refinements per 10 iterations >= 2".into(),
                    ],
                    status: StrategyStatus::Active,
                    created_at: current_iteration,
                    last_reviewed: current_iteration,
                    influence_count: 0,
                };
                self.strategies.push(strat.clone());
                new_or_updated.push(strat);
            }
        }

        // 2. Multi-agent / Harness-of-Harness maturity
        if goal_text.contains("agent") || goal_text.contains("multi") || goal_text.contains("harness") || goal_text.contains("orchestrat") {
            if !self.strategies.iter().any(|s| s.description.contains("multi-agent harness")) {
                let strat = StrategicGoal {
                    id: format!("strat-multi-agent-{}", current_iteration),
                    description: "Evolve HOH into a mature Harness-of-Harnesses with effective multi-agent collaboration and skill evolution".to_string(),
                    horizon_iterations: 60,
                    priority: 0.88,
                    milestones: vec![
                        Milestone { name: "361.6–361.9 features operational".into(), target_iteration: current_iteration + 8, achieved: false, notes: "".into() },
                        Milestone { name: "Real multi-agent tasks routinely succeed with >1 specialist".into(), target_iteration: current_iteration + 25, achieved: false, notes: "".into() },
                    ],
                    tradeoffs: vec!["Coordination overhead vs specialization gains".into()],
                    success_criteria: vec!["Average success rate of orchestrated tasks > 0.75".into()],
                    status: StrategyStatus::Active,
                    created_at: current_iteration,
                    last_reviewed: current_iteration,
                    influence_count: 0,
                };
                self.strategies.push(strat.clone());
                new_or_updated.push(strat);
            }
        }

        // 3. Innovation & creativity acceleration
        if goal_text.contains("creativ") || goal_text.contains("novel") || goal_text.contains("idea") {
            if !self.strategies.iter().any(|s| s.description.contains("innovation")) {
                let strat = StrategicGoal {
                    id: format!("strat-innovation-{}", current_iteration),
                    description: "Accelerate generation and successful integration of novel, high-value ideas".to_string(),
                    horizon_iterations: 45,
                    priority: 0.75,
                    milestones: vec![
                        Milestone { name: "Creative ideas regularly materialize into tasks/patches".into(), target_iteration: current_iteration + 12, achieved: false, notes: "".into() },
                    ],
                    tradeoffs: vec!["Exploration (novelty) vs exploitation (stability)".into()],
                    success_criteria: vec!["At least 1 creative-originated improvement accepted per 5 iterations".into()],
                    status: StrategyStatus::Active,
                    created_at: current_iteration,
                    last_reviewed: current_iteration,
                    influence_count: 0,
                };
                self.strategies.push(strat.clone());
                new_or_updated.push(strat);
            }
        }

        // 4. Long-term knowledge & memory (cross-iteration learning)
        if goal_text.contains("memory") || goal_text.contains("knowledge") || goal_text.contains("okf") {
            if !self.strategies.iter().any(|s| s.description.contains("long-term knowledge")) {
                let strat = StrategicGoal {
                    id: format!("strat-knowledge-{}", current_iteration),
                    description: "Build robust long-term knowledge consolidation and retrieval across iterations".to_string(),
                    horizon_iterations: 70,
                    priority: 0.82,
                    milestones: vec![],
                    tradeoffs: vec!["Storage/compute cost vs recall quality".into()],
                    success_criteria: vec!["High-value past iteration insights are automatically surfaced in planning".into()],
                    status: StrategyStatus::Active,
                    created_at: current_iteration,
                    last_reviewed: current_iteration,
                    influence_count: 0,
                };
                self.strategies.push(strat.clone());
                new_or_updated.push(strat);
            }
        }

        // Review & update existing strategies
        for strat in &mut self.strategies {
            strat.last_reviewed = current_iteration;

            // Simple progress check based on recent evaluations
            let recent_success = recent_evaluations
                .iter()
                .filter_map(|e| e.helix_score)
                .filter(|&s| s > 0.65)
                .count();

            if recent_success > 2 && strat.status == StrategyStatus::Active {
                // Lightly boost influence tracking
                strat.influence_count = strat.influence_count.saturating_add(1);
            }

            // Auto-complete if many milestones achieved
            let achieved = strat.milestones.iter().filter(|m| m.achieved).count();
            if !strat.milestones.is_empty() && achieved as f32 / strat.milestones.len() as f32 >= 0.8 {
                strat.status = StrategyStatus::Completed;
            }
        }

        // Prune very old low-priority abandoned strategies (keep history lean)
        self.strategies.retain(|s| {
            !(s.status == StrategyStatus::Abandoned && current_iteration.saturating_sub(s.created_at) > 120)
        });

        if !self.simulation_mode && !new_or_updated.is_empty() {
            tracing::info!(
                "361.9: Long-Term Strategy Engine generated/updated {} strategic goals",
                new_or_updated.len()
            );
        }

        Ok(new_or_updated)
    }

    /// Inject long-term strategic considerations into the current short-term planning context.
    /// This is the key "influence short-term planning coherently" mechanism.
    pub fn influence_planning(
        &self,
        goals: &mut Vec<String>,
        improvement_suggestions: &mut Vec<String>,
    ) {
        let active: Vec<_> = self.strategies
            .iter()
            .filter(|s| s.status == StrategyStatus::Active && s.priority >= 0.65)
            .collect();

        for strat in &active {
            // Add a strategic goal signal
            let strat_goal = format!("STRATEGIC (361.9): {} [horizon: {} iter]", strat.description, strat.horizon_iterations);
            if !goals.contains(&strat_goal) {
                goals.push(strat_goal);
            }

            // Record influence
            improvement_suggestions.push(format!(
                "361.9-STRATEGY: {} (priority {:.2}, influences so far: {})",
                strat.description, strat.priority, strat.influence_count
            ));

            // Surface key tradeoffs
            for tradeoff in strat.tradeoffs.iter().take(2) {
                improvement_suggestions.push(format!("  TRADEOFF: {}", tradeoff));
            }
        }

        // Add a summary note if we have active long-term strategies
        if !active.is_empty() {
            improvement_suggestions.push(format!(
                "361.9: {} active long-term strategies guiding this iteration",
                active.len()
            ));
        }
    }

    pub fn get_active_strategies(&self) -> Vec<&StrategicGoal> {
        self.strategies
            .iter()
            .filter(|s| s.status == StrategyStatus::Active)
            .collect()
    }

    /// Mark a milestone as achieved (called by outer loop / continual improvement when evidence exists)
    pub fn mark_milestone_achieved(&mut self, strategy_id: &str, milestone_name: &str) {
        if let Some(strat) = self.strategies.iter_mut().find(|s| s.id == strategy_id) {
            for m in &mut strat.milestones {
                if m.name.to_lowercase().contains(&milestone_name.to_lowercase()) {
                    m.achieved = true;
                    m.notes = format!("Achieved around iteration {}", strat.last_reviewed);
                }
            }
        }
    }
}
