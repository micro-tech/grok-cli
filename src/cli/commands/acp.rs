//! ACP (Agent Client Protocol) command handler for Zed integration
//!
//! This module handles all Agent Client Protocol operations, including starting
//! the ACP server for Zed editor integration, testing connections, and managing
//! ACP capabilities.

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::{Result, anyhow};
use colored::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, oneshot};
use tracing::{debug, error, info, warn};

use crate::acp::protocol::{
    AGENT_METHOD_NAMES, AgentCapabilities, AvailableCommandsUpdate, ContentBlock, ContentChunk,
    Implementation, InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse,
    PermissionOutcome, PromptRequest, PromptResponse, ProtocolVersion, RequestPermissionParams,
    SessionId, SessionNotification, SessionUpdate, StopReason, TextContent,
};
use crate::acp::slash_commands::{
    self, BuiltinResult, format_context_text, handle_builtin, parse_slash_command,
};
use crate::acp::tools;
use crate::acp::{GrokAcpAgent, PermissionBridge, SessionConfig};
use crate::cli::{create_spinner, print_error, print_info, print_success, print_warning};
use crate::config::Config;
use crate::utils::chat_logger;

/// Handle ACP-related commands
pub async fn handle_acp_action(action: crate::AcpAction, config: &Config) -> Result<()> {
    match action {
        crate::AcpAction::Server { port, host } => start_acp_server(port, &host, config).await,
        crate::AcpAction::Stdio { model, workspace } => {
            start_acp_stdio(config, model, workspace).await
        }
        crate::AcpAction::Test { address } => test_acp_connection(&address, config).await,
        crate::AcpAction::Capabilities => show_acp_capabilities().await,
    }
}

/// Start the ACP server for Zed integration
async fn start_acp_server(port: Option<u16>, host: &str, config: &Config) -> Result<()> {
    if !config.acp.enabled {
        print_warning(
            "ACP is disabled in configuration. Enable it with 'grok config set acp.enabled true'",
        );
        return Ok(());
    }

    let bind_port = port.or(config.acp.default_port).unwrap_or(0);
    let bind_addr = format!("{}:{}", host, bind_port);

    print_info(&format!("Starting ACP server on {}", bind_addr));

    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| anyhow!("Failed to bind ACP server to {}: {}", bind_addr, e))?;

    let actual_addr = listener.local_addr()?;
    print_success(&format!("ACP server listening on {}", actual_addr));

    if config.acp.dev_mode {
        print_info("Development mode enabled - additional debugging features available");
    }

    println!();
    println!("{}", "🔗 Zed Integration Instructions:".cyan().bold());
    println!("1. Open Zed editor");
    println!("2. Go to Settings → Extensions → Agent Client Protocol");
    println!("3. Add a new agent configuration:");
    println!("   {}", "   - Name: Grok AI".green());
    println!(
        "   {}",
        format!(
            "   - Command: grok acp server --port {}",
            actual_addr.port()
        )
        .green()
    );
    println!("   {}", format!("   - Address: {}", actual_addr).green());
    println!("4. Enable the agent and start coding!");
    println!();
    println!("{}", "Press Ctrl+C to stop the server".dimmed());

    let server_stats = Arc::new(RwLock::new(ServerStats::new()));
    let server_config = config.clone();

    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                info!("New ACP client connected: {}", client_addr);

                let stats = Arc::clone(&server_stats);
                let config = server_config.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_acp_client(stream, client_addr, stats, config).await {
                        error!("ACP client error ({}): {}", client_addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept ACP connection: {}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn start_acp_stdio(
    config: &Config,
    model: Option<String>,
    workspace: Option<String>,
) -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let agent = GrokAcpAgent::new(config.clone(), model).await?;

    // Trust an explicitly-supplied workspace root immediately — before any
    // ACP protocol messages arrive.  This is the most reliable way to handle
    // the case where Zed does not send workspaceRoot in initialize/session.new.
    //
    // In your Zed agent settings pass: --workspace ${workspaceFolder}
    // That tells Zed to substitute the open project's root directory.
    //
    // Also honour the GROK_WORKSPACE_ROOT environment variable as a fallback
    // for shells / CI environments that set it without CLI flags.
    let explicit_workspace = workspace
        .or_else(|| std::env::var("GROK_WORKSPACE_ROOT").ok())
        .or_else(|| std::env::var("WORKSPACE_ROOT").ok());

    if let Some(ref root) = explicit_workspace {
        info!("Explicit workspace root supplied at startup: {}", root);
        register_workspace_root(&agent, root);
    } else {
        // No explicit root.  Log the CWD so the user can see where grok thinks
        // the project is.  This is printed to stderr so it doesn't corrupt the
        // JSON-RPC stream on stdout.
        match std::env::current_dir() {
            Ok(cwd) => info!(
                "No explicit --workspace supplied; trusting CWD: {}",
                cwd.display()
            ),
            Err(e) => warn!("Could not determine CWD: {}", e),
        }
    }

    info!("Starting ACP session on stdio");
    run_acp_session(stdin, stdout, agent).await
}

/// Handle individual ACP client connections
async fn handle_acp_client(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    stats: Arc<RwLock<ServerStats>>,
    config: Config,
) -> Result<()> {
    info!("Handling ACP client: {}", client_addr);

    // Update connection stats
    {
        let mut stats_guard = stats.write().await;
        stats_guard.connections += 1;
        stats_guard.active_connections += 1;
    }

    // Create the Grok ACP agent
    let agent = GrokAcpAgent::new(config, None).await?;

    // Split stream
    let (reader, writer) = stream.into_split();

    // Handle the ACP protocol over the stream
    let result = run_acp_session(reader, writer, agent).await;

    // Update stats on disconnect
    {
        let mut stats_guard = stats.write().await;
        stats_guard.active_connections -= 1;
    }

    match result {
        Ok(()) => info!("ACP client {} disconnected cleanly", client_addr),
        Err(e) => {
            warn!("ACP client {} disconnected with error: {}", client_addr, e);
            return Err(e);
        }
    }

    Ok(())
}

/// Run an ACP session with a connected client
async fn run_acp_session<R, W>(reader: R, mut writer: W, agent: GrokAcpAgent) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Spawn a dedicated reader task to feed the message channel.
    // This allows us to handle bidirectional requests/responses without deadlocking.
    tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if msg_tx.send(line.clone()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("ACP reader error: {}", e);
                    break;
                }
            }
        }
        info!("ACP reader task terminating");
    });

    let mut pending_permissions: HashMap<String, oneshot::Sender<PermissionOutcome>> =
        HashMap::new();

    while let Some(line) = msg_rx.recv().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        debug!("Received message: {}", trimmed);

        // Attempt to parse as JSON
        match serde_json::from_str::<Value>(trimmed) {
            Ok(json_msg) => {
                // Handle JSON-RPC message
                if let Err(e) = handle_json_rpc(
                    &json_msg,
                    &mut writer,
                    &agent,
                    &mut pending_permissions,
                    &mut msg_rx,
                )
                .await
                {
                    error!("Error handling message: {}", e);
                }
            }
            Err(e) => {
                warn!("Invalid JSON received: {} (Error: {})", trimmed, e);
            }
        }
    }

    info!("ACP session completed");

    // End chat logging session
    if let Err(e) = chat_logger::end_session() {
        warn!("Failed to end chat logging session: {}", e);
    }

    Ok(())
}

