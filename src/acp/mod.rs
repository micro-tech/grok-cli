//! ACP (Agent Client Protocol) integration module
//!
//! This module provides the Grok AI agent implementation for the Agent Client Protocol,
//! enabling seamless integration with Zed editor and other ACP-compatible clients.

use crate::acp::protocol::SessionId;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::GrokClient;
use crate::config::Config;
use crate::grok_client_ext::MessageWithFinishReason;
use crate::hooks::HookManager;
use crate::{content_to_string, extract_text_content};

pub mod protocol;
pub mod security;
pub mod tools;

use crate::acp::protocol::{
    AGENT_METHOD_NAMES, AgentCapabilities, ContentBlock, ContentChunk, Implementation,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse, PromptRequest,
    PromptResponse, ProtocolVersion, SessionId as ProtocolSessionId, SessionNotification,
    SessionUpdate, StopReason, TextContent,
};
use security::SecurityManager;

/// Grok AI agent implementation for ACP
pub struct GrokAcpAgent {
    /// Grok API client
    grok_client: GrokClient,

    /// Agent configuration
    config: Config,

    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,

    /// Agent capabilities
    capabilities: GrokAgentCapabilities,

    /// Security manager
    pub security: SecurityManager,

    /// Hook manager
    hook_manager: Arc<RwLock<HookManager>>,

    /// Default model override
    default_model: Option<String>,
}

/// Session data for tracking conversation state
#[derive(Debug, Clone)]
struct SessionData {
    /// Conversation history
    messages: Vec<Value>,

    /// Session configuration
    config: SessionConfig,

    /// Creation timestamp
    created_at: std::time::Instant,

    /// Last activity timestamp
    last_activity: std::time::Instant,
}

/// Session-specific configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Model to use for this session
    pub model: String,

    /// Temperature setting
    pub temperature: f32,

    /// Maximum tokens per response
    pub max_tokens: u32,

    /// System prompt for this session
    pub system_prompt: Option<String>,
}

/// Agent capabilities for ACP
#[derive(Debug, Clone)]
pub struct GrokAgentCapabilities {
    /// Supported models
    pub models: Vec<String>,

    /// Maximum context length
    pub max_context_length: u32,

    /// Supported features
    pub features: Vec<String>,

    /// Tool definitions
    pub tools: Vec<ToolDefinition>,
}

/// Tool definition for ACP
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Tool parameters schema
    pub parameters: Value,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            model: "grok-3".to_string(),
            temperature: 0.5, // Lower temperature for more deterministic coding output
            max_tokens: 4096,
            system_prompt: Some(
                "You are an expert software engineer and coding assistant. \
                Your primary goal is to write high-quality, efficient, and maintainable code. \
                You have access to tools to read files, write files, and list directories. \
                Use these tools to understand the codebase and perform tasks. \
                Follow these guidelines:\n\
                1. Write clean, idiomatic code adhering to standard conventions.\n\
                2. Prioritize correctness, performance, and security.\n\
                3. Provide clear explanations for your design choices.\n\
                4. When modifying existing code, respect the existing style and structure.\n\
                5. Always consider edge cases and error handling.\n\
                6. Suggest tests to verify your code when appropriate."
                    .to_string(),
            ),
        }
    }
}

