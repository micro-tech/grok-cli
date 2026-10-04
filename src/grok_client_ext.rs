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
use crate::utils::rate_limiter::UsageStats;

/// Extended Grok client that wraps grok_api::GrokClient with additional methods
#[derive(Clone, Debug)]
pub struct GrokClient {
    inner: grok_api::GrokClient,
    rate_limit_config: Option<RateLimitConfig>,
    /// Stable per-conversation key sent as the `x-grok-conv-id` header.
    /// xAI caches prompt prefixes server-side per machine; without a stable
    /// key a conversation's turns land on random servers and the prefix cache
    /// never hits, so every turn re-bills the full prompt at full price.
    prompt_cache_key: Option<String>,
}

/// Process-wide fallback prompt-cache key, generated once.
///
/// Used by entry points that don't have a finer-grained session identifier.
/// One process generally serves one conversation (ACP stdio agent, single CLI
/// invocation), so a process-stable key gives correct cache affinity there.
pub fn process_prompt_cache_key() -> String {
    static KEY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    KEY.get_or_init(|| format!("grok-cli-{}", uuid::Uuid::new_v4()))
        .clone()
}

impl GrokClient {
    /// Create a new GrokClient with default settings
    pub fn new(api_key: &str) -> Result<Self> {
        let inner = grok_api::GrokClient::new(api_key)?;
        Ok(Self {
            inner,
            rate_limit_config: None,
            prompt_cache_key: None,
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
            prompt_cache_key: None,
        })
    }

    /// Set rate limit configuration.
    /// When present, chat methods will enforce max_requests_per_minute and max_tokens_per_minute
    /// using the UsageStats token-bucket style limiter before sending requests.
    pub fn with_rate_limits(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = Some(config);
        self
    }

    /// Set the stable per-conversation prompt-cache key.
    ///
    /// Sent as the `x-grok-conv-id` header on every request made through this
    /// client, giving xAI's automatic server-side prefix cache a stable
    /// routing target so repeated turns hit the cache instead of recomputing
    /// (and rebilling) the full prompt. The key should be stable for the
    /// lifetime of one conversation — e.g. a session UUID.
    pub fn with_prompt_cache_key(mut self, key: impl Into<String>) -> Self {
        self.prompt_cache_key = Some(key.into());
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

                // Support both plain string content and array content (for vision/multimodal).
                // Array content is produced by create_vision_message (text + image_url parts).
                let content = if let Some(s) = msg.get("content").and_then(|c| c.as_str()) {
                    s.to_string()
                } else if let Some(arr) = msg.get("content").and_then(|c| c.as_array()) {
                    // Vision / multimodal: collect text parts and image references
                    // so the model receives both the prompt and image information.
                    arr.iter()
                        .filter_map(|part| match part.get("type").and_then(|t| t.as_str()) {
                            Some("text") => part
                                .get("text")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()),
                            Some("image_url") => part
                                .get("image_url")
                                .and_then(|i| i.get("url"))
                                .and_then(|u| u.as_str())
                                .map(|u| format!("[Attached image: {}]", u)),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    msg.get("content")
                        .map(|c| c.to_string())
                        .unwrap_or_default()
                };

                match role {
                    "system" => Some(ChatMessage::system(content)),
                    "user" => Some(ChatMessage::user(content)),
                    "assistant" => {
                        if let Some(tool_calls_val) = msg.get("tool_calls") {
                            // potential tool calls
                            // deserialize tool calls
                            if let Ok(calls) =
                                serde_json::from_value::<Vec<ToolCall>>(tool_calls_val.clone())
                            {
                                let content_opt = if content.is_empty() {
                                    None
                                } else {
                                    Some(content)
                                };
                                return Some(ChatMessage::assistant_with_tools(content_opt, calls));
                            }
                        }
                        Some(ChatMessage::assistant(content))
                    }
                    "tool" => {
                        let tool_call_id = msg.get("tool_call_id")?.as_str()?;
                        // Fallback: report tool result as user message since tool role is missing in grok_api
                        // This ensures the model sees the result even if native tool role is not supported
                        Some(ChatMessage::user(format!(
                            "Tool result (ID: {}): {}",
                            tool_call_id, content
                        )))
                    }
                    _ => None,
                }
            })
            .collect();

