# TGS-RAG — Text-Graph Synergy Retrieval Engine

TGS-RAG is grok-cli's semantic project graph engine. It builds a rich graph of Rust entities (modules, structs, enums, traits, functions, impls) and uses hybrid retrieval (BM25 + embeddings) + graph expansion to deliver highly relevant context to the LLM.

## Key Components

- **GraphNode / GraphEdge** — Core semantic entity model
- **GraphBuilder** — Incremental parser using tree-sitter + syn
- **Persistence** — JSON save/load with mtime-based incremental refresh
- **HybridRetriever** — BM25 + vector retrieval over graph nodes
- **TgsRagContextProvider** — Main integration point for ACP sessions

## Usage from ACP

```rust
use grok_cli::rag::{create_rag_provider_for_session, build_rag_context, TgsRagConfig};

let provider = create_rag_provider_for_session(Some(&graph_dir), config);
if let Some(p) = provider {
    let context = build_rag_context(&p, user_query);
    // inject into system prompt
}
```

## Configuration

See `TgsRagConfig` for available options (`enabled`, `max_retrieved_nodes`, `auto_load_graph`, etc.).

## Status

Most core functionality (112.5–112.17) is implemented with persistence, incremental updates, ACP integration, logging, and basic tests.
