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

use crate::config::{Config, ThinkingMode};
use crate::content_to_string;
use crate::hooks::HookManager;
use crate::router::AppRouter;
use serde::{Deserialize, Serialize};

pub mod protocol;
pub mod security;
pub mod slash_commands;
pub mod tools;

use crate::acp::protocol::{PermissionOutcome, RequestPermissionParams};
use security::SecurityManager;

/// Bridge for passing permission requests from the tool executor to the client writer task
#[derive(Debug)]
pub struct PermissionBridge {
    /// Outbound channel: `(request_id, params, response_sender)`.
    ///
    /// The `request_id` is used as the JSON-RPC message `id` when forwarding
    /// the request to the client and correlating the response.  It is NOT
    /// embedded in `params` — the ACP spec does not include it there.
    pub outbound: mpsc::UnboundedSender<(
        String,
        RequestPermissionParams,
        oneshot::Sender<PermissionOutcome>,
    )>,
}

impl PermissionBridge {
    pub fn new() -> (
        Self,
        mpsc::UnboundedReceiver<(
            String,
            RequestPermissionParams,
            oneshot::Sender<PermissionOutcome>,
        )>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { outbound: tx }, rx)
    }
}

/// Grok AI agent implementation for ACP
pub struct GrokAcpAgent {
    /// Application-level AI router — created lazily on first actual
    /// chat-completion request so that `grok acp stdio` starts instantly
    /// even when an API key is present.
    router: std::sync::OnceLock<AppRouter>,

    /// Agent configuration
    config: Config,

    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,

    /// Agent capabilities (computed lazily on first access to avoid
    /// expensive tool schema construction during Zed ACP startup)
    capabilities: std::sync::OnceLock<GrokAgentCapabilities>,

    /// Security manager (lazy — created on first use)
    security: std::sync::OnceLock<SecurityManager>,

    /// Hook manager (lazy — created on first use)
    hook_manager: std::sync::OnceLock<Arc<RwLock<HookManager>>>,

    /// Default model override
    default_model: Option<String>,
}

/// Session data for tracking conversation state
#[derive(Debug, Clone)]
struct SessionData {
    /// The client-provided working directory for this session
    cwd: String,

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

    /// Slash commands registered by the client (e.g. Gemini CLI sends these
    /// back in a session/update notification after session/new).
    /// Stored for future passthrough / forwarding support.
    client_commands: Vec<String>,

    /// Bayesian inference engine for this session
    bayes_engine: crate::bayes::BayesianEngine,

    /// Active session goal set via `/goal <text>`.
    /// Injected into every refined prompt as a system note so the model
    /// always interprets messages through the lens of this goal.
    current_goal: Option<String>,
}

impl SessionData {
    /// Refines the user prompt by checking Bayesian state for vagueness, repetition,
    /// and uncertainty to shape the LLM's response appropriately.
    pub fn refine_prompt(&mut self, message: &str) -> String {
        self.bayes_engine.update_from_text(message);

        let mut refined_message = message.to_string();

        // 1. Detect Repetition (Intent Drift)
        let mut is_repetition = false;
        if let Some(prev) = self
            .messages
            .iter()
            .rev()
            .find(|m| m["role"] == "user")
            .and_then(|m| m["content"].as_str())
        {
            // Strip any system notes we previously appended to accurately compare
            let prev_clean = prev.split("\n\n[System Note:").next().unwrap_or(prev);
            if prev_clean.trim().to_lowercase() == message.trim().to_lowercase() {
                is_repetition = true;
            }
        }

        if is_repetition {
            refined_message = format!(
                "{}\n\n[System Note: The user seems to be repeating themselves. They might be experiencing intent drift or frustration. Propose alternative interpretations or check if they need a different approach.]",
                refined_message
            );
        }

        // 2. High Uncertainty (Clarification)
        if self.bayes_engine.is_high_uncertainty() {
            refined_message = format!(
                "{}\n\n[System Note: Bayesian uncertainty is high. Please ask a clarifying question before proceeding if you are not sure what to do.]",
                refined_message
            );
        }

        // 3. Vagueness
        if self.bayes_engine.is_vague() {
            refined_message = format!(
                "{}\n\n[System Note: The user's request is vague. Propose possible interpretations to help clarify their goal.]",
                refined_message
            );
        }

        // 4. Active Goal injection
        if let Some(ref goal) = self.current_goal {
            refined_message = format!(
                "{}\n\n[Active Goal: {}  — interpret this message in the context of achieving this goal.]",
                refined_message, goal
            );
        }

        refined_message
    }
}

/// Session-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Model to use for this session
    pub model: String,

    /// Temperature setting
    pub temperature: f32,

    /// Maximum tokens per response
    pub max_tokens: u32,

    /// System prompt for this session
    pub system_prompt: Option<String>,

    /// Reasoning / thinking mode for this session.
    /// Passed as `reasoning_effort` to the API for models that support it.
    pub thinking_mode: ThinkingMode,
}

/// Snapshot of a session that can be written to disk and reloaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PersistedSession {
    pub(crate) session_id: String,
    pub(crate) cwd: String,
    pub(crate) messages: Vec<serde_json::Value>,
    pub(crate) config: SessionConfig,
    pub(crate) current_goal: Option<String>,
    pub(crate) always_allow: Vec<String>,
    pub(crate) saved_at_unix: u64,
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
            model: "grok-4.3".to_string(),
            temperature: 0.5, // Lower temperature for more deterministic coding output
            // grok-4.3 supports higher output token limits; 16 384 balances
            // detailed responses with reasonable response times.
            max_tokens: 16_384,
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
            thinking_mode: ThinkingMode::Off,
        }
    }
}

impl GrokAcpAgent {
    /// Create a new Grok ACP agent
    pub async fn new(config: Config, default_model: Option<String>) -> Result<Self> {
        // NOTE: SecurityManager and HookManager are now created lazily
        // via get_security() / get_hook_manager() on first use.
        // This keeps `grok acp stdio` startup extremely fast (task 126).
        Ok(Self {
            router: std::sync::OnceLock::new(),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            capabilities: std::sync::OnceLock::new(),
            security: std::sync::OnceLock::new(),
            hook_manager: std::sync::OnceLock::new(),
            default_model,
        })
    }

    /// Return agent capabilities, computing them lazily on first access.
    /// This avoids the expensive tool schema construction during Zed ACP startup.
    pub fn capabilities(&self) -> &GrokAgentCapabilities {
        self.capabilities.get_or_init(Self::create_capabilities)
    }

