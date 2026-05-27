//! Session manager that ties together budget, delta prompting, and summarization.

use crate::context::context_budget::ContextBudget;
use crate::context::prompt_delta::PromptDelta;
use crate::context::prompt_diff::should_use_delta;
use crate::context::session_summarizer::SessionSummarizer;

pub struct SessionManager {
    budget: ContextBudget,
    summarizer: SessionSummarizer,
    last_prompt: Option<String>,
}

impl SessionManager {
    pub fn new(max_tokens: u32) -> Self {
        Self {
            budget: ContextBudget::new(max_tokens),
            summarizer: SessionSummarizer::new(8),
            last_prompt: None,
        }
    }

    pub fn record_turn(&mut self, text: String, input_tokens: u32, output_tokens: u32) {
        self.summarizer.add_turn(text.clone());
        self.budget.record_usage(input_tokens, output_tokens);
        self.last_prompt = Some(text);
    }

    pub fn next_delta(&self, new_prompt: &str, system_changed: bool) -> PromptDelta {
        should_use_delta(self.last_prompt.as_deref(), new_prompt, system_changed)
    }

    pub fn should_compress(&self) -> bool {
        self.budget.should_use_delta()
    }

    pub fn summary(&self) -> String {
        self.summarizer.summarize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_flow() {
        let mut sm = SessionManager::new(2000);
        sm.record_turn("first message".into(), 100, 50);
        let d = sm.next_delta("first message\nsecond", false);
        assert!(!matches!(d, PromptDelta::Unchanged));
    }
}
