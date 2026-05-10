# TGS-RAG Architecture — Text-Graph Synergy Retrieval Engine

**Epic:** Task 112  
**Status:** Design Phase (112.1)  
**Date:** 2026-05-10

## 1. Overview

TGS-RAG (Text-Graph Synergy Retrieval Engine) is a hybrid retrieval system that combines:

- A **semantic entity graph** of the project (structs, enums, traits, impls, functions, modules, relationships)
- **Embeddings** + **BM25** hybrid search
- Graph-aware retrieval + reranking
- Retrieval-aware context compression (Context Compression 2.0)

The goal is to move from linear text-based context to **graph-informed, minimal, high-relevance context** for every LLM request.

## 2. Core Principles

- **Hybrid Parsing**: tree-sitter (fast, incremental, language-agnostic boundaries) + syn (deep Rust semantic extraction)
- **Graph First**: Every entity is a node; relationships are first-class edges
- **Retrieval is Graph-Aware**: Return not just the best match, but relevant neighbors, parents, children, and call sites
- **Retrieval-Aware Compression**: When context must be pruned, prefer keeping graph-connected, high-relevance chunks
- **Incremental & Persistent**: Graph can be updated incrementally and persisted to disk

## 3. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      User Query / ACP Prompt                │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    TGS-RAG Retrieval Pipeline               │
│  ┌────────────┐   ┌────────────┐   ┌─────────────────────┐  │
│  │ BM25 Index │──▶│ Vector     │──▶│ Graph-Expanded      │  │
│  │ (text)     │   │ Search     │   │ Retrieval           │  │
│  └────────────┘   └────────────┘   └─────────────────────┘  │
│                                │                            │
│                                ▼                            │
│                    ┌─────────────────────┐                  │
│                    │ Reranker + Scorer   │                  │
│                    └─────────────────────┘                  │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│              Context Assembly + Compression 2.0             │
│  (retrieval-aware pruning + summarization)                  │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    LLM Prompt (ACP / CLI)                   │
└─────────────────────────────────────────────────────────────┘
```

## 4. Data Model

### 4.1 GraphNode

```rust
pub enum NodeKind {
    Module,
    Struct,
    Enum,
    Trait,
    ImplBlock,
    Function,
    Constant,
    TypeAlias,
    Macro,
}

pub struct GraphNode {
    pub id: NodeId,                    // stable UUID or path+name hash
    pub kind: NodeKind,
    pub name: String,
    pub path: String,                  // "crate::module::Type"
    pub file_path: PathBuf,
    pub span: (usize, usize),          // byte offsets or line/col
    pub doc_comment: Option<String>,
    pub signature: Option<String>,     // for functions/traits
    pub visibility: Visibility,
    pub attributes: Vec<String>,
    pub embedding: Option<Vec<f32>>,   // cached embedding vector
}
```

### 4.2 GraphEdge

```rust
pub enum EdgeKind {
    Contains,           // module → item
    Imports,            // use statement
    Calls,              // function call
    Implements,         // impl Trait for Type
    Inherits,           // struct field types, enum variants
    References,         // any other reference
}

pub struct GraphEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
    pub weight: f32,       // optional confidence / frequency
    pub location: Option<(PathBuf, usize)>,
}
```

### 4.3 ProjectGraph

```rust
pub struct ProjectGraph {
    pub nodes: HashMap<NodeId, GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub file_index: HashMap<PathBuf, Vec<NodeId>>,   // fast file → nodes
    pub name_index: HashMap<String, Vec<NodeId>>,    // fast name lookup
}
```

## 5. Retrieval Pipeline

1. **Hybrid Search**
   - BM25 over node signatures + doc comments
   - Vector similarity over node embeddings
   - Merge + deduplicate results

2. **Graph Expansion**
   - For each top hit, pull:
     - Parent module
     - Sibling items
     - Callers / callees (for functions)
     - Impl blocks (for types)

3. **Reranking**
   - Combine lexical score + vector score + graph centrality / connectivity score

4. **Context Compression 2.0**
   - When token budget is tight, prefer:
     - High-relevance nodes
     - Graph-connected clusters
     - Drop low-relevance or isolated nodes first

## 6. Module Structure (Proposed)

```
src/rag/
├── mod.rs
├── graph/
│   ├── node.rs
│   ├── edge.rs
│   ├── project_graph.rs
│   └── builder.rs
├── parser/
│   ├── tree_sitter_scanner.rs
│   └── syn_extractor.rs
├── index/
│   ├── bm25.rs
│   ├── embeddings.rs
│   └── hybrid.rs
├── retrieval/
│   ├── retriever.rs
│   ├── graph_expansion.rs
│   └── reranker.rs
├── compression.rs
└── config.rs
```

## 7. Integration Points

- **ACP Session**: `handle_chat_completion` calls `TgsRag::retrieve_context(query)` before final prompt assembly
- **Context Compressor**: Uses retrieval scores instead of pure recency
- **Session DNA**: Can bias retrieval (e.g., prefer certain modules)
- **Knowledge Loader**: Graph can reference `knowledge/` files as external nodes

## 8. Next Steps (112.2+)

- Add `tree-sitter-rust` + `syn` dependencies
- Implement scanner + extractor
- Define `GraphNode` / `GraphEdge` types
- Build incremental graph builder with persistence

---

**Status**: Design complete. Ready to move to implementation (112.2).
