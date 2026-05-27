//! High-level context engine that combines all context subsystems.

use crate::context::belief_state::BeliefState;
use crate::context::context_budget::ContextBudget;
use crate::context::prompt_applicator::apply_delta;
use crate::context::prompt_delta::PromptDelta;
use crate::context::prompt_diff::should_use_delta;
use crate::context::session_manager::SessionManager;
use crate::context::tool_optimizer::compress_schema;

pub struct ContextEngine {
    pub budget: ContextBudget,
    pub session: SessionManager,
    pub beliefs: BeliefState,
}

impl ContextEngine {
    pub fn new(max_tokens: u32) -> Self {
        // Fall back to a safe default if construction fails
        let budget = ContextBudget::new(max_tokens).unwrap_or_else(|_| ContextBudget::new(8192).unwrap());
        Self {
            budget,
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
        let _ = compress_schema(schema);
    }

    /// Decide whether to use delta prompting for the next turn.
    pub fn decide_delta(&self, previous: Option<&str>, current: &str, system_changed: bool) -> PromptDelta {
        should_use_delta(previous, current, system_changed).unwrap_or_else(|_| PromptDelta::Full { content: current.to_string() })
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

    #[test]
    fn test_decide_delta() {
        let engine = ContextEngine::new(4000);
        let d = engine.decide_delta(Some("hello"), "hello", false);
        assert_eq!(d, PromptDelta::Unchanged);
    }
}
