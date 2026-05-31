//! Graph persistence layer for TGS-RAG.
//!
//! Provides save/load functionality for the semantic entity graph
//! and basic support for incremental updates.

use crate::rag::graph::ProjectGraph;
use std::fs;
use std::path::{Path, PathBuf};

const GRAPH_FILENAME: &str = "project_graph.json";

#[derive(Debug)]
pub enum PersistenceError {
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceError::Io(e) => write!(f, "IO error: {}", e),
            PersistenceError::Serde(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for PersistenceError {}

/// Save the project graph to disk as JSON.
pub fn save_graph(graph: &ProjectGraph, dir: &Path) -> Result<(), PersistenceError> {
    fs::create_dir_all(dir).map_err(PersistenceError::Io)?;
    let path = dir.join(GRAPH_FILENAME);
    let json = serde_json::to_string_pretty(graph).map_err(PersistenceError::Serde)?;
    fs::write(path, json).map_err(PersistenceError::Io)?;
    Ok(())
}

/// Load a project graph from disk.
pub fn load_graph(dir: &Path) -> Result<ProjectGraph, PersistenceError> {
    let path = dir.join(GRAPH_FILENAME);
    let json = fs::read_to_string(path).map_err(PersistenceError::Io)?;
    let graph: ProjectGraph = serde_json::from_str(&json).map_err(PersistenceError::Serde)?;
    Ok(graph)
}

/// Check if a persisted graph exists.
pub fn graph_exists(dir: &Path) -> bool {
    dir.join(GRAPH_FILENAME).exists()
}

/// Get the path to the persisted graph file.
pub fn graph_path(dir: &Path) -> PathBuf {
    dir.join(GRAPH_FILENAME)
}
