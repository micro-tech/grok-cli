//! HOH Task Dependency Graph (Task 327.4)
//!
//! Builds and maintains a directed graph of task dependencies.
//! Supports:
//! - Topological sort (for scheduling)
//! - Cycle detection
//! - Ready-task computation (dependencies satisfied)
//! - Reverse lookup (dependents)
//!
//! Uses simple adjacency lists (no petgraph dependency for now).

use crate::hoh::tasklist_adapter::Task;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Default)]
pub struct TaskDependencyGraph {
    /// Forward edges: task_id -> list of dependencies (must be done before this task)
    pub edges: HashMap<u64, Vec<u64>>,
    /// Reverse edges: task_id -> list of tasks that depend on it (dependents)
    pub reverse_edges: HashMap<u64, Vec<u64>>,
    /// All known task IDs
    pub nodes: HashSet<u64>,
}

impl TaskDependencyGraph {
    /// Build a graph from a flat list of tasks (including subtasks recursively).
    pub fn from_tasks(tasks: &[Task]) -> Self {
        let mut graph = Self::default();
        graph.add_tasks_recursive(tasks);
        graph
    }

    fn add_tasks_recursive(&mut self, tasks: &[Task]) {
        for task in tasks {
            self.nodes.insert(task.id);

            // Record direct dependencies
            if !task.dependencies.is_empty() {
                self.edges.insert(task.id, task.dependencies.clone());

                // Build reverse edges
                for &dep in &task.dependencies {
                    self.reverse_edges
                        .entry(dep)
                        .or_default()
                        .push(task.id);
                }
            } else {
                self.edges.entry(task.id).or_default();
            }

            // Recurse into subtasks
            if !task.subtasks.is_empty() {
                self.add_tasks_recursive(&task.subtasks);
            }
        }
    }

    /// Returns true if there is a cycle in the graph.
    pub fn has_cycles(&self) -> bool {
        self.detect_cycles().is_some()
    }

    /// Returns the first cycle found (if any), as a list of task IDs.
    pub fn detect_cycles(&self) -> Option<Vec<u64>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for &node in &self.nodes {
            if !visited.contains(&node) {
                if let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut rec_stack, vec![]) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    fn dfs_cycle(
        &self,
        node: u64,
        visited: &mut HashSet<u64>,
        rec_stack: &mut HashSet<u64>,
        mut path: Vec<u64>,
    ) -> Option<Vec<u64>> {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        if let Some(deps) = self.edges.get(&node) {
            for &dep in deps {
                if !visited.contains(&dep) {
                    if let Some(cycle) = self.dfs_cycle(dep, visited, rec_stack, path.clone()) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&dep) {
                    // Found cycle - extract the cycle portion
                    if let Some(pos) = path.iter().position(|&x| x == dep) {
                        let mut cycle = path[pos..].to_vec();
                        cycle.push(dep); // close the cycle
                        return Some(cycle);
                    }
                    return Some(path);
                }
            }
        }

        rec_stack.remove(&node);
        path.pop();
        None
    }

    /// Performs a topological sort using Kahn's algorithm.
    /// Returns error if the graph has cycles.
    pub fn topological_sort(&self) -> Result<Vec<u64>, String> {
        if let Some(cycle) = self.detect_cycles() {
            return Err(format!("Cycle detected in task graph: {:?}", cycle));
        }

        let mut in_degree: HashMap<u64, usize> = self
            .nodes
            .iter()
            .map(|&id| (id, self.edges.get(&id).map_or(0, |d| d.len())))
            .collect();

        let mut queue: VecDeque<u64> = in_degree
            .iter()
            .filter_map(|(&id, &deg)| if deg == 0 { Some(id) } else { None })
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node);

            if let Some(dependents) = self.reverse_edges.get(&node) {
                for &dep in dependents {
                    if let Some(deg) = in_degree.get_mut(&dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err("Topological sort failed — graph may still contain a cycle".to_string());
        }

        Ok(result)
    }

    /// Returns tasks whose dependencies are all satisfied (i.e. "ready").
    pub fn get_ready_tasks(&self, completed: &HashSet<u64>) -> Vec<u64> {
        self.nodes
            .iter()
            .filter(|&&id| {
                if completed.contains(&id) {
                    return false;
                }
                self.edges
                    .get(&id)
                    .map_or(true, |deps| deps.iter().all(|d| completed.contains(d)))
            })
            .copied()
            .collect()
    }

    /// Returns direct dependencies of a task.
    pub fn get_dependencies(&self, id: u64) -> Vec<u64> {
        self.edges.get(&id).cloned().unwrap_or_default()
    }

    /// Returns tasks that directly depend on this task.
    pub fn get_dependents(&self, id: u64) -> Vec<u64> {
        self.reverse_edges.get(&id).cloned().unwrap_or_default()
    }

    /// Returns all ancestors (transitive dependencies) of a task.
    pub fn get_all_dependencies(&self, id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        let mut stack = vec![id];
        let mut seen = HashSet::new();

        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            if let Some(deps) = self.edges.get(&current) {
                for &dep in deps {
                    if !seen.contains(&dep) {
                        result.push(dep);
                        stack.push(dep);
                    }
                }
            }
        }
        result
    }

    /// Returns a simple string representation for debugging.
    pub fn to_debug_string(&self) -> String {
        let mut lines = vec!["Task Dependency Graph:".to_string()];
        for &node in &self.nodes {
            let deps = self.get_dependencies(node);
            let dep_str = if deps.is_empty() {
                "no deps".to_string()
            } else {
                deps.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", ")
            };
            lines.push(format!("  {} -> [{}]", node, dep_str));
        }
        lines.join("\n")
    }

