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
use dirs;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::acp::protocol::{
    AcpModeInfo, AcpModelInfo, AcpModelsInfo, AcpModesInfo, AgentCapabilities, AuthEnvVar,
    AuthMethod, AvailableCommandsUpdate, ContentBlock, ContentChunk, Implementation,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse,
    PermissionOutcome, PromptRequest, SessionId, SessionInfo, SessionListRequest,
    SessionListResponse, SessionLoadRequest, SessionNotification, SessionUpdate, TextContent,
};
use crate::acp::slash_commands::{
    self, BuiltinResult, format_context_text, handle_builtin, parse_slash_command,
};
use crate::acp::{GrokAcpAgent, PermissionBridge, SessionConfig};
use crate::cli::{create_spinner, format_error, format_info, format_success, format_warning};
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
        println!(
            "{}",
            format_warning(
                "ACP is disabled in configuration. Enable it with 'grok config set acp.enabled true'"
            )
        );
        return Ok(());
    }

    let bind_port = port.or(config.acp.default_port).unwrap_or(0);
    let bind_addr = format!("{}:{}", host, bind_port);

    println!(
        "{}",
        format_info(&format!("Starting ACP server on {}", bind_addr))
    );

    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| anyhow!("Failed to bind ACP server to {}: {}", bind_addr, e))?;

    let actual_addr = listener.local_addr()?;
    println!(
        "{}",
        format_success(&format!("ACP server listening on {}", actual_addr))
    );

    if config.acp.dev_mode {
        println!(
            "{}",
            format_info("Development mode enabled - additional debugging features available")
        );
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

    // Initialise the global chat logger.
    // Chat logs are project-scoped: we prefer .grok/logs/chat_sessions/ in the
    // current project tree. If none exists we fall back to the global location.
    let chat_log_dir = resolve_chat_log_dir();
    let chat_config = chat_logger::ChatLoggerConfig {
        enabled: true,
        log_dir: chat_log_dir,
        json_format: true,
        text_format: true,
        ..Default::default()
    };
    if let Err(e) = chat_logger::init(chat_config) {
        warn!("Could not initialise chat logger: {e} — chat history will not be saved");
    } else {
        info!("Chat logger initialised (stdio mode)");
    }

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
    debug!("ACP stdio transport initialized — waiting for Zed to send initialize/session.new");
    let result = run_acp_session(stdin, stdout, agent).await;

    // ── IMPORTANT: Detect when Zed closes the connection ─────────────────────
    // When Zed exits or closes the agent panel, it closes stdin/stdout.
    // We must explicitly exit here, otherwise grok-cli stays alive in memory.
    match result {
        Ok(()) => {
            info!("ACP stdio connection closed cleanly (Zed exited or closed agent)");
            debug!("stdin EOF detected — Zed terminated the ACP session");
        }
        Err(e) => {
            warn!("ACP stdio connection closed with error: {}", e);
            debug!("Connection ended due to: {:?}", e);
        }
    }

    info!("grok-cli ACP agent shutting down");

    // Force a clean process exit.
    // Without this, the Tokio runtime and any background tasks (logging,
    // session persistence, MCP, etc.) can keep the process alive after Zed
    // closes stdin/stdout.  This is the standard pattern used by other ACP
    // agents (Gemini CLI, Claude Code, etc.).
    //
    // On Windows this also ensures grok.exe disappears from Task Manager
    // promptly instead of lingering as a zombie process.
    //
    // We use a tiny sleep first so that any final tracing/logs can flush.
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::process::exit(0);
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

    // Initialise the global chat logger (TCP server mode).
    // Only the first call takes effect because OnceCell / Mutex prevents
    // double-initialisation; subsequent clients reuse the same logger.
    // Chat logs are project-scoped.
    let chat_log_dir = resolve_chat_log_dir();
    let chat_config = chat_logger::ChatLoggerConfig {
        enabled: true,
        log_dir: chat_log_dir,
        json_format: true,
        text_format: true,
        ..Default::default()
    };
    if let Err(e) = chat_logger::init(chat_config) {
        warn!("Could not initialise chat logger: {e} — chat history will not be saved");
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

/// Run an ACP session using the official `agent-client-protocol` crate's
/// `Agent::builder()` pattern (Task 111.3).
///
/// ## What changed from the previous BufReader loop
///
/// The old implementation manually read newline-delimited JSON from a
/// `BufReader`, dispatched via a giant `handle_json_rpc` match, and wrote
/// responses by calling `writer.write_all(json_bytes)` directly.
///
/// The new implementation delegates transport management to
/// `ByteStreams::new(writer, reader)` and registers one typed handler per ACP
/// method.  Each handler converts between the crate's typed schema types and
/// our local types (via `serde_json` round-trip where necessary), then uses
/// `cx.send_notification()` for outgoing `session/update` messages instead of
/// raw `writer` writes.
///
/// `session/prompt` is run inside `cx.spawn()` so the event loop remains
/// responsive to new messages (session/cancel, etc.) while the AI call is in
/// flight.
async fn run_acp_session<R, W>(reader: R, writer: W, agent: GrokAcpAgent) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use agent_client_protocol::schema::v1::{
        ClientNotification, ClientRequest, InitializeRequest as AcpInitReq,
        InitializeResponse as AcpInitResp, ListSessionsRequest as AcpListReq,
        ListSessionsResponse as AcpListResp, LoadSessionRequest as AcpLoadReq,
        LoadSessionResponse as AcpLoadResp, NewSessionRequest as AcpNewReq,
        NewSessionResponse as AcpNewResp, PromptRequest as AcpPromptReq,
        PromptResponse as AcpPromptResp,
    };
    use agent_client_protocol::{
        Agent, ByteStreams, Client, ConnectionTo, Dispatch, Responder, on_receive_dispatch,
        on_receive_notification, on_receive_request,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    let agent = Arc::new(agent);
    let initialized = Arc::new(AtomicBool::new(false));

    // Each handler closure needs its own Arc handle.
    let a_init = Arc::clone(&agent);
    let a_new = Arc::clone(&agent);
    let a_prompt = Arc::clone(&agent);
    let a_list = Arc::clone(&agent);
    let a_load = Arc::clone(&agent);
    let a_notif = Arc::clone(&agent);
    let a_ext = Arc::clone(&agent);
    let init_new = Arc::clone(&initialized);

    let transport = ByteStreams::new(writer.compat_write(), reader.compat());

    let r = Agent
        .builder()
        .name("grok-cli")
        // ── initialize ────────────────────────────────────────────────────────
        .on_receive_request(
            move |req: AcpInitReq, responder: Responder<AcpInitResp>, _cx: ConnectionTo<Client>| {
                let agent = Arc::clone(&a_init);
                async move {
                    let params = serde_json::to_value(&req).unwrap_or_else(|_| json!({}));
                    let val = handle_initialize(&params, &agent).await?;
                    let resp: AcpInitResp = serde_json::from_value(val)
                        .map_err(|e| anyhow!("initialize resp serialization: {e}"))?;
                    responder.respond(resp)
                }
            },
            on_receive_request!(),
        )
        // ── session/new ───────────────────────────────────────────────────────
        // We use the crate's typed AcpNewReq to receive the message, then
        // extract the CWD ourselves.  The crate's NewSessionRequest has `cwd`
        // as a required PathBuf, but real clients also send it as
        // `workingDirectory` or `workspaceRoot` — our local EXTEND type handles
        // all those aliases.  We therefore pass the raw JSON params to our
        // existing handle_session_new which uses the local type for parsing.
        //
        // Production note: if the crate fails to parse (e.g. missing cwd) the
        // handler returns an error and the test/client must include cwd.
        .on_receive_request(
            move |req: AcpNewReq, responder: Responder<AcpNewResp>, cx: ConnectionTo<Client>| {
                let agent = Arc::clone(&a_new);
                let init = Arc::clone(&init_new);
                async move {
                    // Auto-init if client skipped initialize (Gemini CLI)
                    if !init.load(Ordering::SeqCst) {
                        ensure_default_initialized(&agent, &mut false);
                        init.store(true, Ordering::SeqCst);
                    }
                    // Convert crate's typed request → Value for our local handler.
                    // Note: the crate's NewSessionRequest requires 'cwd' in the
                    // incoming JSON (this is the ACP spec standard). Clients that
                    // use alternate field names (workingDirectory, workspaceRoot)
                    // are handled by the crate's own alias support (if any).
                    // Known limitation: our old EXTEND type accepted more aliases.
                    // Documented in Doc/acp-migration-map.md — task 111.3.
                    let params = serde_json::to_value(&req).unwrap_or_else(|_| json!({}));
                    let val = handle_session_new(&params, &agent).await?;
                    let sid = val["sessionId"].as_str().unwrap_or("").to_string();
                    let resp: AcpNewResp = serde_json::from_value(val)
                        .map_err(|e| anyhow!("session/new resp serialization: {e}"))?;
                    responder.respond(resp)?;
                    // Advertise slash commands to Zed's picker.
                    // Retry with exponential back-off: the single-shot send
                    // races with Zed processing the session/new response, and
                    // a dropped notification silently empties the slash picker.
                    if !sid.is_empty() {
                        let mut sent = false;
                        for delay_ms in [0u64, 80, 250] {
                            if delay_ms > 0 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms))
                                    .await;
                            }
                            if send_available_commands_update_cx(&cx, &sid).is_ok() {
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            warn!(
                                "available_commands_update failed after 3 attempts \
                                 for session {} — picker will refresh on first prompt",
                                sid
                            );
                        }
                    }
                    Ok(())
                }
            },
            on_receive_request!(),
        )
        // ── session/prompt — uses cx.spawn() to avoid blocking the event loop
        //    while the AI call is in flight.  Streaming chunks and permission
        //    requests are sent via cx from within the spawned task.
        .on_receive_request(
            move |req: AcpPromptReq,
                  responder: Responder<AcpPromptResp>,
                  cx: ConnectionTo<Client>| {
                let agent = Arc::clone(&a_prompt);
                async move {
                    let cx2 = cx.clone();
                    cx.spawn(
                        async move { handle_session_prompt_v2(req, responder, cx2, agent).await },
                    )?;
                    Ok(())
                }
            },
            on_receive_request!(),
        )
        // ── session/list ──────────────────────────────────────────────────────
        .on_receive_request(
            move |_req: AcpListReq,
                  responder: Responder<AcpListResp>,
                  _cx: ConnectionTo<Client>| {
                let agent = Arc::clone(&a_list);
                async move {
                    let val = handle_session_list(&json!({}), &agent).await?;
                    let resp: AcpListResp = serde_json::from_value(val)
                        .map_err(|e| anyhow!("session/list resp serialization: {e}"))?;
                    responder.respond(resp)
                }
            },
            on_receive_request!(),
        )
        // ── session/load ──────────────────────────────────────────────────────
        .on_receive_request(
            move |req: AcpLoadReq, responder: Responder<AcpLoadResp>, cx: ConnectionTo<Client>| {
                let agent = Arc::clone(&a_load);
                async move {
                    // Convert crate's LoadSessionRequest → Value for our handler.
                    let params = serde_json::to_value(&req).unwrap_or_else(|_| json!({}));
                    let sid = params["sessionId"].as_str().unwrap_or("").to_string();

                    // Initialize (or restore) the session so that subsequent
                    // session/prompt calls have a valid session in the agent's
                    // sessions map and a live chat-logger session.
                    if !sid.is_empty() {
                        if let Err(e) = handle_session_load(&params, &agent).await {
                            warn!("session/load: session setup failed for '{}': {}", sid, e);
                        }
                    }

                    // Build a LoadSessionResponse — try several JSON shapes
                    // since the crate type may be #[non_exhaustive].
                    let resp: AcpLoadResp =
                        [json!({"content": []}), json!({"messages": []}), json!({})]
                            .into_iter()
                            .find_map(|j| serde_json::from_value(j).ok())
                            .ok_or_else(|| anyhow!("Cannot construct LoadSessionResponse"))?;
                    responder.respond(resp)?;

                    // Re-advertise commands after load with the same retry
                    // strategy used by session/new.
                    if !sid.is_empty() {
                        let mut sent = false;
                        for delay_ms in [0u64, 80, 250] {
                            if delay_ms > 0 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms))
                                    .await;
                            }
                            if send_available_commands_update_cx(&cx, &sid).is_ok() {
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            warn!(
                                "session/load: commands update failed after 3 attempts \
                                 for session {} — picker will refresh on first prompt",
                                sid
                            );
                        }
                    }
                    Ok(())
                }
            },
            on_receive_request!(),
        )
        // ── Client notifications (e.g. Gemini's available_commands_update) ────
        .on_receive_notification(
            move |notif: ClientNotification, _cx: ConnectionTo<Client>| {
                let agent = Arc::clone(&a_notif);
                async move {
                    handle_client_notification_v2(notif, &agent).await;
                    Ok(())
                }
            },
            on_receive_notification!(),
        )
        // ── Extension / fallthrough: session/load and known-but-unhandled methods.
        .on_receive_dispatch(
            {
                let agent = Arc::clone(&a_ext);
                move |msg: Dispatch<ClientRequest, ClientNotification>, cx: ConnectionTo<Client>| {
                    let agent = Arc::clone(&agent);
                    async move { handle_extension_dispatch(msg, cx, agent).await }
                }
            },
            on_receive_dispatch!(),
        )
        // Note: session/fork and session/set_model are non-standard methods not
        // in ClientRequest. They are not routed by the Builder in this version.
        // Task 140 (was 111.3 follow-up): custom JsonRpcRequest types live in
        // src/acp/protocol.rs as the JsonRpcRequest enum.  The typed handlers
        // for fork/set_model will be added in a later wiring step.
        .connect_to(transport)
        .await;

    if let Err(ref e) = r {
        info!("ACP session closed: {e}");
    }

    if let Err(e) = chat_logger::end_session() {
        warn!("Failed to end chat logging session: {e}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 111.3 helper: send an available_commands_update notification via cx.
// ---------------------------------------------------------------------------

fn send_available_commands_update_cx(
    cx: &agent_client_protocol::ConnectionTo<agent_client_protocol::Client>,
    session_id: &str,
) -> Result<()> {
    use agent_client_protocol::schema::v1::{
        SessionId as CrateSessionId, SessionNotification as CrateNotif,
        SessionUpdate as CrateUpdate,
    };
    let commands = slash_commands::get_available_commands();
    info!(
        "Advertising {} slash commands to ACP client (session {})",
        commands.len(),
        session_id
    );
    let update = CrateUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(commands));
    let notif = CrateNotif::new(CrateSessionId::new(session_id.to_string()), update);
    match cx.send_notification(notif) {
        Ok(_) => {
            info!(
                "Successfully sent available_commands_update for session {}",
                session_id
            );
            Ok(())
        }
        Err(e) => {
            warn!("Failed to send available_commands_update: {}", e);
            Err(anyhow!("send commands update: {e}"))
        }
    }
}

// ---------------------------------------------------------------------------
// 111.3 helper: convert a local SessionNotification to the crate's type.
// Both types serialize to the same ACP wire JSON, so a serde round-trip works.
// ---------------------------------------------------------------------------

/// Returns `true` for `SessionUpdate` variants that the `agent_client_protocol`
/// crate's schema understands on the wire.  Our custom extensions
/// (`StatusBarUpdate`, `ContextUsageUpdate`, `AgentActivity`, `ThinkingUpdate`,
/// `ThinkingBlockUpdate`, `Raw`) are grok-cli additions that Zed ignores for
/// now; the crate round-trip would panic/warn on them, so we skip them here.
/// The status-bar has its own `AgentMessageChunk` text fallback in
/// `emit_status_bar`, so nothing visible is lost.
#[inline]
fn is_crate_compatible(notif: &SessionNotification) -> bool {
    matches!(
        notif.update,
        SessionUpdate::AgentMessageChunk(_)
            | SessionUpdate::AvailableCommandsUpdate(_)
            | SessionUpdate::ToolCall(_)
            | SessionUpdate::ToolCallUpdate(_)
    )
}

fn local_notif_to_crate(
    local: &SessionNotification,
) -> Result<agent_client_protocol::schema::v1::SessionNotification> {
    let json = serde_json::to_value(local).map_err(|e| anyhow!("serialize local notif: {e}"))?;
    serde_json::from_value(json).map_err(|e| anyhow!("deserialize crate notif: {e}"))
}

/// Convert a local `SessionNotification` to the crate's type and send it via
/// `cx`.  Variants not in the crate's schema are silently skipped — they are
/// grok-cli extensions (status bar, context usage, thinking traces) that Zed
/// does not yet support natively.  A warning is only emitted when a supposedly
/// compatible variant fails to round-trip (which would indicate a real bug).
fn send_session_notif(
    notif: &SessionNotification,
    cx: &agent_client_protocol::ConnectionTo<agent_client_protocol::Client>,
) {
    if !is_crate_compatible(notif) {
        // Extension variant — the crate doesn't know about it.  This is
        // expected and not an error; log at debug level only.
        debug!("Skipping crate-incompatible session update (extension variant)");
        return;
    }
    match local_notif_to_crate(notif) {
        Ok(crate_notif) => {
            if let Err(e) = cx.send_notification(crate_notif) {
                warn!("cx.send_notification failed: {e}");
            }
        }
        Err(e) => warn!("local_notif_to_crate failed (notification dropped): {e}"),
    }
}

// ---------------------------------------------------------------------------
// 111.3 handler: session/prompt — runs in cx.spawn() for non-blocking I/O.
// ---------------------------------------------------------------------------

async fn handle_session_prompt_v2(
    req: agent_client_protocol::schema::v1::PromptRequest,
    responder: agent_client_protocol::Responder<agent_client_protocol::schema::v1::PromptResponse>,
    cx: agent_client_protocol::ConnectionTo<agent_client_protocol::Client>,
    agent: Arc<GrokAcpAgent>,
) -> std::result::Result<(), agent_client_protocol::Error> {
    // Convert the crate's PromptRequest → our local type via JSON serde.
    let local_req: PromptRequest = serde_json::to_value(&req)
        .and_then(|v| serde_json::from_value(v))
        .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()))?;

    let session_id = SessionId::new(local_req.session_id.0.clone());

    // ── Commands catch-up ─────────────────────────────────────────────────────
    // Re-send available_commands_update on every prompt.  This guarantees the
    // Zed slash-command picker is populated even when the session/new or
    // session/load send failed (race condition / transient write error).
    // The notification is tiny (~1 KB of JSON) so the overhead is negligible.
    send_available_commands_update_cx(&cx, &session_id.0)
        .unwrap_or_else(|e| warn!("commands catch-up on prompt failed: {e}"));

    // Extract text and trust any resource URIs
    let mut message_text = String::new();
    for block in local_req.prompt {
        match block {
            ContentBlock::Text(t) => message_text.push_str(&t.text),
            ContentBlock::ResourceLink(link) => {
                trust_workspace_from_uri(&link.uri, &agent);
                message_text.push_str(&format!("\n[Resource: {} ({})]", link.name, link.uri));
            }
            ContentBlock::Resource(res) => {
                let crate::acp::protocol::EmbeddedResourceResource::TextResourceContents(text_res) =
                    res.resource;
                trust_workspace_from_uri(&text_res.uri, &agent);
                message_text.push_str(&format!(
                    "\n[Context: {}]\n{}\n",
                    text_res.uri, text_res.text
                ));
            }
        }
    }

    if message_text.is_empty() {
        responder
            .respond(agent_client_protocol::schema::v1::PromptResponse::new(
                agent_client_protocol::schema::v1::StopReason::EndTurn,
            ))
            .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()))?;
        return Ok(());
    }

    // ── Slash-command dispatch ────────────────────────────────────────────────
    if let Some(cmd) = parse_slash_command(&message_text) {
        info!(
            "Slash command detected (v2): {:?}  (raw: {:?})",
            cmd, message_text
        );

        if let Some(builtin) = handle_builtin(&cmd) {
            info!("Handling built-in slash command: {:?}", cmd);
            // Re-use the same dispatch logic as the old handler but without a
            // raw writer — we call the helper that produces the response text.
            let text = handle_builtin_result(builtin, &agent, &session_id).await;
            // Send text as a session/update AgentMessageChunk notification
            let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(&text),
            )));
            let notif = SessionNotification::new(session_id.clone(), update);
            send_session_notif(&notif, &cx);

            // Refresh the status bar immediately after config-changing slash
            // commands so context budget / model labels update without waiting
            // for the next generation turn.
            if let Ok(state) = agent.get_status_bar_state(&session_id).await {
                let status_update = GrokAcpAgent::status_bar_message_update(&state);
                let status_notif = SessionNotification::new(session_id.clone(), status_update);
                send_session_notif(&status_notif, &cx);
            }
            // Respond first so Zed closes the turn immediately — save_session_to_disk
            // can block on a read-lock if another request holds the write lock, and we
            // must not delay the PromptResponse while that resolves.
            let r = responder
                .respond(agent_client_protocol::schema::v1::PromptResponse::new(
                    agent_client_protocol::schema::v1::StopReason::EndTurn,
                ))
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
            agent.save_session_to_disk(&session_id).await.ok();
            return r;
        }

        // AI-assisted slash command
        if let Some(ai_prompt) = slash_commands::command_to_prompt(&cmd) {
            info!(
                "Routing slash command to AI: {:?} → prompt len={}",
                cmd,
                ai_prompt.len()
            );
            let _ = chat_logger::log_user(&message_text);
            let text = match run_ai_and_collect(&agent, &session_id, &ai_prompt, &cx).await {
                Ok(t) => t,
                Err(e) => {
                    // Deliver the error as a visible message so Zed stays
                    // connected instead of receiving a hard protocol error.
                    let err_text = format!("\u{274c} {}", e);
                    let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(
                        ContentBlock::Text(TextContent::new(&err_text)),
                    ));
                    send_session_notif(&SessionNotification::new(session_id.clone(), update), &cx);
                    let r = responder
                        .respond(agent_client_protocol::schema::v1::PromptResponse::new(
                            agent_client_protocol::schema::v1::StopReason::EndTurn,
                        ))
                        .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
                    return r;
                }
            };
            let _ = chat_logger::log_assistant(&text);
            let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(&text),
            )));
            let notif = SessionNotification::new(session_id.clone(), update);
            send_session_notif(&notif, &cx);
            // Same rationale: send PromptResponse before the potentially-blocked disk save.
            let r = responder
                .respond(agent_client_protocol::schema::v1::PromptResponse::new(
                    agent_client_protocol::schema::v1::StopReason::EndTurn,
                ))
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
            agent.save_session_to_disk(&session_id).await.ok();
            return r;
        }
        warn!(
            "Slash command {:?} was parsed but had no handler (falling through to normal chat)",
            cmd
        );
    }

    // ── Action bar click handling (Task 164.7) ────────────────────────────────
    // When the user clicks on the dynamic status bar actions (think_high, think_low, etc.)
    // Zed sends them as a prompt. We intercept them here before normal chat processing.
    let action = message_text.trim();
    if action == "think_high" || action == "think_low" || action == "think_off" {
        let mode = match action {
            "think_high" => crate::config::ThinkingMode::High,
            "think_low" => crate::config::ThinkingMode::Low,
            "think_off" => crate::config::ThinkingMode::Off,
            _ => unreachable!(),
        };

        match agent.set_thinking_mode(&session_id, mode.clone()).await {
            Ok(()) => {
                let label = mode
                    .as_api_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "off".to_string());
                let text = format!("🧠 Thinking mode updated to **{}**", label);
                let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(
                    ContentBlock::Text(TextContent::new(&text)),
                ));
                let notif = SessionNotification::new(session_id.clone(), update);
                send_session_notif(&notif, &cx);

                let r = responder
                    .respond(agent_client_protocol::schema::v1::PromptResponse::new(
                        agent_client_protocol::schema::v1::StopReason::EndTurn,
                    ))
                    .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
                return r;
            }
            Err(e) => {
                let text = format!("❌ Failed to change thinking mode: {}", e);
                let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(
                    ContentBlock::Text(TextContent::new(&text)),
                ));
                let notif = SessionNotification::new(session_id.clone(), update);
                send_session_notif(&notif, &cx);

                let r = responder
                    .respond(agent_client_protocol::schema::v1::PromptResponse::new(
                        agent_client_protocol::schema::v1::StopReason::EndTurn,
                    ))
                    .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
                return r;
            }
        }
    }

    // ── Normal AI chat ──────────────────────────────────────────────────────────
    let _ = chat_logger::log_user(&message_text);
    let text = match run_ai_and_collect(&agent, &session_id, &message_text, &cx).await {
        Ok(t) => t,
        Err(e) => {
            // Return the error as a visible message rather than a hard
            // JSON-RPC error (-32603).  A protocol error causes Zed to close
            // the ACP connection (server exits), while an error message chunk
            // keeps the session alive so the user can retry.
            let err_text = format!("\u{274c} {}", e);
            warn!("AI chat error (returning as message): {}", e);
            let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(&err_text),
            )));
            send_session_notif(&SessionNotification::new(session_id.clone(), update), &cx);
            let r = responder
                .respond(agent_client_protocol::schema::v1::PromptResponse::new(
                    agent_client_protocol::schema::v1::StopReason::EndTurn,
                ))
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
            agent.save_session_to_disk(&session_id).await.ok();
            return r;
        }
    };
    let _ = chat_logger::log_assistant(&text);

    // Send final text chunk
    let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
        TextContent::new(&text),
    )));
    let notif = SessionNotification::new(session_id.clone(), update);
    send_session_notif(&notif, &cx);

    // Respond before saving to disk: the disk save may briefly contend on the
    // sessions read-lock (if another request holds the write lock), and we must
    // not delay the PromptResponse while that resolves.
    let r = responder
        .respond(agent_client_protocol::schema::v1::PromptResponse::new(
            agent_client_protocol::schema::v1::StopReason::EndTurn,
        ))
        .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()));
    agent.save_session_to_disk(&session_id).await.ok();
    r
}

