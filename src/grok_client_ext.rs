//! Grok Client Extensions
//!
//! This module provides compatibility extensions for the grok_api::GrokClient
//! to maintain API compatibility with the previous local implementation.

use anyhow::Result;
use grok_api::{
    ChatMessage, ChatResponse as GrokApiChatResponse, Message, MessageContent, ToolCall,
};
use serde_json::Value;

use crate::config::RateLimitConfig;

/// Extended Grok client that wraps grok_api::GrokClient with additional methods
#[derive(Clone, Debug)]
pub struct GrokClient {
    inner: grok_api::GrokClient,
    rate_limit_config: Option<RateLimitConfig>,
}

impl GrokClient {
    /// Create a new GrokClient with default settings
    pub fn new(api_key: &str) -> Result<Self> {
        let inner = grok_api::GrokClient::new(api_key)?;
        Ok(Self {
            inner,
            rate_limit_config: None,
        })
    }

    /// Create a new GrokClient with custom timeout and retry settings
    pub fn with_settings(api_key: &str, timeout_secs: u64, max_retries: u32) -> Result<Self> {
        let mut builder = grok_api::GrokClient::builder()
            .api_key(api_key)
            .timeout_secs(timeout_secs)
            .max_retries(max_retries);

        // Allow overriding the base URL via environment variable for testing/mocking
        if let Ok(base_url) = std::env::var("GROK_API_BASE_URL") {
            builder = builder.base_url(base_url);
        }

        let inner = builder.build()?;

        Ok(Self {
            inner,
            rate_limit_config: None,
        })
    }

    /// Set rate limit configuration (for compatibility - currently a no-op)
    pub fn with_rate_limits(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = Some(config);
        self
    }

    /// Send a single chat completion request to Grok
    pub async fn chat_completion(
        &self,
        message: &str,
        system_prompt: Option<&str>,
        temperature: f32,
        max_tokens: u32,
        model: &str,
    ) -> Result<String> {
        let mut messages = Vec::new();

        // Add system message if provided
        if let Some(system) = system_prompt {
            messages.push(ChatMessage::system(system));
        }

        // Add user message
        messages.push(ChatMessage::user(message));

        let response = self
            .inner
            .chat_with_history(&messages)
            .temperature(temperature)
            .max_tokens(max_tokens)
            .model(model)
            .send()
            .await?;

        // Extract text content from response
        Ok(response.content().unwrap_or_default().to_string())
    }

    /// Send chat completion with conversation history and optional tools
    /// Returns (Message, finish_reason, thinking_content)
    pub async fn chat_completion_with_history(
        &self,
        messages: &[Value],
        temperature: f32,
        max_tokens: u32,
        model: &str,
        tools: Option<Vec<Value>>,
        reasoning_effort: Option<&str>,
    ) -> Result<MessageWithFinishReason> {
        // Convert JSON messages to ChatMessage format
        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .filter_map(|msg| {
                let role = msg.get("role")?.as_str()?;
                // content might be null for tool calls
                let content = msg.get("content").and_then(|c| c.as_str());

                match role {
                    "system" => Some(ChatMessage::system(content.unwrap_or(""))),
                    "user" => Some(ChatMessage::user(content.unwrap_or(""))),
                    "assistant" => {
                        if let Some(tool_calls_val) = msg.get("tool_calls") {
                            // potential tool calls
                            // deserialize tool calls
                            if let Ok(calls) =
                                serde_json::from_value::<Vec<ToolCall>>(tool_calls_val.clone())
                            {
                                return Some(ChatMessage::assistant_with_tools(content, calls));
                            }
                        }
                        Some(ChatMessage::assistant(content.unwrap_or("")))
                    }
                    "tool" => {
                        let tool_call_id = msg.get("tool_call_id")?.as_str()?;
                        // Fallback: report tool result as user message since tool role is missing in grok_api
                        // This ensures the model sees the result even if native tool role is not supported
                        Some(ChatMessage::user(format!(
                            "Tool result (ID: {}): {}",
                            tool_call_id,
                            content.unwrap_or("")
                        )))
                    }
                    _ => None,
                }
            })
            .collect();

        let mut request = self
            .inner
            .chat_with_history(&chat_messages)
            .temperature(temperature)
            .max_tokens(max_tokens)
            .model(model);

        // Add tools if provided
        if let Some(tool_defs) = tools {
            // Convert tools to the format expected by grok_api
            request = request.tools(tool_defs);
        }

        // Add reasoning_effort if the caller requested a thinking mode.
        // Only send this for models that support it (grok-4.x, grok-3-mini, etc.).
        if let Some(effort) = reasoning_effort {
            request = request.reasoning_effort(effort);
        }

        let response = request.send().await?;

        // Convert the response to the Message format with finish_reason
        convert_response_to_message_with_finish_reason(response)
    }

    /// Test the connection to the Grok API
    pub async fn test_connection(&self) -> Result<()> {
        self.inner.test_connection().await.map_err(|e| e.into())
    }

    /// List available models from the Grok API.
    ///
    /// Note: The actual list returned depends on the version of the `grok_api`
    /// crate (the one published on crates.io). If that crate has not yet been
    /// updated to call the live `/v1/models` endpoint (or to return a richer
    /// response), this will return whatever the current published version
    /// hard-codes or can discover.
    ///
    /// The static list shown by `/model` and in ACP capabilities is maintained
    /// separately in `slash_commands.rs` and `acp/mod.rs`.
    pub async fn list_models(&self) -> Result<Vec<String>> {
        self.inner.list_models().await.map_err(|e| e.into())
    }

    /// Get the underlying grok_api client
    pub fn inner(&self) -> &grok_api::GrokClient {
        &self.inner
    }
}

/// Message with finish_reason and optional thinking content for proper loop control
#[derive(Debug, Clone)]
pub struct MessageWithFinishReason {
    pub message: Message,
    pub finish_reason: Option<String>,
    /// Chain-of-thought reasoning produced by the model when `reasoning_effort`
    /// was set.  `None` for models / modes that do not return a reasoning trace.
    pub thinking_content: Option<String>,
}

/// Convert ChatResponse to Message format with finish_reason
fn convert_response_to_message_with_finish_reason(
    response: GrokApiChatResponse,
) -> Result<MessageWithFinishReason> {
    // Get the first choice
    if let Some(choice) = response.choices.first() {
        // Extract the reasoning / thinking content if the model produced one.
        let thinking_content = choice.message.reasoning_content.clone();
        Ok(MessageWithFinishReason {
            message: choice.message.clone(),
            finish_reason: choice.finish_reason.clone(),
            thinking_content,
        })
    } else {
        // Fallback if no choices
        Ok(MessageWithFinishReason {
            message: Message {
                role: "assistant".to_string(),
                content: response
                    .content()
                    .map(|s| MessageContent::Text(s.to_string())),
                tool_calls: None,
                reasoning_content: None,
            },
            finish_reason: Some("stop".to_string()),
            thinking_content: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grok_client_creation() {
        let client = GrokClient::with_settings("test-key", 30, 3);
        assert!(client.is_ok());

        let empty_key_client = GrokClient::with_settings("", 30, 3);
        assert!(empty_key_client.is_err());
    }

    #[test]
    fn test_with_rate_limits() {
        let client = GrokClient::new("test-key").unwrap();
        let rate_config = RateLimitConfig::default();
        let client_with_limits = client.with_rate_limits(rate_config);

        assert!(client_with_limits.rate_limit_config.is_some());
    }
}
