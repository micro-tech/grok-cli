//! TGS-RAG — Text-Graph Synergy Retrieval Engine
//!
//! Semantic entity graph + hybrid retrieval for project-aware context.

pub mod acp_integration;
pub mod api;
pub mod compression;
pub mod config;
pub mod debug;
pub mod dna_integration;
pub mod graph;
pub mod index;
pub mod parser;
pub mod persistence;
pub mod retrieval;
#[cfg(test)]
pub mod tests;

pub use acp_integration::{build_rag_context, create_rag_provider_for_session};
pub use api::{TgsRag, TgsRagContextProvider};
pub use compression::compress_context;
pub use config::TgsRagConfig;
pub use graph::{
    EdgeKind, GraphBuilder, GraphEdge, GraphNode, NodeId, NodeKind, ProjectGraph, Visibility,
};
pub use index::bm25::Bm25Index;
pub use persistence::{graph_exists, graph_path, load_graph, save_graph};
pub use retrieval::{graph_expansion::expand_with_neighbors, hybrid::HybridRetriever};
