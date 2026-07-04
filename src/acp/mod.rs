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

use crate::config::Config;
use crate::content_to_string;
use crate::hooks::HookManager;
use crate::router::AppRouter;

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
    /// AppRouter — `None` when no API key is configured at startup.
    /// The key is only required when making actual API calls; the agent can
    /// still respond to `initialize` and declare its auth requirements.
    router: Option<AppRouter>,

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

    /// Session DNA
    dna: crate::session::dna::SessionDna,

    /// Current goal for the session
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

        refined_message
    }
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
            model: "grok-code-fast-1".to_string(),
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
                6. Suggest tests to verify your code when appropriate.\n\
                \n\
                Project task management rules (follow exactly):\n\
                7. ALWAYS call the 'task_get' tool when the user asks about a specific \
                task by number. Example: 'what is task 60' → call task_get(id=60). \
                Never skip the tool call and never answer from memory.\n\
                8. When task_get SUCCEEDS: reply in plain English using the title, \
                status, priority, and description from the tool result. \
                Do not invent or change any field values.\n\
                9. When task_get FAILS (tool returns TOOL ERROR): reply ONLY with \
                'I could not retrieve task N. Error: <exact error text>'. \
                Do NOT return any JSON. Do NOT guess the task title or any other field. \
                Do NOT make up a task object.\n\
                10. To list ALL tasks use read_file with path '.zed/task_list.json'."
                    .to_string(),
            ),
        }
    }
}

impl GrokAcpAgent {
    /// Create a new Grok ACP agent
    pub async fn new(config: Config, default_model: Option<String>) -> Result<Self> {
        // Build the API client only when an API key is available.
        // In ACP stdio mode the agent MUST be able to start up and respond to
        // `initialize` (declaring its auth requirements) even before the user
        // has supplied a key, so we defer the hard error to the first actual
        // API call rather than failing here.
        let router = if let Some(ref api_key) = config.api_key {
            match AppRouter::new(api_key, config.timeout_secs) {
                Ok(router) => {
                    info!("✓ AppRouter initialised");
                    Some(router)
                }
                Err(e) => {
                    warn!(
                        "Failed to create Grok client (will retry on first API call): {}",
                        e
                    );
                    None
                }
            }
        } else {
            info!("No API key configured at startup — set GROK_API_KEY to enable API calls");
            None
        };

        let capabilities = Self::create_capabilities();

        let security = SecurityManager::new();
        // Trust current directory by default, canonicalizing to resolve symlinks
        if let Ok(cwd) = std::env::current_dir() {
            let canonical_cwd = cwd.canonicalize().unwrap_or(cwd);
            security.add_trusted_directory(canonical_cwd);
        }
        // Apply the shell-command timeout from config so `tools.shell.command_timeout_secs`
        // in config.toml is honoured. The GROK_SHELL_TIMEOUT_SECS env var still
        // takes precedence (checked at call time in shell_tools::run_shell_command).
        security.set_shell_timeout_secs(config.tools.shell.command_timeout_secs);

        Ok(Self {
            router,
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            capabilities,
            security,
            hook_manager: Arc::new(RwLock::new(HookManager::new())),
            default_model,
        })
    }