async fn handle_json_rpc<W>(
    msg: &Value,
    writer: &mut W,
    agent: &GrokAcpAgent,
    pending_permissions: &mut HashMap<String, oneshot::Sender<PermissionOutcome>>,
    msg_rx: &mut tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    // Check if it's a request (has "method" and "id")
    if let (Some(method), Some(id)) = (msg.get("method").and_then(|m| m.as_str()), msg.get("id")) {
        info!("Handling request: {} (id: {})", method, id);

        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let response_result = if method == AGENT_METHOD_NAMES.initialize {
            handle_initialize(&params, agent).await
        } else if method == AGENT_METHOD_NAMES.session_new {
            handle_session_new(&params, agent).await
        } else if method == AGENT_METHOD_NAMES.session_prompt {
            handle_session_prompt(&params, agent, writer, pending_permissions, msg_rx).await
        } else {
            warn!("Unknown method: {}", method);
            Err(anyhow!("Method not found: {}", method))
        };

        let response = match response_result {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": e.to_string()
                }
            }),
        };

        // Send response
        let response_str = serde_json::to_string(&response)?;
        writer.write_all(response_str.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // After a successful session/new, advertise available slash commands.
        // Per the ACP spec the agent MAY send `available_commands_update`
        // immediately after the session is created.
        if method == AGENT_METHOD_NAMES.session_new
            && let Some(session_id) = response
                .get("result")
                .and_then(|r| r.get("sessionId"))
                .and_then(|s| s.as_str())
        {
            info!(
                "Sending available_commands_update for session: {}",
                session_id
            );
            if let Err(e) = send_available_commands_update(writer, session_id).await {
                warn!("Failed to send available_commands_update: {}", e);
            }
        }
    } else if let Some(id) = msg.get("id").and_then(|i| i.as_str()) {
        // This is a JSON-RPC response (has id, no method).
        // Check if it's a response to a pending permission request.
        if let Some(sender) = pending_permissions.remove(id) {
            info!("Received response for pending permission request: {}", id);
            if let Some(result) = msg.get("result") {
                match serde_json::from_value::<PermissionOutcome>(result.clone()) {
                    Ok(outcome) => {
                        let _ = sender.send(outcome);
                    }
                    Err(e) => {
                        error!("Failed to parse permission outcome: {}", e);
                        let _ = sender.send(PermissionOutcome::cancel(id.to_string()));
                    }
                }
            } else if msg.get("error").is_some() {
                // Client returned a JSON-RPC error (e.g. "Method not found").
                // This typically means the client (e.g. Zed) does not yet
                // support session/request_permission.  Auto-approve so tools
                // can still execute rather than being silently blocked.
                // To disable the permission gate entirely, set
                // acp.require_permission = false in your config.
                warn!(
                    "Client returned error for permission request {} — \
                     client may not support session/request_permission; \
                     auto-approving this tool call.",
                    id
                );
                let _ = sender.send(PermissionOutcome::proceed_once(id.to_string()));
            } else {
                warn!("Received response for {} but it has no 'result' field", id);
                let _ = sender.send(PermissionOutcome::cancel(id.to_string()));
            }
        } else {
            debug!("Received response with unknown id: {}", id);
        }
    } else if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
        // Notification
        info!("Received notification: {}", method);
    }

    Ok(())
}

