use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::mcp::config::McpServerConfig;
use crate::mcp::protocol::{ClientCapabilities, ClientInfo, Tool};

pub struct McpClient {
    servers: HashMap<String, ServerConnection>,
}

struct ServerConnection {
    process: Child,
    stdin: Mutex<ChildStdin>,
    reader: Mutex<BufReader<ChildStdout>>,
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
            McpServerConfig::Stdio { command, args, env } => {
                info!(
                    "Connecting to MCP server '{}' via stdio: {} {:?}",
                    name, command, args
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
                };

                // Initialize handshake
                self.initialize_handshake(&connection).await?;

                self.servers.insert(name.to_string(), connection);
                info!("Connected to MCP server '{}'", name);
                Ok(())
            }
            McpServerConfig::Sse { .. } => Err(anyhow!("SSE transport not yet implemented")),
        }
    }

    async fn initialize_handshake(&self, connection: &ServerConnection) -> Result<()> {
        let init_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "0.1.0",
                "capabilities": {},
                "clientInfo": {
                    "name": "grok-cli",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        });

        self.send_message(connection, &init_msg).await?;
        let response = self.read_response(connection).await?;

        // TODO: Validate response?
        debug!("Initialize response: {:?}", response);

        // Send initialized notification
        let initialized_msg = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        self.send_message(connection, &initialized_msg).await?;

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
        let connection = self
            .servers
            .get(server_name)
            .ok_or_else(|| anyhow!("Server not connected: {}", server_name))?;

        let msg = json!({
            "jsonrpc": "2.0",
            "id": 2, // simple id gen needed
            "method": "tools/list",
            "params": {}
        });

        self.send_message(connection, &msg).await?;
        let response = self.read_response(connection).await?;

        // Parse response
        if let Some(result) = response.get("result")
            && let Some(tools_val) = result.get("tools") {
                let tools: Vec<Tool> = serde_json::from_value(tools_val.clone())?;
                return Ok(tools);
            }

        Ok(Vec::new())
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

        let msg = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": args
            }
        });

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
}
