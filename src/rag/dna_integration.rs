//! Integration between TGS-RAG and Session DNA.
//!
//! Allows Session DNA settings (tone, verbosity, risk tolerance) to
//! influence how TGS-RAG retrieves and compresses context.

use crate::rag::api::TgsRagContextProvider;
use crate::rag::config::TgsRagConfig;

/// Adjusts TGS-RAG config based on Session DNA preferences.
/// This is a lightweight integration point.
pub fn apply_dna_preferences(
    base_config: TgsRagConfig,
    verbosity: Option<&str>,
) -> TgsRagConfig {
    let mut config = base_config;

    if let Some(v) = verbosity {
        match v.to_lowercase().as_str() {
            "concise" | "brief" => {
                config.max_retrieved_nodes = (config.max_retrieved_nodes / 2).max(5);
                config.max_context_tokens = (config.max_context_tokens as f32 * 0.6) as usize;
            }
            "detailed" | "verbose" => {
                config.max_retrieved_nodes = (config.max_retrieved_nodes as f32 * 1.5) as usize;
                config.max_context_tokens = (config.max_context_tokens as f32 * 1.3) as usize;
            }
            _ => {}
        }
    }

    config
}

/// Convenience helper that creates a provider while respecting DNA settings.
pub fn create_provider_with_dna(
    graph: crate::rag::graph::ProjectGraph,
    base_config: TgsRagConfig,
    verbosity: Option<&str>,
) -> TgsRagContextProvider {
    let adjusted = apply_dna_preferences(base_config, verbosity);
    TgsRagContextProvider::new(graph, adjusted)
}
