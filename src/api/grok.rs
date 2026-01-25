//! Grok API client implementation for X (Twitter) API integration
//!
//! This module provides the main GrokClient for interacting with Grok AI
//! through X's API endpoints, with comprehensive error handling and retry logic
//! designed for Starlink network conditions.

use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

use super::{ApiClient, GrokApiError, GrokResponse};
use crate::config::RateLimitConfig;
use crate::utils::rate_limiter::UsageStats;

/// The base URL for xAI API endpoints
const X_API_BASE_URL: &str = "https://api.x.ai";
// Note: xAI deprecated /v1/messages in Feb 2026.
// We use /v1/chat/completions which is the standard OpenAI-compatible endpoint.
const GROK_CHAT_ENDPOINT: &str = "/v1/chat/completions";

/// Grok AI client for X API integration
pub struct GrokClient {
    api_client: ApiClient,
    api_key: String,
    base_url: String,
    usage_stats: Arc<Mutex<UsageStats>>,
    rate_limit_config: RateLimitConfig,
}

impl GrokClient {
    /// Create a new GrokClient with default settings
    pub fn new(api_key: &str) -> Result<Self> {
        Self::with_settings(api_key, 30, 3)
    }

    /// Create a new GrokClient with the provided API key and settings
    pub fn with_settings(api_key: &str, timeout_secs: u64, max_retries: u32) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        let api_client = ApiClient::new(timeout_secs, max_retries)?;

        // Load usage stats from disk, or use default if loading fails
        let usage_stats = match UsageStats::load() {
            Ok(stats) => stats,
            Err(e) => {
                warn!(
                    "Failed to load usage stats: {}. Starting with empty stats.",
                    e
                );
                UsageStats::default()
            }
        };

