//! HOH Risk-Based Planning (Task 361.17)

use crate::hoh::state::HOHPlan;
use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAdjustedPlan {
    pub selected_tasks: Vec<u64>,
    pub mitigations: Vec<(u64, String)>,    // (task_id, mitigation)
    pub total_risk_score: f32,
    pub rationale: String,
}

/// Re-order tasks by risk and inject mitigations into the plan.
pub fn build_risk_adjusted_plan(
    tasks: &[Task],
    initial_plan: &HOHPlan,
) -> RiskAdjustedPlan {
    use crate::hoh::task_risk::assess_risk;

    let mut selected_with_risk: Vec<(u64, f32)> = initial_plan.selected_tasks.iter()
        .filter_map(|&tid| tasks.iter().find(|t| t.id == tid))
        .map(|t| {
            let r = assess_risk(t);
            (t.id, r.risk_score)
        })
        .collect();

    // Sort: low-risk first (tackle safe work before risky changes)
    selected_with_risk.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let selected_tasks: Vec<u64> = selected_with_risk.iter().map(|(id, _)| *id).collect();
    let total_risk: f32 = selected_with_risk.iter().map(|(_, r)| r).sum::<f32>()
        / selected_with_risk.len().max(1) as f32;

    let mitigations: Vec<(u64, String)> = selected_with_risk.iter()
        .filter(|(_, r)| *r > 0.3)
        .map(|(id, r)| {
            let _task = tasks.iter().find(|t| t.id == *id);
            let mit = if *r > 0.6 {
                format!("HIGH RISK task {id}: require human approval and full test run before applying")
            } else {
                format!("MEDIUM RISK task {id}: run full test suite before and after")
            };
            (*id, mit)
        })
        .collect();

    let mit_count = mitigations.len();
    let sel_count  = selected_with_risk.len();
    RiskAdjustedPlan {
        selected_tasks,
        mitigations,
        total_risk_score: total_risk,
        rationale: format!(
            "{} tasks selected, avg risk={:.2}, {} mitigations required",
            sel_count, total_risk, mit_count
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, title: &str) -> Task {
        Task { id, title: title.to_string(), ..Default::default() }
    }

    #[test]
    fn test_risk_plan_orders_safe_first() {
        let tasks = vec![
            t(1, "Fix typo in README"),
            t(2, "Rewrite security auth module with breaking API change"),
        ];
        let plan = HOHPlan { selected_tasks: vec![2, 1], ..Default::default() };
        let adjusted = build_risk_adjusted_plan(&tasks, &plan);
        // Safe task (1) should come before risky task (2)
        assert_eq!(adjusted.selected_tasks[0], 1);
    }
}
