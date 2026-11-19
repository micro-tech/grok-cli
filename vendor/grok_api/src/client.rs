//! Main client implementation for the Grok API

use crate::error::{Error, Result};
use crate::models::{ChatMessage, ChatRequest, ChatResponse, Model};
use crate::retry::{RetryConfig, retry_async};
use reqwest::{Client, ClientBuilder};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Base URL for the Grok API
const DEFAULT_BASE_URL: &str = "https://api.x.ai";

/// Default model (grok-4.7 — current flagship for code, chat, and agentic work)
const DEFAULT_MODEL: &str = "grok-4.7";

/// Main client for interacting with the Grok API
#[derive(Clone)]
pub struct GrokClient {
    client: Client,
    api_key: String,
    base_url: String,
    retry_config: RetryConfig,
}

impl std::fmt::Debug for GrokClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrokClient")
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .field("retry_config", &self.retry_config)
            .finish()
    }
}

impl GrokClient {
    /// Create a new Grok client with the given API key
    ///
    /// # Example
    ///
    /// ```no_run
    /// use grok_api::{GrokClient, Result};
    ///
    /// # async fn example() -> Result<()> {
    /// let client = GrokClient::new("your-api-key")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::builder().api_key(api_key).build()
    }

    /// Create a new builder for configuring the client
    ///
    /// # Example
    ///
    /// ```no_run
    /// use grok_api::{GrokClient, Result};
    ///
    /// # async fn example() -> Result<()> {
    /// let client = GrokClient::builder()
    ///     .api_key("your-api-key")
    ///     .timeout_secs(60)
    ///     .max_retries(5)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> GrokClientBuilder {
        GrokClientBuilder::default()
    }

    /// Send a simple chat message
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use grok_api::{GrokClient, Result};
    /// # async fn example() -> Result<()> {
    /// # let client = GrokClient::new("key")?;
    /// let response = client.chat("What is Rust?", None).await?;
    /// println!("Response: {}", response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chat(&self, message: impl Into<String>, model: Option<&str>) -> Result<String> {
        let message_text: String = message.into();
        let messages = vec![ChatMessage::user(message_text)];
        let response: ChatResponse = self
            .chat_with_history(&messages)
            .model(model.unwrap_or(DEFAULT_MODEL))
            .send()
            .await?;

        Ok(response.content().unwrap_or("").to_string())
    }

    /// Create a chat request builder with message history
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use grok_api::{GrokClient, ChatMessage, Result};
    /// # async fn example() -> Result<()> {
    /// # let client = GrokClient::new("key")?;
    /// let messages = vec![
    ///     ChatMessage::system("You are a helpful assistant"),
    ///     ChatMessage::user("Hello!"),
    /// ];
    ///
    /// let response = client
    ///     .chat_with_history(&messages)
    ///     .temperature(0.7)
    ///     .max_tokens(1000)
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn chat_with_history(&self, messages: &[ChatMessage]) -> ChatRequestBuilder {
        ChatRequestBuilder::new(self.clone(), messages.to_vec())
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);

        let result = retry_async(&self.retry_config, || async {
            let response = self
                .client
                .get(&url)
                .bearer_auth(&self.api_key)
                .send()
                .await
                .map_err(Error::from_reqwest)?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                return Err(Error::Http {
                    status,
                    message: body,
                });
            }

            let json: Value = response.json().await.map_err(Error::from_reqwest)?;

            if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                let models = data
                    .iter()
                    .filter_map(|m| m.get("id")?.as_str())
                    .map(String::from)
                    .collect();
                Ok(models)
            } else {
                // Fallback to known models
                Ok(Model::all().iter().map(|m| m.to_string()).collect())
            }
        })
        .await?;

        Ok(result)
    }

    /// Test connection to the API
    pub async fn test_connection(&self) -> Result<()> {
        info!("Testing connection to Grok API");
        let _ = self.chat("Hello", Some("grok-4.5")).await?;
        info!("Connection test successful");
        Ok(())
    }

    /// Internal method to send a chat completion request
    async fn send_chat_request(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        debug!("Sending chat request to {}", url);
        debug!(
            "Model: {}, Messages: {}",
            request.model,
            request.messages.len()
        );

        // Extract prompt cache key before moving the request into the closure
        let cache_key = request.prompt_cache_key.clone();

        let result = retry_async(&self.retry_config, || async {
            let mut req = self
                .client
                .post(&url)
                .bearer_auth(&self.api_key);

            if let Some(key) = &cache_key {
                req = req.header("x-grok-conv-id", key);
                debug!("Using prompt cache key: {}", key);
            }

            let response = req
                .json(&request)
                .send()
                .await
                .map_err(Error::from_reqwest)?;

            let status = response.status();
            if !status.is_success() {
                let status_code = status.as_u16();
                // H5 backport: preserve the Retry-After header (seconds) so
                // callers can honor it. Read it before consuming the body.
                let retry_after_secs = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok());
                let body = response.text().await.unwrap_or_default();

                warn!("API request failed: {} - {}", status_code, body);

                return Err(match status_code {
                    401 => Error::Authentication,
                    429 => Error::RateLimit {
                        body,
                        retry_after_secs,
                    },
                    404 => Error::ModelNotFound(request.model.clone()),
                    400 => Error::InvalidRequest(body),
                    500..=599 => Error::ServerError(body),
                    _ => Error::Http {
                        status: status_code,
                        message: body,
                    },
                });
            }

            // Guard: reject responses that advertise a body larger than 50 MB.
            // A legitimate chat completion response is never that large.
            const MAX_RESPONSE_BYTES: u64 = 50 * 1024 * 1024;
            if let Some(len) = response.content_length() {
                if len > MAX_RESPONSE_BYTES {
                    return Err(Error::InvalidRequest(format!(
                        "Response body too large: {} bytes (limit {} MB)",
                        len,
                        MAX_RESPONSE_BYTES / 1024 / 1024,
                    )));
                }
            }
            let response_text = response.text().await.map_err(Error::from_reqwest)?;
            debug!("Response received: {} bytes", response_text.len());

            let chat_response: ChatResponse =
                serde_json::from_str(&response_text).map_err(|e| {
                    warn!("Failed to parse response: {}", e);
                    Error::Json(e)
                })?;

            info!(
                "Chat completion successful - Model: {}, Tokens: {}",
                chat_response.model, chat_response.usage.total_tokens
            );

            Ok(chat_response)
        })
        .await?;

        Ok(result)
    }
}

