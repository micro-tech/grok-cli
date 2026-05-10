//! TGS-RAG configuration.

#[derive(Debug, Clone)]
pub struct TgsRagConfig {
    pub enabled: bool,
    pub max_retrieved_nodes: usize,
    pub max_context_tokens: usize,
}

impl Default for TgsRagConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retrieved_nodes: 20,
            max_context_tokens: 8000,
        }
    }
}
