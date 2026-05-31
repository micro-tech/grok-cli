//! Incremental graph builder.
//!
//! Combines tree-sitter scanning + syn extraction to build and update
//! the semantic entity graph for a project.

use crate::rag::graph::{GraphNode, NodeKind, ProjectGraph};
use crate::rag::parser::{syn_extractor::SynExtractor, tree_sitter_scanner::TreeSitterScanner};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

pub struct GraphBuilder {
    scanner: TreeSitterScanner,
    extractor: SynExtractor,
    /// Tracks last known modification time per file for incremental updates.
    file_mtimes: HashMap<PathBuf, SystemTime>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            scanner: TreeSitterScanner::new(),
            extractor: SynExtractor::new(),
            file_mtimes: HashMap::new(),
        }
    }

    /// Build a full graph by walking a directory.
    pub fn build_from_dir(&mut self, root: &Path) -> ProjectGraph {
        let mut graph = ProjectGraph::new();
        self.file_mtimes.clear();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let path = entry.path().to_path_buf();
            if let Ok(source) = std::fs::read_to_string(&path) {
                if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(mtime) = meta.modified() {
                        self.file_mtimes.insert(path.clone(), mtime);
                    }
                }
                self.add_file_to_graph(&mut graph, &path, &source);
            }
        }

        graph
    }

    /// Incrementally add or update a single file.
    pub fn add_file_to_graph(&mut self, graph: &mut ProjectGraph, path: &PathBuf, source: &str) {
        // Remove existing nodes for this file before re-adding
        if let Some(ids) = graph.file_index.get(path).cloned() {
            for id in ids {
                graph.nodes.remove(&id);
            }
            graph.file_index.remove(path);
        }

        // Use tree-sitter to get rough boundaries
        if let Some(tree) = self.scanner.parse_file(source) {
            let _ranges = self.scanner.extract_item_ranges(&tree, source);

            // Use syn for deeper semantic info
            let entities = self.extractor.extract_entities(source);

            for (name, kind_str) in entities {
                let kind = match kind_str.as_str() {
                    "struct" => NodeKind::Struct,
                    "enum" => NodeKind::Enum,
                    "trait" => NodeKind::Trait,
                    "function" => NodeKind::Function,
                    "impl" => NodeKind::ImplBlock,
                    _ => continue,
                };

                let node = GraphNode::new(
                    kind,
                    name.clone(),
                    format!("{}::{}", path.display(), name),
                    path.clone(),
                );

                graph.insert_node(node);
            }
        }

        // Update mtime
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                self.file_mtimes.insert(path.clone(), mtime);
            }
        }
    }

    /// Refresh only files that have changed since the last build.
    /// Returns the number of files that were updated.
    pub fn refresh_changed_files(&mut self, graph: &mut ProjectGraph, root: &Path) -> usize {
        let mut updated = 0;

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let path = entry.path().to_path_buf();

            let current_mtime = match std::fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let is_stale = self
                .file_mtimes
                .get(&path)
                .map_or(true, |&last| current_mtime > last);

            if is_stale {
                if let Ok(source) = std::fs::read_to_string(&path) {
                    self.add_file_to_graph(graph, &path, &source);
                    updated += 1;
                }
            }
        }

        updated
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