/// Builder for configuring a GrokClient
#[derive(Default)]
pub struct GrokClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout_secs: Option<u64>,
    max_retries: Option<u32>,
    retry_config: Option<RetryConfig>,
}

impl GrokClientBuilder {
    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set a custom base URL (for testing or proxies)
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Use the US regional endpoint (`https://us.api.x.ai`).
    ///
    /// This can give lower latency for users in the US at a ~10% premium.
    ///
    /// See: https://docs.x.ai/developers/models/grok-4-7#regional-endpoints-us-only
    pub fn us_region(mut self) -> Self {
        self.base_url = Some("https://us.api.x.ai".to_string());
        self
    }

    /// Use a specific regional endpoint.
    ///
    /// Example: `regional("us")` → `https://us.api.x.ai`
    pub fn regional(mut self, region: &str) -> Self {
        self.base_url = Some(format!("https://{}.api.x.ai", region));
        self
    }

    /// Set request timeout in seconds
    pub fn timeout_secs(mut self, timeout: u64) -> Self {
        self.timeout_secs = Some(timeout);
        self
    }

    /// Set maximum number of retry attempts
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set a custom retry configuration
    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }

    /// Build the GrokClient
    pub fn build(self) -> Result<GrokClient> {
        let api_key = self.api_key.ok_or(Error::EmptyApiKey)?;

        if api_key.is_empty() {
            return Err(Error::EmptyApiKey);
        }

        // When the `starlink` feature is enabled, use a longer default timeout
        // to survive satellite beam-handoff gaps (typically 15–60 s).
        #[cfg(feature = "starlink")]
        let timeout = self.timeout_secs.unwrap_or(90);
        #[cfg(not(feature = "starlink"))]
        let timeout = self.timeout_secs.unwrap_or(crate::DEFAULT_TIMEOUT_SECS);

        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(timeout))
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Duration::from_secs(30))
            .user_agent(format!("grok_api/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::from_reqwest)?;

        // When the `starlink` feature is enabled and no custom retry config was
        // provided, use Starlink-optimised defaults (7 retries, 2 s base delay,
        // 2-minute cap).  An explicit `.retry_config()` or `.max_retries()` call
        // always takes precedence.
        #[cfg(feature = "starlink")]
        let retry_config = {
            let mut cfg = self.retry_config.unwrap_or_else(RetryConfig::for_starlink);
            if let Some(r) = self.max_retries {
                cfg.max_retries = r;
            }
            cfg
        };
        #[cfg(not(feature = "starlink"))]
        let retry_config = self
            .retry_config
            .unwrap_or_else(|| RetryConfig::new(self.max_retries.unwrap_or(3)));

        Ok(GrokClient {
            client,
            api_key,
            base_url: self
                .base_url
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            retry_config,
        })
    }
}

