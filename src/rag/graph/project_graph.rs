//! Project-wide semantic entity graph.

use super::{GraphEdge, GraphNode, NodeId};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ProjectGraph {
    pub nodes: HashMap<NodeId, GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub file_index: HashMap<PathBuf, Vec<NodeId>>,
    pub name_index: HashMap<String, Vec<NodeId>>,
}

impl ProjectGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_node(&mut self, node: GraphNode) {
        let id = node.id;
        let file = node.file_path.clone();
        let name = node.name.clone();

        self.nodes.insert(id, node);
        self.file_index.entry(file).or_default().push(id);
        self.name_index.entry(name).or_default().push(id);
    }

    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    pub fn nodes_for_file(&self, path: &PathBuf) -> Vec<&GraphNode> {
        self.file_index
            .get(path)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }
}
