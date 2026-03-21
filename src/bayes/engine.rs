use std::collections::HashMap;

use crate::bayes::belief_graph::BeliefGraph;
use crate::bayes::likelihoods::{
    likelihood_from_model_confidence, likelihood_from_text, likelihood_from_tool_failure,
};
use crate::bayes::priors::default_priors;
use crate::bayes::profile::{load_profile, save_profile};
use crate::bayes::updater::bayes_update;

#[derive(Debug, Clone)]
pub struct BayesianEngine {
    priors: HashMap<String, f32>,
    graph: BeliefGraph,
}

impl BayesianEngine {
    pub fn new() -> Self {
        let priors = load_profile().unwrap_or_else(default_priors);
        let mut graph = BeliefGraph::new();
        for (k, v) in &priors {
            graph.set(k, *v);
        }
        Self { priors, graph }
    }

    pub fn update_profile(&mut self, executed_intent: &str) {
        // Find the intent that corresponds to the executed action
        let intent_key = match executed_intent {
            "replace" | "write_file" => "intent_edit",
            "run_shell_command" => "intent_shell",
            "web_search" | "web_fetch" => "intent_search",
            _ => "intent_question",
        };

        // Slightly nudge the prior for the chosen intent
        if let Some(prior) = self.priors.get_mut(intent_key) {
            *prior *= 1.1; // 10% boost to the baseline prior
        }

        // Renormalize the baseline priors
        let total: f32 = self.priors.values().sum();
        if total > f32::EPSILON {
            for value in self.priors.values_mut() {
                *value /= total;
            }
        }

        // Sync graph and save to disk
        self.sync_graph();
        let _ = save_profile(&self.priors);
    }

    pub fn update_from_text(&mut self, text: &str) {
        let likelihoods = likelihood_from_text(text);
        bayes_update(&mut self.priors, &likelihoods);
        self.sync_graph();
    }

    pub fn update_from_model_confidence(&mut self, score: f32) {
        let likelihoods = likelihood_from_model_confidence(score);
        bayes_update(&mut self.priors, &likelihoods);
        self.sync_graph();
    }

    pub fn update_from_tool_failure(&mut self) {
        let likelihoods = likelihood_from_tool_failure();
        bayes_update(&mut self.priors, &likelihoods);
        self.sync_graph();
    }

    fn sync_graph(&mut self) {
        for (k, v) in &self.priors {
            self.graph.set(k, *v);
        }
        self.graph.normalize();
    }

    pub fn probability(&self, key: &str) -> f32 {
        self.graph.get(key)
    }

    pub fn best_intent(&self) -> Option<String> {
        self.graph.best_key("intent_")
    }

    pub fn visualize(&self) -> String {
        self.graph.visualize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initialization() {
        let engine = BayesianEngine::new();
        // The default priors set intent_question to 0.3 and others to 0.2
        // After normalization: 0.3 / 1.4 ~ 0.214
        assert!(engine.probability("intent_question") > 0.0);
        assert_eq!(engine.best_intent(), Some("intent_question".to_string()));
    }

    #[test]
    fn test_engine_update_from_text() {
        let mut engine = BayesianEngine::new();

        // Before update, intent_question is highest
        assert_eq!(engine.best_intent(), Some("intent_question".to_string()));

        // Update with text suggesting an edit
        engine.update_from_text("can you edit the config file");

        // After update, intent_edit should become the highest probability
        assert_eq!(engine.best_intent(), Some("intent_edit".to_string()));
    }
}