/// Resolve a raw workspace path string sent by a client into a `PathBuf` that
/// can be trusted by the security policy.
///
/// Handles the following variations that Zed and other ACP clients may send:
///
/// - `file:///H:/GitHub/my-project`  — strip the `file://` URI scheme
/// - `file:///home/user/project`     — strip the `file://` URI scheme (Unix)
/// - `H:/GitHub/my-project`          — Windows path with forward slashes
/// - `/h/GitHub/my-project`          — WSL / Git-bash style Unix path on Windows
/// - `/home/user/project`            — normal Unix path
///
/// After normalisation the path is canonicalized to resolve symlinks.  If
/// canonicalization fails (e.g., the path does not yet exist) the normalised
/// but un-canonicalized path is returned instead of failing — this is
/// intentional because we must never silently drop a legitimate workspace root.
fn resolve_workspace_path(raw: &str) -> PathBuf {
    // Strip file:// URI scheme (handles file:// and file:///)
    let stripped = if raw.starts_with("file:///") {
        // URL-decode the path component
        urlencoding::decode(&raw[7..])
            .map(|s| s.into_owned())
            .unwrap_or_else(|_| raw[7..].to_string())
    } else if let Some(rest) = raw.strip_prefix("file://") {
        urlencoding::decode(rest)
            .map(|s| s.into_owned())
            .unwrap_or_else(|_| rest.to_string())
    } else {
        raw.to_string()
    };

    // On Windows, normalise forward slashes to backslashes.
    // Also handle the Git-bash / WSL path style "/h/foo" → "H:\foo".
    #[cfg(target_os = "windows")]
    let normalised = {
        let s = stripped.replace('/', "\\");
        // Handle two Windows path styles that both start with \X:
        //   \h\GitHub\project  — Git-bash / WSL (char[2] is '\')
        //   \H:\GitHub\project — Windows file URI decoded from raw[7..] of
        //                        "file:///H:/path" where raw[7..] = "/H:/path"
        //                        After replacing '/' with '\' → "\H:\path"
        if s.starts_with('\\')
            && s.len() >= 3
            && s.chars().nth(1).is_some_and(|c| c.is_ascii_alphabetic())
        {
            if s.chars().nth(2) == Some('\\') {
                // Git-bash: \h\path → H:\path
                let drive = s.chars().nth(1).unwrap().to_uppercase().next().unwrap();
                format!("{}:{}", drive, &s[2..])
            } else if s.chars().nth(2) == Some(':') {
                // Windows file URI: \H:\path → H:\path
                s[1..].to_string()
            } else {
                s
            }
        } else {
            s
        }
    };

    #[cfg(not(target_os = "windows"))]
    let normalised = stripped;

    let path = PathBuf::from(&normalised);

    // Attempt full canonicalization; fall back to the normalised path so that
    // we *always* register something rather than silently losing access.
    match path.canonicalize() {
        Ok(canonical) => {
            info!("Workspace path resolved: {} → {:?}", raw, canonical);
            canonical
        }
        Err(e) => {
            warn!(
                "Could not canonicalize workspace path '{}' ({}); \
                 using normalised path '{}' as trusted root",
                raw, e, normalised
            );
            path
        }
    }
}

