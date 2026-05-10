//! Simple reranker (combines lexical + graph connectivity score).

use crate::rag::graph::GraphNode;

pub fn rerank(results: &mut [(&GraphNode, f32)]) {
    // Boost nodes that appear in many files or have doc comments
    for (node, score) in results.iter_mut() {
        if node.doc_comment.is_some() {
            *score *= 1.2;
        }
        if node.visibility == crate::rag::graph::Visibility::Public {
            *score *= 1.1;
        }
    }
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
}
