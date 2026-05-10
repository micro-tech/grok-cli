//! Syn-based semantic entity extractor.
//!
//! Extracts deep semantic information (structs, enums, traits, impls, functions)
//! using the syn crate for accurate Rust AST analysis.

use syn::{parse_file, File, Item};

/// Extracts semantic entities from Rust source code.
pub struct SynExtractor;

impl SynExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Parse source and return a list of top-level item names and kinds.
    pub fn extract_entities(&self, source: &str) -> Vec<(String, String)> {
        let ast: File = match parse_file(source) {
            Ok(f) => f,
            Err(_) => return vec![],
        };

        let mut entities = Vec::new();

        for item in ast.items {
            match item {
                Item::Struct(s) => {
                    entities.push((s.ident.to_string(), "struct".to_string()));
                }
                Item::Enum(e) => {
                    entities.push((e.ident.to_string(), "enum".to_string()));
                }
                Item::Trait(t) => {
                    entities.push((t.ident.to_string(), "trait".to_string()));
                }
                Item::Fn(f) => {
                    entities.push((f.sig.ident.to_string(), "function".to_string()));
                }
                Item::Impl(imp) => {
                    if let Some((_, path, _)) = &imp.trait_ {
                        entities.push((path.segments.last().unwrap().ident.to_string(), "impl".to_string()));
                    } else if let syn::Type::Path(p) = &*imp.self_ty {
                        if let Some(seg) = p.path.segments.last() {
                            entities.push((seg.ident.to_string(), "impl".to_string()));
                        }
                    }
                }
                _ => {}
            }
        }

        entities
    }
}

impl Default for SynExtractor {
    fn default() -> Self {
        Self::new()
    }
}
