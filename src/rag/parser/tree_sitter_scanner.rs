//! Tree-sitter based scanner for file and function boundary detection.
//!
//! This module provides fast, incremental parsing to identify code regions
//! (functions, structs, modules, etc.) without full semantic extraction.

use std::path::Path;
use tree_sitter::{Parser, Tree};

/// Scans a Rust source file and returns structural boundaries.
pub struct TreeSitterScanner {
    parser: Parser,
}

impl TreeSitterScanner {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("Failed to load tree-sitter-rust grammar");
        Self { parser }
    }

    /// Parse a file and return the syntax tree.
    pub fn parse_file(&mut self, source: &str) -> Option<Tree> {
        self.parser.parse(source, None)
    }

    /// Extract top-level item ranges (functions, structs, enums, traits, impls).
    pub fn extract_item_ranges(&self, tree: &Tree, source: &str) -> Vec<(String, usize, usize)> {
        let mut items = Vec::new();
        let root = tree.root_node();

        for i in 0..root.child_count() {
            if let Some(child) = root.child(i as u32) {
                match child.kind() {
                    "function_item" | "struct_item" | "enum_item" | "trait_item" | "impl_item" => {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            let name = name_node.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
                            let start = child.start_byte();
                            let end = child.end_byte();
                            items.push((name, start, end));
                        }
                    }
                    _ => {}
                }
            }
        }
        items
    }
}

impl Default for TreeSitterScanner {
    fn default() -> Self {
        Self::new()
    }
}
