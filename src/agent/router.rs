use crate::bayes::BayesianEngine;
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

impl Router {
    pub fn new() -> Self {
        Self {
            bayes: BayesianEngine::new(),
        }
    }

    pub async fn route(&mut self, user_input: &str) -> RouterAction {
        // 1) update beliefs from text
        self.bayes.update_from_text(user_input);

        // 2) clarification gate
        let clarification_prob = self.bayes.probability("need_clarification");
        if clarification_prob > 0.4 {
            // we decay the clarification need so it doesn't stay stuck forever
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
            "intent_question" | _ => RouterAction::NormalChat,
        }
    }

    pub fn visualize_beliefs(&self) -> String {
        self.bayes.visualize()
    }

    pub fn learn_from_tool(&mut self, tool_name: &str) {
        self.bayes.update_profile(tool_name);
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
                if let Some(function) = t.get("function") {
                    if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                        return keep.contains(&name);
                    }
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

    pub fn is_low_confidence(&self) -> bool {
        self.bayes.probability("low_confidence") > 0.5
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_edit_intent() {
        let mut router = Router::new();
        let action = router.route("can you fix this bug and refactor").await;

        assert!(matches!(action, RouterAction::UseTool(tool) if tool == "replace"));
    }

    #[tokio::test]
    async fn test_router_shell_intent() {
        let mut router = Router::new();
        let action = router.route("run the build command").await;

        assert!(matches!(action, RouterAction::UseTool(tool) if tool == "run_shell_command"));
    }

    #[tokio::test]
    async fn test_router_clarification_gate() {
        let mut router = Router::new();
        let action = router.route("be careful, don't delete").await;

        assert!(matches!(action, RouterAction::AskClarification(_)));

        // Second time should let it pass because we decayed the clarification probability
        let action2 = router.route("I am sure, please proceed").await;
        assert!(!matches!(action2, RouterAction::AskClarification(_)));
    }

    #[tokio::test]
    async fn test_router_normal_chat() {
        let mut router = Router::new();
        // Just a random statement
        let action = router.route("hello there").await;
        assert!(matches!(action, RouterAction::NormalChat));
    }
}