/// Run an AI call, stream chunk/tool notifications via cx, and return the
/// complete response text.  Permission requests auto-approve for now
/// (full `cx.send_request` integration is tracked in task 111.6).
async fn run_ai_and_collect(
    agent: &Arc<GrokAcpAgent>,
    session_id: &SessionId,
    message: &str,
    cx: &agent_client_protocol::ConnectionTo<agent_client_protocol::Client>,
) -> Result<String> {
    let (perm_bridge, mut perm_rx) = PermissionBridge::new();
    let perm_bridge_arc = Arc::new(perm_bridge);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let chat_fut =
        agent.handle_chat_completion(session_id, message, None, Some(tx), Some(perm_bridge_arc));
    tokio::pin!(chat_fut);

    let response_text;
    loop {
        tokio::select! {
            // Forward streaming tool-call / chunk updates to the client
            update = rx.recv() => {
                if let Some(update) = update {
                    let notif = SessionNotification::new(session_id.clone(), update);
                    send_session_notif(&notif, cx);
                }
            }
            // Auto-approve permission requests (full elicitation in 111.6)
            perm_req = perm_rx.recv() => {
                if let Some((_req_id, _params, outcome_tx)) = perm_req {
                    info!("Auto-approving tool permission (111.6 will use cx.send_request)");
                    let _ = outcome_tx.send(PermissionOutcome::proceed_once());
                }
            }
            // AI call completed
            result = &mut chat_fut => {
                response_text = result?;
                break;
            }
        }
    }

    Ok(response_text)
}

