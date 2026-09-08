//! HOH Task Mutation Rules (Task 327.7)
//!
//! Defines the safe, versioned mutation rules for evolving the task list.
//! This is the safety layer for 327.3 (Evolution Engine).
//!
//! Rules are intentionally conservative at first.

use crate::hoh::tasklist_adapter::Task;
use serde::{Deserialize, Serialize};

/// Version of the mutation rule set.
pub const MUTATION_RULE_VERSION: u32 = 1;

/// Represents a proposed change to a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskMutation {
    /// Change priority
    SetPriority {
        task_id: u64,
        new_priority: String,
    },
    /// Change status (with strict rules)
    SetStatus {
        task_id: u64,
        new_status: String,
    },
    /// Update title
    SetTitle {
        task_id: u64,
        new_title: String,
    },
    /// Add or replace description
    SetDescription {
        task_id: u64,
        new_description: String,
    },
    /// Add a new dependency (must not create cycle)
    AddDependency {
        task_id: u64,
        depends_on: u64,
    },
    /// Remove a dependency
    RemoveDependency {
        task_id: u64,
        depends_on: u64,
    },
    /// Split a task into multiple subtasks
    SplitTask {
        task_id: u64,
        new_subtasks: Vec<Task>,
    },
    /// Merge multiple tasks into one (advanced)
    MergeTasks {
        target_id: u64,
        source_ids: Vec<u64>,
        new_title: String,
    },
    /// Add a test strategy
    SetTestStrategy {
        task_id: u64,
        strategy: String,
    },
    /// Add a brand new top-level task (361.3 materialization)
    AddTask {
        new_task: Task,
    },
}

/// Result of applying a mutation.
#[derive(Debug, Clone)]
pub struct MutationResult {
    pub success: bool,
    pub message: String,
    pub task_id: u64,
}

/// The rule engine for safe mutations.
#[derive(Debug, Clone, Default)]
pub struct TaskMutationRules {
    pub version: u32,
}

impl TaskMutationRules {
    pub fn new() -> Self {
        Self {
            version: MUTATION_RULE_VERSION,
        }
    }

    /// Validate that a mutation is allowed and safe.
    /// This does NOT apply the mutation — only checks it.
    pub fn validate_mutation(
        &self,
        mutation: &TaskMutation,
        all_tasks: &[Task],
    ) -> Result<(), String> {
        match mutation {
            TaskMutation::SetPriority { new_priority, .. } => {
                if !["high", "medium", "low"].contains(&new_priority.as_str()) {
                    return Err(format!("Invalid priority: {}", new_priority));
                }
                Ok(())
            }

            TaskMutation::SetStatus { new_status, .. } => {
                let allowed = ["pending", "in_progress", "done", "cancelled", "deferred", "blocked"];
                if !allowed.contains(&new_status.as_str()) {
                    return Err(format!("Invalid status: {}", new_status));
                }
                Ok(())
            }

            TaskMutation::SetTitle { new_title, .. } => {
                if new_title.trim().is_empty() {
                    return Err("Title cannot be empty".to_string());
                }
                if new_title.len() > 200 {
                    return Err("Title too long (max 200 chars)".to_string());
                }
                Ok(())
            }

            TaskMutation::AddDependency { task_id, depends_on } => {
                if task_id == depends_on {
                    return Err("A task cannot depend on itself".to_string());
                }

                // Prevent cycles (basic check)
                if self.would_create_cycle(*task_id, *depends_on, all_tasks) {
                    return Err(format!(
                        "Adding dependency {} → {} would create a cycle",
                        task_id, depends_on
                    ));
                }
                Ok(())
            }

            TaskMutation::SplitTask { task_id, new_subtasks } => {
                if new_subtasks.is_empty() {
                    return Err("Split must produce at least one subtask".to_string());
                }
                for sub in new_subtasks {
                    if sub.id == *task_id {
                        return Err("Subtask cannot have same ID as parent".to_string());
                    }
                    if sub.title.trim().is_empty() {
                        return Err("Subtask title cannot be empty".to_string());
                    }
                }
                Ok(())
            }

            TaskMutation::AddTask { new_task } => {
                if new_task.title.trim().is_empty() {
                    return Err("New task title cannot be empty".to_string());
                }
                if new_task.id == 0 {
                    return Err("New task must have a non-zero ID".to_string());
                }
                // Ensure no duplicate ID
                if all_tasks.iter().any(|t| t.id == new_task.id) {
                    return Err(format!("Task ID {} already exists", new_task.id));
                }
                Ok(())
            }

            TaskMutation::MergeTasks { source_ids, target_id, .. } => {
                if source_ids.is_empty() {
                    return Err("Must specify at least one source task to merge".to_string());
                }
                if source_ids.contains(target_id) {
                    return Err("Target cannot be one of the sources".to_string());
                }
                Ok(())
            }

            _ => Ok(()), // Other mutations are currently lenient
        }
    }