/// Register a workspace root with the security policy, logging the outcome.
/// Always succeeds — a warning is emitted if the path looks suspicious but we
/// still add it so the user doesn't lose access.
fn register_workspace_root(agent: &GrokAcpAgent, raw_path: &str) {
    let resolved = resolve_workspace_path(raw_path);
    info!("Registering workspace root as trusted: {:?}", resolved);
    agent.security.add_trusted_directory(&resolved);
}

/// Walk up from a file path to find the project workspace root by looking for
/// common project markers (.git, Cargo.toml, package.json, .grok, etc.).
/// Falls back to the file's immediate parent directory if no marker is found.
fn find_workspace_root_from_path(file_path: &Path) -> PathBuf {
    // Start from the file's parent directory (or the path itself if it's a dir)
    let start = if file_path.is_dir() {
        file_path.to_path_buf()
    } else {
        file_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| file_path.to_path_buf())
    };

    // Common project root markers — ordered from most specific to least
    const MARKERS: &[&str] = &[
        ".git",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "setup.py",
        "go.mod",
        ".grok",
        "composer.json",
        "pom.xml",
        "build.gradle",
        ".svn",
    ];

    let mut current = start.clone();
    loop {
        // Stop before we reach the filesystem root (depth ≤ 2 on Windows means
        // something like "C:\" or "C:\Users" — too broad to trust wholesale).
        if current.components().count() <= 2 {
            break;
        }
        for marker in MARKERS {
            if current.join(marker).exists() {
                return current;
            }
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }
    // Fallback: trust the starting directory (immediate parent of the file)
    start
}

/// Extract the workspace root from a resource URI (file:// or plain path) and
/// register it as a trusted directory with the security policy.
///
/// This is the mechanism that lets Grok access files in **any project that the
/// user has open in Zed**, not only the directory where the `grok` binary was
/// originally launched. When Zed embeds @-mentioned files via `ResourceLink` or
/// `Resource` blocks inside a `session/prompt` message, those URIs give us the
/// exact on-disk location of the resource. We walk up from that location to
/// find the project root and trust the entire project tree.
fn trust_workspace_from_uri(uri: &str, agent: &GrokAcpAgent) {
    // Only process URIs that look like local file references
    let looks_like_file = uri.starts_with("file://")
        || uri.starts_with('/')
        || (uri.len() > 2 && uri.chars().nth(1) == Some(':'))   // Windows  C:\...
        || uri.contains(":\\")
        || uri.contains(":/");

    if !looks_like_file {
        return;
    }

    let file_path = resolve_workspace_path(uri);
    let workspace_root = find_workspace_root_from_path(&file_path);

    info!(
        "Auto-trusting workspace root from resource URI '{}' → {:?}",
        uri, workspace_root
    );
    agent.security.add_trusted_directory(&workspace_root);
}

