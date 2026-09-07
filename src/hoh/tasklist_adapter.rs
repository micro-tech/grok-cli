//! HOH TaskList Adapter (Task 327.1)
//!
//! Safe read/write + validation layer for .zed/task_list.json from the HOH outer loop.
//! This is the foundation for TaskList Intelligence (327.x).
//!
//! Responsibilities:
//! - Load / Save with atomic writes + backups
//! - Strong consistency validation (327.33)
//! - Diff generation
//! - Selection / prioritization hooks for planner
//! - Simulation mode (no writes)

use crate::hoh::state::HOHError;
use crate::hoh::task_dependency_graph::TaskDependencyGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Serializable representation of a task (mirrors the JSON schema used by task_tools).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub dependencies: Vec<u64>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default)]
    pub details: String,
    #[serde(rename = "testStrategy", default)]
    pub test_strategy: String,
    #[serde(default)]
    pub subtasks: Vec<Task>,
}

fn default_status() -> String {
    "pending".to_string()
}
fn default_priority() -> String {
    "medium".to_string()
}

/// The full task list document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskList {
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
pub struct TaskListAdapter {
    pub task_file: PathBuf,
    pub simulation_mode: bool,
}

impl TaskListAdapter {
    pub fn new(base_dir: impl AsRef<Path>, simulation_mode: bool) -> Self {
        let task_file = base_dir.as_ref().join(".zed").join("task_list.json");
        Self {
            task_file,
            simulation_mode,
        }
    }

    /// Load the current task list.
    pub async fn load(&self) -> Result<TaskList, HOHError> {
        if !self.task_file.exists() {
            return Ok(TaskList { tasks: vec![] });
        }

        let content = tokio::fs::read_to_string(&self.task_file)
            .await
            .map_err(|e| HOHError::Other(format!("Failed to read task_list.json: {}", e)))?;

        // Support both {"tasks": [...]} and plain array for robustness
        let list: TaskList = if content.trim().starts_with('[') {
            let tasks: Vec<Task> = serde_json::from_str(&content)
                .map_err(|e| HOHError::Other(format!("Failed to parse task array: {}", e)))?;
            TaskList { tasks }
        } else {
            serde_json::from_str(&content)
                .map_err(|e| HOHError::Other(format!("Failed to parse task_list.json: {}", e)))?
        };

        Ok(list)
    }