    /// Check if adding `depends_on` to `task_id` would create a cycle.
    fn would_create_cycle(&self, task_id: u64, depends_on: u64, all_tasks: &[Task]) -> bool {
        // Simple DFS from depends_on looking for task_id
        fn visits(tasks: &[Task], current: u64, target: u64, seen: &mut std::collections::HashSet<u64>) -> bool {
            if current == target {
                return true;
            }
            if !seen.insert(current) {
                return false;
            }
            if let Some(task) = find_task(current, tasks) {
                for &dep in &task.dependencies {
                    if visits(tasks, dep, target, seen) {
                        return true;
                    }
                }
            }
            false
        }

        let mut seen = std::collections::HashSet::new();
        visits(all_tasks, depends_on, task_id, &mut seen)
    }

    /// Apply a validated mutation to a mutable list of tasks.
    /// Returns the affected task ID and a message.
    pub fn apply_mutation(
        &self,
        mutation: &TaskMutation,
        tasks: &mut Vec<Task>,
    ) -> Result<MutationResult, String> {
        // First validate
        self.validate_mutation(mutation, tasks)?;

        match mutation {
            TaskMutation::SetPriority { task_id, new_priority } => {
                if let Some(task) = find_task_mut(*task_id, tasks) {
                    task.priority = new_priority.clone();
                    Ok(MutationResult {
                        success: true,
                        message: format!("Priority set to {}", new_priority),
                        task_id: *task_id,
                    })
                } else {
                    Err(format!("Task {} not found", task_id))
                }
            }

            TaskMutation::SetStatus { task_id, new_status } => {
                if let Some(task) = find_task_mut(*task_id, tasks) {
                    task.status = new_status.clone();
                    Ok(MutationResult {
                        success: true,
                        message: format!("Status set to {}", new_status),
                        task_id: *task_id,
                    })
                } else {
                    Err(format!("Task {} not found", task_id))
                }
            }

            TaskMutation::SetTitle { task_id, new_title } => {
                if let Some(task) = find_task_mut(*task_id, tasks) {
                    task.title = new_title.clone();
                    Ok(MutationResult {
                        success: true,
                        message: "Title updated".to_string(),
                        task_id: *task_id,
                    })
                } else {
                    Err(format!("Task {} not found", task_id))
                }
            }

            TaskMutation::AddDependency { task_id, depends_on } => {
                if let Some(task) = find_task_mut(*task_id, tasks) {
                    if !task.dependencies.contains(depends_on) {
                        task.dependencies.push(*depends_on);
                    }
                    Ok(MutationResult {
                        success: true,
                        message: format!("Added dependency on {}", depends_on),
                        task_id: *task_id,
                    })
                } else {
                    Err(format!("Task {} not found", task_id))
                }
            }

            TaskMutation::SplitTask { task_id, new_subtasks } => {
                if let Some(task) = find_task_mut(*task_id, tasks) {
                    // Append new subtasks
                    for sub in new_subtasks {
                        task.subtasks.push(sub.clone());
                    }
                    Ok(MutationResult {
                        success: true,
                        message: format!("Split into {} subtasks", new_subtasks.len()),
                        task_id: *task_id,
                    })
                } else {
                    Err(format!("Task {} not found", task_id))
                }
            }

            TaskMutation::AddTask { new_task } => {
                // Append the new top-level task
                tasks.push(new_task.clone());
                Ok(MutationResult {
                    success: true,
                    message: format!("Added new task {}: {}", new_task.id, new_task.title),
                    task_id: new_task.id,
                })
            }

            _ => Err("Mutation type not yet implemented for apply".to_string()),
        }
    }
}

fn find_task_mut<'a>(id: u64, tasks: &'a mut [Task]) -> Option<&'a mut Task> {
    for t in tasks {
        if t.id == id {
            return Some(t);
        }
        if let Some(found) = find_task_mut(id, &mut t.subtasks) {
            return Some(found);
        }
    }
    None
}

fn find_task<'a>(id: u64, tasks: &'a [Task]) -> Option<&'a Task> {
    for t in tasks {
        if t.id == id {
            return Some(t);
        }
        if let Some(found) = find_task(id, &t.subtasks) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task(id: u64) -> Task {
        Task {
            id,
            title: format!("Task {}", id),
            ..Default::default()
        }
    }

    #[test]
    fn test_priority_mutation() {
        let rules = TaskMutationRules::new();
        let mut tasks = vec![sample_task(42)];

        let mutation = TaskMutation::SetPriority {
            task_id: 42,
            new_priority: "high".to_string(),
        };

        assert!(rules.validate_mutation(&mutation, &tasks).is_ok());
        let result = rules.apply_mutation(&mutation, &mut tasks).unwrap();
        assert!(result.success);
        assert_eq!(tasks[0].priority, "high");
    }

    #[test]
    fn test_cycle_prevention() {
        let rules = TaskMutationRules::new();
        let mut tasks = vec![
            Task { id: 1, dependencies: vec![2], ..Default::default() },
            Task { id: 2, dependencies: vec![], ..Default::default() },
        ];

        let bad = TaskMutation::AddDependency {
            task_id: 2,
            depends_on: 1,
        };

        assert!(rules.validate_mutation(&bad, &tasks).is_err());
    }
}
