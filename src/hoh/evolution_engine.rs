//! HOH Task Evolution Engine (Task 327.3)
//!
//! Uses the dependency graph + mutation rules to intelligently evolve the task list.
//! This is the "brain" that proposes and applies safe improvements to tasks over time.

use crate::hoh::state::HOHError;
use crate::hoh::task_dependency_graph::TaskDependencyGraph;
use crate::hoh::task_mutation::{MutationResult, TaskMutation, TaskMutationRules};
use crate::hoh::tasklist_adapter::{Task, TaskList, TaskListAdapter};

/// The main evolution engine.
#[derive(Debug)]
pub struct TaskEvolutionEngine {
    adapter: TaskListAdapter,
    rules: TaskMutationRules,
}

impl TaskEvolutionEngine {
    pub fn new(adapter: TaskListAdapter) -> Self {
        Self {
            adapter,
            rules: TaskMutationRules::new(),
        }
    }

    /// Analyze the current task list and propose a set of safe mutations.
    pub async fn propose_evolutions(&self) -> Result<Vec<TaskMutation>, HOHError> {
        let list = self.adapter.load().await?;
        let _graph = self.adapter.build_dependency_graph().await?;
        let mut proposals = Vec::new();

        // 1. Promote high-value pending tasks that have good test strategies
        for task in &list.tasks {
            if task.status == "pending" && task.priority != "high" {
                if !task.test_strategy.is_empty() && task.test_strategy.len() > 30 {
                    proposals.push(TaskMutation::SetPriority {
                        task_id: task.id,
                        new_priority: "high".to_string(),
                    });
                }
            }
        }

        // 2. Split overly large tasks (heuristic)
        for task in &list.tasks {
            if task.details.len() > 2000 && task.subtasks.is_empty() {
                // Propose splitting (we generate simple child tasks)
                let sub1 = Task {
                    id: task.id * 1000 + 1,
                    title: format!("Part 1: {}", task.title),
                    details: "Auto-generated subtask (evolution)".to_string(),
                    ..Default::default()
                };
                let sub2 = Task {
                    id: task.id * 1000 + 2,
                    title: format!("Part 2: {}", task.title),
                    details: "Auto-generated subtask (evolution)".to_string(),
                    ..Default::default()
                };

                proposals.push(TaskMutation::SplitTask {
                    task_id: task.id,
                    new_subtasks: vec![sub1, sub2],
                });
            }
        }

        // 3. Add missing dependencies for tasks that mention other task IDs in details
        for task in &list.tasks {
            if task.dependencies.is_empty() {
                if let Some(dep_id) = self.extract_mentioned_task_id(&task.details, &list) {
                    if dep_id != task.id {
                        proposals.push(TaskMutation::AddDependency {
                            task_id: task.id,
                            depends_on: dep_id,
                        });
                    }
                }
            }
        }

        // 4. Defer or cancel very old low-priority tasks (placeholder logic)
        for task in &list.tasks {
            if task.priority == "low" && task.status == "pending" {
                // Could add more sophisticated staleness detection later
                if task.title.to_lowercase().contains("deprecated") {
                    proposals.push(TaskMutation::SetStatus {
                        task_id: task.id,
                        new_status: "cancelled".to_string(),
                    });
                }
            }
        }

        // Filter proposals through the rule engine
        let mut valid_proposals = Vec::new();
        for proposal in proposals {
            if self.rules.validate_mutation(&proposal, &list.tasks).is_ok() {
                valid_proposals.push(proposal);
            }
        }

        Ok(valid_proposals)
    }

    /// Apply a list of mutations (after validation).
    pub async fn apply_mutations(&self, mutations: &[TaskMutation]) -> Result<Vec<MutationResult>, HOHError> {
        let mut list = self.adapter.load().await?;
        let mut results = Vec::new();

        for mutation in mutations {
            match self.rules.apply_mutation(mutation, &mut list.tasks) {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    tracing::warn!("Mutation failed: {}", e);
                    results.push(MutationResult {
                        success: false,
                        message: e,
                        task_id: 0,
                    });
                }
            }
        }

        // Save the evolved task list
        self.adapter.save(&list).await?;
        Ok(results)
    }

    /// Full evolution cycle: propose + optionally apply.
    pub async fn run_evolution_cycle(
        &self,
        auto_apply: bool,
    ) -> Result<(Vec<TaskMutation>, Vec<MutationResult>), HOHError> {
        let proposals = self.propose_evolutions().await?;

        if auto_apply && !proposals.is_empty() {
            let results = self.apply_mutations(&proposals).await?;
            Ok((proposals, results))
        } else {
            Ok((proposals, vec![]))
        }
    }

    /// Simple heuristic to extract a task ID mentioned in text (e.g. "see task 123")
    fn extract_mentioned_task_id(&self, text: &str, list: &TaskList) -> Option<u64> {
        let text_lower = text.to_lowercase();

        for task in &list.tasks {
            let id_str = task.id.to_string();
            if text_lower.contains(&format!("task {}", id_str))
                || text_lower.contains(&format!("#{}", id_str))
                || text_lower.contains(&format!("t{}", id_str))
            {
                return Some(task.id);
            }
        }
        None
    }

    /// Get the current dependency graph (convenience)
    pub async fn get_graph(&self) -> Result<TaskDependencyGraph, HOHError> {
        self.adapter.build_dependency_graph().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_propose_and_apply() {
        let dir = tempdir().unwrap();
        let adapter = TaskListAdapter::new(dir.path().to_path_buf(), true);

        // Seed a simple list
        let mut list = TaskList::default();
        list.tasks.push(Task {
            id: 100,
            title: "Huge task with lots of details".to_string(),
            details: "x".repeat(2100),
            priority: "medium".to_string(),
            status: "pending".to_string(),
            ..Default::default()
        });

        adapter.save(&list).await.unwrap();

        let engine = TaskEvolutionEngine::new(adapter);
        let (proposals, _) = engine.run_evolution_cycle(false).await.unwrap();

        assert!(!proposals.is_empty());
        // Should have proposed a split
        assert!(proposals.iter().any(|p| matches!(p, TaskMutation::SplitTask { .. })));
    }
}
