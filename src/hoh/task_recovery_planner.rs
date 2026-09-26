//! HOH Task Recovery Planner (Task 327.26)
//!
//! Creates recovery plans for blocked or failed tasks.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub task_id: u64,
    pub reason: String,
    pub actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Add a new subtask to unblock the parent.
    AddSubtask { title: String, description: String },
    /// Change the priority of a dependency.
    EscalateDependency { dep_id: u64 },
    /// Suggest an alternative implementation approach.
    SuggestAlternative(String),
    /// Defer the task to a later iteration.
    Defer(String),
}

pub fn plan_recovery(task: &Task, blocked_by: &[u64], failure_reason: Option<&str>) -> RecoveryPlan {
    let mut actions = Vec::new();
    let mut reason = String::new();

    if !blocked_by.is_empty() {
        reason = format!("Task {} blocked by tasks: {:?}", task.id, blocked_by);
        for &dep_id in blocked_by {
            actions.push(RecoveryAction::EscalateDependency { dep_id });
        }
        actions.push(RecoveryAction::AddSubtask {
            title: format!("Unblock task {} — resolve dependency issues", task.id),
            description: format!("Investigate why tasks {:?} are not progressing and unblock them.", blocked_by),
        });
    }

    if let Some(fail) = failure_reason {
        reason = format!("{} | failure: {}", reason, fail);
        let text = fail.to_lowercase();
        if text.contains("test") || text.contains("fail") {
            actions.push(RecoveryAction::AddSubtask {
                title: "Fix failing tests".to_string(),
                description: format!("Address test failures related to task {}: {}", task.id, fail),
            });
        }
        if text.contains("compile") || text.contains("error") {
            actions.push(RecoveryAction::SuggestAlternative(
                format!("Consider a simpler implementation approach for task {} to avoid compile errors", task.id)
            ));
        }
        if text.contains("time") || text.contains("budget") {
            actions.push(RecoveryAction::Defer(
                format!("Task {} exceeds time budget — defer to next iteration", task.id)
            ));
        }
    }

    if actions.is_empty() {
        reason = format!("Task {} stalled without clear cause", task.id);
        actions.push(RecoveryAction::SuggestAlternative(
            "Break this task into smaller, more achievable subtasks".to_string()
        ));
    }

    RecoveryPlan { task_id: task.id, reason, actions }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64) -> Task {
        Task { id, title: format!("Task {}", id), ..Default::default() }
    }

    #[test]
    fn test_blocked_task_gets_escalation_actions() {
        let plan = plan_recovery(&t(5), &[1, 2], None);
        assert!(!plan.actions.is_empty());
        assert!(plan.actions.iter().any(|a| matches!(a, RecoveryAction::EscalateDependency { .. })));
    }

    #[test]
    fn test_test_failure_gets_fix_subtask() {
        let plan = plan_recovery(&t(3), &[], Some("tests fail in CI"));
        assert!(plan.actions.iter().any(|a| matches!(a, RecoveryAction::AddSubtask { .. })));
    }
}
