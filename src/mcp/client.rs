use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::mcp::config::McpServerConfig;
use crate::mcp::protocol::Tool;
use crate::mcp::{MCP_CLIENT_NAME, MCP_PROTOCOL_VERSION};

pub struct McpClient {
    servers: HashMap<String, ServerConnection>,
}

struct ServerConnection {
    #[expect(
        dead_code,
        reason = "must be kept alive — dropping it would kill the child process"
    )]
    process: Child,
    stdin: Mutex<ChildStdin>,
    reader: Mutex<BufReader<ChildStdout>>,
    /// If true, we performed the old `initialize` handshake (for very old servers).
    /// If false (the default), we are in stateless mode: client info goes in `_meta` on every request.
    use_legacy_handshake: bool,
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    pub async fn connect(&mut self, name: &str, config: &McpServerConfig) -> Result<()> {
        match config {
            McpServerConfig::Stdio {
                command,
                args,
                env,
                use_legacy_handshake,
            } => {
                info!(
                    "Connecting to MCP server '{}' via stdio: {} {:?} (legacy_handshake={})",
                    name, command, args, use_legacy_handshake
                );

                let mut cmd = Command::new(command);
                cmd.args(args);
                cmd.envs(env);
                cmd.stdin(Stdio::piped());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::inherit()); // Log stderr to parent's stderr

                let mut child = cmd
                    .spawn()
                    .map_err(|e| anyhow!("Failed to spawn MCP server: {}", e))?;

                let stdin = child
                    .stdin
                    .take()
                    .ok_or_else(|| anyhow!("Failed to open stdin"))?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow!("Failed to open stdout"))?;

                let connection = ServerConnection {
                    process: child,
                    stdin: Mutex::new(stdin),
                    reader: Mutex::new(BufReader::new(stdout)),
                    use_legacy_handshake: *use_legacy_handshake,
                };

                if *use_legacy_handshake {
                    // Legacy 2024/2025 behavior
                    self.initialize_handshake(&connection).await?;
                } else {
                    // 2026+ stateless mode: no handshake.
                    // Client info will be sent via _meta on every subsequent request.
                    debug!("Stateless 2026-style connection for '{}'", name);
                }

                self.servers.insert(name.to_string(), connection);
                info!("Connected to MCP server '{}'", name);
                Ok(())
            }
            McpServerConfig::Sse { .. } => Err(anyhow!("SSE transport not yet implemented")),
        }
    }

    async fn initialize_handshake(&self, connection: &ServerConnection) -> Result<()> {
        // Legacy handshake path — use the old protocol version for compatibility
        let init_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": crate::mcp::MCP_PROTOCOL_VERSION_LEGACY,
                "capabilities": {},
                "clientInfo": {
                    "name": "grok-cli",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        });

        self.send_message(connection, &init_msg).await?;
        let response = self.read_response(connection).await?;

        // ── Proper MCP response validation (Task 146) ─────────────────────
        self.validate_initialize_response(&response)?;

        debug!("Initialize response validated successfully: {:?}", response);

        // Send initialized notification
        let initialized_msg = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        self.send_message(connection, &initialized_msg).await?;

        Ok(())
    }

    /// Validate an MCP `initialize` response.
    fn validate_initialize_response(&self, response: &Value) -> Result<()> {
        // 1. Must be a valid JSON-RPC response
        if response.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            return Err(anyhow!("Invalid JSON-RPC version in initialize response"));
        }

        // 2. Check for JSON-RPC level error
        if let Some(err) = response.get("error") {
            return Err(anyhow!(
                "MCP server returned error during initialize: {}",
                err
            ));
        }

        // 3. Must contain a "result" object
        let result = response
            .get("result")
            .ok_or_else(|| anyhow!("Initialize response missing 'result' field"))?;

        // 4. Result must contain protocolVersion
        let server_version = result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Initialize result missing 'protocolVersion'"))?;

        // 5. Basic protocol version compatibility check
        // Accept legacy "0.1.0", 2024/2025, and the new 2026-07-28 stateless model.
        if server_version != "0.1.0"
            && !server_version.starts_with("2024-")
            && !server_version.starts_with("2025-")
            && !server_version.starts_with("2026-")
        {
            tracing::warn!(
                "MCP server protocol version {} may not be fully compatible with client {}",
                server_version,
                MCP_PROTOCOL_VERSION
            );
        }

        // 6. Optional but recommended: check serverInfo
        if let Some(server_info) = result.get("serverInfo")
            && let Some(name) = server_info.get("name").and_then(|v| v.as_str()) {
                debug!("Connected to MCP server: {}", name);
            }

        Ok(())
    }

    async fn send_message(&self, connection: &ServerConnection, message: &Value) -> Result<()> {
        let mut stdin = connection.stdin.lock().await;
        let json_str = serde_json::to_string(message)?;
        stdin.write_all(json_str.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn read_response(&self, connection: &ServerConnection) -> Result<Value> {
        let mut reader = connection.reader.lock().await;
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        if line.is_empty() {
            return Err(anyhow!("MCP server closed connection"));
        }

        let value: Value = serde_json::from_str(&line)?;
        Ok(value)
    }

    pub async fn list_tools(&self, server_name: &str) -> Result<Vec<Tool>> {
        let (tools, _meta) = self.list_tools_with_meta(server_name).await?;
        Ok(tools)
    }

    /// List tools and return both the tools and any 2026+ metadata (ttlMs, cacheScope, etc.).
    ///
    /// This is the preferred method for 2026 stateless clients that want to respect
    /// server-provided caching hints.
    pub async fn list_tools_with_meta(&self, server_name: &str) -> Result<(Vec<Tool>, Option<crate::mcp::protocol::ToolListMeta>)> {
        let connection = self
            .servers
            .get(server_name)
            .ok_or_else(|| anyhow!("Server not connected: {}", server_name))?;

        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        // 2026+ stateless mode: always put client identity in _meta (no handshake needed)
        if !connection.use_legacy_handshake
            && let Some(obj) = msg.as_object_mut() {
                obj.insert("_meta".to_string(), self.client_meta());
            }

        self.send_message(connection, &msg).await?;
        let response = self.read_response(connection).await?;

        if let Some(result) = response.get("result") {
            let tools: Vec<Tool> = result
                .get("tools")
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()?
                .unwrap_or_default();

            // 2026+ support: extract _meta for ttlMs / cacheScope
            let meta = result
                .get("_meta")
                .and_then(|m| serde_json::from_value::<crate::mcp::protocol::ToolListMeta>(m.clone()).ok());

            if let Some(ref m) = meta {
                if let Some(ttl) = m.ttl_ms {
                    debug!("MCP tools/list for '{}' has ttlMs={}", server_name, ttl);
                }
            }

            return Ok((tools, meta));
        }

        Ok((Vec::new(), None))
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        args: Value,
    ) -> Result<Value> {
        let connection = self
            .servers
            .get(server_name)
            .ok_or_else(|| anyhow!("Server not connected: {}", server_name))?;

        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": args
            }
        });

        // Phase 0 (2026 readiness): inject client info via _meta when using stateless mode
        if !connection.use_legacy_handshake
            && let Some(obj) = msg.as_object_mut() {
                obj.insert("_meta".to_string(), self.client_meta());
            }

        self.send_message(connection, &msg).await?;
        let response = self.read_response(connection).await?;

        // Check for error
        if let Some(error) = response.get("error") {
            return Err(anyhow!("Tool call failed: {:?}", error));
        }

        if let Some(result) = response.get("result") {
            return Ok(result.clone());
        }

        Err(anyhow!("Invalid response from tool call"))
    }

    /// Build the `_meta` object containing client info for stateless 2026+ mode.
    ///
    /// Per the 2026-07-28 spec (SEP-2575), client identity and protocol version
    /// travel on **every** request inside `_meta` instead of a one-time initialize handshake.
    /// This is the default and preferred path.
    fn client_meta(&self) -> Value {
        json!({
            "io.modelcontextprotocol/clientInfo": {
                "name": MCP_CLIENT_NAME,
                "version": env!("CARGO_PKG_VERSION")
            },
            "protocolVersion": MCP_PROTOCOL_VERSION
        })
    }

    /// Optional: Call the new `server/discover` method (2026+ stateless servers).
    ///
    /// This is a Phase 0 stub. It will only work if the server supports the
    /// method and we are in stateless mode.
    pub async fn server_discover(&self, server_name: &str) -> Result<Value> {
        let connection = self
            .servers
            .get(server_name)
            .ok_or_else(|| anyhow!("Server not connected: {}", server_name))?;

        let mut msg = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "server/discover",
            "params": {}
        });

        if !connection.use_legacy_handshake
            && let Some(obj) = msg.as_object_mut() {
                obj.insert("_meta".to_string(), self.client_meta());
            }

        self.send_message(connection, &msg).await?;
        let response = self.read_response(connection).await?;

        if let Some(result) = response.get("result") {
            return Ok(result.clone());
        }

        if let Some(error) = response.get("error") {
            return Err(anyhow!("server/discover failed: {:?}", error));
        }

        Ok(json!({}))
    }
}