/// Builder for creating chat requests
pub struct ChatRequestBuilder {
    client: GrokClient,
    messages: Vec<ChatMessage>,
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tools: Option<Vec<Value>>,
    top_p: Option<f32>,
    reasoning_effort: Option<String>,
    /// When true, `Message::reasoning_content` is cleared before returning the
    /// response, saving output-token cost for the chain-of-thought text.
    suppress_reasoning_content: bool,

    /// Prompt cache key. When set, sent as `x-grok-conv-id` header.
    /// Highly recommended for Grok 4.7 multi-turn conversations.
    prompt_cache_key: Option<String>,
}

impl ChatRequestBuilder {
    fn new(client: GrokClient, messages: Vec<ChatMessage>) -> Self {
        Self {
            client,
            messages,
            model: None,
            temperature: None,
            max_tokens: None,
            tools: None,
            top_p: None,
            reasoning_effort: None,
            suppress_reasoning_content: false,
            prompt_cache_key: None,
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the sampling temperature (0.0 to 2.0)
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set maximum tokens to generate
    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set tools/functions available to the model
    pub fn tools(mut self, tools: Vec<Value>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set top-p sampling parameter
    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p.clamp(0.0, 1.0));
        self
    }

    /// Set the reasoning effort level for thinking-capable models.
    /// Valid values: `"low"`, `"medium"`, `"high"`.
    /// Has no effect on models that do not support reasoning (field is
    /// silently omitted from the request if not set).
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    /// Set the reasoning depth using the typed [`ReasoningDepth`] enum, and
    /// **include** `reasoning_content` in the response.
    ///
    /// Use this when you want to read or log the model's chain-of-thought.
    /// If you don't need to inspect it, prefer [`Self::think`] or the shorthand
    /// methods to keep CoT off the wire and out of your message history.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use grok_api::{GrokClient, ChatMessage, ReasoningDepth, Result};
    /// # async fn example() -> Result<()> {
    /// # let client = GrokClient::new("key")?;
    /// let response = client
    ///     .chat_with_history(&[ChatMessage::user("Explain monads.")])
    ///     .model("grok-4.7")
    ///     .with_reasoning(ReasoningDepth::High)
    ///     .send()
    ///     .await?;
    /// // chain-of-thought is available here:
    /// if let Some(cot) = response.message().and_then(|m| m.reasoning_content.as_deref()) {
    ///     println!("Reasoning: {cot}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_reasoning(self, depth: crate::types::ReasoningDepth) -> Self {
        self.reasoning_effort(depth.as_str())
    }