    /// Return a clone of the underlying [`AppRouter`], lazily creating it
    /// on first use if an API key is configured.  This keeps `new()` fast
    /// for ACP stdio startup.
    fn get_router(&self) -> Result<AppRouter> {
        if self.router.get().is_none() {
            if let Some(ref api_key) = self.config.api_key {
                if let Ok(r) = AppRouter::new(api_key, self.config.timeout_secs) {
                    let _ = self.router.set(r);
                }
            }
        }

        self.router.get().cloned().ok_or_else(|| {
            anyhow!(
                "API key not configured. \
                     Set the GROK_API_KEY environment variable and restart the agent, \
                     or use 'grok config set api_key <key>'."
            )
        })
    }

    /// Return a reference to the SecurityManager, lazily initializing it
    /// (and trusting the current directory) on first use.
    pub fn get_security(&self) -> &SecurityManager {
        self.security.get_or_init(|| {
            let sm = SecurityManager::new();
            if let Ok(cwd) = std::env::current_dir() {
                let canonical_cwd = cwd.canonicalize().unwrap_or(cwd);
                sm.add_trusted_directory(canonical_cwd);
            }
            sm
        })
    }

    /// Return a reference to the HookManager, lazily initializing it on first use.
    fn get_hook_manager(&self) -> &Arc<RwLock<HookManager>> {
        self.hook_manager
            .get_or_init(|| Arc::new(RwLock::new(HookManager::new())))
    }

    /// Public helper to add a trusted directory (used by workspace registration
    /// code in the ACP command handler). Lazily initializes SecurityManager.
    pub fn add_trusted_directory(&self, path: std::path::PathBuf) {
        self.get_security().add_trusted_directory(path);
    }

    /// Create agent capabilities
    fn create_capabilities() -> GrokAgentCapabilities {
        GrokAgentCapabilities {
            models: vec![
                "grok-4.3".to_string(), // Default — 1 M token context
                "grok-3".to_string(),
                "grok-3-mini".to_string(),
                "grok-2-vision-1212".to_string(),
                "grok-2".to_string(), // Fallback
            ],
            // grok-4.3 exposes a 1,048,576-token context window.
            // This is reported here so ACP clients (e.g. Zed) can make
            // informed decisions about context insertion.
            max_context_length: 1_048_576,
            features: vec![
                "chat_completion".to_string(),
                "code_generation".to_string(),
                "code_review".to_string(),
                "code_explanation".to_string(),
                "streaming".to_string(),
                "function_calling".to_string(),
                "1m_context".to_string(),
                "vision".to_string(),
            ],
            // Build the tool list live from the registry so this list
            // automatically reflects any newly added tools without requiring
            // manual updates here.
            tools: crate::tools::registry::get_available_tool_definitions()
                .into_iter()
                .filter_map(|v| {
                    let func = v.get("function")?;
                    Some(ToolDefinition {
                        name: func.get("name")?.as_str()?.to_string(),
                        description: func
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        parameters: func.get("parameters").cloned().unwrap_or(json!({})),
                    })
                })
                .collect(),
        }
    }

    /// Initialize a new session
    pub async fn initialize_session(
        &self,
        session_id: SessionId,
        cwd: String,
        config: Option<SessionConfig>,
        event_sender: Option<
            tokio::sync::mpsc::UnboundedSender<crate::acp::protocol::SessionUpdate>,
        >,
    ) -> Result<()> {
        let mut session_config = config.unwrap_or_default();

        // Apply default model override if present and config matches default
        if let Some(model) = &self.default_model {
            session_config.model = model.clone();
        }

        let mut session_data = SessionData {
            cwd,
            messages: Vec::new(),
            config: session_config,
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            always_allow: std::collections::HashSet::new(),
            client_commands: Vec::new(),
            bayes_engine: crate::bayes::BayesianEngine::new_with_config(&self.config.bayesian),
            current_goal: None,
        };

        // --- Task 102: Knowledge Pack Loader ---
        // Load project-local knowledge files (knowledge/*.md, knowledge/*.json) and
        // inject them as a system message so the model has project-specific context.
        if let Ok(knowledge) = crate::knowledge::loader::KnowledgeLoader::load() {
            let entries = knowledge.get_all();
            if !entries.is_empty() {
                let combined: String = entries
                    .iter()
                    .map(|e| format!("## {}\n{}", e.source, e.content))
                    .collect::<Vec<_>>()
                    .join("\n\n---\n\n");
                session_data.messages.push(serde_json::json!({
                    "role": "system",
                    "content": format!("## Project Knowledge\n\nThe following project-specific knowledge has been loaded from the `knowledge/` directory:\n\n{}", combined),
                }));
                tracing::info!(
                    "Injected {} knowledge pack(s) into session {}",
                    entries.len(),
                    session_id.0
                );
            }
        }

        // --- Task 103: Session DNA ---
        // Load session_dna.json (tone, verbosity, coding_style, etc.) and append
        // its fields to the system prompt so the model adopts the user's preferred
        // persona and style throughout the session.
        let dna = crate::session::dna::SessionDna::load();
        if let Some(sys_msg) = session_data
            .messages
            .iter_mut()
            .find(|m| m["role"] == "system")
        {
            if let Some(content) = sys_msg["content"].as_str() {
                let mut prompt = content.to_string();
                dna.inject_into_prompt(&mut prompt);
                sys_msg["content"] = serde_json::Value::String(prompt);
            }
        } else {
            // No system message yet -- create one from DNA alone
            let mut prompt = String::new();
            dna.inject_into_prompt(&mut prompt);
            if !prompt.trim().is_empty() {
                session_data.messages.push(serde_json::json!({
                    "role": "system",
                    "content": prompt.trim().to_string(),
                }));
            }
        }
        tracing::debug!(
            "Session DNA injected: tone={}, verbosity={}",
            dna.tone,
            dna.verbosity
        );

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.0.clone(), session_data);

        // ── Advertise slash commands to the client ─────────────────────────────
        // This is the critical fix: ACP clients (Zed, etc.) only show /commands
        // if the agent sends an `available_commands_update` notification.
        if let Some(sender) = event_sender {
            let commands = crate::acp::slash_commands::get_available_commands();
            let update = crate::acp::protocol::SessionUpdate::AvailableCommandsUpdate(
                crate::acp::protocol::AvailableCommandsUpdate::new(commands.clone()),
            );
            let _ = sender.send(update);
            info!("Sent {} slash commands to ACP client", commands.len());
        }

