use grok_api::{Message, MessageContent, ToolCall};
use serde::{Deserialize, Serialize};

use crate::MessageWithFinishReason;

/// Usage statistics returned by the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Normalised response from any backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterResponse {
    /// Plain-text content of the response, if the model produced one.
    pub text: Option<String>,
    /// Tool calls requested by the model, if any.
    pub tool_calls: Vec<ToolCall>,
    /// Raw JSON body returned by the backend (useful for debugging).
    pub raw: serde_json::Value,
    /// The model identifier that produced this response.
    pub model: String,
    /// Optional token-usage statistics.
    pub usage: Option<UsageStats>,
    /// Chain-of-thought reasoning content from the model, if `reasoning_effort`
    /// was set and the model produced a reasoning trace.
    pub thinking_content: Option<String>,
}

impl RouterResponse {
    /// Convenience constructor for a plain-text response.
    pub fn text(text: impl Into<String>, model: impl Into<String>) -> Self {
        let t = text.into();
        Self {
            raw: serde_json::json!({ "text": t }),
            text: Some(t),
            tool_calls: Vec::new(),
            model: model.into(),
            usage: None,
            thinking_content: None,
        }
    }

    /// Returns `true` when the response contains at least one tool call.
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Returns the text content, falling back to an empty string.
    pub fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    /// Convert into the legacy [`MessageWithFinishReason`] format used
    /// throughout the rest of the app.
    ///
    /// This lets call sites that previously consumed a `GrokClient` response
    /// consume an `AppRouter` response without any further changes.
    pub fn into_message_with_finish_reason(self) -> MessageWithFinishReason {
        let has_tool_calls = !self.tool_calls.is_empty();
        let content = self.text.map(MessageContent::Text);
        MessageWithFinishReason {
            message: Message {
                role: "assistant".to_string(),
                content,
                tool_calls: if has_tool_calls {
                    Some(self.tool_calls)
                } else {
                    None
                },
                reasoning_content: None,
            },
            // When the model wants to call tools the API returns finish_reason
            // "tool_calls".  Hardcoding "stop" here caused handle_chat_completion
            // to exit the tool loop immediately with empty content.
            finish_reason: if has_tool_calls {
                Some("tool_calls".to_string())
            } else {
                Some("stop".to_string())
            },
            thinking_content: self.thinking_content,
        }
    }
}