/// Dispatch the big result enum from handle_builtin to a response string
/// without needing a raw writer.  Mirrors the logic in the old handler.
async fn handle_builtin_result(
    builtin: BuiltinResult,
    agent: &GrokAcpAgent,
    session_id: &SessionId,
) -> String {
    match builtin {
        BuiltinResult::Text(text) => text,
        BuiltinResult::ClearHistory => {
            let _ = agent.clear_session_history(session_id).await;
            let _ = chat_logger::log_system("Conversation history cleared");
            "✅ Conversation history cleared. Starting fresh!".to_string()
        }
        BuiltinResult::SwitchModel(model_name) => {
            // Capture previous config for a richer confirmation message.
            let old_cfg = agent.get_session_config(session_id).await.ok();
            let old_model = old_cfg.as_ref().map(|c| c.model.clone());
            let old_max = old_cfg.as_ref().map(|c| c.max_tokens);

            match agent
                .set_session_model(session_id, model_name.clone())
                .await
            {
                Ok(()) => {
                    let new_cfg = agent.get_session_config(session_id).await.ok();
                    let new_max = new_cfg.as_ref().map(|c| c.max_tokens).unwrap_or(0);
                    let budget_note = match (old_max, old_model.as_deref()) {
                        (Some(old), Some(om)) if om != model_name.as_str() => {
                            format!(
                                "\n\nOutput budget: **{old}** -> **{new_max}** tokens\n\
                                 Context window is selected automatically from the model family \
                                 (1M for grok-4.x, legacy budget for others)."
                            )
                        }
                        _ => format!("\n\nOutput budget: **{new_max}** tokens"),
                    };
                    format!("Switched to model **`{model_name}`**.{budget_note}")
                }
                Err(e) => format!("Could not switch model: {e}"),
            }
        }
        BuiltinResult::ShowCurrentModel => match agent.get_session_config(session_id).await {
            Ok(cfg) => format!("🧠 Current model: **`{}`**", cfg.model),
            Err(e) => format!("❌ Could not retrieve current model: {e}"),
        },
        BuiltinResult::ShowContext => match agent.get_session_config(session_id).await {
            Ok(cfg) => {
                let msg_count = agent
                    .get_session_message_count(session_id)
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
        BuiltinResult::ShowBayes => match agent.get_bayes_visualize(session_id).await {
            Ok(t) => t,
            Err(e) => format!("❌ Bayesian state unavailable: {e}"),
        },
        BuiltinResult::ResetBayes => match agent.reset_bayes(session_id).await {
            Ok(t) => t,
            Err(e) => format!("❌ Could not reset Bayesian priors: {e}"),
        },
        BuiltinResult::ExplainBayes => match agent.get_bayes_explain(session_id).await {
            Ok(t) => t,
            Err(e) => format!("Bayesian explanation unavailable: {e}"),
        },
        BuiltinResult::SetGoal(goal) => match agent.set_session_goal(session_id, goal).await {
            Ok(t) => t,
            Err(e) => format!("Could not set goal: {e}"),
        },
        BuiltinResult::ClearGoal => match agent.clear_session_goal(session_id).await {
            Ok(t) => t,
            Err(e) => format!("Could not clear goal: {e}"),
        },
        BuiltinResult::ShowGoal => match agent.get_session_goal(session_id).await {
            Ok(t) => t,
            Err(e) => format!("Could not retrieve goal: {e}"),
        },
        BuiltinResult::ShowVisualizer => crate::visualizer::generate_pipeline_markdown(None),
        BuiltinResult::SetThinkingMode(opt_mode) => match opt_mode {
            Some(mode) => {
                let is_off = matches!(mode, crate::config::ThinkingMode::Off);
                let label = mode
                    .as_api_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "off".to_string());
                match agent.set_thinking_mode(session_id, mode).await {
                    Ok(()) => {
                        if is_off {
                            "🔇 Thinking mode **disabled**. Use `/think low` or `/think high` to enable."
                                    .to_string()
                        } else {
                            format!(
                                "🧠 Thinking mode set to **{label}**. \
                                     Use `/think off` to disable."
                            )
                        }
                    }
                    Err(e) => format!("❌ Could not set thinking mode: {e}"),
                }
            }
            None => match agent.get_thinking_mode(session_id).await {
                Some(mode) => {
                    let label = mode
                        .as_api_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "off".to_string());
                    format!(
                        "🧠 Current thinking mode: **{label}**\n\n\
                                 - `off` — standard, no reasoning trace\n\
                                 - `low` — light reasoning\n\
                                 - `high` — deep reasoning"
                    )
                }
                None => "Session not found.".to_string(),
            },
        },
        BuiltinResult::RecallArchive(chunk_id) => {
            let base = slash_commands::format_archives_text(Some(&session_id.0));
            match chunk_id {
                Some(id) => format!(
                    "{base}\n\n_Recall of chunk {id} will be fully implemented in a follow-up._"
                ),
                None => base,
            }
        }
        BuiltinResult::ShowDiagnostics => slash_commands::format_diagnostics_text(),
        BuiltinResult::AddRule(text) => match agent.add_session_rule(session_id, text).await {
            Ok(t) => t,
            Err(e) => format!("❌ Could not add rule: {e}"),
        },
        BuiltinResult::RemoveRule(id) => match agent.remove_session_rule(session_id, id).await {
            Ok(t) => t,
            Err(e) => format!("❌ Could not remove rule: {e}"),
        },
        BuiltinResult::ListRules => match agent.list_session_rules(session_id).await {
            Ok(t) => t,
            Err(e) => format!("❌ Could not list rules: {e}"),
        },
        BuiltinResult::ClearRules => match agent.clear_session_rules(session_id).await {
            Ok(t) => t,
            Err(e) => format!("❌ Could not clear rules: {e}"),
        },
    }
}