async fn handle_initialize(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {
    info!("Received initialize request with params: {}", params);

    // Parse the initialize request with better error handling
    let req: InitializeRequest = match serde_json::from_value::<InitializeRequest>(params.clone()) {
        Ok(req) => {
            info!(
                "Successfully parsed initialize request: protocol_version={}",
                req.protocol_version
            );
            req
        }
        Err(e) => {
            error!("Failed to parse initialize parameters: {}", e);
            error!(
                "Raw params received: {}",
                serde_json::to_string_pretty(params).unwrap_or_default()
            );
            return Err(anyhow!(
                "Invalid initialize parameters: {}. Received: {}",
                e,
                params
            ));
        }
    };

    info!("Client info: {}", req.client_info);
    info!("Client capabilities: {}", req.capabilities);

    // Some clients (including certain Zed versions) send the workspace root
    // as part of the initialize request rather than (or in addition to)
    // session/new.  Register it immediately so that file access works even
    // before session/new is received.
    let workspace_root = req.workspace_root.or(req.working_directory).or_else(|| {
        // Also check well-known environment variables as a last resort.
        std::env::var("WORKSPACE_ROOT")
            .or_else(|_| std::env::var("CODER_AGENT_WORKSPACE_PATH"))
            .ok()
    });

    if let Some(ref root) = workspace_root {
        info!("Workspace root received in initialize: {}", root);
        register_workspace_root(agent, root);
    } else {
        info!("No workspace root in initialize request; will rely on session/new or CWD");
    }

    let mut caps = AgentCapabilities::new();
    // Enable session capabilities
    caps.session_capabilities = crate::acp::protocol::SessionCapabilities::new();
    // Configure other capabilities as needed

    // Echo back the client's protocol version
    let response = InitializeResponse::new(&req.protocol_version)
        .agent_capabilities(caps)
        .agent_info(Implementation::new("grok-cli", env!("CARGO_PKG_VERSION")));

    info!(
        "Sending initialize response: protocol_version={}",
        req.protocol_version
    );
    Ok(serde_json::to_value(response)?)
}

async fn handle_session_new(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {
    let req: NewSessionRequest = serde_json::from_value(params.clone())
        .map_err(|e| anyhow!("Invalid session/new parameters: {}", e))?;

    // Extract workspace context from request or environment.
    // Use the robust resolver so that file:// URIs, forward-slash Windows
    // paths, and canonicalization failures are all handled gracefully.
    let workspace_root = req
        .workspace_root
        .or(req.working_directory)
        .or_else(|| std::env::var("CODER_AGENT_WORKSPACE_PATH").ok())
        .or_else(|| std::env::var("WORKSPACE_ROOT").ok());

    if let Some(ref workspace_path) = workspace_root {
        info!(
            "session/new: registering workspace root '{}'",
            workspace_path
        );
        register_workspace_root(agent, workspace_path);
    } else {
        // No workspace root provided — make sure the CWD is trusted.
        // (GrokAcpAgent::new already does this, but we re-add it here as a
        //  safety net in case the binary was launched from a different dir.)
        match std::env::current_dir() {
            Ok(cwd) => {
                let canonical_cwd = cwd.canonicalize().unwrap_or(cwd);
                info!(
                    "session/new: no workspace root provided, \
                     trusting CWD {:?}",
                    canonical_cwd
                );
                agent.security.add_trusted_directory(canonical_cwd);
            }
            Err(e) => {
                warn!("session/new: could not determine CWD: {}", e);
            }
        }
    }

    // Generate a session ID
    let session_id_str = uuid::Uuid::new_v4().to_string();
    let session_id = SessionId::new(session_id_str.clone());

    // Initialize session in GrokAcpAgent
    agent
        .initialize_session(session_id, Some(SessionConfig::default()))
        .await?;

    // Start chat logging for this session
    if let Err(e) = chat_logger::start_session(&session_id_str) {
        warn!(
            "Failed to start chat logging for session {}: {}",
            session_id_str, e
        );
    } else {
        info!("Chat logging started for session: {}", session_id_str);
        // Log session initialization
        if let Err(e) = chat_logger::log_system(format!("Session {} initialized", session_id_str)) {
            warn!("Failed to log system message: {}", e);
        }
    }

    let response = NewSessionResponse::new(SessionId::new(session_id_str));
    Ok(serde_json::to_value(response)?)
}

async fn handle_session_prompt<W>(
    params: &Value,
    agent: &GrokAcpAgent,
    writer: &mut W,
    pending_permissions: &mut HashMap<String, oneshot::Sender<PermissionOutcome>>,
    msg_rx: &mut tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<Value>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let req: PromptRequest = serde_json::from_value(params.clone())
        .map_err(|e| anyhow!("Invalid session/prompt parameters: {}", e))?;

    let session_id = SessionId::new(req.session_id.0.clone());

    // Extract text from prompt.
    let mut message_text = String::new();
    for block in req.prompt {
        match block {
            ContentBlock::Text(text) => message_text.push_str(&text.text),
            ContentBlock::ResourceLink(link) => {
                trust_workspace_from_uri(&link.uri, agent);
                message_text.push_str(&format!("\n[Resource: {} ({})]", link.name, link.uri));
            }
            ContentBlock::Resource(res) => {
                let crate::acp::protocol::EmbeddedResourceResource::TextResourceContents(text_res) =
                    res.resource;
                trust_workspace_from_uri(&text_res.uri, agent);
                message_text.push_str(&format!(
                    "\n[Context: {}]\n{}\n",
                    text_res.uri, text_res.text
                ));
            }
        }
    }

    if message_text.is_empty() {
        return Err(anyhow!("Empty prompt received"));
    }

    // Create PermissionBridge for this prompt execution
    let (perm_bridge, mut perm_req_rx) = PermissionBridge::new();
    let perm_bridge_arc = Arc::new(perm_bridge);

    // --- Slash command detection & dispatch ---
    if let Some(cmd) = parse_slash_command(&message_text) {
        info!("Slash command detected: {:?}", cmd);

        if let Some(builtin) = handle_builtin(&cmd) {
            let response_text = match builtin {
                BuiltinResult::Text(text) => text,
                BuiltinResult::ClearHistory => {
                    let _ = agent.clear_session_history(&session_id).await;
                    let _ = chat_logger::log_system("Conversation history cleared");
                    "✅ Conversation history cleared. Starting fresh!".to_string()
                }
                BuiltinResult::SwitchModel(model_name) => {
                    match agent
                        .set_session_model(&session_id, model_name.clone())
                        .await
                    {
                        Ok(()) => format!("✅ Switched to model **`{model_name}`**."),
                        Err(e) => format!("❌ Could not switch model: {e}"),
                    }
                }
                BuiltinResult::ShowContext => match agent.get_session_config(&session_id).await {
                    Ok(cfg) => {
                        let msg_count = agent
                            .get_session_message_count(&session_id)
                            .await
                            .unwrap_or(0);
                        format_context_text(
                            &session_id.0,
                            &cfg.model,
                            cfg.temperature,
                            cfg.max_tokens,
                            msg_count,
                        )
                    }
                    Err(e) => format!("❌ Could not retrieve context: {e}"),
                },
            };

            send_text_update(writer, &session_id.0, &response_text).await?;
            return Ok(serde_json::to_value(PromptResponse::new(
                StopReason::EndTurn,
            ))?);
        }

        if let Some(ai_prompt) = slash_commands::command_to_prompt(&cmd) {
            let _ = chat_logger::log_user(&message_text);
            let response_text = agent
                .handle_chat_completion(
                    &session_id,
                    &ai_prompt,
                    None,
                    None,
                    Some(perm_bridge_arc.clone()),
                )
                .await?;
            send_text_update(writer, &session_id.0, &response_text).await?;
            return Ok(serde_json::to_value(PromptResponse::new(
                StopReason::EndTurn,
            ))?);
        }
    }

    // Log user prompt
    let _ = chat_logger::log_user(&message_text);

    info!("Calling Grok API for session {}...", session_id.0);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let chat_fut = agent.handle_chat_completion(
        &session_id,
        &message_text,
        None,
        Some(tx),
        Some(perm_bridge_arc),
    );
    tokio::pin!(chat_fut);

    let response_text;
    loop {
        tokio::select! {
            // 1. Agent updates (notifications like tool calls)
            update = rx.recv() => {
                if let Some(update) = update {
                    let params = SessionNotification::new(session_id.clone(), update);
                    let notification = json!({
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": params
                    });
                    let msg = serde_json::to_string(&notification)?;
                    writer.write_all(msg.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
            }

            // 2. Permission requests from the agent
            perm_req = perm_req_rx.recv() => {
                if let Some((params, outcome_tx)) = perm_req {
                    info!("Forwarding permission request {} to client", params.request_id);
                    let request = json!({
                        "jsonrpc": "2.0",
                        "id": params.request_id,
                        "method": "session/request_permission",
                        "params": params
                    });

                    let msg = serde_json::to_string(&request)?;
                    if let Err(e) = writer.write_all(msg.as_bytes()).await {
                        error!("Failed to write permission request: {}", e);
                        let _ = outcome_tx.send(PermissionOutcome::cancel("io_error"));
                        continue;
                    }
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;

                    // Track this pending permission so handle_json_rpc can complete it
                    pending_permissions.insert(params.request_id.clone(), outcome_tx);
                }
            }

            // 3. New messages from the client (e.g. permission responses)
            line = msg_rx.recv() => {
                if let Some(line) = line
                    && let Ok(json_msg) = serde_json::from_str::<Value>(&line)
                {
                        // Check if it's a response to a pending permission request.
                        // We handle this directly here to avoid async recursion with handle_json_rpc.
                        // Accept both string and numeric JSON-RPC response IDs.
                        let id_opt = json_msg.get("id")
                            .and_then(|v| v.as_str().map(str::to_string)
                                .or_else(|| v.as_u64().map(|n| n.to_string())));
                        if let Some(id) = id_opt {
                            if let Some(sender) = pending_permissions.remove(&id) {
                                info!("Received response for pending permission request: {}", id);
                                if let Some(result) = json_msg.get("result") {
                                    match serde_json::from_value::<PermissionOutcome>(result.clone()) {
                                        Ok(outcome) => {
                                            let _ = sender.send(outcome);
                                        }
                                        Err(e) => {
                                            error!("Failed to parse permission outcome: {}", e);
                                            let _ = sender.send(PermissionOutcome::cancel(id.clone()));
                                        }
                                    }
                                } else if json_msg.get("error").is_some() {
                                    // Client returned a JSON-RPC error (e.g. "Method not found").
                                    // This typically means the client (e.g. Zed) does not yet
                                    // support session/request_permission.  Auto-approve so tools
                                    // can still execute rather than being silently blocked.
                                    // To disable the permission gate entirely, set
                                    // acp.require_permission = false in your config.
                                    warn!(
                                        "Client returned error for permission request {} — \
                                         client may not support session/request_permission; \
                                         auto-approving this tool call.",
                                        id
                                    );
                                    let _ = sender.send(PermissionOutcome::proceed_once(id.clone()));
                                } else {
                                    warn!("Received response for {} but it has no 'result' field", id);
                                    let _ = sender.send(PermissionOutcome::cancel(id.clone()));
                                }
                            } else {
                                // If it's not a permission response, it might be a new request.
                                // In that case, we should probably ignore it or log a warning
                                // because we are busy processing a prompt.
                                debug!("Received message with unknown/untethered id while in prompt: {}", id);
                            }
                        }
                    }
            }

            // 4. Final chat result
            res = &mut chat_fut => {
                response_text = res?;
                break;
            }
        }
    }

    let final_text = if response_text.is_empty() {
        "[No response content]".to_string()
    } else {
        response_text
    };
    let _ = chat_logger::log_assistant(&final_text);
    send_text_update(writer, &session_id.0, &final_text).await?;

    Ok(serde_json::to_value(PromptResponse::new(
        StopReason::EndTurn,
    ))?)
}

/// Send an `available_commands_update` notification to the client advertising
/// all slash commands that this agent supports.
///
/// Per the ACP spec this is a JSON-RPC notification (no `id` field) sent over
/// the same channel as regular responses.  It MUST be sent after the
/// `session/new` response so the client can populate its command palette.
async fn send_available_commands_update<W>(writer: &mut W, session_id: &str) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let commands = slash_commands::get_available_commands();
    let count = commands.len();

    let update = SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(commands));
    let params = SessionNotification::new(SessionId::new(session_id), update);

    let notification = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": params
    });

    let msg = serde_json::to_string(&notification)?;
    info!(
        "Sending available_commands_update ({} commands) for session {}",
        count, session_id
    );
    debug!("available_commands_update payload: {}", msg);

    writer.write_all(msg.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    info!("available_commands_update sent successfully");
    Ok(())
}

/// Helper to send text update notification
async fn send_text_update<W>(writer: &mut W, session_id: &str, text: &str) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    info!(
        "Sending text update for session {}: {} chars",
        session_id,
        text.len()
    );
    debug!("Text content: {}", text);

    // Create the content block with text
    let content = ContentBlock::Text(TextContent::new(text));
    debug!("Created content block: {:?}", content);

    // Create the update chunk
    let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(content));
    debug!("Created update: {:?}", update);

    // Create the notification params
    let params = SessionNotification::new(SessionId::new(session_id), update);
    debug!("Created notification params: {:?}", params);

    // ACP uses `session/update` notification
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": params
    });

    let msg = serde_json::to_string(&notification)?;
    info!("Sending notification JSON:");
    info!("{}", msg);

    // Pretty print for debugging
    if let Ok(pretty) = serde_json::to_string_pretty(&notification) {
        debug!("Pretty notification:\n{}", pretty);
    }

    writer.write_all(msg.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    info!("Text update notification sent successfully");
    Ok(())
}