        info!("Initialized new ACP session: {}", session_id.0);
        Ok(())
    }

    /// Check if a tool execution is permitted by the user
    #[allow(dead_code)]
    pub(crate) async fn check_tool_permission(
        &self,
        session_id: &SessionId,
        function_name: &str,
        _args: &Value,
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

            let params = RequestPermissionParams::new(
                session_id.clone(),
                tool_call_id.to_string(),
                Some(format!("Run {}", function_name)),
                Some(crate::acp::protocol::ToolKind::Execute),
            );

            let (tx, rx) = oneshot::channel();
            if bridge.outbound.send((req_id, params, tx)).is_ok() {
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
                    Ok(Ok(outcome)) => {
                        if outcome.is_cancelled() {
                            return Ok(false);
                        }
                        // Any `selected` outcome is treated as approval.
                        // Record it permanently for the session if "Always Allow".
                        if outcome.is_always_allow() {
                            session.always_allow.insert(function_name.to_string());
                        }
                        return Ok(true);
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

        // ── Phase 1: Session setup (brief write lock) ─────────────────────────────
        // All trimming, compression, and option extraction happen here.  After this
        // block the write lock is released so slash commands and context queries are
        // not blocked during the potentially-long API call loop.
        //
        // TODO: make compression lock-free (currently makes an async API call
        //       while holding the write lock — known limitation).
        let (
            mut messages,
            temperature,
            max_tokens,
            model,
            thinking_mode,
            mut local_bayes,
            local_always_allow,
        ) = {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(&session_id.0)
                .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))?;

            // Update last activity
            session.last_activity = std::time::Instant::now();

            let refined_message = session.refine_prompt(message);

            // Add user message to history
            session.messages.push(json!({
                "role": "user",
                "content": refined_message
            }));

            info!("📚 Session history: {} messages", session.messages.len());

            // ── 1. Per-message truncation ────────────────────────────────────────
            // Cap individual tool-result messages so a single large file read
            // cannot consume the whole context window.
            let max_tool_chars = self.config.acp.max_tool_result_chars;
            if max_tool_chars > 0 {
                truncate_tool_results(&mut session.messages, max_tool_chars);
            }

            // ── 2. Count-based trim ──────────────────────────────────────────────
            // Keep the most recent max_history_messages entries so the model always
            // has fresh context without exceeding the API context window.
            // We trim here (after adding the user turn) so we never split a
            // tool-call sequence that was already committed to history.
            let max_history = self.config.acp.max_history_messages;
            if session.messages.len() > max_history {
                let trim_count = session.messages.len() - max_history;
                session.messages.drain(0..trim_count);
                debug!(
                    "Trimmed {} old messages from session history (keeping last {})",
                    trim_count, max_history
                );
            }

            // ── 3. Token-budget trim ─────────────────────────────────────────────
            // Even within the message-count limit, rich tool outputs can bloat
            // the context beyond the model's token window.  Estimate tokens
            // (4 chars ≈ 1 token) and drop oldest messages until we fit.
            // Use a model-aware budget: grok-4.x gets the 1 M-token budget;
            // grok-3 and older use the legacy 220 k budget.
            let max_ctx_tokens = model_context_budget(
                &session.config.model,
                self.config.acp.max_context_tokens,
                self.config.acp.grok4_max_context_tokens,
            );
            let estimated = estimate_tokens(&session.messages);
            if estimated > max_ctx_tokens {
                let before = session.messages.len();
                trim_to_token_budget(&mut session.messages, max_ctx_tokens);
                warn!(
                    "⚠️  Context trimmed by token budget: ~{} estimated tokens > {} limit. \
                     Dropped {} old messages (kept {}).",
                    estimated,
                    max_ctx_tokens,
                    before - session.messages.len(),
                    session.messages.len()
                );
            }

            // ── 4. Smart compression (summarise-and-archive instead of drop) ──────
            // When the context exceeds the compression threshold we call the AI to
            // summarise the oldest chunk of messages, archive the raw messages to
            // disk, and replace them with a compact notice.  If the API call fails
            // (Starlink drop) we restore the drained messages and fall through.
            if self.config.acp.auto_compress {
                // Use the same model-aware budget for the compression threshold.
                let active_ctx_budget = model_context_budget(
                    &session.config.model,
                    self.config.acp.max_context_tokens,
                    self.config.acp.grok4_max_context_tokens,
                );
                let threshold = (active_ctx_budget as f64
                    * self.config.acp.compression_threshold as f64)
                    as usize;
                let estimated = estimate_tokens(&session.messages);

                if estimated > threshold {
                    // Collect indices of non-system messages (preserve system at index 0).
                    let non_system_indices: Vec<usize> = session
                        .messages
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| m.get("role").and_then(|r| r.as_str()) != Some("system"))
                        .map(|(i, _)| i)
                        .collect();

                    let compress_count = ((non_system_indices.len() as f64
                        * self.config.acp.compression_chunk_ratio as f64)
                        as usize)
                        .max(4)
                        .min(non_system_indices.len());

                    if compress_count > 0 {
                        // Drain the oldest `compress_count` non-system messages.
                        // They start at the first non-system index; because messages
                        // are ordered and non_system_indices[0] is the lowest index,
                        // we can drain a contiguous slice.
                        let start = non_system_indices[0];
                        let end = non_system_indices
                            .get(compress_count - 1)
                            .copied()
                            .unwrap_or(start);
                        let to_compress: Vec<Value> = session.messages.drain(start..=end).collect();

                        let tokens_saved = estimate_tokens(&to_compress);
                        let model = session.config.model.clone();

                        match self.get_router() {
                            Ok(router) => {
                                match crate::memory::context_compressor::compress(
                                    &to_compress,
                                    &router,
                                    &model,
                                )
                                .await
                                {
                                    Ok((summary, key_facts)) => {
                                        use crate::memory::context_archive::{
                                            ContextArchive, ContextChunk,
                                        };
                                        match ContextArchive::for_session(&session_id.0) {
                                            Ok(mut archive) => {
                                                let chunk_id = archive.next_chunk_id();
                                                let chunk = ContextChunk {
                                                    chunk_id,
                                                    session_id: session_id.0.clone(),
                                                    created_at: chrono::Utc::now(),
                                                    message_count: to_compress.len(),
                                                    estimated_tokens_saved: tokens_saved,
                                                    summary,
                                                    key_facts,
                                                    raw_messages: to_compress,
                                                };
                                                if let Err(e) = archive.save_chunk(&chunk) {
                                                    warn!(
                                                        "⚠️  Failed to save context archive chunk: {}",
                                                        e
                                                    );
                                                }
                                                let insert_at = if session
                                                    .messages
                                                    .first()
                                                    .and_then(|m| m.get("role"))
                                                    .and_then(|r| r.as_str())
                                                    == Some("system")
                                                {
                                                    1
                                                } else {
                                                    0
                                                };
                                                let notice = build_archive_notice(&chunk);
                                                session.messages.insert(insert_at, notice);
                                                warn!(
                                                    "📦 Archived {} messages → chunk #{} \
                                                     (~{} tokens saved). \
                                                     Active context: {} messages.",
                                                    chunk.message_count,
                                                    chunk_id,
                                                    tokens_saved,
                                                    session.messages.len()
                                                );
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "⚠️  Could not open context archive, \
                                                     messages dropped: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // Compression failed — restore drained messages so
                                        // we don't silently lose history.
                                        warn!(
                                            "⚠️  Context compression failed (network drop?): {}. \
                                             Restoring {} messages.",
                                            e,
                                            to_compress.len()
                                        );
                                        for (offset, msg) in to_compress.into_iter().enumerate() {
                                            session.messages.insert(start + offset, msg);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("⚠️  No router available for compression: {}", e);
                                // Restore messages
                                for (offset, msg) in to_compress.into_iter().enumerate() {
                                    session.messages.insert(start + offset, msg);
                                }
                            }
                        }
                    }
                }
            }

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

            // Clone everything needed for the lock-free loop.
            let msgs = session.messages.clone();
            let mdl = session.config.model.clone();
            let thk = session.config.thinking_mode.clone();
            let bayes = session.bayes_engine.clone();
            let aall = session.always_allow.clone();
            (msgs, temperature, max_tokens, mdl, thk, bayes, aall)
        }; // ← write lock released here

        // ── Phase 2: Tool loop (NO write lock during API calls) ────────────────────
        let tool_defs = tools::get_available_tool_definitions();
        info!("🔧 Available tools: {}", tool_defs.len());
        let mut loop_count = 0;
        let max_loops = self.config.acp.max_tool_loop_iterations;
        let mut newly_always_allowed: Vec<String> = Vec::new();

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

            // ── Re-trim before every API call ────────────────────────────────────
            // The initial trim (steps 1-4 above) happens once before the loop, but
            // each iteration appends an assistant message plus one or more tool-result
            // messages.  Without re-trimming here, a session with many large file reads
            // can grow to 10× the context limit and hit a 400 from the API.
            {
                // 1. Truncate oversized tool-result messages first (cheapest).
                let max_tc = self.config.acp.max_tool_result_chars;
                if max_tc > 0 {
                    truncate_tool_results(&mut messages, max_tc);
                }
                // 2. Count-based guard: never exceed max_history_messages.
                let max_hist = self.config.acp.max_history_messages;
                if messages.len() > max_hist {
                    let drop = messages.len() - max_hist;
                    messages.drain(0..drop);
                }
                // 3. Token-budget guard: drop oldest messages until we fit.
                let iter_limit = model_context_budget(
                    &model,
                    self.config.acp.max_context_tokens,
                    self.config.acp.grok4_max_context_tokens,
                );
                let iter_est = estimate_tokens(&messages);
                if iter_est > iter_limit {
                    let before = messages.len();
                    trim_to_token_budget(&mut messages, iter_limit);
                    warn!(
                        "⚠️  Mid-loop context trim (iter {}): ~{} est. tokens > {} limit. \
                         Dropped {} old messages (kept {}).",
                        loop_count,
                        iter_est,
                        iter_limit,
                        before - messages.len(),
                        messages.len()
                    );
                }
            }

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
                model, temperature, max_tokens, self.config.timeout_secs,
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
                        .get_router()?
                        .chat_completion_with_history(
                            &messages,
                            temperature,
                            max_tokens,
                            &model,
                            Some(tool_defs.clone()),
                            thinking_mode.as_api_str(),
                        )
                        .await
                    {
                        Ok(resp) => break resp,
                        Err(e) => {
                            // ── Detect "prompt too long" before anything else ─────────
                            // This is not a network error; retrying will not help.
                            // Give the user clear guidance instead.
                            {
                                let raw = e.to_string();
                                let lower = raw.to_lowercase();
                                if lower.contains("maximum prompt length")
                                    || lower.contains("prompt length")
                                    || (lower.contains("invalid argument")
                                        && lower.contains("token"))
                                {
                                    let current_est = estimate_tokens(&messages);
                                    error!("❌ Context-window overflow: {}", raw);
                                    return Err(anyhow!(
                                        "Context window overflow — the request was too large \
                                         for the model.\n\
                                         Estimated tokens in history: ~{}\n\
                                         \n\
                                         💡 Quick fixes:\n\
                                         • Type `/clear` to reset the conversation history.\n\
                                         • Lower `max_history_messages` in .grok/config.toml \
                                           (currently {}).\n\
                                         • Lower `max_context_tokens` in .grok/config.toml \
                                           (currently {}).\n\
                                         • Lower `max_tool_result_chars` in .grok/config.toml \
                                           (currently {}) to truncate large file reads earlier.",
                                        current_est,
                                        self.config.acp.max_history_messages,
                                        self.config.acp.max_context_tokens,
                                        self.config.acp.max_tool_result_chars,
                                    ));
                                }
                            }

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
            let thinking_content = response_with_finish.thinking_content;

            info!("📋 Finish reason: {:?}", finish_reason);

            // If the model produced a reasoning/thinking trace, log it and
            // prepend it as a collapsible block before the main response.
            if let Some(ref tc) = thinking_content {
                let think_tokens = tc.len() / 4;
                debug!("🧠 Thinking trace received: ~{} tokens", think_tokens);
            }

            // Add assistant response to history
            messages.push(serde_json::to_value(&response_msg)?);

            // Check if we have tool calls to process FIRST.
            // finish_reason is checked AFTER the tool loop so that Grok's
            // "stop" signal never short-circuits pending tool calls mid-flight.
            let has_tool_calls = response_msg
                .tool_calls
                .as_ref()
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);

            let elapsed = start_time.elapsed();
            let response_text = content_to_string(response_msg.content.as_ref());

            if !has_tool_calls {
                // No tool calls — return whatever the model said (including "stop").
                info!(
                    "✨ Chat completion finished in {:?} (finish_reason: {:?}, {} loops, {} chars)",
                    elapsed,
                    finish_reason,
                    loop_count,
                    response_text.len()
                );
                // Prepend thinking block if present
                let final_response = if let Some(tc) = thinking_content {
                    format!(
                        "<details><summary>\u{1f9e0} Thinking\u{2026}</summary>\n\n{}\n\n</details>\n\n{}",
                        tc, response_text
                    )
                } else {
                    response_text
                };
                // ── Phase 3: Final sync (brief write lock) ─────────────────────────────
                {
                    let mut sessions = self.sessions.write().await;
                    if let Some(s) = sessions.get_mut(&session_id.0) {
                        s.messages = messages;
                        s.bayes_engine = local_bayes;
                        for name in &newly_always_allowed {
                            s.always_allow.insert(name.clone());
                        }
                        s.last_activity = std::time::Instant::now();
                    }
                }
                return Ok(final_response);
            }

            // We have tool calls to process
            let Some(tool_calls) = response_msg.tool_calls.as_ref() else {
                // ── Phase 3: Final sync (brief write lock) ─────────────────────────────
                {
                    let mut sessions = self.sessions.write().await;
                    if let Some(s) = sessions.get_mut(&session_id.0) {
                        s.messages = messages;
                        s.bayes_engine = local_bayes;
                        for name in &newly_always_allowed {
                            s.always_allow.insert(name.clone());
                        }
                        s.last_activity = std::time::Instant::now();
                    }
                }
                return Ok(response_text);
            };
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
                    let hooks = self.get_hook_manager().read().await;
                    if !hooks.execute_before_tool(function_name, &args)? {
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "content": "Tool execution blocked by hook."
                        }));
                        continue;
                    }
                }

                // --- PERMISSION GATE ---
                if self.config.acp.require_permission
                    && !local_always_allow.contains(function_name.as_str())
                    && !newly_always_allowed.contains(function_name)
                    && let Some(bridge) = &permission_bridge
                {
                    let req_id = uuid::Uuid::new_v4().to_string();

                    let params = RequestPermissionParams::new(
                        session_id.clone(),
                        tool_call.id.clone(),
                        Some(format!("Run {}", function_name)),
                        Some(crate::acp::protocol::ToolKind::Execute),
                    );

                    let (tx, rx) = oneshot::channel();
                    if bridge.outbound.send((req_id, params, tx)).is_ok() {
                        let timeout_secs = self.config.acp.permission_timeout_secs;
                        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx)
                            .await
                        {
                            Ok(Ok(outcome)) => {
                                if outcome.is_cancelled() {
                                    messages.push(json!({
                                        "role": "tool",
                                        "tool_call_id": tool_call.id,
                                        "content": "User rejected the tool execution."
                                    }));
                                    continue;
                                }
                                // Any `selected` outcome is approval; record if always-allow.
                                if outcome.is_always_allow() {
                                    newly_always_allowed.push(function_name.clone());
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

                // Route ALL tool calls through the unified registry + arbitration layer.
                // This ensures every tool defined in get_tool_definitions() is reachable
                // from the ACP path without maintaining a duplicate match block here.
                let policy = self.get_security().get_policy();
                let tool_ctx = tools::ToolContext::new(policy);

                // run_shell_command honours the per-project shell timeout; pass it
                // via the args so the registry shim can pick it up.
                let shell_timeout = self.config.tools.shell.command_timeout_secs;
                let mut augmented_args = args.clone();
                if function_name == "run_shell_command"
                    && augmented_args.get("timeout_secs").is_none()
                    && shell_timeout > 0
                {
                    augmented_args["timeout_secs"] =
                        serde_json::Value::Number(shell_timeout.into());
                }

                let result = tools::execute_tool(function_name, &augmented_args, &tool_ctx).await;

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
                        warn!("⚠️ Tool failed in {:?}: {}", tool_duration, e);

                        // 4. Update Bayesian Engine for Tool Failure
                        local_bayes.update_from_tool_failure();

                        let mut error_content =
                            format!("Error executing tool {}: {}", function_name, e);

                        // If confidence drops significantly due to failure, add a recovery system prompt
                        if local_bayes.is_low_confidence() {
                            error_content = format!(
                                "{}\n\n[System Note: This tool failed and Bayesian confidence is low. Please rewrite your approach, verify your assumptions, or ask the user for clarification instead of repeating the same action.]",
                                error_content
                            );
                        }

                        (error_content, crate::acp::protocol::ToolCallStatus::Failed)
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
                    let hooks = self.get_hook_manager().read().await;
                    hooks.execute_after_tool(function_name, &args, &content)?;
                }

                // Add tool result to history
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": content
                }));
            }

            let loop_duration = loop_start.elapsed();
            info!("🔄 Loop iteration completed in {:?}", loop_duration);

            // Brief write-lock sync: persist the updated message history so
            // save_session_to_disk and /context queries see fresh data between iterations.
            {
                let mut sessions = self.sessions.write().await;
                if let Some(s) = sessions.get_mut(&session_id.0) {
                    s.messages = messages.clone();
                    s.last_activity = std::time::Instant::now();
                }
            }

            // Post-tool-loop guard: if the model signalled "stop" alongside
            // tool calls, return now instead of spinning up a redundant extra
            // API iteration (the tools have already completed).
            if finish_reason == Some("stop") || finish_reason == Some("end_turn") {
                info!(
                    "✅ Model flagged stop after tool execution — returning ({} loops)",
                    loop_count
                );
                // ── Phase 3: Final sync (brief write lock) ──────────────────────────
                {
                    let mut sessions = self.sessions.write().await;
                    if let Some(s) = sessions.get_mut(&session_id.0) {
                        s.messages = messages.clone();
                        s.bayes_engine = local_bayes.clone();
                        for name in &newly_always_allowed {
                            s.always_allow.insert(name.clone());
                        }
                        s.last_activity = std::time::Instant::now();
                    }
                }
                return Ok(String::new());
            }
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
            .get_router()?
            .chat_completion_with_history(
                &messages,
                session.config.temperature,
                session.config.max_tokens,
                &session.config.model,
                None,
                None, // no thinking mode for code operations
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
    #[allow(dead_code)]
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
        &self.capabilities()
    }

    // ── Bayesian engine commands (───────────────────────────────────────────────

    /// Return an ASCII bar-chart of the current Bayesian belief state for a session.
    ///
    /// Used by the `/bayes show` slash command.
    pub async fn get_bayes_visualize(&self, session_id: &SessionId) -> Result<String> {
        let sessions = self.sessions.read().await;
        match sessions.get(&session_id.0) {
            None => Ok("Session not found — no Bayesian state to display.".to_string()),
            Some(session) => {
                let vis = session.bayes_engine.visualize();
                let best = session
                    .bayes_engine
                    .best_intent()
                    .unwrap_or_else(|| "(none)".to_string());
                Ok(format!(
                    "## 🧠 Bayesian Belief State\n\n\
                     **Best intent:** `{best}`\n\n\
                     ```\n{vis}\n```",
                ))
            }
        }
    }

    /// Reset the Bayesian engine for a session back to compiled-in defaults.
    ///
    /// Used by the `/bayes reset` slash command.
    pub async fn reset_bayes(&self, session_id: &SessionId) -> Result<String> {
        let mut sessions = self.sessions.write().await;
        match sessions.get_mut(&session_id.0) {
            None => Ok("Session not found — nothing to reset.".to_string()),
            Some(session) => {
                session.bayes_engine = crate::bayes::BayesianEngine::new_with_default_priors();
                info!("Bayesian engine reset for session: {}", session_id.0);
                Ok(
                    "✅ Bayesian priors reset to defaults. The engine will re-learn from \
                    the next few messages."
                        .to_string(),
                )
            }
        }
    }

    /// Return a plain-English explanation of the current Bayesian state for a session.
    ///
    /// Used by the `/bayes explain` slash command.
    pub async fn get_bayes_explain(&self, session_id: &SessionId) -> Result<String> {
        let sessions = self.sessions.read().await;
        match sessions.get(&session_id.0) {
            None => Ok("Session not found — no Bayesian state to explain.".to_string()),
            Some(session) => {
                let e = &session.bayes_engine;
                let best = e.best_intent().unwrap_or_else(|| "(none)".to_string());
                let intent_label = match best.as_str() {
                    "intent_edit" => "editing files / writing code",
                    "intent_shell" => "running shell commands",
                    "intent_search" => "searching the web or codebase",
                    "intent_question" => "answering a question",
                    _ => "an unrecognised intent",
                };
                let clarify = if e.needs_clarification() {
                    "🟡 **Clarification gate is OPEN** — the engine thinks a clarifying \
                     question may be needed before proceeding."
                } else {
                    "🟢 **Clarification gate is closed** — the intent is clear enough to act."
                };
                let uncertain = if e.is_high_uncertainty() {
                    "⚠️  **High uncertainty** — system uncertainty notes will be injected \
                     into prompts to encourage the AI to ask before making changes."
                } else {
                    "✅ **Uncertainty is low** — the AI can proceed with reasonable confidence."
                };
                let vague = if e.is_vague() {
                    "🟡 **Vagueness flag is SET** — the last message appeared vague; the AI \
                     will be prompted to propose alternative interpretations."
                } else {
                    "🟢 **No vagueness detected** — the request was specific enough."
                };
                Ok(format!(
                    "## 🧠 Bayesian State Explanation\n\n\
                     **Most likely intent:** `{best}` ({intent_label})\n\n\
                     {clarify}\n\n\
                     {uncertain}\n\n\
                     {vague}\n\n\
                     Use `/bayes show` to see the raw probability bar-chart, \
                     or `/bayes reset` to wipe the learned priors."
                ))
            }
        }
    }

    /// Set the active goal for a session (used by the `/goal <text>` slash command).
    pub async fn set_session_goal(&self, session_id: &SessionId, goal: String) -> Result<String> {
        let mut sessions = self.sessions.write().await;
        match sessions.get_mut(&session_id.0) {
            None => Ok("Session not found -- goal not set.".to_string()),
            Some(session) => {
                session.current_goal = Some(goal.clone());
                info!("Goal set for session {}: {}", session_id.0, goal);
                Ok(format!(
                    "**Goal set:** {}\n\nAll subsequent messages will be interpreted through \
                     the lens of this goal. Type `/goal clear` to remove it.",
                    goal
                ))
            }
        }
    }

    /// Clear the active goal for a session (used by the `/goal clear` slash command).
    pub async fn clear_session_goal(&self, session_id: &SessionId) -> Result<String> {
        let mut sessions = self.sessions.write().await;
        match sessions.get_mut(&session_id.0) {
            None => Ok("Session not found.".to_string()),
            Some(session) => {
                session.current_goal = None;
                info!("Goal cleared for session {}", session_id.0);
                Ok(
                    "Goal cleared. Messages will be interpreted without a persistent goal."
                        .to_string(),
                )
            }
        }
    }

    /// Return the current goal for a session (used by `/goal show`).
    pub async fn get_session_goal(&self, session_id: &SessionId) -> Result<String> {
        let sessions = self.sessions.read().await;
        match sessions.get(&session_id.0) {
            None => Ok("Session not found.".to_string()),
            Some(session) => Ok(match &session.current_goal {
                Some(goal) => format!("**Current goal:** {}", goal),
                None => "No active goal set. Use `/goal <description>` to set one.".to_string(),
            }),
        }
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

    /// Return `true` if a session with the given ID currently exists in the
    /// active sessions map.  Used by the ACP request handler to detect stale
    /// session IDs sent by clients that reconnected after a grok restart.
    pub async fn session_exists(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(session_id)
    }

    pub async fn set_model(&self, session_id: &str, model: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.config.model = model.to_string();
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", session_id))
        }
    }

    pub async fn get_session_cwd(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.cwd.clone())
    }

    /// Store the list of slash commands the client advertised in a
    /// `session/update { sessionUpdate: "available_commands_update" }` notification.
    ///
    /// Called from the ACP notification handler in `src/cli/commands/acp.rs`.
    pub async fn set_client_commands(
        &self,
        session_id: &SessionId,
        commands: Vec<String>,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id.0) {
            let count = commands.len();
            session.client_commands = commands;
            info!(
                "Stored {} client command(s) for session '{}'",
                count, session_id.0
            );
        }
        Ok(())
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

    /// Set the thinking / reasoning mode for a session.
    ///
    /// The mode is stored in `SessionConfig::thinking_mode` and passed as
    /// `reasoning_effort` in every subsequent API call.
    pub async fn set_thinking_mode(
        &self,
        session_id: &SessionId,
        mode: ThinkingMode,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id.0) {
            let old = &session.config.thinking_mode;
            info!(
                "Thinking mode: {:?} → {:?} for session: {}",
                old, mode, session_id.0
            );
            session.config.thinking_mode = mode;
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", session_id.0))
        }
    }

    /// Get the current thinking mode for a session.
    pub async fn get_thinking_mode(&self, session_id: &SessionId) -> Option<ThinkingMode> {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id.0)
            .map(|s| s.config.thinking_mode.clone())
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

    /// Returns the path to the sessions persistence directory: ~/.grok/sessions/
    fn sessions_dir() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|h| h.join(".grok").join("sessions"))
    }

    /// Persist session state to ~/.grok/sessions/<id>.json
    /// Called automatically after each successful prompt response.
    /// Retries up to 3 times on I/O errors (Starlink-safe).
    pub async fn save_session_to_disk(&self, session_id: &SessionId) -> Result<()> {
        let state = {
            let sessions = self.sessions.read().await;
            let Some(session) = sessions.get(&session_id.0) else {
                return Ok(());
            };
            PersistedSession {
                session_id: session_id.0.clone(),
                cwd: session.cwd.clone(),
                messages: session.messages.clone(),
                config: session.config.clone(),
                current_goal: session.current_goal.clone(),
                always_allow: session.always_allow.iter().cloned().collect(),
                saved_at_unix: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }
        };

        let Some(dir) = Self::sessions_dir() else {
            return Ok(());
        };

        for attempt in 1u32..=3 {
            match tokio::fs::create_dir_all(&dir).await {
                Ok(()) => break,
                Err(e) if attempt < 3 => {
                    warn!(
                        "save_session: create_dir failed (attempt {}): {}",
                        attempt, e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64))
                        .await;
                }
                Err(e) => return Err(anyhow!("save_session: cannot create sessions dir: {}", e)),
            }
        }

        let path = dir.join(format!("{}.json", session_id.0));
        let json = serde_json::to_string_pretty(&state)?;

        for attempt in 1u32..=3 {
            match tokio::fs::write(&path, &json).await {
                Ok(()) => {
                    info!(
                        "Session '{}' persisted to {:?} ({} messages)",
                        session_id.0,
                        path,
                        state.messages.len()
                    );
                    return Ok(());
                }
                Err(e) if attempt < 3 => {
                    warn!("save_session: write failed (attempt {}): {}", attempt, e);
                    tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64))
                        .await;
                }
                Err(e) => return Err(anyhow!("save_session: write failed: {}", e)),
            }
        }
        Ok(())
    }

    /// Load a persisted session from ~/.grok/sessions/<id>.json.
    /// Returns None if no saved state exists or if the file cannot be parsed.
    pub async fn load_session_from_disk(&self, session_id: &str) -> Option<PersistedSession> {
        let dir = Self::sessions_dir()?;
        let path = dir.join(format!("{}.json", session_id));

        for attempt in 1u32..=3 {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    return serde_json::from_str::<PersistedSession>(&content)
                        .map_err(|e| warn!("load_session: parse error for '{}': {}", session_id, e))
                        .ok();
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
                Err(e) if attempt < 3 => {
                    warn!("load_session: read failed (attempt {}): {}", attempt, e);
                    tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64))
                        .await;
                }
                Err(_) => return None,
            }
        }
        None
    }

    /// Restore a session from a `PersistedSession` snapshot loaded from disk.
    pub async fn restore_session_from_disk(&self, state: PersistedSession) -> Result<()> {
        let sid = SessionId::new(state.session_id.clone());
        // Initialize a fresh session with the saved config
        self.initialize_session(sid.clone(), state.cwd.clone(), Some(state.config), None)
            .await?;
        // Overwrite the parts that initialize_session can't set
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&state.session_id) {
            session.messages = state.messages;
            session.current_goal = state.current_goal;
            session.always_allow = state.always_allow.into_iter().collect();
        }
        info!("Session '{}' restored from disk", sid.0);
        Ok(())
    }

    /// Clone an existing session into a new session ID (session/fork support).
    pub async fn fork_session(&self, source_id: &SessionId, new_id: SessionId) -> Result<()> {
        let forked = {
            let sessions = self.sessions.read().await;
            let source = sessions
                .get(&source_id.0)
                .ok_or_else(|| anyhow!("fork_session: source '{}' not found", source_id.0))?;
            SessionData {
                cwd: source.cwd.clone(),
                messages: source.messages.clone(),
                config: source.config.clone(),
                created_at: std::time::Instant::now(),
                last_activity: std::time::Instant::now(),
                always_allow: source.always_allow.clone(),
                client_commands: source.client_commands.clone(),
                bayes_engine: crate::bayes::BayesianEngine::new_with_default_priors(),
                current_goal: source.current_goal.clone(),
            }
        };
        let mut sessions = self.sessions.write().await;
        sessions.insert(new_id.0.clone(), forked);
        info!("Session '{}' forked → '{}'", source_id.0, new_id.0);
        Ok(())
    }

    /// Return a list of currently active session IDs.
    /// Used by the `session/list` ACP handler to advertise known sessions
    /// to clients such as Zed.
    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
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
                "models": self.capabilities().models,
                "features": self.capabilities().features,
                "max_context_length": self.capabilities().max_context_length
            }
        }))
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Token-management helpers
// ───────────────────────────────────────────────────────────────────────────────