    /// Set the reasoning depth using the typed [`ReasoningDepth`] enum, and
    /// **suppress** `reasoning_content` from the response.
    ///
    /// The model still reasons at the requested depth — `reasoning_tokens` are
    /// computed and **billed regardless**.  What this method does is strip the
    /// chain-of-thought *text* from the HTTP response before it reaches your
    /// code, keeping it off the wire and out of your message history.  That
    /// matters because echoing `reasoning_content` back as context in follow-up
    /// requests can add significant prompt-token cost.
    ///
    /// **To reduce the reasoning bill itself**, lower the effort level
    /// (e.g. `Low` vs `High`) or use [`ReasoningDepth::None`] for zero
    /// reasoning overhead.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use grok_api::{GrokClient, ChatMessage, ReasoningDepth, Result};
    /// # async fn example() -> Result<()> {
    /// # let client = GrokClient::new("key")?;
    /// let response = client
    ///     .chat_with_history(&[ChatMessage::user("Solve this hard maths problem.")])
    ///     .model("grok-4.7")
    ///     .think(ReasoningDepth::High)   // deep reasoning, no chain-of-thought in response
    ///     .send()
    ///     .await?;
    /// println!("{}", response.content().unwrap_or(""));
    /// # Ok(())
    /// # }
    /// ```
    pub fn think(mut self, depth: crate::types::ReasoningDepth) -> Self {
        self.suppress_reasoning_content = true;
        self.reasoning_effort(depth.as_str())
    }

    /// Enable prompt caching for this request using a stable conversation ID.
    ///
    /// This is the recommended way to get reliable caching on Grok 4.7+.
    /// The key should be stable per conversation/thread (e.g. a UUID or
    /// user+session identifier).
    ///
    /// Sent as the `x-grok-conv-id` header.
    ///
    /// See official docs: https://docs.x.ai/developers/advanced-api-usage/prompt-caching
    pub fn prompt_cache_key(mut self, key: impl Into<String>) -> Self {
        self.prompt_cache_key = Some(key.into());
        self
    }

    /// Shorthand for `.prompt_cache_key(key)`.
    pub fn with_cache_key(self, key: impl Into<String>) -> Self {
        self.prompt_cache_key(key)
    }

    /// Light reasoning, chain-of-thought kept off the wire.
    ///
    /// Sends `reasoning_effort: "low"` — small reasoning_token budget.
    /// Fast and low-latency; best for agentic loops and simple tool calls.
    /// `Message::reasoning_content` is stripped from the response so it
    /// does not accumulate in your message history.
    pub fn think_low(self) -> Self {
        self.think(crate::types::ReasoningDepth::Low)
    }

    /// Balanced reasoning, chain-of-thought kept off the wire.
    ///
    /// Sends `reasoning_effort: "medium"` — moderate reasoning_token budget.
    /// Good balance of quality and cost for complex analysis or long-context tasks.
    /// `Message::reasoning_content` is stripped from the response.
    pub fn think_medium(self) -> Self {
        self.think(crate::types::ReasoningDepth::Medium)
    }

    /// Deep reasoning, chain-of-thought kept off the wire.
    ///
    /// Sends `reasoning_effort: "high"` (the API default when set) —
    /// large reasoning_token budget; best for hard problems and multi-step logic.
    /// `Message::reasoning_content` is stripped from the response so it
    /// does not accumulate in your message history.
    pub fn think_deep(self) -> Self {
        self.think(crate::types::ReasoningDepth::High)
    }

    /// Deepest reasoning level, chain-of-thought kept off the wire (Grok 4.6 and 4.7).
    ///
    /// Sends `reasoning_effort: "xhigh"` — maximum reasoning_token budget.
    /// Returns an API error on models older than Grok 4.6.
    /// `Message::reasoning_content` is stripped from the response.
    pub fn think_xhigh(self) -> Self {
        self.think(crate::types::ReasoningDepth::XHigh)
    }