    // ─────────────────────────────────────────────────────────────
    // 327.15: Impact Analysis (number of transitive dependents)
    // ─────────────────────────────────────────────────────────────

    /// 327.15: Compute impact score for a task.
    /// Impact = number of tasks that (transitively) depend on this one.
    /// Higher = more work unblocked by completing this task.
    pub fn compute_impact(&self, task_id: u64) -> usize {
        let mut visited = HashSet::new();
        let mut stack = vec![task_id];

        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            if let Some(dependents) = self.reverse_edges.get(&current) {
                for &d in dependents {
                    if !visited.contains(&d) {
                        stack.push(d);
                    }
                }
            }
        }

        // subtract self
        visited.len().saturating_sub(1)
    }

    /// Batch version: impact for all nodes.
    pub fn compute_all_impacts(&self) -> HashMap<u64, usize> {
        self.nodes
            .iter()
            .map(|&id| (id, self.compute_impact(id)))
            .collect()
    }

    // ─────────────────────────────────────────────────────────────
    // 327.16 + 327.25: Difficulty + Stalling heuristics (lightweight)
    // ─────────────────────────────────────────────────────────────

    /// Rough difficulty estimate (0 = trivial, higher = harder).
    /// Used by prioritization (327.16) and stalling detector (327.25).
    pub fn estimate_difficulty(&self, task: &Task) -> f32 {
        let mut score: f32 = 0.0;

        // Size signals
        if task.details.len() > 1200 { score += 2.0; }
        if task.details.len() > 2500 { score += 1.5; }

        // Definition quality
        if task.test_strategy.trim().is_empty() { score += 1.5; }
        if task.description.trim().len() < 30 { score += 1.0; }

        // Dependency fan-out / fan-in
        let deps = self.get_dependencies(task.id).len() as f32;
        let dependents = self.get_dependents(task.id).len() as f32;
        if deps >= 3.0 { score += 1.0; }
        if dependents >= 4.0 { score += 0.8; } // high impact but also coordination cost

        // Subtask complexity
        if task.subtasks.len() > 5 { score += 1.5; }

        score.min(8.0)
    }

    /// 327.15 helper: impact score for a specific task (wrapper around compute_impact).
    pub fn get_impact(&self, id: u64) -> usize {
        self.compute_impact(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::tasklist_adapter::Task;

    fn make_task(id: u64, deps: Vec<u64>) -> Task {
        Task {
            id,
            title: format!("Task {}", id),
            dependencies: deps,
            ..Default::default()
        }
    }

    #[test]
    fn test_simple_graph() {
        let tasks = vec![
            make_task(1, vec![]),
            make_task(2, vec![1]),
            make_task(3, vec![1, 2]),
        ];
        let graph = TaskDependencyGraph::from_tasks(&tasks);

        assert!(!graph.has_cycles());
        let topo = graph.topological_sort().unwrap();
        assert_eq!(topo, vec![1, 2, 3]);
    }

    #[test]
    fn test_cycle_detection() {
        let tasks = vec![
            make_task(1, vec![2]),
            make_task(2, vec![1]),
        ];
        let graph = TaskDependencyGraph::from_tasks(&tasks);
        assert!(graph.has_cycles());
        assert!(graph.detect_cycles().is_some());
    }

    #[test]
    fn test_ready_tasks() {
        let tasks = vec![
            make_task(1, vec![]),
            make_task(2, vec![1]),
            make_task(3, vec![1]),
            make_task(4, vec![2, 3]),
        ];
        let graph = TaskDependencyGraph::from_tasks(&tasks);

        let mut completed = HashSet::new();
        let ready = graph.get_ready_tasks(&completed);
        assert!(ready.contains(&1));

        completed.insert(1);
        let ready = graph.get_ready_tasks(&completed);
        assert!(ready.contains(&2) && ready.contains(&3));
        assert!(!ready.contains(&4));

        completed.insert(2);
        completed.insert(3);
        let ready = graph.get_ready_tasks(&completed);
        assert!(ready.contains(&4));
    }
}