/// Rough token estimate: 4 characters ≈ 1 token (standard heuristic for
/// mixed prose/code/JSON).  We serialise each message value so that all
/// fields (role, content, tool_calls …) are counted, not just the visible
/// text.
#[inline]
fn estimate_tokens(messages: &[Value]) -> usize {
    messages
        .iter()
        .map(|m| m.to_string().len().saturating_add(3) / 4) // +3 for per-message overhead
        .sum()
}

/// Select the appropriate context-token budget based on the active model.
///
/// - grok-4.x models (grok-4.3 and later) have a 1 M token context window;
///   use `grok4_budget` for those.
/// - All other models (grok-3, grok-2, …) use `legacy_budget`.
#[inline]
fn model_context_budget(model: &str, legacy_budget: usize, grok4_budget: usize) -> usize {
    if model.starts_with("grok-4") {
        grok4_budget
    } else {
        legacy_budget
    }
}

/// Remove the oldest messages (from index 0 upward) until the estimated
/// token count is ≤ `budget`.  Always keeps at least the last message so
/// the user turn is never lost.
fn trim_to_token_budget(messages: &mut Vec<Value>, budget: usize) {
    while messages.len() > 1 && estimate_tokens(messages) > budget {
        messages.remove(0);
    }
}

/// Truncate the `content` field of tool-result messages (role = "tool")
/// that exceed `max_chars` characters.  A truncation notice is appended so
/// the model knows the output was cut.
fn truncate_tool_results(messages: &mut [Value], max_chars: usize) {
    for msg in messages.iter_mut() {
        // Only touch tool-result messages
        if msg.get("role").and_then(|r| r.as_str()) != Some("tool") {
            continue;
        }
        if let Some(content) = msg.get_mut("content") {
            match content {
                Value::String(s) if s.len() > max_chars => {
                    let mut end = max_chars;
                    while !s.is_char_boundary(end) {
                        end -= 1;
                    }
                    let truncated = &s[..end];
                    *s = format!(
                        "{}\n\n[... output truncated to {} chars to fit context window ...]",
                        truncated, max_chars
                    );
                }
                // Array-of-content-blocks form used by some providers
                Value::Array(blocks) => {
                    for block in blocks.iter_mut() {
                        if let Some(text_val) = block.get_mut("text") {
                            if let Some(s) = text_val.as_str() {
                                if s.len() > max_chars {
                                    let mut end = max_chars;
                                    while !s.is_char_boundary(end) {
                                        end -= 1;
                                    }
                                    let truncated = &s[..end];
                                    *text_val = Value::String(format!(
                                        "{}\n\n[... output truncated to {} chars to fit \
                                         context window ...]",
                                        truncated, max_chars
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Build the compact system-role archive notice injected into the active
/// context after a chunk is compressed and saved.
///
/// The notice is intentionally short (≤ 400 chars) to minimise its own
/// token footprint while giving the model enough information to decide
/// whether it needs to recall the archived content.
fn build_archive_notice(chunk: &crate::memory::context_archive::ContextChunk) -> Value {
    let ts = chunk.created_at.format("%Y-%m-%d %H:%M UTC").to_string();

    let facts = if chunk.key_facts.is_empty() {
        String::new()
    } else {
        let bullets: String = chunk
            .key_facts
            .iter()
            .take(5)
            .map(|f| format!("\n\u{2022} {f}"))
            .collect();
        format!("\nKey facts:{}", bullets)
    };

    // Keep summary under ~200 chars for the notice
    let preview: String = chunk.summary.chars().take(200).collect();
    let preview = if chunk.summary.len() > 200 {
        format!("{}\u{2026}", preview)
    } else {
        preview
    };

    let content = format!(
        "[\u{1F4E6} Context Archive #{id} | {ts} | {count} messages]\n\
         Summary: {preview}{facts}\n\
         Type `/recall {id}` or say \"recall archive {id}\" to restore this context.",
        id = chunk.chunk_id,
        ts = ts,
        count = chunk.message_count,
        preview = preview,
        facts = facts,
    );

    json!({
        "role": "system",
        "content": content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.model, "grok-4.3");
        assert_eq!(config.temperature, 0.5);
        // grok-4.3 supports higher output token limits; default raised to 16_384
        assert_eq!(config.max_tokens, 16_384);
        assert!(config.system_prompt.is_some());
    }

    #[test]
    fn test_capabilities_creation() {
        let capabilities = GrokAcpAgent::create_capabilities();
        assert!(!capabilities.models.is_empty());
        assert!(!capabilities.features.is_empty());
        assert!(!capabilities.tools.is_empty());
        // grok-4.3 exposes a 1 M token context window
        assert_eq!(capabilities.max_context_length, 1_048_576);
        // grok-4.3 must be the first / default model
        assert_eq!(capabilities.models[0], "grok-4.3");
        // 1m_context and vision feature flags
        assert!(capabilities.features.contains(&"1m_context".to_string()));
        assert!(capabilities.features.contains(&"vision".to_string()));
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
            cwd: String::new(),
            messages: Vec::new(),
            config: SessionConfig::default(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            always_allow: std::collections::HashSet::new(),
            client_commands: Vec::new(),
            bayes_engine: crate::bayes::BayesianEngine::new(),
            current_goal: None,
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
            .initialize_session(session_id.clone(), ".".to_string(), None, None)
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
                if let Some((_req_id, _params, reply_tx)) = rx.recv().await {
                    let _ = reply_tx.send(PermissionOutcome {
                        outcome: crate::acp::protocol::OutcomeDetail::Selected {
                            option_id: "proceed_always".to_string(),
                        },
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
                if let Some((_req_id, _params, reply_tx)) = rx.recv().await {
                    let _ = reply_tx.send(PermissionOutcome {
                        outcome: crate::acp::protocol::OutcomeDetail::Selected {
                            option_id: "proceed_once".to_string(),
                        },
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
                if let Some((_req_id, _params, reply_tx)) = rx.recv().await {
                    let _ = reply_tx.send(PermissionOutcome::cancel());
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

    #[test]
    fn build_archive_notice_has_correct_role_and_chunk_id() {
        use crate::memory::context_archive::ContextChunk;
        use chrono::Utc;

        let chunk = ContextChunk {
            chunk_id: 3,
            session_id: "test-sess".to_string(),
            created_at: Utc::now(),
            message_count: 12,
            estimated_tokens_saved: 4500,
            summary: "We discussed Rust async patterns.".to_string(),
            key_facts: vec![
                "tokio is used".to_string(),
                "async/await preferred".to_string(),
            ],
            raw_messages: vec![],
        };

        let notice = build_archive_notice(&chunk);
        assert_eq!(notice["role"], "system");
        let content = notice["content"].as_str().unwrap();
        assert!(content.contains("#3"), "should contain chunk id");
        assert!(content.contains("/recall 3"), "should contain recall hint");
        assert!(
            content.contains("12 messages"),
            "should contain message count"
        );
    }

    // ── Task 109: model_context_budget ─────────────────────────────────────────

    #[test]
    fn test_model_context_budget_grok4_uses_grok4_budget() {
        assert_eq!(model_context_budget("grok-4.3", 220_000, 950_000), 950_000);
        assert_eq!(
            model_context_budget("grok-4-latest", 220_000, 950_000),
            950_000
        );
        assert_eq!(model_context_budget("grok-4", 220_000, 950_000), 950_000);
    }

    #[test]
    fn test_model_context_budget_legacy_models_use_legacy_budget() {
        assert_eq!(model_context_budget("grok-3", 220_000, 950_000), 220_000);
        assert_eq!(
            model_context_budget("grok-3-mini", 220_000, 950_000),
            220_000
        );
        assert_eq!(
            model_context_budget("grok-2-latest", 220_000, 950_000),
            220_000
        );
        assert_eq!(model_context_budget("grok-beta", 220_000, 950_000), 220_000);
    }

    #[test]
    fn test_truncate_tool_results_utf8_boundary() {
        // '─' is 3 bytes: 0xE2, 0x94, 0x80
        // Starts at 29998, ends at 30001.
        let long_string = "A".repeat(29998) + "─" + &"B".repeat(10);
        let mut messages = vec![json!({
            "role": "tool",
            "content": long_string
        })];

        // This should truncate at index 30000, but index 30000 is inside '─' (29998..30001)
        // Our fix should back off to 29998.
        truncate_tool_results(&mut messages, 30000);

        let content = messages[0]["content"].as_str().unwrap();
        assert!(content.starts_with(&"A".repeat(29998)));
        assert!(!content.contains('─'));
        assert!(content.contains("truncated"));
    }

    #[test]
    fn test_truncate_tool_results_array_utf8_boundary() {
        let long_string = "A".repeat(29998) + "─" + &"B".repeat(10);
        let mut messages = vec![json!({
            "role": "tool",
            "content": [
                {
                    "type": "text",
                    "text": long_string
                }
            ]
        })];

        truncate_tool_results(&mut messages, 30000);

        let text = messages[0]["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with(&"A".repeat(29998)));
        assert!(!text.contains('─'));
        assert!(text.contains("truncated"));
    }
}
