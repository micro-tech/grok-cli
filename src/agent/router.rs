use crate::bayes::BayesianEngine;
use crate::config::BayesianConfig;
use serde_json::Value;

pub enum RouterAction {
    UseSkill(String),
    UseTool(String),
    AskClarification(String),
    NormalChat,
}

pub struct Router {
    bayes: BayesianEngine,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Create a router using compiled-in defaults.
    pub fn new() -> Self {
        Self {
            bayes: BayesianEngine::new(),
        }
    }

    /// Create a router that uses the compiled-in default priors.
    ///
    /// Unlike [`new`], this never reads the on-disk saved profile, making it
    /// deterministic for unit tests.
    pub fn new_with_default_priors() -> Self {
        Self {
            bayes: BayesianEngine::new_with_default_priors(),
        }
    }

    /// Create a router whose engine is initialised from `[bayesian]` config.
    ///
    /// This applies configured priors, thresholds, likelihood weights, and
    /// the profile learning rate to the underlying [`BayesianEngine`].
    pub fn new_with_config(config: &BayesianConfig) -> Self {
        Self {
            bayes: BayesianEngine::new_with_config(config),
        }
    }

    pub async fn route(&mut self, user_input: &str) -> RouterAction {
        // 1) Update beliefs from the user's text.
        self.bayes.update_from_text(user_input);

        // 2) Clarification gate — threshold comes from the engine's config.
        if self.bayes.needs_clarification() {
            // Decay the clarification probability so the gate doesn't stay
            // stuck in a loop after firing once.
            self.bayes.update_from_text("reset_clarification");
            return RouterAction::AskClarification(
                "I want to make sure I do this safely. Could you clarify exactly what you want me to do?".to_string(),
            );
        }

        // 3) choose intent
        let intent = self
            .bayes
            .best_intent()
            .unwrap_or_else(|| "intent_question".to_string());

        match intent.as_str() {
            "intent_edit" => RouterAction::UseTool("replace".to_string()),
            "intent_shell" => RouterAction::UseTool("run_shell_command".to_string()),
            "intent_search" => RouterAction::UseTool("web_search".to_string()),
            _ => RouterAction::NormalChat,
        }
    }

    pub fn visualize_beliefs(&self) -> String {
        self.bayes.visualize()
    }

    /// Return the clarification threshold currently used by this router.
    pub fn clarification_threshold(&self) -> f32 {
        self.bayes.clarification_threshold()
    }

    pub fn learn_from_tool(&mut self, tool_name: &str) {
        self.bayes.update_profile(tool_name);
    }

    /// Return the uncertainty threshold used for the low-confidence check.
    pub fn uncertainty_threshold(&self) -> f32 {
        self.bayes.uncertainty_threshold()
    }

    pub fn get_contextual_tools(&self, all_tools: Vec<Value>) -> Vec<Value> {
        let best = self
            .bayes
            .best_intent()
            .unwrap_or_else(|| "intent_question".to_string());

        // Base tools that are almost always safe/useful
        let mut keep = vec!["read_file", "list_directory", "glob_search"];

        match best.as_str() {
            "intent_edit" => {
                keep.extend(vec!["write_file", "replace", "run_shell_command"]);
            }
            "intent_shell" => {
                keep.extend(vec!["run_shell_command", "replace", "write_file"]);
            }
            "intent_search" => {
                keep.extend(vec!["web_search", "web_fetch"]);
            }
            _ => {
                // Return all tools for general intent
                return all_tools;
            }
        }

        all_tools
            .into_iter()
            .filter(|t| {
                if let Some(name) = t
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                {
                    return keep.contains(&name);
                }
                true
            })
            .collect()
    }

    pub fn get_adaptive_system_prompt(&self) -> Option<String> {
        let best = self.bayes.best_intent()?;

        match best.as_str() {
            "intent_edit" => Some("System Persona: You are an expert Software Engineer. Focus on writing clean, idiomatic code and precise file replacements. Double-check indentation and scope bounds.".to_string()),
            "intent_shell" => Some("System Persona: You are a Senior DevOps Engineer. Focus on writing safe, cross-platform, one-line shell commands. Prefer silent/quiet flags to minimize output noise.".to_string()),
            "intent_search" => Some("System Persona: You are an exhaustive Research Assistant. Focus on finding authoritative documentation, extracting citations, and synthesizing the best answer without making assumptions.".to_string()),
            _ => None,
        }
    }

    /// Returns `true` when `P(low_confidence)` exceeds the configured
    /// uncertainty threshold (replaces the former hardcoded `> 0.5` check).
    pub fn is_low_confidence(&self) -> bool {
        self.bayes.is_low_confidence()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_edit_intent() {
        let mut router = Router::new_with_default_priors();
        let action = router.route("can you fix this bug and refactor").await;

        assert!(matches!(action, RouterAction::UseTool(tool) if tool == "replace"));
    }

    #[tokio::test]
    async fn test_router_shell_intent() {

        let mut router = Router::new_with_default_priors();
        let action = router.route("run the build command").await;

        assert!(matches!(action, RouterAction::UseTool(tool) if tool == "run_shell_command"));
    }

    #[tokio::test]
    async fn test_router_clarification_gate() {

        let mut router = Router::new_with_default_priors();
        let action = router.route("be careful, don't delete").await;

        assert!(matches!(action, RouterAction::AskClarification(_)));

        // Second time should let it pass because we decayed the clarification probability
        let action2 = router.route("I am sure, please proceed").await;
        assert!(!matches!(action2, RouterAction::AskClarification(_)));
    }

    #[tokio::test]
    async fn test_router_normal_chat() {

        let mut router = Router::new_with_default_priors();
        // Just a random statement
        let action = router.route("hello there").await;
        assert!(matches!(action, RouterAction::NormalChat));
    }
}