/// Test ACP connection to a running server
async fn test_acp_connection(address: &str, config: &Config) -> Result<()> {
    print_info(&format!("Testing ACP connection to {}", address));

    let spinner = create_spinner("Connecting...");

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        test_acp_connection_inner(address, config),
    )
    .await;

    spinner.finish_and_clear();

    match result {
        Ok(Ok(())) => {
            print_success("ACP connection test successful");
            Ok(())
        }
        Ok(Err(e)) => {
            print_error(&format!("ACP connection test failed: {}", e));
            Err(e)
        }
        Err(_) => {
            let err = anyhow!("ACP connection test timed out");
            print_error(&err.to_string());
            Err(err)
        }
    }
}

async fn test_acp_connection_inner(address: &str, _config: &Config) -> Result<()> {
    // Try to connect to the ACP server
    let stream = tokio::net::TcpStream::connect(address)
        .await
        .map_err(|e| anyhow!("Failed to connect to ACP server at {}: {}", address, e))?;

    debug!("Connected to ACP server at {}", address);

    // Send a basic ping/test message
    // This would be a proper ACP message in a full implementation

    // Close connection cleanly
    drop(stream);

    Ok(())
}

/// Show ACP capabilities and information
async fn show_acp_capabilities() -> Result<()> {
    println!("{}", "🔧 Grok CLI ACP Capabilities".cyan().bold());
    println!();

    let capabilities = get_acp_capabilities();

    // Protocol Information
    println!("{}", "Protocol Information:".green().bold());
    println!("  Version: {}", capabilities.protocol_version);
    println!("  Implementation: {}", capabilities.implementation);
    println!("  Features: {}", capabilities.features.join(", "));
    println!();

    // Tools
    println!("{}", "Available Tools:".green().bold());
    for tool in &capabilities.tools {
        println!("  • {} - {}", tool.name.cyan(), tool.description);
        if !tool.parameters.is_empty() {
            println!("    Parameters: {}", tool.parameters.join(", "));
        }
    }
    println!();

    // Models
    println!("{}", "Supported Models:".green().bold());
    for model in &capabilities.models {
        println!("  • {} - {}", model.name.cyan(), model.description);
        println!(
            "    Max tokens: {}, Context: {}",
            model.max_tokens, model.context_length
        );
    }
    println!();

    // Configuration
    println!("{}", "Configuration:".green().bold());
    println!("  Default timeout: {}s", capabilities.default_timeout);
    println!("  Max retries: {}", capabilities.max_retries);
    println!(
        "  Concurrent sessions: {}",
        capabilities.max_concurrent_sessions
    );

    Ok(())
}

