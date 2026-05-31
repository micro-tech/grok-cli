//! MCP-over-ACP bridging (Task 111.8).
//!
//! Exposes Grok-CLI's tools as an MCP server endpoint over the ACP transport.
//! This allows any MCP client to discover and invoke the 32+ LLM-callable tools.

use serde_json::{json, Value};

/// Returns an MCP-compatible tool list for the current agent.
pub fn get_mcp_tool_list() -> Value {
    let tools = crate::tools::registry::get_available_tool_definitions();

    json!({
        "tools": tools,
        "serverInfo": {
            "name": "grok-cli",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

/// Placeholder for handling an MCP tool call over ACP.
pub async fn handle_mcp_tool_call(name: &str, arguments: Value) -> Result<Value, String> {
    // In a full implementation this would route through the same
    // tool execution path used by the ACP and CLI layers.
    Ok(json!({
        "result": format!("MCP call to '{}' received (stub)", name),
        "arguments": arguments
    }))
}
