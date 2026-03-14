use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BeliefNode {
    pub key: String,
    pub probability: f32,
}

#[derive(Debug, Clone)]
pub struct BeliefGraph {
    pub nodes: HashMap<String, BeliefNode>,
}

impl BeliefGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &str, probability: f32) {
        self.nodes.insert(
            key.to_string(),
            BeliefNode {
                key: key.to_string(),
                probability,
            },
        );
    }

    pub fn get(&self, key: &str) -> f32 {
        self.nodes
            .get(key)
            .map(|node| node.probability)
            .unwrap_or(0.0)
    }

    pub fn normalize(&mut self) {
        let total: f32 = self.nodes.values().map(|n| n.probability).sum();
        if total <= f32::EPSILON {
            return;
        }
        for node in self.nodes.values_mut() {
            node.probability /= total;
        }
    }

    pub fn best_key(&self, prefix: &str) -> Option<String> {
        self.nodes
            .values()
            .filter(|n| n.key.starts_with(prefix))
            .max_by(|a, b| {
                a.probability
                    .partial_cmp(&b.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.key.clone())
    }

    pub fn visualize(&self) -> String {
        if self.nodes.is_empty() {
            return "No beliefs to display.".to_string();
        }

        let mut lines = vec![String::from("📊 Belief Graph:")];
        let mut sorted_nodes: Vec<_> = self.nodes.values().collect();
        // Sort descending by probability
        sorted_nodes.sort_by(|a, b| {
            b.probability
                .partial_cmp(&a.probability)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let max_key_len = sorted_nodes.iter().map(|n| n.key.len()).max().unwrap_or(0);

        for node in sorted_nodes {
            if node.probability < 0.01 {
                continue;
            } // Hide very low probabilities

            let bar_length = (node.probability * 20.0).round() as usize;
            let bar = "█".repeat(bar_length);
            let empty = " ".repeat(20 - bar_length);

            let percentage = node.probability * 100.0;
            lines.push(format!(
                "  {key:<width$} |{bar}{empty}| {pct:5.1}%",
                key = node.key,
                width = max_key_len,
                bar = bar,
                empty = empty,
                pct = percentage
            ));
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belief_graph_set_get() {
        let mut graph = BeliefGraph::new();
        graph.set("intent_edit", 0.5);
        assert_eq!(graph.get("intent_edit"), 0.5);
        assert_eq!(graph.get("unknown"), 0.0);
    }

    #[test]
    fn test_belief_graph_normalize() {
        let mut graph = BeliefGraph::new();
        graph.set("a", 1.0);
        graph.set("b", 3.0);
        graph.normalize();

        // Total was 4.0, so "a" -> 0.25, "b" -> 0.75
        assert!((graph.get("a") - 0.25).abs() < f32::EPSILON);
        assert!((graph.get("b") - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_belief_graph_best_key() {
        let mut graph = BeliefGraph::new();
        graph.set("intent_edit", 0.2);
        graph.set("intent_shell", 0.8);
        graph.set("meta_data", 0.9); // Highest, but wrong prefix

        assert_eq!(graph.best_key("intent_"), Some("intent_shell".to_string()));
    }
}