    /// Save with validation, backup, and atomic write.
    pub async fn save(&self, list: &TaskList) -> Result<(), HOHError> {
        if self.simulation_mode {
            tracing::info!("HOH TaskListAdapter: simulation mode — skipping write");
            return Ok(());
        }

        self.validate(list)?;

        // Ensure .zed directory exists
        if let Some(parent) = self.task_file.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| HOHError::Other(format!("Failed to create .zed dir: {}", e)))?;
        }

        // Create backup
        if self.task_file.exists() {
            let bak = self.task_file.with_file_name("task_list.json.hoh.bak");
            let _ = tokio::fs::copy(&self.task_file, &bak).await;
        }

        let json = serde_json::to_string_pretty(list)
            .map_err(|e| HOHError::Other(format!("Failed to serialize task list: {}", e)))?;

        // Atomic write via .tmp
        let tmp = self.task_file.with_file_name("task_list.json.tmp");
        tokio::fs::write(&tmp, &json)
            .await
            .map_err(|e| HOHError::Other(format!("Failed to write temp task list: {}", e)))?;

        tokio::fs::rename(&tmp, &self.task_file)
            .await
            .map_err(|e| HOHError::Other(format!("Failed to finalize task list: {}", e)))?;

        tracing::info!(path = %self.task_file.display(), "HOH wrote task_list.json");
        Ok(())
    }

    /// Run full consistency validation (327.33).
    pub fn validate(&self, list: &TaskList) -> Result<(), HOHError> {
        let problems = self.check_consistency(list);
        if !problems.is_empty() {
            return Err(HOHError::Other(format!(
                "Task list failed consistency checks (327.33):\n{}",
                problems.join("\n")
            )));
        }
        Ok(())
    }

    /// Detailed consistency checker. Empty vec = healthy.
    pub fn check_consistency(&self, list: &TaskList) -> Vec<String> {
        let mut problems = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 1. Duplicate IDs (including subtasks)
        for task in &list.tasks {
            self.check_duplicates(task, &mut seen, &mut problems);
        }

        // 2. Circular dependencies
        for task in &list.tasks {
            if self.has_cycle(task.id, &list.tasks, &mut vec![]) {
                problems.push(format!("Circular dependency detected involving task {}", task.id));
            }
        }

        // 3. Done tasks depending on non-done work
        for task in &list.tasks {
            if task.status == "done" {
                for &dep in &task.dependencies {
                    if let Some(d) = self.find_task(dep, &list.tasks) {
                        if d.status != "done" {
                            problems.push(format!(
                                "Task {} is 'done' but depends on unfinished task {}",
                                task.id, dep
                            ));
                        }
                    }
                }
            }
        }

        // 4. Basic quality
        for task in &list.tasks {
            if task.title.trim().is_empty() {
                problems.push(format!("Task {} has empty title", task.id));
            }
            if task.status.is_empty() {
                problems.push(format!("Task {} has empty status", task.id));
            }
        }

        problems
    }

    fn check_duplicates(&self, task: &Task, seen: &mut std::collections::HashSet<u64>, problems: &mut Vec<String>) {
        if !seen.insert(task.id) {
            problems.push(format!("Duplicate task ID found: {}", task.id));
        }
        for sub in &task.subtasks {
            self.check_duplicates(sub, seen, problems);
        }
    }

    fn has_cycle(&self, id: u64, all: &[Task], path: &mut Vec<u64>) -> bool {
        if path.contains(&id) {
            return true;
        }
        path.push(id);

        if let Some(task) = self.find_task(id, all) {
            for &dep in &task.dependencies {
                if self.has_cycle(dep, all, path) {
                    return true;
                }
            }
        }

        path.pop();
        false
    }

    fn find_task<'a>(&self, id: u64, tasks: &'a [Task]) -> Option<&'a Task> {
        for t in tasks {
            if t.id == id {
                return Some(t);
            }
            if let Some(found) = self.find_task(id, &t.subtasks) {
                return Some(found);
            }
        }
        None
    }

    /// Human-readable diff between two snapshots (327.14).
    /// Produces a structured, human + machine friendly diff.
    pub fn diff(&self, before: &TaskList, after: &TaskList) -> String {
        use std::collections::{HashMap, HashSet};

        let before_map: HashMap<u64, &Task> = before
            .tasks
            .iter()
            .flat_map(|t| std::iter::once((t.id, t)).chain(t.subtasks.iter().map(|s| (s.id, s))))
            .collect();

        let after_map: HashMap<u64, &Task> = after
            .tasks
            .iter()
            .flat_map(|t| std::iter::once((t.id, t)).chain(t.subtasks.iter().map(|s| (s.id, s))))
            .collect();

        let before_ids: HashSet<_> = before_map.keys().copied().collect();
        let after_ids: HashSet<_> = after_map.keys().copied().collect();

        let mut lines = Vec::new();

        // Added tasks
        for id in after_ids.difference(&before_ids) {
            if let Some(task) = after_map.get(id) {
                lines.push(format!(
                    "+ Task {} added: \"{}\" [status={}, prio={}]",
                    id, task.title, task.status, task.priority
                ));
            }
        }

        // Removed tasks
        for id in before_ids.difference(&after_ids) {
            lines.push(format!("- Task {} removed", id));
        }

        // Changed tasks
        for id in before_ids.intersection(&after_ids) {
            let b = before_map.get(id).unwrap();
            let a = after_map.get(id).unwrap();

            let mut changes = vec![];

            if b.title != a.title {
                changes.push(format!("title: \"{}\" → \"{}\"", b.title, a.title));
            }
            if b.status != a.status {
                changes.push(format!("status: {} → {}", b.status, a.status));
            }
            if b.priority != a.priority {
                changes.push(format!("priority: {} → {}", b.priority, a.priority));
            }
            if b.details != a.details {
                changes.push("details changed".to_string());
            }
            if b.test_strategy != a.test_strategy {
                changes.push("test_strategy changed".to_string());
            }
            if b.dependencies != a.dependencies {
                changes.push(format!(
                    "dependencies: {:?} → {:?}",
                    b.dependencies, a.dependencies
                ));
            }
            if b.subtasks.len() != a.subtasks.len() {
                changes.push(format!(
                    "subtasks: {} → {}",
                    b.subtasks.len(),
                    a.subtasks.len()
                ));
            }

            if !changes.is_empty() {
                lines.push(format!("~ Task {} changed: {}", id, changes.join(", ")));
            }
        }

        if lines.is_empty() {
            "No changes detected.".to_string()
        } else {
            lines.join("\n")
        }
    }

    /// Return all non-done tasks (used by planner).
    pub async fn get_pending_tasks(&self) -> Result<Vec<Task>, HOHError> {
        let list = self.load().await?;
        Ok(list
            .tasks
            .into_iter()
            .filter(|t| t.status != "done" && t.status != "cancelled" && t.status != "deferred")
            .collect())
    }

    /// Build a dependency graph for the current task list (327.4).
    pub async fn build_dependency_graph(&self) -> Result<TaskDependencyGraph, HOHError> {
        let list = self.load().await?;
        Ok(TaskDependencyGraph::from_tasks(&list.tasks))
    }

    /// Get tasks that are ready to run (all dependencies satisfied).
    pub async fn get_ready_tasks(&self, completed: &HashSet<u64>) -> Result<Vec<Task>, HOHError> {
        let graph = self.build_dependency_graph().await?;
        let list = self.load().await?;

        let ready_ids: HashSet<_> = graph.get_ready_tasks(completed).into_iter().collect();

        let ready_tasks: Vec<Task> = list
            .tasks
            .into_iter()
            .filter(|t| ready_ids.contains(&t.id) && !completed.contains(&t.id))
            .collect();

        Ok(ready_tasks)
    }

    /// Get topological order of all tasks (respecting dependencies).
    pub async fn get_topological_order(&self) -> Result<Vec<u64>, HOHError> {
        let graph = self.build_dependency_graph().await?;
        graph
            .topological_sort()
            .map_err(|e| HOHError::Other(format!("Topological sort failed: {}", e)))
    }
}