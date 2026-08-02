//! Semantic entity graph module.

pub mod builder;
pub mod edge;
pub mod node;
pub mod project_graph;

pub use builder::GraphBuilder;
pub use edge::{EdgeKind, GraphEdge};
pub use node::{GraphNode, NodeId, NodeKind, Visibility};
pub use project_graph::ProjectGraph;