    /// Send the chat request
    pub async fn send(self) -> Result<ChatResponse> {
        let request = ChatRequest {
            model: self.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: Some(false),
            tools: self.tools,
            top_p: self.top_p,
            frequency_penalty: None,
            presence_penalty: None,
            reasoning_effort: self.reasoning_effort,
            prompt_cache_key: self.prompt_cache_key,
        };

        let mut response = self.client.send_chat_request(request).await?;

        // Strip chain-of-thought text before returning.
        // reasoning_tokens are still billed; this keeps CoT off the wire
        // and out of the caller's message history (which saves prompt tokens
        // on follow-up requests if CoT would otherwise be echoed back).
        if self.suppress_reasoning_content {
            for choice in &mut response.choices {
                choice.message.reasoning_content = None;
            }
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_think_deep_sets_effort_and_suppress_flag() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .build()
            .unwrap();
        let builder = client
            .chat_with_history(&[])
            .think_deep();
        assert_eq!(builder.reasoning_effort.as_deref(), Some("high"));
        assert!(builder.suppress_reasoning_content);
    }

    #[test]
    fn test_with_reasoning_does_not_suppress() {
        use crate::types::ReasoningDepth;
        let client = GrokClient::builder()
            .api_key("test-key")
            .build()
            .unwrap();
        let builder = client
            .chat_with_history(&[])
            .with_reasoning(ReasoningDepth::High);
        assert_eq!(builder.reasoning_effort.as_deref(), Some("high"));
        assert!(!builder.suppress_reasoning_content);
    }

    #[test]
    fn test_think_xhigh_serialises_correctly() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .build()
            .unwrap();
        let builder = client
            .chat_with_history(&[])
            .think_xhigh();
        assert_eq!(builder.reasoning_effort.as_deref(), Some("xhigh"));
        assert!(builder.suppress_reasoning_content);
    }

    #[test]
    fn test_think_low_medium_set_correct_effort() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .build()
            .unwrap();
        let low = client.chat_with_history(&[]).think_low();
        assert_eq!(low.reasoning_effort.as_deref(), Some("low"));

        let medium = client.chat_with_history(&[]).think_medium();
        assert_eq!(medium.reasoning_effort.as_deref(), Some("medium"));
    }

    #[test]
    fn test_client_builder() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .timeout_secs(60)
            .max_retries(5)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.api_key, "test-key");
    }

    #[test]
    fn test_empty_api_key_error() {
        let result = GrokClient::new("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::EmptyApiKey));
    }

    #[test]
    fn test_builder_missing_api_key() {
        let result = GrokClient::builder().timeout_secs(30).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_debug_does_not_leak_api_key() {
        let client = GrokClient::builder()
            .api_key("super-secret-key-abc123")
            .build()
            .unwrap();
        let debug_output = format!("{:?}", client);
        assert!(
            !debug_output.contains("super-secret-key-abc123"),
            "API key must not appear in Debug output: {debug_output}"
        );
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output should show [REDACTED] for api_key: {debug_output}"
        );
    }

    #[test]
    fn test_prompt_cache_key_builder() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .build()
            .unwrap();

        let builder = client
            .chat_with_history(&[])
            .model("grok-4.7")
            .prompt_cache_key("conv-123-abc");

        assert_eq!(builder.prompt_cache_key.as_deref(), Some("conv-123-abc"));
    }

    #[test]
    fn test_with_cache_key_shorthand() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .build()
            .unwrap();

        let builder = client
            .chat_with_history(&[])
            .with_cache_key("my-session-key");

        assert_eq!(builder.prompt_cache_key.as_deref(), Some("my-session-key"));
    }

    #[test]
    fn test_us_region() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .us_region()
            .build()
            .unwrap();

        assert_eq!(client.base_url, "https://us.api.x.ai");
    }

    #[test]
    fn test_regional() {
        let client = GrokClient::builder()
            .api_key("test-key")
            .regional("eu")
            .build()
            .unwrap();

        assert_eq!(client.base_url, "https://eu.api.x.ai");
    }
}
