//! Core graph node and edge definitions for the semantic entity graph.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Stable identifier for a graph node.
pub type NodeId = Uuid;

/// Kind of semantic entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    Field,
    Variant,
}

/// Visibility of the entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Crate,
    Super,
    Private,
}

/// A node in the semantic entity graph.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    pub path: String,                    // e.g. "crate::module::MyStruct"
    pub file_path: PathBuf,
    pub span: (usize, usize),            // byte offsets in file
    pub doc_comment: Option<String>,
    pub signature: Option<String>,
    pub visibility: Visibility,
    pub attributes: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

impl GraphNode {
    pub fn new(kind: NodeKind, name: impl Into<String>, path: impl Into<String>, file_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            name: name.into(),
            path: path.into(),
            file_path,
            span: (0, 0),
            doc_comment: None,
            signature: None,
            visibility: Visibility::Private,
            attributes: vec![],
            embedding: None,
        }
    }
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Module => "module",
            NodeKind::Struct => "struct",
            NodeKind::Enum => "enum",
            NodeKind::Trait => "trait",
            NodeKind::ImplBlock => "impl",
            NodeKind::Function => "function",
            NodeKind::Constant => "const",
            NodeKind::TypeAlias => "type",
            NodeKind::Macro => "macro",
            NodeKind::Field => "field",
            NodeKind::Variant => "variant",
        }
    }
}