// ---------------------------------------------------------------------------
// 111.3 handler: client notifications (Gemini's available_commands_update, etc.)
// ---------------------------------------------------------------------------

async fn handle_client_notification_v2(
    notif: agent_client_protocol::schema::v1::ClientNotification,
    agent: &GrokAcpAgent,
) {
    // Serialize to Value so we can inspect method-agnostic fields
    match serde_json::to_value(&notif) {
        Ok(json) => {
            // Extract sessionId and sessionUpdate kind for logging
            let sid = json["sessionId"].as_str().unwrap_or("").to_string();
            let kind = json["update"]["sessionUpdate"]
                .as_str()
                .unwrap_or("unknown");
            info!("Client notification '{}' for session '{}'", kind, sid);

            if kind == "available_commands_update" {
                let commands: Vec<String> = json["update"]["availableCommands"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|c| c["name"].as_str().map(str::to_string))
                    .collect();
                info!(
                    "Received {} client command(s) for session '{}'",
                    commands.len(),
                    sid
                );
                if !sid.is_empty() {
                    let sess_id = SessionId::new(&sid);
                    if let Err(e) = agent.set_client_commands(&sess_id, commands).await {
                        warn!("store client commands: {e}");
                    }
                }
            }
        }
        Err(e) => warn!("client notification serialize: {e}"),
    }
}

