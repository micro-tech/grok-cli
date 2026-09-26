//! HOH Self-Optimization Engine (Task 361.40)
//!
//! Analyzes HOH's own performance metrics and proposes improvements
//! to its planning, evaluation, and execution strategies.

use crate::hoh::state::{HOHPlan, IterationState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOpportunity {
    pub component: String,
    pub current_behavior: String,
    pub proposed_change: String,
    pub expected_gain: f32,   // 0.0–1.0
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfOptimizationReport {
    pub iteration_id: u64,
    pub opportunities: Vec<OptimizationOpportunity>,
    pub priority_action: Option<String>,
    pub overall_efficiency: f32,
}

/// Analyze HOH performance history and produce self-optimization proposals.
pub fn analyze_and_propose(
    history: &[IterationState],
    current: &IterationState,
) -> SelfOptimizationReport {
    let mut opportunities = Vec::new();

    if history.is_empty() {
        return SelfOptimizationReport {
            iteration_id: current.iteration_id,
            opportunities: vec![OptimizationOpportunity {
                component: "bootstrap".to_string(),
                current_behavior: "No history available".to_string(),
                proposed_change: "Collect at least 3 iterations before optimizing".to_string(),
                expected_gain: 0.1, confidence: 0.9,
            }],
            priority_action: Some("Gather more iteration data".to_string()),
            overall_efficiency: 0.5,
        };
    }

    // Analyze evaluation trends
    let scores: Vec<f32> = history.iter()
        .flat_map(|s| s.evaluations.iter().filter_map(|e| e.helix_score))
        .collect();

    let avg_score = if scores.is_empty() { 0.0 }
        else { scores.iter().sum::<f32>() / scores.len() as f32 };

    // Trend: is score improving?
    let is_improving = scores.windows(2).all(|w| w[1] >= w[0] - 0.05);
    if !is_improving && scores.len() >= 2 {
        opportunities.push(OptimizationOpportunity {
            component: "planner".to_string(),
            current_behavior: format!("Helix scores trending down (avg={:.2})", avg_score),
            proposed_change: "Increase weight of test-strategy signal in task prioritization".to_string(),
            expected_gain: 0.1, confidence: 0.65,
        });
    }

    // Analyze patch counts — too many patches may indicate thrashing
    let avg_patches = history.iter().map(|s| s.patches.len()).sum::<usize>() as f32
        / history.len() as f32;
    if avg_patches > 10.0 {
        opportunities.push(OptimizationOpportunity {
            component: "execution".to_string(),
            current_behavior: format!("{:.1} patches/iteration on average", avg_patches),
            proposed_change: "Cap patches per iteration at 5 and focus on quality over quantity".to_string(),
            expected_gain: 0.15, confidence: 0.7,
        });
    }

    // Analyze failure rate
    let fail_rate = history.iter()
        .flat_map(|s| s.evaluations.iter())
        .filter(|e| e.test_passed == Some(false))
        .count() as f32 / history.iter().flat_map(|s| s.evaluations.iter()).count().max(1) as f32;

    if fail_rate > 0.3 {
        opportunities.push(OptimizationOpportunity {
            component: "test_selection".to_string(),
            current_behavior: format!("{:.0}% evaluation failure rate", fail_rate * 100.0),
            proposed_change: "Prioritize tasks with strong testStrategy; use simulation before real execution".to_string(),
            expected_gain: 0.2, confidence: 0.75,
        });
    }

    // Planning phase: are goals too vague?
    if let Some(plan) = &current.plan {
        let avg_goal_len = plan.goals.iter().map(|g| g.len()).sum::<usize>()
            / plan.goals.len().max(1);
        if avg_goal_len < 20 {
            opportunities.push(OptimizationOpportunity {
                component: "goal_setting".to_string(),
                current_behavior: format!("Goals are short ({} chars avg)", avg_goal_len),
                proposed_change: "Generate more specific, measurable goals in the planning phase".to_string(),
                expected_gain: 0.1, confidence: 0.6,
            });
        }
    }

    // Sort by expected gain
    opportunities.sort_by(|a, b| b.expected_gain.partial_cmp(&a.expected_gain)
        .unwrap_or(std::cmp::Ordering::Equal));

    let priority_action = opportunities.first().map(|o| o.proposed_change.clone());
    let overall_efficiency = (avg_score * 0.6 + (1.0 - fail_rate) * 0.4).clamp(0.0, 1.0);

    tracing::info!(
        iteration = current.iteration_id,
        opportunities = opportunities.len(),
        efficiency = overall_efficiency,
        "[HOH SelfOptimizer] analysis complete"
    );

    SelfOptimizationReport { iteration_id: current.iteration_id, opportunities, priority_action, overall_efficiency }
}

/// Apply the top optimization opportunity to the plan for the next iteration.
pub fn apply_top_opportunity(report: &SelfOptimizationReport, plan: &mut HOHPlan) {
    if let Some(action) = &report.priority_action {
        plan.improvement_suggestions.push(format!("[SelfOpt] {}", action));
        plan.goals.push(format!("Self-optimize: {}", action));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::state::{EvaluationReport, IterationStatus};

    fn state_with_score(id: u64, score: f32, test_passed: bool) -> IterationState {
        let mut s = IterationState::new(id);
        s.status = IterationStatus::Completed;
        s.evaluations.push(EvaluationReport {
            helix_score: Some(score), test_passed: Some(test_passed),
            ..Default::default()
        });
        s
    }

    #[test]
    fn test_no_history_returns_bootstrap_opportunity() {
        let current = IterationState::new(1);
        let report = analyze_and_propose(&[], &current);
        assert!(!report.opportunities.is_empty());
        assert!(report.opportunities[0].component == "bootstrap");
    }

    #[test]
    fn test_declining_scores_triggers_planner_opportunity() {
        let history = vec![
            state_with_score(1, 0.8, true),
            state_with_score(2, 0.6, true),
            state_with_score(3, 0.4, false),
        ];
        let current = state_with_score(4, 0.3, false);
        let report = analyze_and_propose(&history, &current);
        assert!(report.opportunities.iter().any(|o| o.component == "planner"
            || o.component == "test_selection"));
    }

    #[test]
    fn test_apply_adds_to_plan() {
        let report = SelfOptimizationReport {
            iteration_id: 1,
            opportunities: vec![],
            priority_action: Some("Increase test coverage".to_string()),
            overall_efficiency: 0.7,
        };
        let mut plan = HOHPlan::default();
        apply_top_opportunity(&report, &mut plan);
        assert!(!plan.improvement_suggestions.is_empty());
        assert!(!plan.goals.is_empty());
    }
}
