use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use crate::tools::{ToolContext, execute_tool};
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
type ExecuteFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + 'a>>;

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

    pub fn execute(&self, context: &ToolContext) -> ExecuteFuture<'_> {
        let nodes = self.nodes.clone();
        let context = context.clone();
        Box::pin(async move {
            // Topological sort
            let mut sorted = Vec::new();
            let mut visited = HashSet::new();
            let mut visiting = HashSet::new();

            for id in nodes.keys() {
                if !visited.contains(id) {
                    Self::topo_sort_static(id, &nodes, &mut visited, &mut visiting, &mut sorted)?;
                }
            }

            // Execute in topological order
            for id in sorted {
                let node = &nodes[&id];
                // Execute the tool
                execute_tool(&node.action.tool_name, &node.action.arguments, &context).await?;
                // Results are handled by the tool execution itself
            }

            Ok(())
        })
    }

    fn topo_sort_static(
        id: &str,
        nodes: &HashMap<String, TaskNode>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        sorted: &mut Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if visiting.contains(id) {
            return Err("Cycle detected in task graph".into());
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
