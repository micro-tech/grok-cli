//! Minimal MCP stdio echo server for testing the stateless client path.
//!
//! It supports:
//! - tools/list
//! - tools/call with an "echo" tool that returns the arguments it received
//!
//! It works in both legacy handshake mode and pure stateless (_meta) mode.
//! Used by integration tests for MCP 2026 stateless changes.

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = request.get("id").cloned().unwrap_or(json!(null));
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "serverInfo": { "name": "mcp-test-echo", "version": "0.1.0" }
                }
            }),

            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echoes back the arguments it received (stateless test tool)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "message": { "type": "string" }
                                }
                            }
                        }
                    ]
                }
            }),

            "tools/call" => {
                let empty = json!({});
                let params = request.get("params").unwrap_or(&empty);
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");

                if tool_name == "echo" {
                    let args = params.get("arguments").cloned().unwrap_or(json!({}));
                    let echo_text = format!("ECHO: {}", serde_json::to_string(&args).unwrap_or_default());
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": echo_text
                                }
                            ]
                        }
                    })
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": "Unknown tool" }
                    })
                }
            }

            "notifications/initialized" => {
                // No response needed for notifications
                continue;
            }

            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            }),
        };

        let response_line = serde_json::to_string(&response).unwrap() + "\n";
        let _ = stdout.write_all(response_line.as_bytes());
        let _ = stdout.flush();
    }
}
