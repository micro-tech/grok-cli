//! ACP (Agent Client Protocol) integration module
//!
//! This module provides the Grok AI agent implementation for the Agent Client Protocol,
//! enabling seamless integration with Zed editor and other ACP-compatible clients.

use crate::acp::protocol::SessionId;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};

use crate::GrokClient;
use crate::config::Config;
use crate::grok_client_ext::MessageWithFinishReason;
use crate::hooks::HookManager;
use crate::{content_to_string, extract_text_content};

pub mod protocol;
pub mod security;
pub mod slash_commands;
pub mod tools;

use crate::acp::protocol::{
    AGENT_METHOD_NAMES, AgentCapabilities, ContentBlock, ContentChunk, Implementation,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse,
    PermissionOutcome, PromptRequest, PromptResponse, ProtocolVersion, RequestPermissionParams,
    SessionId as ProtocolSessionId, SessionNotification, SessionUpdate, StopReason, TextContent,
};
use security::SecurityManager;

/// Bridge for passing permission requests from the tool executor to the client writer task
#[derive(Debug)]
pub struct PermissionBridge {
    pub outbound:
        mpsc::UnboundedSender<(RequestPermissionParams, oneshot::Sender<PermissionOutcome>)>,
}

impl PermissionBridge {
    pub fn new() -> (
        Self,
        mpsc::UnboundedReceiver<(RequestPermissionParams, oneshot::Sender<PermissionOutcome>)>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { outbound: tx }, rx)
    }
}

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

    /// Per-session always-allow set: tool names that the user has chosen
    /// "Always Allow" for. Subsequent calls to those tools within this session
    /// skip the permission prompt entirely.
    always_allow: std::collections::HashSet<String>,
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
            model: "grok-4-1-fast-reasoning".to_string(),
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
            always_allow: std::collections::HashSet::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.0.clone(), session_data);

        info!("Initialized new ACP session: {}", session_id.0);
        Ok(())
    }

    /// Check if a tool execution is permitted by the user
    pub(crate) async fn check_tool_permission(
        &self,
        session_id: &SessionId,
        function_name: &str,
        args: &Value,
        tool_call_id: &str,
        permission_bridge: Option<&Arc<PermissionBridge>>,
    ) -> Result<bool> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id.0)
            .ok_or_else(|| anyhow!("Session not found"))?;

        if !self.config.acp.require_permission || session.always_allow.contains(function_name) {
            return Ok(true);
        }

        if let Some(bridge) = permission_bridge {
            let req_id = uuid::Uuid::new_v4().to_string();
            let message_summary = serde_json::to_string_pretty(args).unwrap_or_default();

            let params = RequestPermissionParams::new(
                session_id.clone(),
                req_id,
                tool_call_id.to_string(),
                format!("Run {}", function_name),
                format!("Tool {}:\n{}", function_name, message_summary),
                Some(crate::acp::protocol::ToolKind::Execute),
            );

            let (tx, rx) = oneshot::channel();
            if bridge.outbound.send((params, tx)).is_ok() {
                // Drop the write lock before awaiting the response!
                // This allows the rest of the application (like handling the client's response)
                // to read/write the session if needed.
                drop(sessions);

                let timeout_secs = self.config.acp.permission_timeout_secs;
                let outcome_res =
                    tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await;

                // Re-acquire lock to update session state
                let mut sessions = self.sessions.write().await;
                let session = sessions
                    .get_mut(&session_id.0)
                    .ok_or_else(|| anyhow!("Session not found"))?;

                match outcome_res {
                    Ok(Ok(outcome)) => match outcome.option_id.as_str() {
                        "proceed_always" => {
                            session.always_allow.insert(function_name.to_string());
                            return Ok(true);
                        }
                        "proceed_once" => {
                            return Ok(true);
                        }
                        _ => {
                            return Ok(false);
                        }
                    },
                    Ok(Err(_)) => {
                        return Err(anyhow!("Permission bridge closed unexpectedly"));
                    }
                    Err(_) => {
                        return Err(anyhow!(
                            "Timed out waiting for permission ({}s)",
                            timeout_secs
                        ));
                    }
                }
            }
        }

        // If require_permission is true but there's no bridge, default to false
        Ok(false)
    }

    /// Handle a chat completion request
    pub async fn handle_chat_completion(
        &self,
        session_id: &SessionId,
        message: &str,
        options: Option<Value>,
        event_sender: Option<
            tokio::sync::mpsc::UnboundedSender<crate::acp::protocol::SessionUpdate>,
        >,
        permission_bridge: Option<Arc<PermissionBridge>>,
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

            // Make request to Grok — with per-call retry/backoff for Starlink drops.
            // Starlink handovers can take 20-60 s; we need enough retries + delays
            // to outlast a satellite dropout before giving up on this iteration.
            //
            // ⚠️  KNOWN UPSTREAM BUG (grok_api ≤ 0.1.2):
            //     The crate's `from_reqwest` always emits "Request timeout after 30
            //     seconds" regardless of the real configured timeout_secs value.
            //     The hardcoded "30" in that message is NOT the actual timeout — it
            //     is a bug in the crate.  Our real timeout is `config.timeout_secs`.
            const MAX_API_RETRIES: u32 = 5;
            // Starlink-safe delays: 5 → 10 → 20 → 40 → 60 s (capped)
            const BASE_RETRY_DELAY_SECS: u64 = 5;
            const MAX_RETRY_DELAY_SECS: u64 = 60;

            info!(
                "📡 Calling Grok API (model: {}, temp: {}, max_tokens: {}, \
                 real_timeout: {}s)…",
                session.config.model, temperature, max_tokens, self.config.timeout_secs,
            );
            // NOTE: if you see "Request timeout after 30 seconds" the "30" is a
            // hardcoded value in the grok_api crate error formatter — the actual
            // HTTP timeout driving the request is the real_timeout printed above.
            let api_call_start = std::time::Instant::now();

            let response_with_finish = {
                let mut attempt = 0u32;
                loop {
                    attempt += 1;
                    match self
                        .grok_client
                        .chat_completion_with_history(
                            &session.messages,
                            temperature,
                            max_tokens,
                            &session.config.model,
                            Some(tool_defs.clone()),
                        )
                        .await
                    {
                        Ok(resp) => break resp,
                        Err(e) => {
                            let is_retriable = crate::utils::network::detect_network_drop(&e) || {
                                let msg = e.to_string().to_lowercase();
                                msg.contains("timeout")
                                    || msg.contains("timed out")
                                    || msg.contains("reset")
                                    || msg.contains("connection")
                                    || msg.contains("503")
                                    || msg.contains("502")
                                    || msg.contains("504")
                            };

                            if attempt <= MAX_API_RETRIES && is_retriable {
                                // Exponential backoff capped at MAX_RETRY_DELAY_SECS.
                                // Using saturating_mul + min(6) on the shift to avoid
                                // overflow on large attempt counts.
                                let delay = BASE_RETRY_DELAY_SECS
                                    .saturating_mul(1u64 << (attempt - 1).min(6))
                                    .min(MAX_RETRY_DELAY_SECS);

                                let err_kind = {
                                    let m = e.to_string().to_lowercase();
                                    if m.contains("timeout") || m.contains("timed out") {
                                        // ⚠️  grok_api always says "30 seconds" here —
                                        //     the real timeout is config.timeout_secs
                                        format!(
                                            "TIMEOUT (real timeout={}s; grok_api \
                                             hardcodes '30' in the error message)",
                                            self.config.timeout_secs
                                        )
                                    } else {
                                        "NETWORK DROP".to_string()
                                    }
                                };

                                warn!(
                                    "⚠️  API call failed (attempt {}/{}) [{}]: {}. \
                                     Waiting {}s before retry (Starlink recovery)…",
                                    attempt, MAX_API_RETRIES, err_kind, e, delay
                                );
                                sleep(Duration::from_secs(delay)).await;
                                continue;
                            } else {
                                let tip = {
                                    let m = e.to_string().to_lowercase();
                                    if m.contains("timeout") || m.contains("timed out") {
                                        format!(
                                            "\n💡 The error says '30 seconds' but that is a \
                                             grok_api crate bug — your real timeout_secs={s}. \
                                             If this is a Starlink dropout the connection usually \
                                             recovers; try again or increase max_tool_loop_\
                                             iterations in .grok/config.toml.",
                                            s = self.config.timeout_secs
                                        )
                                    } else {
                                        String::new()
                                    }
                                };
                                if is_retriable {
                                    error!(
                                        "❌ API call failed after {} retries: {}{}",
                                        MAX_API_RETRIES, e, tip
                                    );
                                } else {
                                    error!("❌ Non-retriable API error: {}{}", e, tip);
                                }
                                return Err(anyhow!("{}{}", e, tip));
                            }
                        }
                    }
                }
            };
            // suppress unused-variable warning on last_err path
            let _ = &response_with_finish;

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

                // Emit ToolCall event to ACP client
                if let Some(sender) = &event_sender {
                    let tool_call_event = crate::acp::protocol::ToolCall {
                        tool_call_id: tool_call.id.clone(),
                        title: format!("Running tool: {}", function_name),
                        kind: Some(crate::acp::protocol::ToolKind::Execute),
                        status: Some(crate::acp::protocol::ToolCallStatus::InProgress),
                        raw_input: Some(args.clone()),
                        raw_output: None,
                        locations: None,
                        content: None,
                    };
                    let _ = sender.send(crate::acp::protocol::SessionUpdate::ToolCall(
                        tool_call_event,
                    ));
                }

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

                // --- PERMISSION GATE ---
                if self.config.acp.require_permission
                    && !session.always_allow.contains(function_name.as_str())
                    && let Some(bridge) = &permission_bridge
                {
                    let req_id = uuid::Uuid::new_v4().to_string();
                    let message_summary = serde_json::to_string_pretty(&args).unwrap_or_default();

                    let params = RequestPermissionParams::new(
                        session_id.clone(),
                        req_id,
                        tool_call.id.clone(),
                        format!("Run {}", function_name),
                        format!("Tool {}:\n{}", function_name, message_summary),
                        Some(crate::acp::protocol::ToolKind::Execute),
                    );

                    let (tx, rx) = oneshot::channel();
                    if bridge.outbound.send((params, tx)).is_ok() {
                        let timeout_secs = self.config.acp.permission_timeout_secs;
                        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx)
                            .await
                        {
                            Ok(Ok(outcome)) => {
                                match outcome.option_id.as_str() {
                                    "proceed_always" => {
                                        session.always_allow.insert(function_name.clone());
                                        // fall through
                                    }
                                    "proceed_once" => {
                                        // fall through
                                    }
                                    _ => {
                                        session.messages.push(json!({
                                            "role": "tool",
                                            "tool_call_id": tool_call.id,
                                            "content": "User rejected the tool execution."
                                        }));
                                        continue;
                                    }
                                }
                            }
                            Ok(Err(_)) => {
                                return Err(anyhow!("Permission bridge closed unexpectedly"));
                            }
                            Err(_) => {
                                return Err(anyhow!(
                                    "Timed out waiting for permission ({}s)",
                                    timeout_secs
                                ));
                            }
                        }
                    }
                }
                // --- END PERMISSION GATE ---

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

                let (content, status) = match result {
                    Ok(s) => {
                        let tool_duration = tool_start.elapsed();
                        info!(
                            "✅ Tool completed in {:?} ({} bytes)",
                            tool_duration,
                            s.len()
                        );
                        (s, crate::acp::protocol::ToolCallStatus::Completed)
                    }
                    Err(e) => {
                        let tool_duration = tool_start.elapsed();
                        warn!("⚠️  Tool failed in {:?}: {}", tool_duration, e);
                        (
                            format!("Error executing tool {}: {}", function_name, e),
                            crate::acp::protocol::ToolCallStatus::Failed,
                        )
                    }
                };

                // Emit ToolCallUpdate event
                if let Some(sender) = &event_sender {
                    let tool_call_update = crate::acp::protocol::ToolCallUpdate {
                        tool_call_id: tool_call.id.clone(),
                        kind: None,
                        status: Some(status),
                        locations: None,
                        content: Some(vec![crate::acp::protocol::ToolCallContent::Text(
                            crate::acp::protocol::TextContent::new(content.clone()),
                        )]),
                    };
                    let _ = sender.send(crate::acp::protocol::SessionUpdate::ToolCallUpdate(
                        tool_call_update,
                    ));
                }

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
    /// Returns `true` if the given tool name is in the session's always-allow
    /// set (i.e. the user previously chose "Always Allow" for this tool).
    ///
    /// Silently returns `false` if the session no longer exists.
    pub async fn is_always_allowed(&self, session_id: &SessionId, tool_name: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id.0)
            .map(|s| s.always_allow.contains(tool_name))
            .unwrap_or(false)
    }

    /// Adds `tool_name` to the session's always-allow set so that future calls
    /// to that tool within the same session skip the permission prompt.
    ///
    /// Silently no-ops if the session no longer exists.
    pub(crate) async fn set_always_allowed(&self, session_id: &SessionId, tool_name: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id.0) {
            session.always_allow.insert(tool_name.to_string());
            info!(
                "Always-allow granted for tool '{}' in session '{}'",
                tool_name, session_id.0
            );
        }
    }

    pub fn get_capabilities(&self) -> &GrokAgentCapabilities {
        &self.capabilities
    }

    /// Clear the conversation history for a session (used by the `/clear` slash command).
    pub async fn clear_session_history(&self, session_id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id.0) {
            session.messages.clear();
            info!("Cleared conversation history for session: {}", session_id.0);
        }
        Ok(())
    }

    /// Return a clone of the [`SessionConfig`] for a session.
    ///
    /// Used by the `/context` slash command to report the active model, temperature, etc.
    pub async fn get_session_config(&self, session_id: &SessionId) -> Result<SessionConfig> {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id.0)
            .map(|s| s.config.clone())
            .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))
    }

    /// Return the number of messages currently stored in the session history.
    ///
    /// Used by the `/context` slash command to show conversation depth.
    pub async fn get_session_message_count(&self, session_id: &SessionId) -> Result<usize> {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id.0)
            .map(|s| s.messages.len())
            .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))
    }

    /// Switch the model used for a session (used by the `/model` slash command).
    pub async fn set_session_model(&self, session_id: &SessionId, model: String) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id.0) {
            let old_model = session.config.model.clone();
            session.config.model = model.clone();
            info!(
                "Switched model from '{}' to '{}' for session: {}",
                old_model, model, session_id.0
            );
        } else {
            return Err(anyhow!("Session not found: {}", session_id.0));
        }
        Ok(())
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
        assert_eq!(config.model, "grok-4-1-fast-reasoning");
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

    /// Verify the always-allow round-trip:
    /// set_always_allowed  →  is_always_allowed returns true for that tool,
    ///                        false for a different tool,
    ///                        and silently no-ops for an unknown session.
    #[tokio::test]
    async fn test_always_allow_round_trip() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // Build a minimal sessions map with one session entry.
        let session_id = SessionId::new("sess-perm-test");
        let session_data = SessionData {
            messages: Vec::new(),
            config: SessionConfig::default(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            always_allow: std::collections::HashSet::new(),
        };
        let mut map: HashMap<String, SessionData> = HashMap::new();
        map.insert(session_id.0.clone(), session_data);
        let sessions: Arc<RwLock<HashMap<String, SessionData>>> = Arc::new(RwLock::new(map));

        // Helper closures that operate directly on the sessions map,
        // mirroring is_always_allowed / set_always_allowed logic.
        let is_allowed = |sessions: &HashMap<String, SessionData>, sid: &str, tool: &str| {
            sessions
                .get(sid)
                .map(|s| s.always_allow.contains(tool))
                .unwrap_or(false)
        };

        // Before any grant: both tools should be denied.
        {
            let sess = sessions.read().await;
            assert!(
                !is_allowed(&sess, &session_id.0, "run_shell_command"),
                "should not be allowed before grant"
            );
            assert!(
                !is_allowed(&sess, &session_id.0, "read_file"),
                "read_file should not be allowed before grant"
            );
        }

        // Grant always-allow for run_shell_command.
        {
            let mut sess = sessions.write().await;
            if let Some(s) = sess.get_mut(&session_id.0) {
                s.always_allow.insert("run_shell_command".to_string());
            }
        }

        // After grant: run_shell_command allowed, read_file still not.
        {
            let sess = sessions.read().await;
            assert!(
                is_allowed(&sess, &session_id.0, "run_shell_command"),
                "run_shell_command should be allowed after grant"
            );
            assert!(
                !is_allowed(&sess, &session_id.0, "read_file"),
                "read_file must remain unaffected by a different tool's grant"
            );
        }

        // Missing session must return false without panicking.
        {
            let sess = sessions.read().await;
            assert!(
                !is_allowed(&sess, "no-such-session", "run_shell_command"),
                "missing session should silently return false"
            );
        }
    }

    #[tokio::test]
    async fn test_permission_outcomes() {
        use crate::acp::protocol::PermissionOutcome;
        use serde_json::json;

        let mut config = Config::default();
        config.api_key = Some("dummy_key".to_string());
        config.acp.require_permission = true;
        config.acp.permission_timeout_secs = 1;

        let agent = GrokAcpAgent::new(config, None).await.unwrap();
        let session_id = SessionId::new("test-session-perm");
        agent
            .initialize_session(session_id.clone(), None)
            .await
            .unwrap();

        let tool_name = "test_tool";
        let args = json!({"arg": "val"});
        let tool_call_id = "call_123";

        // Test 1: Proceed Always
        {
            let (bridge, mut rx) = PermissionBridge::new();
            let bridge_arc = Arc::new(bridge);

            // Spawn a task to act as the client and respond
            tokio::spawn(async move {
                if let Some((_, reply_tx)) = rx.recv().await {
                    let _ = reply_tx.send(PermissionOutcome {
                        request_id: "req".to_string(),
                        option_id: "proceed_always".to_string(),
                    });
                }
            });

            let allowed = agent
                .check_tool_permission(
                    &session_id,
                    tool_name,
                    &args,
                    tool_call_id,
                    Some(&bridge_arc),
                )
                .await
                .unwrap();
            assert!(allowed, "Tool should be allowed on proceed_always");
            assert!(
                agent.is_always_allowed(&session_id, tool_name).await,
                "Tool should be in always_allow set"
            );
        }

        // Test 2: Always Allow Fast Path
        {
            // Tool is already in always_allow set from Test 1, so it should allow immediately without using bridge
            let allowed = agent
                .check_tool_permission(&session_id, tool_name, &args, tool_call_id, None)
                .await
                .unwrap();
            assert!(allowed, "Tool should be allowed immediately on fast path");
        }

        // Test 3: Proceed Once
        {
            let tool_name_once = "test_tool_once";
            let (bridge, mut rx) = PermissionBridge::new();
            let bridge_arc = Arc::new(bridge);

            tokio::spawn(async move {
                if let Some((_, reply_tx)) = rx.recv().await {
                    let _ = reply_tx.send(PermissionOutcome {
                        request_id: "req".to_string(),
                        option_id: "proceed_once".to_string(),
                    });
                }
            });

            let allowed = agent
                .check_tool_permission(
                    &session_id,
                    tool_name_once,
                    &args,
                    tool_call_id,
                    Some(&bridge_arc),
                )
                .await
                .unwrap();
            assert!(allowed, "Tool should be allowed on proceed_once");
            assert!(
                !agent.is_always_allowed(&session_id, tool_name_once).await,
                "Tool should NOT be in always_allow set"
            );
        }

        // Test 4: Cancel
        {
            let tool_name_cancel = "test_tool_cancel";
            let (bridge, mut rx) = PermissionBridge::new();
            let bridge_arc = Arc::new(bridge);

            tokio::spawn(async move {
                if let Some((_, reply_tx)) = rx.recv().await {
                    let _ = reply_tx.send(PermissionOutcome {
                        request_id: "req".to_string(),
                        option_id: "cancel".to_string(),
                    });
                }
            });

            let allowed = agent
                .check_tool_permission(
                    &session_id,
                    tool_name_cancel,
                    &args,
                    tool_call_id,
                    Some(&bridge_arc),
                )
                .await
                .unwrap();
            assert!(!allowed, "Tool should be rejected on cancel");
        }

        // Test 5: Timeout
        {
            let tool_name_timeout = "test_tool_timeout";
            let (bridge, mut _rx) = PermissionBridge::new(); // Never respond
            let bridge_arc = Arc::new(bridge);

            let result = agent
                .check_tool_permission(
                    &session_id,
                    tool_name_timeout,
                    &args,
                    tool_call_id,
                    Some(&bridge_arc),
                )
                .await;
            assert!(result.is_err(), "Should timeout and return error");
            assert!(result.unwrap_err().to_string().contains("Timed out"));
        }
    }
}
