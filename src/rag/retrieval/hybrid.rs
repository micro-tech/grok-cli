//! Hybrid retrieval combining BM25 + vector similarity + graph expansion.

use crate::rag::graph::{GraphNode, ProjectGraph};
use crate::rag::index::bm25::Bm25Index;
use std::collections::HashMap;

pub struct HybridRetriever {
    bm25: Bm25Index,
    graph: ProjectGraph,
}

impl HybridRetriever {
    pub fn new(graph: ProjectGraph) -> Self {
        let mut bm25 = Bm25Index::new();

        // Index all nodes
        for (id, node) in &graph.nodes {
            let text = format!(
                "{} {} {}",
                node.name,
                node.path,
                node.doc_comment.as_deref().unwrap_or("")
            );
            bm25.add_document(id.to_string(), &text);
        }

        Self { bm25, graph }
    }

    /// Retrieve top-k relevant nodes for a query.
    pub fn retrieve(&self, query: &str, k: usize) -> Vec<(&GraphNode, f32)> {
        let mut scores: Vec<(&GraphNode, f32)> = self
            .graph
            .nodes
            .values()
            .map(|node| {
                let score = self.bm25.score(query, &node.id.to_string());
                (node, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }
}