    /// Ensure a session exists. If it does not, create a minimal default one.
    /// This prevents “Session not found” errors on first-use or when the client
    /// sends a slash command before the normal session/new handshake.
    async fn ensure_session(&self, session_id: &str) -> SessionData {
        {
            let sessions = self.sessions.read().await;
            if let Some(s) = sessions.get(session_id) {
                return s.clone();
            }
        }

        // Create a minimal default session
        let mut session_data = SessionData {
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            messages: Vec::new(),
            config: SessionConfig::default(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            always_allow: std::collections::HashSet::new(),
            client_commands: Vec::new(),
            bayes_engine: crate::bayes::BayesianEngine::new_with_config(&self.config.bayesian),
            dna: crate::session::dna::SessionDna::default(),
            current_goal: None,
        };

        // Inject DNA into the (empty) system prompt so the session is usable
        let dna = crate::session::dna::SessionDna::load();
        let mut prompt = String::new();
        dna.inject_into_prompt(&mut prompt);
        prompt.push_str(&format!("\n\n**Current DNA Mode:** {}", dna.get_mode()));
        if !prompt.trim().is_empty() {
            session_data.messages.push(serde_json::json!({
                "role": "system",
                "content": prompt.trim().to_string(),
            }));
        }
        session_data.dna = dna;

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), session_data.clone());
        info!("Auto-created minimal session for missing ID: {}", session_id);
        session_data
    }

    /// Public helper used by all slash-command handlers that need a session.
    /// Guarantees the session exists (creating a minimal one if necessary).
    pub async fn ensure_session_exists(&self, session_id: &str) {
        self.ensure_session(session_id).await;
    }

    // ── MCP / Security / Thinking helpers (wired for acp.rs) ─────────────────

    /// Return a reference to the MCP client (placeholder until full MCP impl).
    pub fn get_mcp_client(&self) -> Arc<RwLock<crate::mcp::client::McpClient>> {
        // Create a lazy static or return a new empty one for now.
        use std::sync::OnceLock;
        static MCP_CLIENT: OnceLock<Arc<RwLock<crate::mcp::client::McpClient>>> = OnceLock::new();
        MCP_CLIENT
            .get_or_init(|| Arc::new(RwLock::new(crate::mcp::client::McpClient::new())))
            .clone()
    }

    /// Return a reference to the discovered MCP tools map.
    pub fn get_discovered_mcp_tools(
        &self,
    ) -> Arc<RwLock<std::collections::HashMap<String, Vec<serde_json::Value>>>> {
        use std::sync::OnceLock;
        static DISCOVERED: OnceLock<
            Arc<RwLock<std::collections::HashMap<String, Vec<serde_json::Value>>>>,
        > = OnceLock::new();
        DISCOVERED
            .get_or_init(|| Arc::new(RwLock::new(std::collections::HashMap::new())))
            .clone()
    }

    /// Delegate to the security manager (public for acp.rs call sites).
    pub fn add_trusted_directory(&self, path: std::path::PathBuf) {
        self.security.add_trusted_directory(path);
    }

    /// Stub: persist session to disk (no-op for now).
    pub async fn save_session_to_disk(&self, _session_id: &SessionId) -> Result<()> {
        Ok(())
    }

    /// Stub: set thinking mode on the session (stored in SessionData later).
    pub async fn set_thinking_mode(
        &self,
        session_id: &SessionId,
        _mode: crate::config::ThinkingMode,
    ) -> Result<()> {
        self.ensure_session_exists(&session_id.0).await;
        // For now we just ensure the session exists; real storage can be added later.
        Ok(())
    }

    /// Stub: get thinking mode (returns None until we store it).
    pub async fn get_thinking_mode(
        &self,
        session_id: &SessionId,
    ) -> Option<crate::config::ThinkingMode> {
        self.ensure_session_exists(&session_id.0).await;
        None
    }

    /// Return a reference to the underlying [`AppRouter`], or a descriptive
    /// error if no API key was configured when the agent was created.
    ///
    /// Call this inside any method that needs to reach the xAI API instead of
    /// accessing `self.grok_client` directly.
    fn get_router(&self) -> Result<AppRouter> {
        self.router.clone().ok_or_else(|| {
            anyhow!(
                "API key not configured. \
                 Set the GROK_API_KEY environment variable and restart the agent, \
                 or use 'grok config set api_key <key>'."
            )
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
            // Build the tool list live from the registry so this list
            // automatically reflects any newly added tools without requiring
            // manual updates here.
            tools: crate::tools::registry::get_available_tool_definitions()
                .iter()
                .filter_map(|t| {
                    let v: serde_json::Value = t.clone();
                    let func = v.get("function")?;
                    Some(ToolDefinition {
                        name: func.get("name")?.as_str()?.to_string(),
                        description: func.get("description")?.as_str()?.to_string(),
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
        _thinking_mode: Option<crate::config::ThinkingMode>,
    ) -> Result<()> {
        // ── Update security policy working directory ─────────────────────────────
        // Zed passes the actual workspace root as `cwd` in `session/new`.  Without
        // updating the security policy here, relative paths like
        // `.zed/task_list.json` would resolve against the process launch directory
        // rather than the workspace root, causing silent file-not-found errors that
        // make the LLM hallucinate the file contents.
        let cwd_path = std::path::PathBuf::from(&cwd);
        self.security.set_working_directory(&cwd_path);
        tracing::info!(
            session = %session_id.0,
            workspace = %cwd_path.display(),
            "initialize_session: security policy workspace root updated"
        );

        let mut session_config = config.unwrap_or_default();

        // Apply default model override if present and config matches default
        if let Some(model) = &self.default_model {
            session_config.model = model.clone();
        }

        // Capture values for logging before they are moved into SessionData.
        let log_cwd = std::path::PathBuf::from(&cwd);
        let log_model = session_config.model.clone();
        let log_session_id = session_id.0.clone();

        // Seed the message history with the system prompt so the LLM has
        // context from the very first API call.  Without this the system
        // prompt field in SessionConfig is stored but never forwarded to the
        // model — leaving it with no instructions and causing hallucination.
        let initial_messages: Vec<serde_json::Value> =
            if let Some(ref sys) = session_config.system_prompt {
                vec![serde_json::json!({ "role": "system", "content": sys })]
            } else {
                Vec::new()
            };

        let session_data = SessionData {
            cwd,
            messages: initial_messages,
            config: session_config,
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            always_allow: std::collections::HashSet::new(),
            client_commands: Vec::new(),
            bayes_engine: crate::bayes::BayesianEngine::new_with_config(&self.config.bayesian),
            dna: crate::session::dna::SessionDna::default(),
            current_goal: None,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.0.clone(), session_data);

        info!("Initialized new ACP session: {}", session_id.0);

        // Write a session-start banner to the dedicated tool log so each
        // session is clearly delimited when tailing the log file.
        crate::utils::tool_logger::log_session_start(&log_session_id, &log_cwd, &log_model);

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

        // Trim history to prevent unbounded context growth.
        // Keep the most recent max_history_messages entries so the model always
        // has fresh context without exceeding the API context window.
        // We trim here (after adding the user turn) so we never split a
        // tool-call sequence that was already committed to history.
        let max_history = self.config.acp.max_history_messages;
        if session.messages.len() > max_history {
            // Never evict the system-prompt message (role="system" at index 0).
            // Losing it removes all task-management instructions and causes
            // the LLM to ignore tools and hallucinate answers.
            let has_system = session
                .messages
                .first()
                .and_then(|m| m.get("role"))
                .and_then(|r| r.as_str())
                == Some("system");
            let trim_from = usize::from(has_system); // 1 if system present, else 0

            let available = session.messages.len().saturating_sub(trim_from);
            let need = session.messages.len() - max_history;
            let actual = need.min(available.saturating_sub(1));
            if actual > 0 {
                session.messages.drain(trim_from..trim_from + actual);
            }
            debug!(
                "Trimmed {} messages (system prompt preserved: {}; keeping ≤{})",
                actual, has_system, max_history
            );
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
            // ── DATA-TRACE: snapshot what we are about to send to the LLM ───────
            // Written to BOTH tracing (visible in Zed's log panel with
            // RUST_LOG=grok_cli::acp=debug) AND to the persistent tool log file
            // so every iteration is preserved on disk even on a Starlink drop.
            {
                let roles: Vec<&str> = session
                    .messages
                    .iter()
                    .filter_map(|m| m.get("role")?.as_str())
                    .collect();
                let sizes: Vec<usize> = session
                    .messages
                    .iter()
                    .map(|m| {
                        m.get("content")
                            .and_then(|c| c.as_str())
                            .map(|s| s.len())
                            .unwrap_or(0)
                    })
                    .collect();
                let summary = roles
                    .iter()
                    .zip(sizes.iter())
                    .enumerate()
                    .map(|(i, (r, s))| format!("[{i}]{r}:{s}B"))
                    .collect::<Vec<_>>()
                    .join(" ");

                info!(
                    "📨 DATA-TRACE iter={} total_msgs={} layout={}",
                    loop_count,
                    session.messages.len(),
                    summary
                );

                crate::utils::tool_logger::log_note(&format!(
                    "DATA-TRACE iter={} total_msgs={} layout={}",
                    loop_count,
                    session.messages.len(),
                    summary
                ));

                // At DEBUG level log a 150-char preview of every message body.
                for (i, msg) in session.messages.iter().enumerate() {
                    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("?");
                    let body = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let preview = body[..body.len().min(150)]
                        .replace('\n', "↵")
                        .replace('\r', "");
                    let has_tc = msg.get("tool_calls").is_some();
                    let tcid = msg
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    tracing::debug!(
                        "[msg {i}] role={role} bytes={bytes} has_tool_calls={has_tc} \
                         tool_call_id='{tcid}' | {preview}",
                        bytes = body.len(),
                    );
                }
            }
            // ────────────────────────────────────────────────────────────────────

            let api_call_start = std::time::Instant::now();

            let response_with_finish = {
                let mut attempt = 0u32;
                loop {
                    attempt += 1;
                    match self
                        .get_router()?
                        .chat_completion_with_history(
                            &session.messages,
                            temperature,
                            max_tokens,
                            &session.config.model,
                            Some(tool_defs.clone()),
                            None,
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

            // ── DATA-TRACE: what did the LLM just return? ────────────────────
            {
                let tc_count = response_msg
                    .tool_calls
                    .as_ref()
                    .map(|tc| tc.len())
                    .unwrap_or(0);
                let resp_text = content_to_string(response_msg.content.as_ref());
                let preview = resp_text[..resp_text.len().min(200)]
                    .replace('\n', "↵")
                    .replace('\r', "");
                info!(
                    "🤖 DATA-TRACE LLM response: finish={:?} tool_calls={} \
                     text_bytes={} preview='{}'",
                    finish_reason,
                    tc_count,
                    resp_text.len(),
                    preview
                );
                crate::utils::tool_logger::log_note(&format!(
                    "DATA-TRACE LLM response: finish={:?} tool_calls={} text_bytes={} \
                     preview='{}'",
                    finish_reason,
                    tc_count,
                    resp_text.len(),
                    preview
                ));
            }
            // ────────────────────────────────────────────────────────────────

            // Add assistant response to history
            session.messages.push(serde_json::to_value(&response_msg)?);

            // Check tool calls BEFORE finish_reason: some models (e.g. Grok) can
            // return finish_reason "stop" in the same response as tool_calls.
            // If we honour finish_reason first we silently drop pending tool calls.
            let has_tool_calls = response_msg
                .tool_calls
                .as_ref()
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);

            let elapsed = start_time.elapsed();
            let response_text = content_to_string(response_msg.content.as_ref());

            if !has_tool_calls {
                // No tool calls – safe to honour finish_reason now.
                if finish_reason == Some("stop") || finish_reason == Some("end_turn") {
                    info!(
                        "✅ Model signaled completion (finish_reason: {:?}) in {:?} ({} loops, {} chars)",
                        finish_reason,
                        elapsed,
                        loop_count,
                        response_text.len()
                    );
                } else {
                    info!(
                        "✨ Chat completion finished in {:?} ({} loops, {} chars)",
                        elapsed,
                        loop_count,
                        response_text.len()
                    );
                }

                return Ok(response_text);
            }

            // We have tool calls to process
            let Some(tool_calls) = response_msg.tool_calls.as_ref() else {
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
                                    session.messages.push(json!({
                                        "role": "tool",
                                        "tool_call_id": tool_call.id,
                                        "content": "User rejected the tool execution."
                                    }));
                                    continue;
                                }
                                // Any `selected` outcome is approval; record if always-allow.
                                if outcome.is_always_allow() {
                                    session.always_allow.insert(function_name.clone());
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

                // Acquire the policy once so we don't clone it for every arm.
                let policy = self.security.get_policy();

                let result = match function_name.as_str() {
                    "read_file" => tools::read_file(args["path"].as_str().ok_or(anyhow!("Missing path"))?, &policy).await,
                    "write_file" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        let content = args["content"].as_str().ok_or(anyhow!("Missing content"))?;
                        tools::write_file(path, content, &policy, false).await
                    }
                    "list_directory" => tools::list_directory(args["path"].as_str().ok_or(anyhow!("Missing path"))?, &policy),
                    "glob_search" => tools::glob_search(args["pattern"].as_str().ok_or(anyhow!("Missing pattern"))?, &policy),
                    "search_file_content" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        let pattern = args["pattern"].as_str().ok_or(anyhow!("Missing pattern"))?;
                        tools::search_file_content(path, pattern, &policy)
                    }
                    "run_shell_command" => {
                        let command = args["command"].as_str().ok_or(anyhow!("Missing command"))?;
                        tools::run_shell_command(command, &policy, 0).await
                    }
                    "replace" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        let old_string = args["old_string"].as_str().ok_or(anyhow!("Missing old_string"))?;
                        let new_string = args["new_string"].as_str().ok_or(anyhow!("Missing new_string"))?;
                        let expected_replacements = args["expected_replacements"].as_u64().map(|n| n as u32);
                        tools::replace(path, old_string, new_string, expected_replacements, &policy, false).await
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
                        tools::read_multiple_files(paths?, &policy).await
                    }
                    "list_code_definitions" => {
                        let path = args["path"].as_str().ok_or(anyhow!("Missing path"))?;
                        tools::list_code_definitions(path, &policy).await
                    }
                    _ => {
                        // Fall back to the full tool registry for any tool that is
                        // not in the built-in ACP dispatch above.  This covers:
                        //   task_create, task_update, enter_plan_mode, exit_plan_mode,
                        //   enter_worktree, exit_worktree, notebook_edit, execute_skill,
                        //   list_skills, spawn_agent, send_message, team_create,
                        //   team_delete, mcp_call, lsp_query, tool_search, cron_create,
                        //   remote_trigger, sleep, synthetic_output, …
                        //
                        // The registry is the single source of truth for all tool
                        // implementations; keeping the ACP dispatch in sync manually
                        // was what caused the "Unknown tool: task_update" errors.
                        let ctx = tools::ToolContext::new(policy.clone());
                        tools::execute_tool(function_name, &args, &ctx).await
                    }
                };

                let (content, status) = match result {
                    Ok(s) => {
                        let tool_duration = tool_start.elapsed();
                        info!(
                            "✅ Tool completed in {:?} ({} bytes)",
                            tool_duration,
                            s.len()
                        );

                        // ── Tool success: write a compact entry to the tool log ──
                        crate::utils::tool_logger::log_tool_success(
                            function_name,
                            &args,
                            s.len(),
                            tool_duration.as_micros(),
                        );

                        (s, crate::acp::protocol::ToolCallStatus::Completed)
                    }
                    Err(e) => {
                        let tool_duration = tool_start.elapsed();
                        warn!("⚠️ Tool failed in {:?}: {}", tool_duration, e);

                        // 4. Update Bayesian Engine for Tool Failure
                        session.bayes_engine.update_from_tool_failure();

                        // Build a structured, actionable error message so the LLM knows
                        // exactly what went wrong and what to try instead of retrying blindly.
                        let error_str = e.to_string();

                        // ── Tool failure: write a detailed diagnostic entry to the tool log ──
                        // This records the working directory, trusted directories, and a
                        // human-readable hint so you can immediately see WHY the tool failed
                        // (e.g. "Access denied" because Grok was not launched from the project
                        // root, or a path typo causing "os error 3").
                        crate::utils::tool_logger::log_tool_error(
                            function_name,
                            &args,
                            &error_str,
                            tool_duration.as_micros(),
                            policy.working_directory(),
                            policy.trusted_directories(),
                        );

                        let mut error_content = crate::tools::tool_error::format_tool_error_for_llm(
                            function_name,
                            &args,
                            &error_str,
                        );

                        // If Bayesian confidence is also low, append an additional note
                        // encouraging the model to change strategy rather than looping.
                        if session.bayes_engine.is_low_confidence() {
                            error_content.push_str(
                                "\n\n[System Note: Bayesian confidence is low after this failure. \
                                Do NOT retry the same action. Re-evaluate your plan, verify your \
                                assumptions, or ask the user for clarification.]",
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
                    let hooks = self.hook_manager.read().await;
                    hooks.execute_after_tool(function_name, &args, &content)?;
                }
                // Push tool result into LLM message history so the model can see it
                session.messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": content,
                }));

                // ── DATA-TRACE: confirm exactly what is being pushed as tool result ──
                {
                    let preview = content[..content.len().min(300)]
                        .replace('\n', "↵")
                        .replace('\r', "");
                    info!(
                        "📦 DATA-TRACE tool_result_push: tool='{}' tcid='{}' \
                         bytes={} preview='{}'",
                        function_name,
                        tool_call.id,
                        content.len(),
                        preview
                    );
                    crate::utils::tool_logger::log_note(&format!(
                        "DATA-TRACE tool_result_push: tool='{}' tcid='{}' bytes={} \
                         preview='{}'",
                        function_name,
                        tool_call.id,
                        content.len(),
                        preview
                    ));
                }
                // ────────────────────────────────────────────────────────────

                // ── Anti-hallucination guard ─────────────────────────────────
                // When a tool returns a TOOL ERROR the LLM tends to ignore it
                // and fabricate an answer anyway.  Injecting an explicit system
                // message immediately after the error — right before the LLM
                // generates its reply — overrides that tendency.
                if content.starts_with("TOOL ERROR") {
                    session.messages.push(json!({
                        "role": "system",
                        "content": "⚠️ STOP — the tool call above returned an error. \
                                   You MUST NOT fabricate, guess, or invent any information. \
                                   Your only permitted response is to report the error to the \
                                   user in plain language and suggest they try again or check \
                                   the file manually. Do NOT return any JSON, task data, \
                                   titles, statuses, or descriptions."
                    }));
                }
                // 🔍 DEBUG: verify the tool message was actually inserted
                {
                    let last = session.messages.last().unwrap();
                    let role = last.get("role").and_then(|v| v.as_str()).unwrap_or("?");
                    let tcid = last
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("NONE");
                    let bytes = last
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.len())
                        .unwrap_or(0);

                    tracing::error!(
                        "🧪 DEBUG TOOL-PUSH → role='{}' tool_call_id='{}' bytes={} (should be >0)",
                        role,
                        tcid,
                        bytes
                    );

                    crate::utils::tool_logger::log_note(&format!(
                        "🧪 DEBUG TOOL-PUSH → role='{}' tool_call_id='{}' bytes={}",
                        role, tcid, bytes
                    ));
                }
                // ────────────────────────────────────────────────────────────
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
            .get_router()?
            .chat_completion_with_history(
                &messages,
                session.config.temperature,
                session.config.max_tokens,
                &session.config.model,
                None,
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

    // ── Bayes helpers (used by /bayes slash commands) ────────────────────────

    /// Return a human-readable visualization of the current Bayesian state
    /// (counts, probabilities, uncertainty, etc.).
    pub async fn get_bayes_visualize(&self, session_id: &SessionId) -> Result<String> {
        self.ensure_session_exists(&session_id.0).await;
        let sessions = self.sessions.read().await;
        if let Some(s) = sessions.get(&session_id.0) {
            Ok(s.bayes_engine.visualize())
        } else {
            Err(anyhow!("Session not found: {}", session_id.0))
        }
    }

    /// Reset the Bayesian engine for the given session.
    pub async fn reset_bayes(&self, session_id: &SessionId) -> Result<String> {
        let mut sessions = self.sessions.write().await;
        if let Some(s) = sessions.get_mut(&session_id.0) {
            s.bayes_engine = crate::bayes::BayesianEngine::new_with_config(&self.config.bayesian);
            Ok("Bayesian engine has been reset for this session.".to_string())
        } else {
            Err(anyhow!("Session not found: {}", session_id.0))
        }
    }

    /// Return a textual explanation of the current Bayesian state.
    pub async fn get_bayes_explain(&self, session_id: &SessionId) -> Result<String> {
        self.ensure_session_exists(&session_id.0).await;
        let sessions = self.sessions.read().await;
        if let Some(s) = sessions.get(&session_id.0) {
            // `explain` not implemented yet — fall back to visualize
            Ok(s.bayes_engine.visualize())
        } else {
            Err(anyhow!("Session not found: {}", session_id.0))
        }
    }

    // ── Goal helpers (used by /goal slash commands) ──────────────────────────

    /// Set (or replace) the current goal for the session.
    pub async fn set_session_goal(&self, session_id: &SessionId, goal: String) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(s) = sessions.get_mut(&session_id.0) {
            s.current_goal = Some(goal);
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", session_id.0))
        }
    }

    /// Clear the current goal for the session.
    pub async fn clear_session_goal(&self, session_id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(s) = sessions.get_mut(&session_id.0) {
            s.current_goal = None;
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", session_id.0))
        }
    }

    /// Return the current goal (if any) for the session.
    pub async fn get_session_goal(&self, session_id: &SessionId) -> Result<Option<String>> {
        let sessions = self.sessions.read().await;
        if let Some(s) = sessions.get(&session_id.0) {
            Ok(s.current_goal.clone())
        } else {
            Err(anyhow!("Session not found: {}", session_id.0))
        }
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
        assert_eq!(config.model, "grok-code-fast-1");
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
            cwd: String::new(),
            messages: Vec::new(),
            config: SessionConfig::default(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            always_allow: std::collections::HashSet::new(),
            client_commands: Vec::new(),
            bayes_engine: crate::bayes::BayesianEngine::new(),
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
            .initialize_session(session_id.clone(), ".".to_string(), None)
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
}
