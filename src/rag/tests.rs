//! Basic tests for TGS-RAG core functionality.

#[cfg(test)]
mod tests {
    use crate::rag::graph::{GraphNode, NodeKind, ProjectGraph};
    use crate::rag::persistence::{graph_exists, save_graph, load_graph};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_insert_and_retrieve_node() {
        let mut graph = ProjectGraph::new();
        let node = GraphNode::new(
            NodeKind::Struct,
            "TestStruct",
            "crate::test::TestStruct",
            PathBuf::from("src/test.rs"),
        );
        let id = node.id;
        graph.insert_node(node);

        assert!(graph.nodes.contains_key(&id));
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_file_index_updates() {
        let mut graph = ProjectGraph::new();
        let path = PathBuf::from("src/example.rs");
        let node = GraphNode::new(
            NodeKind::Function,
            "example_fn",
            "crate::example::example_fn",
            path.clone(),
        );
        graph.insert_node(node);

        assert!(graph.file_index.contains_key(&path));
        assert_eq!(graph.file_index[&path].len(), 1);
    }

    #[test]
    fn test_persistence_roundtrip() {
        let mut graph = ProjectGraph::new();
        let node = GraphNode::new(
            NodeKind::Enum,
            "TestEnum",
            "crate::test::TestEnum",
            PathBuf::from("src/test.rs"),
        );
        graph.insert_node(node);

        let dir = tempdir().unwrap();
        assert!(save_graph(&graph, dir.path()).is_ok());
        assert!(graph_exists(dir.path()));

        let loaded = load_graph(dir.path()).expect("Failed to load graph");
        assert_eq!(loaded.nodes.len(), 1);
    }

    #[test]
    fn test_context_provider_disabled() {
        use crate::rag::api::TgsRagContextProvider;
        use crate::rag::config::TgsRagConfig;

        let graph = ProjectGraph::new();
        let mut config = TgsRagConfig::default();
        config.enabled = false;

        let provider = TgsRagContextProvider::new(graph, config);
        assert!(!provider.is_enabled());
        assert!(provider.get_context_for_query("anything").is_empty());
    }
}
