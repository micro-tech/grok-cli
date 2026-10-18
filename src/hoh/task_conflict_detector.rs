//! HOH Task Conflict Detector (Task 327.18)
//!
//! Detects tasks that are contradictory, duplicate, or cannot safely coexist.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskConflictKind {
    /// Two tasks modify the same area of code.
    SameCodeArea(String),
    /// Tasks have nearly identical titles/descriptions.
    Duplicate,
    /// Tasks have mutually exclusive goals.
    Contradictory(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConflict {
    pub task_a: u64,
    pub task_b: u64,
    pub kind: TaskConflictKind,
    pub description: String,
}

/// Detect conflicts in a set of tasks.
pub fn detect_task_conflicts(tasks: &[Task]) -> Vec<TaskConflict> {
    let mut conflicts = Vec::new();

    for i in 0..tasks.len() {
        for j in (i + 1)..tasks.len() {
            let a = &tasks[i];
            let b = &tasks[j];

            // Skip if either is done
            if a.status == "done" || b.status == "done" { continue; }

            // Duplicate detection: title similarity
            if title_similar(&a.title, &b.title) {
                conflicts.push(TaskConflict {
                    task_a: a.id, task_b: b.id,
                    kind: TaskConflictKind::Duplicate,
                    description: format!(
                        "Tasks {} and {} have very similar titles: '{}' vs '{}'",
                        a.id, b.id, a.title, b.title
                    ),
                });
                continue;
            }

            // Code area conflict: both mention the same module/file
            if let Some(area) = shared_code_area(a, b) {
                conflicts.push(TaskConflict {
                    task_a: a.id, task_b: b.id,
                    kind: TaskConflictKind::SameCodeArea(area.clone()),
                    description: format!(
                        "Tasks {} and {} both touch '{}' — coordinate carefully",
                        a.id, b.id, area
                    ),
                });
            }

            // Contradictory: one removes what the other adds
            if is_contradictory(a, b) {
                conflicts.push(TaskConflict {
                    task_a: a.id, task_b: b.id,
                    kind: TaskConflictKind::Contradictory("remove vs add".to_string()),
                    description: format!(
                        "Tasks {} and {} appear to have conflicting goals",
                        a.id, b.id
                    ),
                });
            }
        }
    }
    conflicts
}

fn title_similar(a: &str, b: &str) -> bool {
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if a_words.is_empty() || b_words.is_empty() { return false; }
    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    (intersection as f32 / union as f32) > 0.75
}

fn shared_code_area(a: &Task, b: &Task) -> Option<String> {
    let extract_modules = |t: &Task| -> Vec<String> {
        let text = format!("{} {} {}", t.title, t.description, t.details);
        text.split_whitespace()
            .filter(|w| w.contains("src/") || w.ends_with(".rs"))
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.').to_string())
            .filter(|w| !w.is_empty())
            .collect()
    };
    let ma: std::collections::HashSet<String> = extract_modules(a).into_iter().collect();
    let mb: std::collections::HashSet<String> = extract_modules(b).into_iter().collect();
    ma.intersection(&mb).next().cloned()
}

fn is_contradictory(a: &Task, b: &Task) -> bool {
    let a_text = format!("{} {}", a.title, a.description).to_lowercase();
    let b_text = format!("{} {}", b.title, b.description).to_lowercase();
    (a_text.contains("remove") || a_text.contains("delete") || a_text.contains("deprecate"))
        && (b_text.contains("add") || b_text.contains("implement") || b_text.contains("create"))
        && title_similar(&a.title, &b.title)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u64, title: &str, status: &str) -> Task {
        Task { id, title: title.to_string(), status: status.to_string(), ..Default::default() }
    }

    #[test]
    fn test_duplicate_detected() {
        let tasks = vec![
            t(1, "Add HOH Scheduler Engine Module", "pending"),
            t(2, "Add HOH Scheduler Engine Module", "pending"),
        ];
        let c = detect_task_conflicts(&tasks);
        assert!(!c.is_empty());
        assert!(matches!(c[0].kind, TaskConflictKind::Duplicate));
    }

    #[test]
    fn test_done_tasks_skipped() {
        let tasks = vec![
            t(1, "Add HOH Scheduler Engine Module", "done"),
            t(2, "Add HOH Scheduler Engine Module", "pending"),
        ];
        let c = detect_task_conflicts(&tasks);
        assert!(c.is_empty());
    }
}