impl GrokAcpAgent {
    /// Create a new Grok ACP agent
    pub async fn new(config: Config, default_model: Option<String>) -> Result<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("API key not configured"))?;

        let grok_client =
            GrokClient::with_settings(api_key, config.timeout_secs, config.max_retries)?;

        let capabilities = Self::create_capabilities();

        let security = SecurityManager::new();
        // Trust current directory by default, canonicalizing to resolve symlinks
        if let Ok(cwd) = std::env::current_dir() {
            let canonical_cwd = cwd.canonicalize().unwrap_or(cwd);
            security.add_trusted_directory(canonical_cwd);
        }

        Ok(Self {
            grok_client,
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            capabilities,
            security,
            hook_manager: Arc::new(RwLock::new(HookManager::new())),
            default_model,
        })
    }

    /// Create agent capabilities
    fn create_capabilities() -> GrokAgentCapabilities {
        GrokAgentCapabilities {
            models: vec![
                "grok-4-1-fast-reasoning".to_string(),
                "grok-4-1-fast-non-reasoning".to_string(),
                "grok-code-fast-1".to_string(),
                "grok-4-fast-reasoning".to_string(),
                "grok-4-fast-non-reasoning".to_string(),
                "grok-4-0709".to_string(),
                "grok-3".to_string(),
                "grok-3-mini".to_string(),
                "grok-2-vision-1212".to_string(),
                "grok-2".to_string(), // Fallback
            ],
            max_context_length: 131072,
            features: vec![
                "chat_completion".to_string(),
                "code_generation".to_string(),
                "code_review".to_string(),
                "code_explanation".to_string(),
                "streaming".to_string(),
                "function_calling".to_string(),
            ],
            tools: vec![
                ToolDefinition {
                    name: "chat_complete".to_string(),
                    description: "Generate chat completions using Grok AI".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "The message to send to Grok"
                            },
                            "temperature": {
                                "type": "number",
                                "minimum": 0.0,
                                "maximum": 2.0,
                                "description": "Creativity level (0.0 to 2.0)"
                            },
                            "max_tokens": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 131072,
                                "description": "Maximum tokens in response"
                            }
                        },
                        "required": ["message"]
                    }),
                },
                ToolDefinition {
                    name: "code_explain".to_string(),
                    description: "Explain code functionality and structure".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "code": {
                                "type": "string",
                                "description": "The code to explain"
                            },
                            "language": {
                                "type": "string",
                                "description": "Programming language (optional)"
                            },
                            "detail_level": {
                                "type": "string",
                                "enum": ["basic", "detailed", "expert"],
                                "description": "Level of detail in explanation"
                            }
                        },
                        "required": ["code"]
                    }),
                },
                ToolDefinition {
                    name: "code_review".to_string(),
                    description: "Review code for issues and improvements".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "code": {
                                "type": "string",
                                "description": "The code to review"
                            },
                            "focus": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "enum": ["security", "performance", "style", "bugs", "maintainability"]
                                },
                                "description": "Areas to focus on during review"
                            },
                            "language": {
                                "type": "string",
                                "description": "Programming language"
                            }
                        },
                        "required": ["code"]
                    }),
                },
                ToolDefinition {
                    name: "code_generate".to_string(),
                    description: "Generate code from natural language descriptions".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "description": {
                                "type": "string",
                                "description": "Description of what to generate"
                            },
                            "language": {
                                "type": "string",
                                "description": "Target programming language"
                            },
                            "style": {
                                "type": "string",
                                "enum": ["functional", "object-oriented", "procedural"],
                                "description": "Programming style preference"
                            },
                            "include_tests": {
                                "type": "boolean",
                                "description": "Whether to include unit tests"
                            }
                        },
                        "required": ["description"]
                    }),
                },
            ],
        }
    }

    /// Initialize a new session
    pub async fn initialize_session(
        &self,
        session_id: SessionId,
        config: Option<SessionConfig>,
    ) -> Result<()> {
        let mut session_config = config.unwrap_or_default();

        // Apply default model override if present and config matches default
        if let Some(model) = &self.default_model {
            session_config.model = model.clone();
        }

        let session_data = SessionData {
            messages: Vec::new(),
            config: session_config,
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.0.clone(), session_data);

        info!("Initialized new ACP session: {}", session_id.0);
        Ok(())
    }

    /// Handle a chat completion request
    pub async fn handle_chat_completion(
        &self,
        session_id: &SessionId,
        message: &str,
        options: Option<Value>,
    ) -> Result<String> {
        let start_time = std::time::Instant::now();
        info!("🚀 Starting chat completion for session: {}", session_id.0);
        info!("📝 User message: {} chars", message.len());

        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id.0)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))?;

        // Update last activity
        session.last_activity = std::time::Instant::now();

        // Add user message to history
        session.messages.push(json!({
            "role": "user",
            "content": message
        }));

        info!("📚 Session history: {} messages", session.messages.len());

        // Extract options
        let temperature = options
            .as_ref()
            .and_then(|o| o.get("temperature"))
            .and_then(|t| t.as_f64())
            .map(|t| t as f32)
            .unwrap_or(session.config.temperature);

        let max_tokens = options
            .as_ref()
            .and_then(|o| o.get("max_tokens"))
            .and_then(|t| t.as_u64())
            .map(|t| t as u32)
            .unwrap_or(session.config.max_tokens);

        let tool_defs = tools::get_available_tool_definitions();
        info!("🔧 Available tools: {}", tool_defs.len());

        let mut loop_count = 0;
        let max_loops = self.config.acp.max_tool_loop_iterations;

        loop {
            if loop_count >= max_loops {
                let elapsed = start_time.elapsed();
                error!(
                    "❌ Max tool loop iterations reached ({} iterations) after {:?}",
                    max_loops, elapsed
                );
                return Err(anyhow!(
                    "Max tool loop iterations reached ({} iterations). \
                    Consider increasing 'acp.max_tool_loop_iterations' in config or breaking task into smaller steps.",
                    max_loops
                ));
            }
            loop_count += 1;

            let loop_start = std::time::Instant::now();
            info!("🔄 Tool loop iteration {}/{}", loop_count, max_loops);

            // Make request to Grok
            info!(
                "📡 Calling Grok API (model: {}, temp: {}, max_tokens: {})...",
                session.config.model, temperature, max_tokens
            );
            let api_call_start = std::time::Instant::now();

            let response_with_finish = self
                .grok_client
                .chat_completion_with_history(
                    &session.messages,
                    temperature,
                    max_tokens,
                    &session.config.model,
                    Some(tool_defs.clone()),
                )
                .await?;

            let api_duration = api_call_start.elapsed();
            info!("✅ Grok API responded in {:?}", api_duration);

            let response_msg = response_with_finish.message;
            let finish_reason = response_with_finish.finish_reason.as_deref();

            info!("📋 Finish reason: {:?}", finish_reason);

            // Add assistant response to history
            session.messages.push(serde_json::to_value(&response_msg)?);

            // Check finish_reason - if "stop", we're done regardless of tool_calls
            if finish_reason == Some("stop") || finish_reason == Some("end_turn") {
                let elapsed = start_time.elapsed();
                let response_text = content_to_string(response_msg.content.as_ref());
                info!(
                    "✅ Model signaled completion (finish_reason: {:?}) in {:?} ({} loops, {} chars)",
                    finish_reason,
                    elapsed,
                    loop_count,
                    response_text.len()
                );
                return Ok(response_text);
            }

            // Check if we have tool calls to process
            let has_tool_calls = response_msg
                .tool_calls
                .as_ref()
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);

            if !has_tool_calls {
                // No tool calls and no explicit stop - return content
                let elapsed = start_time.elapsed();
                let response_text = content_to_string(response_msg.content.as_ref());
                info!(
                    "✨ Chat completion finished in {:?} ({} loops, {} chars)",
                    elapsed,
                    loop_count,
                    response_text.len()
                );
                return Ok(response_text);
            }

            // We have tool calls to process
            let tool_calls = response_msg.tool_calls.as_ref().unwrap();
            info!("🛠️  Processing {} tool calls", tool_calls.len());

            for (tool_idx, tool_call) in tool_calls.iter().enumerate() {
                let tool_start = std::time::Instant::now();
                info!(
                    "🔨 Tool {}/{}: {}",
                    tool_idx + 1,
                    tool_calls.len(),
                    tool_call.function.name
                );
                let function_name = &tool_call.function.name;
                let arguments = &tool_call.function.arguments;
                let args: Value = serde_json::from_str(arguments).map_err(|e| {
                    error!("❌ Invalid tool arguments for {}: {}", function_name, e);
                    anyhow!("Invalid tool arguments for {}: {}", function_name, e)
                })?;

                debug!("📋 Tool args: {}", arguments);

                // Execute before_tool hooks
                {
                    let hooks = self.hook_manager.read().await;
                    if !hooks.execute_before_tool(function_name, &args)? {
                        session.messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "content": "Tool execution blocked by hook."
                        }));
                        continue;
                    }
                }

                let result = match function_name.as_str() {
                    "read_file" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        tools::read_file(path, &self.security.get_policy())
                    }
                    "write_file" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        let content = args["content"].as_str().ok_or(anyhow!("Missing content"))?;
                        tools::write_file(path, content, &self.security.get_policy())
                    }
                    "list_directory" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        tools::list_directory(path, &self.security.get_policy())
                    }
                    "glob_search" => {
                        let pattern = args["pattern"].as_str().ok_or(anyhow!("Missing pattern"))?;
                        tools::glob_search(pattern, &self.security.get_policy())
                    }
                    "search_file_content" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        let pattern = args["pattern"].as_str().ok_or(anyhow!("Missing pattern"))?;
                        tools::search_file_content(path, pattern, &self.security.get_policy())
                    }
                    "run_shell_command" => {
                        let command = args["command"].as_str().ok_or(anyhow!("Missing command"))?;
                        tools::run_shell_command(command, &self.security.get_policy())
                    }
                    "replace" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        let old_string = args["old_string"]
                            .as_str()
                            .ok_or(anyhow!("Missing old_string"))?;
                        let new_string = args["new_string"]
                            .as_str()
                            .ok_or(anyhow!("Missing new_string"))?;
                        let expected_replacements =
                            args["expected_replacements"].as_u64().map(|n| n as u32);
                        tools::replace(
                            path,
                            old_string,
                            new_string,
                            expected_replacements,
                            &self.security.get_policy(),
                        )
                    }
                    "save_memory" => {
                        let fact = args["fact"].as_str().ok_or(anyhow!("Missing fact"))?;
                        tools::save_memory(fact)
                    }
                    "web_search" => {
                        let query = args["query"].as_str().ok_or(anyhow!("Missing query"))?;
                        tools::web_search(query).await
                    }
                    "web_fetch" => {
                        let url = args["url"].as_str().ok_or(anyhow!("Missing url"))?;
                        tools::web_fetch(url).await
                    }
                    "read_multiple_files" => {
                        let paths_value =
                            args["paths"].as_array().ok_or(anyhow!("Missing paths"))?;
                        let paths: Result<Vec<String>> = paths_value
                            .iter()
                            .map(|v| {
                                v.as_str()
                                    .ok_or(anyhow!("Invalid path"))
                                    .map(|s| s.to_string())
                            })
                            .collect();
                        tools::read_multiple_files(paths?, &self.security.get_policy())
                    }
                    "list_code_definitions" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        tools::list_code_definitions(path, &self.security.get_policy())
                    }
                    _ => Err(anyhow!("Unknown tool: {}", function_name)),
                };

                let content = match result {
                    Ok(s) => {
                        let tool_duration = tool_start.elapsed();
                        info!(
                            "✅ Tool completed in {:?} ({} bytes)",
                            tool_duration,
                            s.len()
                        );
                        s
                    }
                    Err(e) => {
                        let tool_duration = tool_start.elapsed();
                        warn!("⚠️  Tool failed in {:?}: {}", tool_duration, e);
                        format!("Error executing tool {}: {}", function_name, e)
                    }
                };

                // Execute after_tool hooks
                {
                    let hooks = self.hook_manager.read().await;
                    hooks.execute_after_tool(function_name, &args, &content)?;
                }

                // Add tool result to history
                session.messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": content
                }));
            }

            let loop_duration = loop_start.elapsed();
            info!("🔄 Loop iteration completed in {:?}", loop_duration);
            // Continue loop to get next response from model with tool results
        }
    }

    /// Handle code explanation request
    pub async fn handle_code_explain(
        &self,
        session_id: &SessionId,
        code: &str,
        language: Option<&str>,
        detail_level: Option<&str>,
    ) -> Result<String> {
        let detail = detail_level.unwrap_or("detailed");
        let lang_hint = language
            .map(|l| format!(" (language: {})", l))
            .unwrap_or_default();

        let system_prompt = format!(
            "You are an expert code reviewer and teacher. Explain the provided code with {} detail. Focus on:\n\
            - What the code does\n\
            - How it works\n\
            - Key concepts and patterns used\n\
            - Potential improvements\n\
            Be clear and educational in your explanation.",
            detail
        );

        let user_message = format!(
            "Please explain this code{}:\n\n```\n{}\n```",
            lang_hint, code
        );

        self.handle_chat_with_system_prompt(session_id, &user_message, &system_prompt)
            .await
    }

    /// Handle code review request
    pub async fn handle_code_review(
        &self,
        session_id: &SessionId,
        code: &str,
        focus_areas: Option<&[String]>,
        language: Option<&str>,
    ) -> Result<String> {
        let focus = focus_areas
            .map(|areas| format!("Focus areas: {}", areas.join(", ")))
            .unwrap_or_else(|| "Comprehensive review".to_string());

        let lang_hint = language
            .map(|l| format!(" (language: {})", l))
            .unwrap_or_default();

        let system_prompt = format!(
            "You are an expert code reviewer. Review the provided code for:\n\
            - Bugs and potential issues\n\
            - Security vulnerabilities\n\
            - Performance improvements\n\
            - Code style and best practices\n\
            - Maintainability\n\
            Provide specific, actionable feedback. {}",
            focus
        );

        let user_message = format!(
            "Please review this code{}:\n\n```\n{}\n```",
            lang_hint, code
        );

        self.handle_chat_with_system_prompt(session_id, &user_message, &system_prompt)
            .await
    }

    /// Handle code generation request
    pub async fn handle_code_generate(
        &self,
        session_id: &SessionId,
        description: &str,
        language: Option<&str>,
        style: Option<&str>,
        include_tests: Option<bool>,
    ) -> Result<String> {
        let lang = language.unwrap_or("Python");
        let prog_style = style.unwrap_or("object-oriented");
        let tests = if include_tests.unwrap_or(false) {
            "Include comprehensive unit tests."
        } else {
            ""
        };

        let system_prompt = format!(
            "You are an expert software developer. Generate clean, well-documented {} code \
            using {} programming style. Follow best practices and include helpful comments. {}",
            lang, prog_style, tests
        );

        let user_message = format!("Generate code for: {}", description);

        self.handle_chat_with_system_prompt(session_id, &user_message, &system_prompt)
            .await
    }

    /// Handle chat with a specific system prompt
    async fn handle_chat_with_system_prompt(
        &self,
        session_id: &SessionId,
        message: &str,
        system_prompt: &str,
    ) -> Result<String> {
        // Create a temporary session with the system prompt
        let messages = vec![
            json!({
                "role": "system",
                "content": system_prompt
            }),
            json!({
                "role": "user",
                "content": message
            }),
        ];

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&session_id.0)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))?;

        let response_with_finish = self
            .grok_client
            .chat_completion_with_history(
                &messages,
                session.config.temperature,
                session.config.max_tokens,
                &session.config.model,
                None,
            )
            .await?;

        let response = response_with_finish.message;

        debug!(
            "Code operation for session {}: {} -> {}",
            session_id.0,
            message,
            content_to_string(response.content.as_ref())
        );

        Ok(content_to_string(response.content.as_ref()))
    }

    /// Get agent capabilities
    pub fn get_capabilities(&self) -> &GrokAgentCapabilities {
        &self.capabilities
    }

    /// Clean up expired sessions
    pub async fn cleanup_sessions(&self, max_age: std::time::Duration) -> Result<usize> {
        let mut sessions = self.sessions.write().await;
        let now = std::time::Instant::now();
        let initial_count = sessions.len();

        sessions.retain(|session_id, session_data| {
            let expired = now.duration_since(session_data.last_activity) > max_age;
            if expired {
                info!("Cleaning up expired session: {}", session_id);
            }
            !expired
        });

        let cleaned = initial_count - sessions.len();
        if cleaned > 0 {
            info!("Cleaned up {} expired sessions", cleaned);
        }

        Ok(cleaned)
    }

    /// Get session statistics
    pub async fn get_session_stats(&self) -> Result<Value> {
        let sessions = self.sessions.read().await;
        let now = std::time::Instant::now();

        let mut active_sessions = 0;
        let mut total_messages = 0;
        let mut oldest_session = now;

        for session_data in sessions.values() {
            active_sessions += 1;
            total_messages += session_data.messages.len();
            if session_data.created_at < oldest_session {
                oldest_session = session_data.created_at;
            }
        }

        let uptime = now.duration_since(oldest_session).as_secs();

        Ok(json!({
            "active_sessions": active_sessions,
            "total_messages": total_messages,
            "uptime_seconds": uptime,
            "capabilities": {
                "models": self.capabilities.models,
                "features": self.capabilities.features,
                "max_context_length": self.capabilities.max_context_length
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.model, "grok-3");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 4096);
        assert!(config.system_prompt.is_some());
    }

    #[test]
    fn test_capabilities_creation() {
        let capabilities = GrokAcpAgent::create_capabilities();
        assert!(!capabilities.models.is_empty());
        assert!(!capabilities.features.is_empty());
        assert!(!capabilities.tools.is_empty());
        assert!(capabilities.max_context_length > 0);
    }

    #[tokio::test]
    async fn test_session_management() {
        // This would require a mock config and API key for full testing
        // For now, just test the structure
        let session_id = SessionId::new("test-session");
        assert_eq!(session_id.0.as_str(), "test-session");
    }
}
