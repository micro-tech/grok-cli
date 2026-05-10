//! TGS-RAG — Text-Graph Synergy Retrieval Engine
//!
//! Semantic entity graph + hybrid retrieval for project-aware context.

pub mod graph;
pub mod parser;
pub mod index;
pub mod retrieval;
pub mod compression;
pub mod config;
pub mod api;
#[cfg(test)]
pub mod tests;

pub use graph::{GraphNode, GraphEdge, ProjectGraph, NodeId, NodeKind, EdgeKind, GraphBuilder, Visibility};
pub use index::bm25::Bm25Index;
pub use retrieval::{hybrid::HybridRetriever, graph_expansion::expand_with_neighbors};
pub use compression::compress_context;
pub use config::TgsRagConfig;
pub use api::TgsRag;
