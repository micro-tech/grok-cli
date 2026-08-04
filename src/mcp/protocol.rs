use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current MCP protocol version we implement (2026-07-28 is the current
/// stateless model per the post-2025 spec updates).
pub const DEFAULT_PROTOCOL_VERSION: &str = "2026-07-28";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum JsonRpcMessage {
    #[serde(rename = "initialize")]
    Initialize {
        protocol_version: String,
        capabilities: ClientCapabilities,
        client_info: ClientInfo,
    },
    #[serde(rename = "tools/list")]
    ListTools {},
    #[serde(rename = "tools/call")]
    CallTool { name: String, arguments: Value },
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    /// Optional metadata for 2026+ stateless caching (SEP-2575 style).
    /// Present on the result object (not inside tools array).
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<ToolListMeta>,
}

/// 2026+ metadata returned with tools/list (and similar list operations).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolListMeta {
    /// Time-to-live in milliseconds for this tool list.
    /// Clients may cache the list for up to this duration.
    #[serde(rename = "ttlMs")]
    pub ttl_ms: Option<u64>,

    /// Scope of the cache: "session", "connection", "global", etc.
    #[serde(rename = "cacheScope")]
    pub cache_scope: Option<String>,

    /// Any other vendor-specific metadata.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
        text: Option<String>,
        blob: Option<String>,
    },
}

/// Standard initialize result returned by MCP servers (2024-11-05+).
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}
