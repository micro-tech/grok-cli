//! TGS-RAG public API.

use crate::rag::graph::{GraphNode, ProjectGraph};
use crate::rag::retrieval::hybrid::HybridRetriever;
use crate::rag::retrieval::graph_expansion::expand_with_neighbors;
use crate::rag::retrieval::reranker::rerank;
use crate::rag::compression::compress_context;
use crate::rag::config::TgsRagConfig;

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
