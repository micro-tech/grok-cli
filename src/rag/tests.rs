//! Unit tests for TGS-RAG core components.

#[cfg(test)]
mod tests {
    use crate::rag::graph::{GraphNode, NodeKind, ProjectGraph};
    use crate::rag::index::bm25::Bm25Index;
    use crate::rag::parser::syn_extractor::SynExtractor;
    use crate::rag::retrieval::hybrid::HybridRetriever;
    use std::path::PathBuf;

    #[test]
    fn test_graph_node_creation() {
        let node = GraphNode::new(
            NodeKind::Struct,
            "User",
            "models::user::User",
            PathBuf::from("src/models/user.rs"),
        );
        assert_eq!(node.name, "User");
        assert_eq!(node.kind, NodeKind::Struct);
    }

    #[test]
    fn test_syn_extractor() {
        let source = r#"
            pub struct User { name: String }
            pub enum Role { Admin, User }
            pub fn create_user() {}
        "#;

        let extractor = SynExtractor::new();
        let entities = extractor.extract_entities(source);

        assert!(entities.iter().any(|(n, k)| n == "User" && k == "struct"));
        assert!(entities.iter().any(|(n, k)| n == "Role" && k == "enum"));
        assert!(entities.iter().any(|(n, k)| n == "create_user" && k == "function"));
    }

    #[test]
    fn test_bm25_index() {
        let mut index = Bm25Index::new();
        index.add_document("1".to_string(), "struct User with name field");
        index.add_document("2".to_string(), "enum Role admin user");

        let score = index.score("struct user", "1");
        assert!(score > 0.0);
    }

    #[test]
    fn test_project_graph_insert() {
        let mut graph = ProjectGraph::new();
        let node = GraphNode::new(
            NodeKind::Function,
            "main",
            "main",
            PathBuf::from("src/main.rs"),
        );
        graph.insert_node(node.clone());
        assert!(graph.nodes.contains_key(&node.id));
    }

    #[test]
    fn test_hybrid_retriever() {
        let mut graph = ProjectGraph::new();
        let node = GraphNode::new(
            NodeKind::Struct,
            "Config",
            "config::Config",
            PathBuf::from("src/config.rs"),
        );
        graph.insert_node(node);

        let retriever = HybridRetriever::new(graph);
        let results = retriever.retrieve("config struct", 5);
        assert!(!results.is_empty());
    }
}