// ---------------------------------------------------------------------------
// 111.3 catch-all: session/load, session/fork, session/set_model, unknown
// ---------------------------------------------------------------------------

/// Extension / fallthrough dispatch: receives any `ClientRequest` variant that
/// was not matched by the typed `on_receive_request` handlers above.
/// All standard methods (initialize, session/new, session/prompt, session/list,
/// session/load) now have dedicated typed handlers, so reaching here means an
/// unrecognised standard-looking method was sent — return method-not-found.
async fn handle_extension_dispatch(
    msg: agent_client_protocol::Dispatch<
        agent_client_protocol::schema::v1::ClientRequest,
        agent_client_protocol::schema::v1::ClientNotification,
    >,
    _cx: agent_client_protocol::ConnectionTo<agent_client_protocol::Client>,
    _agent: Arc<GrokAcpAgent>,
) -> std::result::Result<(), agent_client_protocol::Error> {
    use agent_client_protocol::Dispatch;
    match msg {
        Dispatch::Request(_req, _responder) => {
            // Reached for unhandled ClientRequest variants.
            Err(agent_client_protocol::Error::method_not_found())
        }
        Dispatch::Notification(_) => Ok(()),
        Dispatch::Response(result, router) => {
            router
                .respond_with_result(result)
                .map_err(|e| agent_client_protocol::Error::new(-32603, e.to_string()))?;
            Ok(())
        }
    }
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
    // ── Step 0: strip URI fragment (#L1:854, #anchor, etc.) ─────────────────
    // Zed appends line-number anchors to @-mentioned file URIs (e.g.
    // "file:///H:/GitHub/bot/mod.rs#L1:854").  We must remove the fragment
    // before treating the remainder as a file-system path, otherwise the
    // file's name would be something like "mod.rs#L1:854" which never
    // canonicalises and ends up registered as a bogus trusted root.
    let raw = if let Some(pos) = raw.find('#') {
        &raw[..pos]
    } else {
        raw
    };

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

    // Strip any URI fragment (e.g. "#L864:865" appended by Zed when the user
    // ctrl-clicks a file reference).  The fragment is never part of the file
    // system path and causes canonicalize() to fail with os error 2/3, which
    // in turn registers a garbage string as the trusted workspace root and
    // silently blocks all file-tool access for that session.
    let stripped = match stripped.find('#') {
        Some(idx) => {
            let without_fragment = stripped[..idx].to_string();
            debug!(
                "Stripped URI fragment '{}' from workspace path",
                &stripped[idx..]
            );
            without_fragment
        }
        None => stripped,
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
                let drive = s
                    .chars()
                    .nth(1)
                    .unwrap_or('C')
                    .to_uppercase()
                    .next()
                    .unwrap_or('C');
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
    // If the resolved path points to a file rather than a directory (common
    // when Zed sends an @-mentioned file URI as the workspace hint), walk up
    // the directory tree to find the real project root by looking for markers
    // such as .git, Cargo.toml, package.json, etc.
    let workspace_root = find_workspace_root_from_path(&resolved);
    info!(
        "Registering workspace root as trusted: {:?} (resolved from {:?})",
        workspace_root, resolved
    );
    agent.add_trusted_directory(workspace_root);
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

/// Resolve the directory where chat session logs should be written.
///
/// Priority (as per project policy):
/// 1. Project-local `.grok/logs/chat_sessions/` (walk up from CWD)
/// 2. System/global `~/.grok-cli/logs/chat_sessions/`
///
/// Chat logs are project-scoped. Error logs and cross-project memory use the
/// system `~/.grok-cli/` location.
fn resolve_chat_log_dir() -> PathBuf {
    // Try to find a project-local .grok directory
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            let candidate = dir.join(".grok").join("logs").join("chat_sessions");
            if candidate.exists() || dir.join(".grok").exists() {
                return candidate;
            }
            if let Some(parent) = dir.parent() {
                dir = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    // Fall back to system location (~/.grok-cli)
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".grok-cli")
        .join("logs")
        .join("chat_sessions")
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
    agent.add_trusted_directory(workspace_root);
}

/// Build the list of authentication methods that grok-cli supports.
///
/// The ACP registry verify script reads `authMethods` from the `initialize`
/// response and reports them as `{id}({type})`.  Declaring the env-var auth
/// method here lets the registry mark the agent as "Auth OK: xai-api-key(env_var)"
/// even when no key is present in the sandbox environment.
fn build_auth_methods() -> Vec<AuthMethod> {
    vec![
        // Terminal Auth — satisfies the ACP Registry requirement.
        // The client launches `grok setup` which runs the interactive TUI
        // API-key wizard.  Once the key is saved, the agent is ready for
        // normal ACP communication.
        AuthMethod::terminal("grok-setup", "Run in terminal", vec!["setup"]).with_description(
            "Interactive terminal setup wizard — enter your xAI API key \
             and grok-cli will save it automatically.",
        ),
        // env_var method kept for clients that prefer to inject the key
        // directly (e.g. CI environments, Gemini CLI).
        AuthMethod::env_var(
            "xai-api-key",
            "xAI API Key",
            vec![AuthEnvVar::new("GROK_API_KEY").with_label("xAI / Grok API Key")],
        )
        .with_description(
            "API key from the xAI developer console. \
             Set the GROK_API_KEY environment variable or run 'grok setup'.",
        )
        .with_link("https://console.x.ai/"),
    ]
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

    // Echo back the client's protocol version and declare auth requirements so
    // ACP clients (and the acp_registry verifier) know what credentials to
    // request from the user before the first session/prompt.
    let response = InitializeResponse::new(&req.protocol_version)
        .agent_capabilities(caps)
        .agent_info(Implementation::new("grok-cli", env!("CARGO_PKG_VERSION")))
        .auth_methods(build_auth_methods());

    info!(
        "Sending initialize response: protocol_version={}",
        req.protocol_version
    );
    Ok(serde_json::to_value(response)?)
}

async fn handle_session_new(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {
    let req: NewSessionRequest = serde_json::from_value(params.clone())
        .map_err(|e| anyhow!("Invalid session/new parameters: {}", e))?;

    // ── MCP server connection (Phase 1 of MCP bridge) ────────────────────────
    if !req.mcp_servers.is_empty() {
        info!(
            "session/new: client forwarded {} MCP server(s) — attempting connection",
            req.mcp_servers.len()
        );

        for server_val in &req.mcp_servers {
            // Expect shape: { "name": "...", "command": "...", "args": [...], "env": {...} }
            let name = server_val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");

            let command = match server_val.get("command").and_then(|v| v.as_str()) {
                Some(c) => c.to_string(),
                None => {
                    warn!("MCP server '{}' missing 'command' field — skipping", name);
                    continue;
                }
            };

            let args: Vec<String> = server_val
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let env: std::collections::HashMap<String, String> = server_val
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            let cfg = crate::mcp::config::McpServerConfig::Stdio { command, args, env };

            // Acquire the lock for this server only
            let mcp_client = agent.get_mcp_client();
            let mut client_guard = mcp_client.write().await;

            match client_guard.connect(name, &cfg).await {
                Ok(()) => {
                    info!("Successfully connected to MCP server '{}'", name);

                    // Discover tools from the newly connected server
                    match client_guard.list_tools(name).await {
                        Ok(tools) => {
                            info!(
                                "Discovered {} tool(s) from MCP server '{}'",
                                tools.len(),
                                name
                            );
                            // Release client lock before acquiring discovered_tools lock
                            drop(client_guard);
                            let discovered = agent.get_discovered_mcp_tools();
                            let mut map = discovered.write().await;
                            map.insert(name.to_string(), tools.clone());

                            // Also update the global registry so mcp_list tool can see it
                            let mut global_map = crate::tools::registry::get_discovered_mcp_tools();
                            global_map.insert(name.to_string(), tools);
                            crate::tools::registry::set_discovered_mcp_tools(global_map);
                        }
                        Err(e) => {
                            warn!("Failed to list tools from MCP server '{}': {}", name, e);
                        }
                    }
                }
                Err(e) => warn!("Failed to connect to MCP server '{}': {}", name, e),
            }
        }
    }

    // Extract workspace context from request or environment.
    // Handles: workspaceRoot (Zed), workingDirectory (Zed), cwd (Gemini CLI),
    // and environment variable fallbacks.
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
                agent.add_trusted_directory(canonical_cwd);
            }
            Err(e) => {
                warn!("session/new: could not determine CWD: {}", e);
            }
        }
    }

    // Generate a session ID
    let session_id_str = uuid::Uuid::new_v4().to_string();
    let session_id = SessionId::new(session_id_str.clone());

    let fallback_cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let initial_cwd = params
        .get("cwd")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .unwrap_or(fallback_cwd);

    // Initialize session in GrokAcpAgent.
    // Pass None so initialize_session pulls thinking_mode (and future
    // ACP defaults) from the hierarchically-loaded agent config
    // (project .grok/config.toml → ~/.grok-cli/config.toml → built-in).
    agent
        .initialize_session(session_id, initial_cwd, None, None)
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

    // Task 27: build modes and models blocks required by Gemini CLI.
    // currentModeId reflects whether permission prompts are on or off.
    let current_mode_id = if agent
        .get_capabilities()
        .features
        .contains(&"function_calling".to_string())
    {
        "autoEdit"
    } else {
        "default"
    };

    let modes = AcpModesInfo::new(
        vec![
            AcpModeInfo::new("default", "Default")
                .with_description("Prompts for approval before each tool call"),
            AcpModeInfo::new("autoEdit", "Auto Edit")
                .with_description("Auto-approves file edit tools"),
            AcpModeInfo::new("yolo", "YOLO")
                .with_description("Auto-approves all tools without prompting"),
            AcpModeInfo::new("plan", "Plan")
                .with_description("Read-only planning mode — no file writes"),
        ],
        current_mode_id,
    );

    let caps = agent.get_capabilities();
    let default_model = SessionConfig::default().model;
    let current_model_id = if caps.models.contains(&default_model) {
        default_model.clone()
    } else {
        caps.models
            .first()
            .cloned()
            .unwrap_or_else(|| default_model.clone())
    };
    let available_models: Vec<AcpModelInfo> = caps
        .models
        .iter()
        .map(|m| AcpModelInfo::new(m.clone(), m.clone()))
        .collect();
    let models = AcpModelsInfo::new(available_models, current_model_id);

    let response = NewSessionResponse::new(SessionId::new(session_id_str))
        .with_modes(modes)
        .with_models(models);
    Ok(serde_json::to_value(response)?)
}

