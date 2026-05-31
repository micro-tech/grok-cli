//! TGS-RAG configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TgsRagConfig {
    pub enabled: bool,
    pub max_retrieved_nodes: usize,
    pub max_context_tokens: usize,
    /// Whether to automatically load a persisted graph on startup.
    pub auto_load_graph: bool,
    /// Directory where the project graph is persisted.
    pub graph_dir: Option<String>,
}

impl Default for TgsRagConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retrieved_nodes: 20,
            max_context_tokens: 8000,
            auto_load_graph: true,
            graph_dir: None,
        }
    }
}
