use crate::bayes::BayesianEngine;

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
