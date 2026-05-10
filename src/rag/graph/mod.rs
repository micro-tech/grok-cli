//! Semantic entity graph module.

pub mod node;
pub mod edge;
pub mod project_graph;
pub mod builder;

pub use node::{GraphNode, NodeId, NodeKind, Visibility};
pub use edge::{GraphEdge, EdgeKind};
pub use project_graph::ProjectGraph;
pub use builder::GraphBuilder;
