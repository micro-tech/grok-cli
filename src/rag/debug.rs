//! Debug and logging utilities for TGS-RAG.

use crate::rag::graph::GraphNode;

/// Log selected nodes for debugging / telemetry.
/// In a real implementation this would use the `tracing` or `log` crate.
pub fn log_selected_nodes(nodes: &[&GraphNode], query: &str) {
    eprintln!("[TGS-RAG] Query: {}", query);
    eprintln!("[TGS-RAG] Selected {} nodes:", nodes.len());
    for node in nodes.iter().take(10) {
        eprintln!(
            "  - {} ({}) — {}",
            node.path,
            node.kind.as_str(),
            node.name
        );
    }
    if nodes.len() > 10 {
        eprintln!("  ... and {} more", nodes.len() - 10);
    }
}
