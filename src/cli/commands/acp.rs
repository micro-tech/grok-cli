//! ACP (Agent Client Protocol) command handler for Zed integration
//!
//! This module handles all Agent Client Protocol operations, including starting
//! the ACP server for Zed editor integration, testing connections, and managing
//! ACP capabilities.

use anyhow::{Result, anyhow};
use colored::*;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::acp::protocol::{
    AGENT_METHOD_NAMES, AgentCapabilities, ContentBlock, ContentChunk, Implementation,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse, PromptRequest,
    PromptResponse, ProtocolVersion, SessionId, SessionNotification, SessionUpdate, StopReason,
    TextContent,
};
use crate::acp::{GrokAcpAgent, SessionConfig};
use crate::cli::{create_spinner, print_error, print_info, print_success, print_warning};
use crate::config::Config;
use crate::utils::chat_logger;

/// Handle ACP-related commands
pub async fn handle_acp_action(action: crate::AcpAction, config: &Config) -> Result<()> {
    match action {
        crate::AcpAction::Server { port, host } => start_acp_server(port, &host, config).await,
        crate::AcpAction::Stdio { model } => start_acp_stdio(config, model).await,
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

async fn start_acp_stdio(config: &Config, model: Option<String>) -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let agent = GrokAcpAgent::new(config.clone(), model).await?;

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
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                debug!("Received message: {}", trimmed);

                // Attempt to parse as JSON
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(json_msg) => {
                        // Handle JSON-RPC message
                        if let Err(e) = handle_json_rpc(&json_msg, &mut writer, &agent).await {
                            error!("Error handling message: {}", e);
                            // Optionally send error response back to client
                        }
                    }
                    Err(e) => {
                        warn!("Invalid JSON received: {} (Error: {})", trimmed, e);
                    }
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    info!("ACP session completed");

    // End chat logging session
    if let Err(e) = chat_logger::end_session() {
        warn!("Failed to end chat logging session: {}", e);
    }

    Ok(())
}

async fn handle_json_rpc<W>(msg: &Value, writer: &mut W, agent: &GrokAcpAgent) -> Result<()>
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
            handle_session_prompt(&params, agent, writer).await
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
    } else if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
        // Notification
        info!("Received notification: {}", method);
    }

    Ok(())
}

async fn handle_initialize(params: &Value, _agent: &GrokAcpAgent) -> Result<Value> {
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

    // Extract workspace context from request or environment
    let workspace_root = req
        .workspace_root
        .or(req.working_directory)
        .or_else(|| std::env::var("CODER_AGENT_WORKSPACE_PATH").ok())
        .or_else(|| std::env::var("WORKSPACE_ROOT").ok());

    // If workspace root is provided, update trusted directories
    if let Some(workspace_path) = &workspace_root {
        use std::path::PathBuf;
        let path = PathBuf::from(workspace_path);
        if let Ok(canonical_path) = path.canonicalize() {
            info!(
                "Adding workspace root to trusted directories: {:?}",
                canonical_path
            );
            agent.security.add_trusted_directory(canonical_path);
        } else {
            warn!("Failed to canonicalize workspace path: {}", workspace_path);
        }
    } else {
        info!("No workspace root provided, using current directory from agent initialization");
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
) -> Result<Value>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let req: PromptRequest = serde_json::from_value(params.clone())
        .map_err(|e| anyhow!("Invalid session/prompt parameters: {}", e))?;

    let session_id = SessionId::new(req.session_id.0.clone());

    // Extract text from prompt
    let mut message_text = String::new();
    for block in req.prompt {
        match block {
            ContentBlock::Text(text) => message_text.push_str(&text.text),
            ContentBlock::ResourceLink(link) => {
                message_text.push_str(&format!("\n[Resource: {} ({})]", link.name, link.uri));
            }
            ContentBlock::Resource(res) => {
                let crate::acp::protocol::EmbeddedResourceResource::TextResourceContents(text_res) =
                    res.resource;
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

    // Log user prompt
    if let Err(e) = chat_logger::log_user(&message_text) {
        warn!("Failed to log user message: {}", e);
    }

    // Call Grok agent
    // Note: options mapping is simplified here. Real implementation would map model/temp params.
    info!(
        "Calling Grok API for session {} with message: {}",
        session_id.0, message_text
    );
    let response_text = agent
        .handle_chat_completion(&session_id, &message_text, None)
        .await?;

    info!("Received response from Grok: {} chars", response_text.len());
    debug!("Response text: {}", response_text);

    let final_text = if response_text.is_empty() {
        warn!("Grok returned empty response text!");
        "[No response content received from model]".to_string()
    } else {
        response_text
    };

    // Log assistant response
    if let Err(e) = chat_logger::log_assistant(&final_text) {
        warn!("Failed to log assistant response: {}", e);
    }

    // Send update notification with response text
    info!("About to send text update notification...");
    send_text_update(writer, &session_id.0, &final_text).await?;
    info!("Text update notification sent");

    // Return response indicating turn end
    info!("Returning final response with stopReason: EndTurn");
    let response = PromptResponse::new(StopReason::EndTurn);
    Ok(serde_json::to_value(response)?)
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