/// Get the current ACP capabilities
fn get_acp_capabilities() -> AcpCapabilities {
    AcpCapabilities {
        protocol_version: "1.0".to_string(),
        implementation: "grok-cli".to_string(),
        features: vec![
            "chat_completion".to_string(),
            "code_generation".to_string(),
            "code_review".to_string(),
            "code_explanation".to_string(),
            "file_operations".to_string(),
        ],
        tools: vec![
            ToolInfo {
                name: "chat_complete".to_string(),
                description: "Generate chat completions using Grok AI".to_string(),
                parameters: vec![
                    "message".to_string(),
                    "temperature".to_string(),
                    "max_tokens".to_string(),
                ],
            },
            ToolInfo {
                name: "code_explain".to_string(),
                description: "Explain code functionality and structure".to_string(),
                parameters: vec!["code".to_string(), "language".to_string()],
            },
            ToolInfo {
                name: "code_review".to_string(),
                description: "Review code for issues and improvements".to_string(),
                parameters: vec!["code".to_string(), "focus".to_string()],
            },
            ToolInfo {
                name: "code_generate".to_string(),
                description: "Generate code from natural language descriptions".to_string(),
                parameters: vec!["description".to_string(), "language".to_string()],
            },
        ],
        models: vec![
            ModelInfo {
                name: "grok-3".to_string(),
                description: "Grok 3 flagship model".to_string(),
                max_tokens: 131072,
                context_length: 131072,
            },
            ModelInfo {
                name: "grok-3-mini".to_string(),
                description: "Efficient Grok 3 mini model".to_string(),
                max_tokens: 131072,
                context_length: 131072,
            },
            ModelInfo {
                name: "grok-2".to_string(),
                description: "Grok 2 model".to_string(),
                max_tokens: 131072,
                context_length: 131072,
            },
            ModelInfo {
                name: "grok-beta".to_string(),
                description: "Grok Beta model".to_string(),
                max_tokens: 131072,
                context_length: 131072,
            },
        ],
        default_timeout: 30,
        max_retries: 3,
        max_concurrent_sessions: 10,
    }
}

