//! MCP-over-ACP bridging (Task 111.8 + Task 142).
//!
//! Exposes Grok-CLI's tools as an MCP server endpoint over the ACP transport.
//! This allows any MCP client to discover and invoke the 32+ LLM-callable tools.
//!
//! Real implementation (Task 142): routes tool calls through the actual
//! tool execution registry instead of returning a stub string.

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

/// Real MCP tool call handler (Task 142).
/// Routes the call through the tool registry / execution path and returns
/// a proper MCP result (or error) instead of a stub message.
pub async fn handle_mcp_tool_call(name: &str, arguments: Value) -> Result<Value, String> {
    // Convert the MCP arguments into the format the tool registry expects.
    // Most tools accept a JSON object; we pass it through directly.
    let args = if arguments.is_object() {
        arguments
    } else {
        json!({ "input": arguments })
    };

    // Attempt to execute the tool via the registry.
    // We create a minimal policy context (no external access by default for MCP).
    let policy = crate::acp::security::SecurityManager::new().get_policy();
    let ctx = crate::tools::ToolContext::new(policy);

    match crate::tools::registry::execute_tool(name, &args, &ctx).await {
        Ok(result) => Ok(json!({
            "result": result,
            "tool": name,
            "status": "success"
        })),
        Err(e) => Err(format!("Tool '{}' failed: {}", name, e)),
    }
}
