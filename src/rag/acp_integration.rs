//! ACP integration layer for TGS-RAG.
//!
//! This module provides the bridge between the TGS-RAG engine and
//! the ACP session context assembly pipeline.

use crate::rag::api::TgsRagContextProvider;
use crate::rag::config::TgsRagConfig;
use crate::rag::graph::ProjectGraph;
use std::path::Path;

/// High-level integration point for ACP sessions.
/// ACP code can call this to obtain relevant project context.
pub fn build_rag_context(
    provider: &TgsRagContextProvider,
    query: &str,
) -> Vec<String> {
    if !provider.is_enabled() {
        return vec![];
    }
    provider.get_context_for_query(query)
}

/// Convenience constructor used by ACP when starting a session.
/// Tries to load a persisted graph from the given directory.
pub fn create_rag_provider_for_session(
    graph_dir: Option<&Path>,
    config: TgsRagConfig,
) -> Option<TgsRagContextProvider> {
    if !config.enabled {
        return None;
    }

    if let Some(dir) = graph_dir {
        TgsRagContextProvider::from_persisted(dir, config)
    } else {
        None
    }
}
