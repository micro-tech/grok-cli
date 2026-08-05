use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use crate::tools::{ToolContext, execute_tool};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub action: ToolCall,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// Boxed async result type used by [`TaskGraph::execute`].
/// Now returns a map of node_id -> tool output string on success (Task 237 parallel support).
type ExecuteFuture<'a> =
    Pin<Box<dyn Future<Output = Result<HashMap<String, String>>> + Send + 'a>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskGraph {
    pub nodes: HashMap<String, TaskNode>,
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: TaskNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Execute the graph in topological order.
    /// Independent nodes (no shared dependencies) are executed **concurrently**
    /// using tokio tasks for true parallelism (Task 237).
    ///
    /// Returns a map of node_id -> tool output for successful nodes.
    pub fn execute(&self, context: &ToolContext) -> ExecuteFuture<'_> {
        let nodes = self.nodes.clone();
        let context = context.clone();
        Box::pin(async move {
            // Topological sort first (for order + cycle detection)
            let mut sorted = Vec::new();
            let mut visited = HashSet::new();
            let mut visiting = HashSet::new();

            for id in nodes.keys() {
                if !visited.contains(id) {
                    Self::topo_sort_static(id, &nodes, &mut visited, &mut visiting, &mut sorted)
                        .map_err(|e| anyhow!("Topo sort error: {}", e))?;
                }
            }

            // Build dependency graph for parallel scheduling (Kahn's algorithm style with ready queue)
            let mut in_degree: HashMap<String, usize> = HashMap::new();
            let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

            for (id, node) in &nodes {
                in_degree.insert(id.clone(), node.dependencies.len());
                for dep in &node.dependencies {
                    dependents.entry(dep.clone()).or_default().push(id.clone());
                }
            }

            // Ready queue: nodes with no pending dependencies
            let mut ready: Vec<String> = in_degree
                .iter()
                .filter(|&(_, &deg)| deg == 0)
                .map(|(id, _)| id.clone())
                .collect();

            let mut completed = HashSet::new();
            let mut results: HashMap<String, String> = HashMap::new();
            let mut join_set = tokio::task::JoinSet::new();

            while !ready.is_empty() || !join_set.is_empty() {
                // Launch all currently ready independent nodes in parallel
                for id in ready.drain(..) {
                    if completed.contains(&id) {
                        continue;
                    }
                    let node = nodes[&id].clone();
                    let ctx = context.clone();
                    let id_clone = id.clone();

                    join_set.spawn(async move {
                        let res = execute_tool(&node.action.tool_name, &node.action.arguments, &ctx).await;
                        (id_clone, res)
                    });
                }

                // Wait for the next completion
                if let Some(result) = join_set.join_next().await {
                    match result {
                        Ok((id, Ok(output))) => {
                            results.insert(id.clone(), output);
                            completed.insert(id.clone());

                            // Unblock dependents
                            if let Some(deps) = dependents.get(&id) {
                                for dep_id in deps {
                                    if let Some(deg) = in_degree.get_mut(dep_id) {
                                        *deg = deg.saturating_sub(1);
                                        if *deg == 0 {
                                            ready.push(dep_id.clone());
                                        }
                                    }
                                }
                            }
                        }
                        Ok((id, Err(e))) => {
                            completed.insert(id);
                            // Propagate the first error (strict for validation workflows)
                            return Err(e);
                        }
                        Err(e) => {
                            return Err(anyhow!("Task join error: {}", e));
                        }
                    }
                }
            }

            // Drain any remaining
            while let Some(result) = join_set.join_next().await {
                if let Ok((id, Ok(output))) = result {
                    results.insert(id.clone(), output);
                    completed.insert(id);
                } else if let Ok((_, Err(e))) = result {
                    return Err(e);
                }
            }

            Ok(results)
        })
    }

    fn topo_sort_static(
        id: &str,
        nodes: &HashMap<String, TaskNode>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        sorted: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        if visiting.contains(id) {
            return Err(anyhow!("Cycle detected in task graph"));
        }
        if visited.contains(id) {
            return Ok(());
        }

        visiting.insert(id.to_string());
        if let Some(node) = nodes.get(id) {
            for dep in &node.dependencies {
                Self::topo_sort_static(dep, nodes, visited, visiting, sorted)?;
            }
        }
        visiting.remove(id);
        visited.insert(id.to_string());
        sorted.push(id.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_task_graph_creation() {
        let mut graph = TaskGraph::new();
        let node = TaskNode {
            id: "test".to_string(),
            action: ToolCall {
                tool_name: "read_file".to_string(),
                arguments: json!({"path": "test.txt"}),
            },
            dependencies: vec![],
        };
        graph.add_node(node);
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = TaskGraph::new();
        let node1 = TaskNode {
            id: "1".to_string(),
            action: ToolCall {
                tool_name: "read_file".to_string(),
                arguments: json!({"path": "input.txt"}),
            },
            dependencies: vec![],
        };
        let node2 = TaskNode {
            id: "2".to_string(),
            action: ToolCall {
                tool_name: "write_file".to_string(),
                arguments: json!({"path": "output.txt", "content": "data"}),
            },
            dependencies: vec!["1".to_string()],
        };
        graph.add_node(node1);
        graph.add_node(node2);

        // Test that topo sort works (without executing)
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        for id in graph.nodes.keys() {
            if !visited.contains(id) {
                TaskGraph::topo_sort_static(
                    id,
                    &graph.nodes,
                    &mut visited,
                    &mut visiting,
                    &mut sorted,
                )
                .unwrap();
            }
        }
        assert_eq!(sorted, vec!["1", "2"]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = TaskGraph::new();
        let node1 = TaskNode {
            id: "1".to_string(),
            action: ToolCall {
                tool_name: "read_file".to_string(),
                arguments: json!({"path": "input.txt"}),
            },
            dependencies: vec!["2".to_string()],
        };
        let node2 = TaskNode {
            id: "2".to_string(),
            action: ToolCall {
                tool_name: "write_file".to_string(),
                arguments: json!({"path": "output.txt", "content": "data"}),
            },
            dependencies: vec!["1".to_string()],
        };
        graph.add_node(node1);
        graph.add_node(node2);

        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        let result = TaskGraph::topo_sort_static(
            "1",
            &graph.nodes,
            &mut visited,
            &mut visiting,
            &mut sorted,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle detected"));
    }
}