/// Handle `session/load` — resume (or recreate) a session that the client
/// already holds a session ID for.
///
/// Zed sends `session/load` instead of `session/new` when it has a persisted
/// session ID from a previous run.  The old stub returned an empty response
/// without ever calling `initialize_session`, which left the sessions map
/// empty and the chat-logger without an active session.  Any subsequent
/// `session/prompt` would then fire "Attempted to log message without an
/// active session" and fail to find the session in the agent's map.
///
/// This handler now:
/// 1. Connects any MCP servers forwarded by the client.
/// 2. Registers the workspace root so file-system tools are trusted.
/// 3. Restores the session from disk if a snapshot exists; otherwise
///    creates a fresh session under the supplied session ID.
/// 4. Starts the chat logger for the session.
async fn handle_session_load(params: &Value, agent: &GrokAcpAgent) -> Result<()> {
    let req: SessionLoadRequest = serde_json::from_value(params.clone())
        .map_err(|e| anyhow!("Invalid session/load parameters: {}", e))?;

    let sid = req.session_id.0.clone();

    // ── 1. MCP server connection ────────────────────────────────────────────
    if !req.mcp_servers.is_empty() {
        info!(
            "session/load: client forwarded {} MCP server(s) — attempting connection",
            req.mcp_servers.len()
        );
        for server_val in &req.mcp_servers {
            let name = server_val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");

            let command = match server_val.get("command").and_then(|v| v.as_str()) {
                Some(c) => c.to_string(),
                None => {
                    warn!(
                        "session/load: MCP server '{}' missing 'command' — skipping",
                        name
                    );
                    continue;
                }
            };

            let args: Vec<String> = server_val
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let env: std::collections::HashMap<String, String> = server_val
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            let cfg = crate::mcp::config::McpServerConfig::Stdio { command, args, env };
            let mcp_client = agent.get_mcp_client();
            let mut client_guard = mcp_client.write().await;

            match client_guard.connect(name, &cfg).await {
                Ok(()) => {
                    info!("session/load: connected to MCP server '{}'", name);
                    match client_guard.list_tools(name).await {
                        Ok(tools) => {
                            info!(
                                "session/load: discovered {} tool(s) from '{}'",
                                tools.len(),
                                name
                            );
                            drop(client_guard);
                            let discovered = agent.get_discovered_mcp_tools();
                            let mut map = discovered.write().await;
                            map.insert(name.to_string(), tools.clone());
                            let mut global_map = crate::tools::registry::get_discovered_mcp_tools();
                            global_map.insert(name.to_string(), tools);
                            crate::tools::registry::set_discovered_mcp_tools(global_map);
                        }
                        Err(e) => {
                            warn!("session/load: failed to list tools from '{}': {}", name, e)
                        }
                    }
                }
                Err(e) => warn!(
                    "session/load: failed to connect to MCP server '{}': {}",
                    name, e
                ),
            }
        }
    }

    // ── 2. Workspace registration ───────────────────────────────────────────
    let cwd = req.cwd.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });
    register_workspace_root(agent, &cwd);

    // ── 3. Session initialization ───────────────────────────────────────────
    // Prefer restoring a persisted snapshot; fall back to a fresh session.
    let session_id_obj = SessionId::new(sid.clone());
    match agent.load_session_from_disk(&sid).await {
        Some(state) => {
            let msg_count = state.messages.len();
            info!(
                "session/load: restoring '{}' from disk ({} messages)",
                sid, msg_count
            );
            if let Err(e) = agent.restore_session_from_disk(state).await {
                warn!(
                    "session/load: restore from disk failed for '{}': {} — \
                     creating fresh session",
                    sid, e
                );
                agent
                    .initialize_session(session_id_obj, cwd, None, None)
                    .await?;
            }
        }
        None => {
            info!(
                "session/load: no saved state for '{}' — creating fresh session",
                sid
            );
            agent
                .initialize_session(session_id_obj, cwd, None, None)
                .await?;
        }
    }

    // ── 4. Chat logging ─────────────────────────────────────────────────────
    if let Err(e) = chat_logger::start_session(&sid) {
        warn!(
            "session/load: failed to start chat logging for '{}': {}",
            sid, e
        );
    } else {
        let _ = chat_logger::log_system(format!("Session '{}' loaded via session/load", sid));
        info!("Chat logging started for session: {}", sid);
    }

    Ok(())
}

