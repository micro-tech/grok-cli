//! High-level context engine that combines all context subsystems.

use crate::context::belief_state::BeliefState;
use crate::context::context_budget::ContextBudget;
use crate::context::prompt_applicator::apply_delta;
use crate::context::prompt_delta::PromptDelta;
use crate::context::session_manager::SessionManager;
use crate::context::tool_optimizer::compress_schema;

pub struct ContextEngine {
    pub budget: ContextBudget,
    pub session: SessionManager,
    pub beliefs: BeliefState,
}

impl ContextEngine {
    pub fn new(max_tokens: u32) -> Self {
        Self {
            budget: ContextBudget::new(max_tokens),
            session: SessionManager::new(max_tokens),
            beliefs: BeliefState::new(),
        }
    }

    /// Returns the final prompt text after applying delta logic.
    pub fn build_final_prompt(&mut self, base: &str, delta: &PromptDelta) -> String {
        let mut prompt = apply_delta(base, delta);

        if self.budget.should_use_delta() {
            let summary = self.session.summary();
            if !summary.is_empty() {
                prompt = format!("{}\n[context summary] {}", prompt, summary);
            }
        }

        prompt
    }

    pub fn compress_tool_schema(&self, schema: &mut serde_json::Value) {
        compress_schema(schema);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ContextEngine::new(4000);
        assert_eq!(engine.budget.remaining(), 4000);
    }
}