        Ok(Self {
            api_client,
            api_key: api_key.to_string(),
            base_url: X_API_BASE_URL.to_string(),
            usage_stats: Arc::new(Mutex::new(usage_stats)),
            rate_limit_config: RateLimitConfig::default(),
        })
    }

    /// Set rate limit configuration
    pub fn with_rate_limits(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = config;
        self
    }

    /// Set a custom base URL for testing or alternative endpoints
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
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
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        // Add user message
        messages.push(json!({
            "role": "user",
            "content": message
        }));

        let response = self
            .chat_completion_with_messages(messages, temperature, max_tokens, model, None)
            .await?;

        Ok(response.content.unwrap_or_default())
    }

    /// Send chat completion with conversation history
    pub async fn chat_completion_with_history(
        &self,
        messages: &[Value],
        temperature: f32,
        max_tokens: u32,
        model: &str,
        tools: Option<Vec<Value>>,
    ) -> Result<super::Message> {
        self.chat_completion_with_messages(messages.to_vec(), temperature, max_tokens, model, tools)
            .await
    }

    /// Internal method to handle chat completion requests
    async fn chat_completion_with_messages(
        &self,
        messages: Vec<Value>,
        temperature: f32,
        max_tokens: u32,
        model: &str,
        tools: Option<Vec<Value>>,
    ) -> Result<super::Message> {
        let url = format!("{}{}", self.base_url, GROK_CHAT_ENDPOINT);

        let mut payload = json!({
            "model": model,
            "messages": messages,
            "temperature": temperature.clamp(0.0, 2.0),
            "max_tokens": max_tokens,
            "stream": false,
        });

        if let Some(t) = tools {
            payload["tools"] = json!(t);
        }

        // Estimate tokens and check rate limits
        let payload_str = serde_json::to_string(&payload)?;
        let estimated_tokens = (payload_str.len() as u32) / 4;

        {
            let mut stats = self
                .usage_stats
                .lock()
                .map_err(|_| anyhow!("Failed to lock usage stats"))?;
            stats
                .check_limit(&self.rate_limit_config, estimated_tokens)
                .map_err(|e| anyhow!(GrokApiError::RateLimit))?; // Or use a custom error for client-side rate limit
        }

        debug!("Sending request to Grok API: {}", url);
        debug!("Payload: {}", serde_json::to_string_pretty(&payload)?);

        let response = self
            .api_client
            .execute_with_retry(|| async {
                let headers = self.create_headers()?;

                let response = self
                    .api_client
                    .client()
                    .post(&url)
                    .headers(headers)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| {
                        let error_string = e.to_string();
                        if error_string.to_lowercase().contains("timeout")
                            || error_string.to_lowercase().contains("connection")
                            || error_string.to_lowercase().contains("network")
                        {
                            warn!("Network issue detected during request: {}", error_string);
                            anyhow!(GrokApiError::NetworkDrop)
                        } else {
                            anyhow!(GrokApiError::Network(e))
                        }
                    })?;

                self.handle_response(response, model).await
            })
            .await?;

        // Extract the response content
        if let Some(choice) = response.choices.first() {
            Ok(choice.message.clone())
        } else {
            Err(anyhow!("No response choices received from Grok API"))
        }
    }

    /// Create HTTP headers for API requests
    fn create_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // Authorization header with Bearer token
        let auth_value = HeaderValue::from_str(&format!("Bearer {}", self.api_key))
            .map_err(|_| anyhow!("Invalid API key format"))?;
        headers.insert(AUTHORIZATION, auth_value);

        // Content-Type header
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // User-Agent header
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("grok-cli/0.1.0 (Rust)"),
        );

        Ok(headers)
    }

    /// Handle HTTP response and convert to GrokResponse
    async fn handle_response(
        &self,
        response: reqwest::Response,
        model_name: &str,
    ) -> Result<GrokResponse> {
        let status = response.status();
        let status_code = status.as_u16();

        debug!("Received response with status: {}", status);

        if status.is_success() {
            let response_text = response.text().await.map_err(|e| {
                warn!("Failed to read response text: {}", e);
                GrokApiError::Network(e)
            })?;

            debug!("Response body: {}", response_text);

            let grok_response: GrokResponse =
                serde_json::from_str(&response_text).map_err(|e| {
                    warn!("Failed to parse JSON response: {}", e);
                    warn!("Response was: {}", response_text);
                    GrokApiError::Json(e)
                })?;

            info!(
                "Grok API call successful. Model: {}, Tokens used: {}",
                grok_response.model, grok_response.usage.total_tokens
            );

            // Update usage stats
            {
                if let Ok(mut stats) = self.usage_stats.lock() {
                    stats.record_usage(
                        grok_response.usage.prompt_tokens,
                        grok_response.usage.completion_tokens,
                    );
                } else {
                    warn!("Failed to lock usage stats for update");
                }
            }

            Ok(grok_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            warn!(
                "API request failed with status {}: {}",
                status_code, error_text
            );

            let error = match status_code {
                401 => GrokApiError::Authentication,
                429 => GrokApiError::RateLimit,
                404 => {
                    if error_text.to_lowercase().contains("model") {
                        GrokApiError::ModelNotFound {
                            model: model_name.to_string(),
                        }
                    } else {
                        GrokApiError::Server {
                            status: status_code,
                            message: error_text,
                        }
                    }
                }
                400 => GrokApiError::InvalidRequest {
                    message: error_text,
                },
                500..=599 => GrokApiError::Server {
                    status: status_code,
                    message: error_text,
                },
                _ => GrokApiError::Server {
                    status: status_code,
                    message: error_text,
                },
            };

            Err(error.into())
        }
    }

    /// Test the connection to the Grok API
    pub async fn test_connection(&self) -> Result<()> {
        info!("Testing connection to Grok API...");

        let test_message = "Hello, this is a connection test.";
        let response = self
            .chat_completion(test_message, None, 0.1, 50, "grok-2-latest")
            .await?;

        debug!("Connection test response: {}", response);
        info!("Grok API connection test successful");

        Ok(())
    }

    /// Get available models from the API
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);

        debug!("Fetching available models from: {}", url);

        let response = self
            .api_client
            .execute_with_retry(|| async {
                let headers = self.create_headers()?;

                let response = self
                    .api_client
                    .client()
                    .get(&url)
                    .headers(headers)
                    .send()
                    .await
                    .map_err(|e| {
                        let error_string = e.to_string();
                        if error_string.to_lowercase().contains("timeout")
                            || error_string.to_lowercase().contains("connection")
                            || error_string.to_lowercase().contains("network")
                        {
                            warn!(
                                "Network issue detected during models request: {}",
                                error_string
                            );
                            anyhow!(GrokApiError::NetworkDrop)
                        } else {
                            anyhow!(GrokApiError::Network(e))
                        }
                    })?;

                let status = response.status();
                let status_code = status.as_u16();
                if status.is_success() {
                    let models_response: serde_json::Value = response
                        .json()
                        .await
                        .map_err(|e| anyhow!(GrokApiError::Network(e)))?;

                    if let Some(data) = models_response.get("data")
                        && let Some(models_array) = data.as_array() {
                            let model_names: Vec<String> = models_array
                                .iter()
                                .filter_map(|model| model.get("id")?.as_str())
                                .map(String::from)
                                .collect();
                            return Ok(model_names);
                        }

                    // Fallback if response structure is different
                    warn!("Unexpected models API response structure");
                    Ok(vec![
                        "grok-3".to_string(),
                        "grok-3-mini".to_string(),
                        "grok-2-latest".to_string(),
                        "grok-2".to_string(),
                        "grok-beta".to_string(),
                        "grok-vision-beta".to_string(),
                    ])
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(anyhow!(GrokApiError::Server {
                        status: status_code,
                        message: error_text,
                    }))
                }
            })
            .await?;

        Ok(response)
    }

    /// Get API usage statistics (if available)
    pub async fn get_usage_stats(&self) -> Result<Value> {
        // Placeholder implementation
        // In a real implementation, this would call the appropriate endpoint
        Ok(json!({
            "message": "Usage statistics not yet implemented",
            "available": false
        }))
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

    #[tokio::test]
    async fn test_headers_creation() {
        let client = GrokClient::with_settings("test-api-key", 30, 3).unwrap();
        let headers = client.create_headers().unwrap();

        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));

        let auth_header = headers.get(AUTHORIZATION).unwrap();
        assert_eq!(auth_header, "Bearer test-api-key");
    }

    #[tokio::test]
    async fn test_with_custom_base_url() {
        let custom_url = "https://api.example.com/v1";
        let client = GrokClient::with_settings("test-key", 30, 3)
            .unwrap()
            .with_base_url(custom_url.to_string());

        assert_eq!(client.base_url, custom_url);
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_models() {
        let client = GrokClient::with_settings("test-key", 30, 3).unwrap();
        let models = client.list_models().await.unwrap();

        assert!(!models.is_empty());
        assert!(models.contains(&"grok-3".to_string()));
    }
}
