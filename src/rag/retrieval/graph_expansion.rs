//! Graph expansion: pull in related nodes around top retrieval hits.

use crate::rag::graph::{GraphNode, ProjectGraph};
use std::collections::HashSet;

pub fn expand_with_neighbors<'a>(
    graph: &'a ProjectGraph,
    seed_nodes: &'a [&'a GraphNode],
    max_neighbors: usize,
) -> Vec<&'a GraphNode> {
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for node in seed_nodes {
        if seen.insert(node.id) {
            results.push(*node);
        }

        // Find neighbors via edges (simplified: look for nodes in same file)
        if let Some(neighbors) = graph.file_index.get(&node.file_path) {
            for nid in neighbors.iter().take(max_neighbors) {
                if let Some(n) = graph.nodes.get(nid) {
                    if seen.insert(*nid) {
                        results.push(n);
                    }
                }
            }
        }
    }

    results
}