        // === Rate limit enforcement (COR-5) ===
        // If RateLimitConfig was provided via with_rate_limits(), enforce before the call.
        // We use a conservative token estimate based on prompt size.
        if let Some(ref cfg) = self.rate_limit_config {
            let prompt_chars: usize = messages.iter().map(|v| v.to_string().len()).sum();
            let estimated_tokens: u32 = ((prompt_chars / 3) as u32).saturating_add(600); // rough prompt + response headroom

            let mut stats = UsageStats::load().unwrap_or_default();
            if let Err(msg) = stats.check_limit(cfg, estimated_tokens) {
                return Err(anyhow::anyhow!("Rate limit exceeded: {}", msg));
            }
        }

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

        // Prompt-cache affinity: route this conversation's requests to the
        // same xAI server so the automatic prefix cache hits across turns.
        if let Some(key) = &self.prompt_cache_key {
            request = request.prompt_cache_key(key.clone());
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
    /// Real token counts reported by the API for this response.
    /// (Full path: `crate::utils::rate_limiter::UsageStats` is a *different*
    /// struct already imported above.)
    pub usage: Option<crate::router::response::UsageStats>,
}

/// Convert ChatResponse to Message format with finish_reason
fn convert_response_to_message_with_finish_reason(
    response: GrokApiChatResponse,
) -> Result<MessageWithFinishReason> {
    // Get the first choice
    if let Some(choice) = response.choices.first() {
        // Extract the reasoning / thinking content if the model produced one.
        let thinking_content = choice.message.reasoning_content.clone();
        // Keep the real token counts the API reported — callers feed these
        // into ContextBudget / spend tracking instead of the 4-chars-per-token
        // estimate (Task 463).
        let u = &response.usage;
        let usage = crate::router::response::UsageStats {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        };
        Ok(MessageWithFinishReason {
            message: choice.message.clone(),
            finish_reason: choice.finish_reason.clone(),
            thinking_content,
            usage: Some(usage),
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
            usage: None,
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

    #[test]
    fn test_prompt_cache_key_defaults_to_none() {
        let client = GrokClient::new("test-key").unwrap();
        assert!(client.prompt_cache_key.is_none());
    }

    #[test]
    fn test_with_prompt_cache_key_sets_key() {
        let client = GrokClient::new("test-key")
            .unwrap()
            .with_prompt_cache_key("conv-42");
        assert_eq!(client.prompt_cache_key.as_deref(), Some("conv-42"));
    }

    #[test]
    fn test_process_prompt_cache_key_is_stable() {
        // Same process must see the same key on every call so all of its
        // requests share one xAI cache affinity target.
        assert_eq!(process_prompt_cache_key(), process_prompt_cache_key());
        assert!(process_prompt_cache_key().starts_with("grok-cli-"));
    }

    #[test]
    fn test_convert_response_carries_real_usage() {
        // Task 463: the real API-reported token counts must survive the
        // conversion so ContextBudget / spend tracking run on real numbers.
        use grok_api::{Choice, Usage};
        let response = GrokApiChatResponse {
            id: "chatcmpl-1".to_string(),
            object: "chat.completion".to_string(),
            created: 1_700_000_000,
            model: "grok-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(MessageContent::Text("hi".to_string())),
                    tool_calls: None,
                    reasoning_content: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: 1234,
                completion_tokens: 56,
                total_tokens: 1290,
                cached_prompt_tokens: None,
                reasoning_tokens: None,
            },
        };
        let mwfr = convert_response_to_message_with_finish_reason(response).unwrap();
        let usage = mwfr.usage.expect("usage must be populated from the API response");
        assert_eq!(usage.prompt_tokens, 1234);
        assert_eq!(usage.completion_tokens, 56);
        assert_eq!(usage.total_tokens, 1290);
    }

    #[test]
    fn test_convert_response_without_choices_has_no_usage() {
        let response = GrokApiChatResponse {
            id: "chatcmpl-2".to_string(),
            object: "chat.completion".to_string(),
            created: 1_700_000_000,
            model: "grok-4".to_string(),
            choices: vec![],
            usage: grok_api::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                cached_prompt_tokens: None,
                reasoning_tokens: None,
            },
        };
        let mwfr = convert_response_to_message_with_finish_reason(response).unwrap();
        assert!(mwfr.usage.is_none());
    }
}