/// ACP capabilities structure
#[derive(Debug)]
struct AcpCapabilities {
    protocol_version: String,
    implementation: String,
    features: Vec<String>,
    tools: Vec<ToolInfo>,
    models: Vec<ModelInfo>,
    default_timeout: u64,
    max_retries: u32,
    max_concurrent_sessions: u32,
}

/// Tool information for ACP capabilities
#[derive(Debug)]
struct ToolInfo {
    name: String,
    description: String,
    parameters: Vec<String>,
}

/// Model information for ACP capabilities
#[derive(Debug)]
struct ModelInfo {
    name: String,
    description: String,
    max_tokens: u32,
    context_length: u32,
}

/// Server statistics tracking
#[derive(Debug, Default)]
struct ServerStats {
    connections: u64,
    active_connections: u64,
    requests_processed: u64,
    errors: u64,
    start_time: Option<std::time::Instant>,
}

impl ServerStats {
    fn new() -> Self {
        Self {
            start_time: Some(std::time::Instant::now()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acp_capabilities() {
        let capabilities = get_acp_capabilities();
        assert_eq!(capabilities.protocol_version, "1.0");
        assert!(!capabilities.tools.is_empty());
        assert!(!capabilities.models.is_empty());
    }

    #[test]
    fn test_server_stats() {
        let mut stats = ServerStats::new();
        assert_eq!(stats.connections, 0);
        assert_eq!(stats.active_connections, 0);

        stats.connections += 1;
        stats.active_connections += 1;
        assert_eq!(stats.connections, 1);
        assert_eq!(stats.active_connections, 1);
    }
}