/// Task 29: Apply safe initialization defaults when a client skips the
/// `initialize` handshake and jumps straight to `session/new` (e.g. Gemini CLI).
///
/// This is idempotent — calling it after a real `initialize` is a no-op.
fn ensure_default_initialized(agent: &GrokAcpAgent, initialized: &mut bool) {
    if *initialized {
        return;
    }
    warn!(
        "Client skipped initialize; applying defaults \
         (trusting CWD as workspace root)"
    );
    match std::env::current_dir() {
        Ok(cwd) => {
            let canonical = cwd.canonicalize().unwrap_or(cwd);
            info!("ensure_default_initialized: trusting CWD {:?}", canonical);
            agent.add_trusted_directory(canonical);
        }
        Err(e) => warn!("ensure_default_initialized: could not read CWD: {}", e),
    }
    *initialized = true;
}

/// Handle `session/list` — return the currently active in-memory sessions.
///
/// Per the ACP spec, an empty `sessions` array is a valid response when no
/// sessions are available.  We advertise `sessionCapabilities.list: {}` in
/// the `initialize` response, so Zed will always call this method.
async fn handle_session_list(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {
    let req: SessionListRequest = serde_json::from_value(params.clone()).unwrap_or_default();

    info!("session/list called (cwd filter: {:?})", req.cwd);

    // cwd is now Option<PathBuf> (crate type); convert to &str for filtering.
    let cwd_filter = req.cwd.as_deref().and_then(|p| p.to_str()).unwrap_or("");

    let agent_sids = agent.list_sessions().await;
    let mut sessions = Vec::new();

    for sid in agent_sids {
        let stored_cwd = agent.get_session_cwd(&sid).await.unwrap_or_default();
        if cwd_filter.is_empty() || stored_cwd == cwd_filter {
            sessions.push(SessionInfo::new(sid, stored_cwd));
        }
    }

    info!("session/list returning {} session(s)", sessions.len());
    let response = SessionListResponse::new(sessions);
    Ok(serde_json::to_value(response)?)
}

async fn handle_session_set_model(params: &Value, agent: &GrokAcpAgent) -> Result<Value> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SetModelRequest {
        session_id: String,
        model_id: String,
    }

    let req: SetModelRequest = serde_json::from_value(params.clone())?;
    info!(
        "session/set_model called (session: {}, model: {})",
        req.session_id, req.model_id
    );
    agent.set_model(&req.session_id, &req.model_id).await?;

    Ok(serde_json::Value::Null)
}

