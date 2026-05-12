//! Context Compression 2.0 — retrieval-aware pruning and summarization.

use crate::rag::graph::GraphNode;

/// Given a list of retrieved nodes and a token budget, return the most important subset.
pub fn compress_context<'a>(nodes: &'a [&'a GraphNode], max_tokens: usize) -> Vec<&'a GraphNode> {
    // Very naive implementation: just take the first N nodes
    // Real version would use graph connectivity + importance scoring
    let mut result = nodes.to_vec();
    result.truncate(max_tokens / 50); // rough estimate
    result
}
