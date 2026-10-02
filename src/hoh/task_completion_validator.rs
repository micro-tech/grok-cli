//! HOH Task Completion Validator (Task 327.27)
//!
//! Validates that tasks marked "done" actually satisfy their testStrategy.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationResult { Passed, Failed(String), Inconclusive(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionValidation {
    pub task_id: u64,
    pub result: ValidationResult,
    pub evidence: Vec<String>,
}

/// Validate a task that was just marked done.
///
/// `test_output`: raw cargo test output (if available).
/// `files_changed`: files that were actually modified.
pub fn validate_completion(
    task: &Task,
    test_output: Option<&str>,
    files_changed: &[String],
) -> CompletionValidation {
    let mut evidence = Vec::new();

    // Must be marked done
    if task.status != "done" {
        return CompletionValidation {
            task_id: task.id,
            result: ValidationResult::Failed("task is not marked done".to_string()),
            evidence,
        };
    }

    // Must have a testStrategy
    if task.test_strategy.trim().is_empty() {
        return CompletionValidation {
            task_id: task.id,
            result: ValidationResult::Inconclusive(
                "no testStrategy defined — cannot validate".to_string()
            ),
            evidence,
        };
    }
    evidence.push(format!("testStrategy: {}", task.test_strategy.chars().take(120).collect::<String>()));

    // If test output is available, scan for failures
    if let Some(output) = test_output {
        if output.contains("FAILED") || output.contains("error[") {
            return CompletionValidation {
                task_id: task.id,
                result: ValidationResult::Failed(
                    "test output contains FAILED or errors".to_string()
                ),
                evidence,
            };
        }
        if output.contains("test result: ok") {
            evidence.push("cargo test passed".to_string());
        }
    }

    // Check that at least one file was changed (crude proxy for real work)
    if files_changed.is_empty() {
        evidence.push("warning: no files changed recorded".to_string());
    } else {
        evidence.push(format!("files changed: {}", files_changed.join(", ")));
    }

    // All subtasks done?
    if !task.subtasks.is_empty() {
        let pending: Vec<_> = task.subtasks.iter()
            .filter(|s| s.status != "done")
            .map(|s| s.title.as_str())
            .collect();
        if !pending.is_empty() {
            return CompletionValidation {
                task_id: task.id,
                result: ValidationResult::Failed(
                    format!("subtasks still pending: {:?}", pending)
                ),
                evidence,
            };
        }
        evidence.push(format!("all {} subtasks done", task.subtasks.len()));
    }

    CompletionValidation {
        task_id: task.id,
        result: ValidationResult::Passed,
        evidence,
    }
}

/// Validate a slice of tasks, returning only failures/inconclusive.
pub fn validate_all(
    tasks: &[Task],
    test_output: Option<&str>,
    files_changed: &[String],
) -> Vec<CompletionValidation> {
    tasks.iter()
        .filter(|t| t.status == "done")
        .map(|t| validate_completion(t, test_output, files_changed))
        .filter(|v| v.result != ValidationResult::Passed)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn done_task() -> Task {
        Task {
            id: 1, status: "done".to_string(),
            test_strategy: "cargo test must pass".to_string(),
            title: "Do stuff".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_passing_output_validates() {
        let v = validate_completion(&done_task(), Some("test result: ok. 5 passed"), &["src/a.rs".to_string()]);
        assert_eq!(v.result, ValidationResult::Passed);
    }

    #[test]
    fn test_failed_output_fails_validation() {
        let v = validate_completion(&done_task(), Some("test result: FAILED. 1 failed"), &[]);
        assert!(matches!(v.result, ValidationResult::Failed(_)));
    }

    #[test]
    fn test_no_test_strategy_is_inconclusive() {
        let mut t = done_task();
        t.test_strategy = String::new();
        let v = validate_completion(&t, None, &[]);
        assert!(matches!(v.result, ValidationResult::Inconclusive(_)));
    }
}