async fn test_acp_connection(address: &str, config: &Config) -> Result<()> {
    println!(
        "{}",
        format_info(&format!("Testing ACP connection to {}", address))
    );

    let spinner = create_spinner("Connecting...");

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        test_acp_connection_inner(address, config),
    )
    .await;

    spinner.finish_and_clear();

    match result {
        Ok(Ok(())) => {
            println!("{}", format_success("ACP connection test successful"));
            Ok(())
        }
        Ok(Err(e)) => {
            println!(
                "{}",
                format_error(&format!("ACP connection test failed: {}", e))
            );
            Err(e)
        }
        Err(_) => {
            let err = anyhow!("ACP connection test timed out");
            println!("{}", format_error(&err.to_string()));
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
#[allow(dead_code)]
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

// ---------------------------------------------------------------------------
// Public test entry-point (task 111.3 integration tests)
// ---------------------------------------------------------------------------

/// Thin public wrapper around `run_acp_session` for integration tests in
/// `tests/acp_protocol.rs`.  Allows the test crate to drive a real session
/// over in-memory `tokio::io::duplex` pipes without requiring a network or
/// a real xAI API key (no session/prompt is exercised by the protocol tests).
///
/// Not gated on `#[cfg(test)]` because it must be accessible from the
/// `tests/` directory (a separate crate in Rust's test model).
pub async fn run_acp_session_for_test<R, W>(reader: R, writer: W, agent: GrokAcpAgent) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    run_acp_session(reader, writer, agent).await
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
