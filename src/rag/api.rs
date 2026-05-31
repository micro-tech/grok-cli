//! TGS-RAG public API.

use crate::rag::graph::{GraphNode, ProjectGraph};
use crate::rag::retrieval::hybrid::HybridRetriever;
use crate::rag::retrieval::graph_expansion::expand_with_neighbors;
use crate::rag::retrieval::reranker::rerank;
use crate::rag::compression::compress_context;
use crate::rag::config::TgsRagConfig;

/// High-level TGS-RAG engine.
pub struct TgsRag {
    graph: ProjectGraph,
    config: TgsRagConfig,
}

impl TgsRag {
    pub fn new(graph: ProjectGraph, config: TgsRagConfig) -> Self {
        Self { graph, config }
    }

    pub fn retrieve_context(&self, query: &str) -> Vec<String> {
        if !self.config.enabled {
            return vec![];
        }

        let retriever = HybridRetriever::new(self.graph.clone());
        let mut results = retriever.retrieve(query, self.config.max_retrieved_nodes);

        rerank(&mut results);

        let seeds: Vec<&GraphNode> = results.iter().map(|(n, _)| *n).collect();
        let expanded = expand_with_neighbors(&self.graph, &seeds, 3);
        let compressed = compress_context(&expanded, self.config.max_context_tokens);

        compressed
            .into_iter()
            .map(|node| {
                format!(
                    "{} ({})\n{}",
                    node.path,
                    node.kind.as_str(),
                    node.doc_comment.as_deref().unwrap_or("")
                )
            })
            .collect()
    }
}

/// Context provider designed for ACP session integration.
/// This is the main entry point the ACP layer should use.
pub struct TgsRagContextProvider {
    rag: TgsRag,
}

impl TgsRagContextProvider {
    pub fn new(graph: ProjectGraph, config: TgsRagConfig) -> Self {
        Self {
            rag: TgsRag::new(graph, config),
        }
    }

    /// Create a provider by loading a persisted graph if available.
    pub fn from_persisted(dir: &std::path::Path, config: TgsRagConfig) -> Option<Self> {
        if crate::rag::persistence::graph_exists(dir) {
            if let Ok(graph) = crate::rag::persistence::load_graph(dir) {
                return Some(Self::new(graph, config));
            }
        }
        None
    }

    /// Retrieve relevant context for a user query.
    /// Returns formatted strings suitable for injection into the system prompt.
    pub fn get_context_for_query(&self, query: &str) -> Vec<String> {
        self.rag.retrieve_context(query)
    }

    /// Check if TGS-RAG is enabled.
    pub fn is_enabled(&self) -> bool {
        self.rag.config.enabled
    }
}
