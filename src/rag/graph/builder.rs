//! Incremental graph builder.
//!
//! Combines tree-sitter scanning + syn extraction to build and update
//! the semantic entity graph for a project.

use crate::rag::graph::{GraphNode, NodeKind, ProjectGraph};
use crate::rag::parser::{syn_extractor::SynExtractor, tree_sitter_scanner::TreeSitterScanner};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct GraphBuilder {
    scanner: TreeSitterScanner,
    extractor: SynExtractor,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            scanner: TreeSitterScanner::new(),
            extractor: SynExtractor::new(),
        }
    }

    /// Build a full graph by walking a directory.
    pub fn build_from_dir(&mut self, root: &Path) -> ProjectGraph {
        let mut graph = ProjectGraph::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let path = entry.path().to_path_buf();
            if let Ok(source) = std::fs::read_to_string(&path) {
                self.add_file_to_graph(&mut graph, &path, &source);
            }
        }

        graph
    }

    /// Incrementally add or update a single file.
    pub fn add_file_to_graph(&mut self, graph: &mut ProjectGraph, path: &PathBuf, source: &str) {
        // Use tree-sitter to get rough boundaries
        if let Some(tree) = self.scanner.parse_file(source) {
            let ranges = self.scanner.extract_item_ranges(&tree, source);

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
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
