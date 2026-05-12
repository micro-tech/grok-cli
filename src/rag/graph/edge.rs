//! Graph edge definitions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Kind of relationship between two nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    Contains,      // module contains item
    Imports,       // use / extern crate
    Calls,         // function call
    Implements,    // impl Trait for Type
    Inherits,      // struct field type, enum variant, supertrait
    References,    // any other reference (type use, etc.)
}

/// An edge in the semantic entity graph.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub kind: EdgeKind,
    pub weight: f32,
    pub location: Option<(std::path::PathBuf, usize)>,
}
