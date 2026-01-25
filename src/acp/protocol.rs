use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    #[serde(default, rename = "sessionCapabilities")]
    pub session_capabilities: SessionCapabilities,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentCapabilities {
    pub fn new() -> Self {
        Self {
            session_capabilities: SessionCapabilities::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionCapabilities {}

impl SessionCapabilities {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(
        default = "default_protocol_version",
        deserialize_with = "deserialize_protocol_version",
        alias = "protocolVersion"
    )]
    pub protocol_version: String,
    #[serde(default, alias = "clientCapabilities")]
    pub capabilities: Value,
    #[serde(default, alias = "clientInfo")]
    pub client_info: Value,
}

fn default_protocol_version() -> String {
    "1".to_string()
}

fn deserialize_protocol_version<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_string())
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_string())
            } else {
                Ok("1".to_string())
            }
        }
        Value::String(s) => Ok(s),
        _ => Err(D::Error::custom(
            "protocol_version must be a number or string",
        )),
    }
}

fn serialize_protocol_version<S>(version: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Try to parse as integer, if successful serialize as number, otherwise as string
    if let Ok(num) = version.parse::<i64>() {
        serializer.serialize_i64(num)
    } else {
        serializer.serialize_str(version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    #[serde(
        rename = "protocolVersion",
        serialize_with = "serialize_protocol_version"
    )]
    pub protocol_version: String,
    #[serde(rename = "agentCapabilities")]
    pub agent_capabilities: AgentCapabilities,
    #[serde(rename = "agentInfo")]
    pub agent_info: Implementation,
}

impl InitializeResponse {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            protocol_version: version.into(),
            agent_capabilities: AgentCapabilities::new(),
            agent_info: Implementation {
                name: "grok-cli".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    pub fn agent_capabilities(mut self, caps: AgentCapabilities) -> Self {
        self.agent_capabilities = caps;
        self
    }

    pub fn agent_info(mut self, info: Implementation) -> Self {
        self.agent_info = info;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

impl Implementation {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewSessionRequest {
    #[serde(default, alias = "sessionCapabilities")]
    pub capabilities: Value,
    #[serde(default, alias = "workspaceRoot")]
    pub workspace_root: Option<String>,
    #[serde(default, alias = "workingDirectory")]
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionResponse {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
}

impl NewSessionResponse {
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRequest {
    #[serde(alias = "sessionId")]
    pub session_id: SessionId,
    pub prompt: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "resource")]
    Resource(ResourceContent),
    #[serde(rename = "resource_link")]
    ResourceLink(ResourceLinkContent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
}

impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub resource: EmbeddedResourceResource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddedResourceResource {
    TextResourceContents(TextResourceContents),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResourceContents {
    pub uri: String,
    pub text: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLinkContent {
    pub uri: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptResponse {
    #[serde(rename = "stopReason")]
    pub stop_reason: StopReason,
}

impl PromptResponse {
    pub fn new(stop_reason: StopReason) -> Self {
        Self { stop_reason }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNotification {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub update: SessionUpdate,
}

impl SessionNotification {
    pub fn new(session_id: SessionId, update: SessionUpdate) -> Self {
        Self { session_id, update }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "sessionUpdate")]
pub enum SessionUpdate {
    #[serde(rename = "agent_message_chunk")]
    AgentMessageChunk(ContentChunk),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    pub content: ContentBlock,
}

impl ContentChunk {
    pub fn new(content: ContentBlock) -> Self {
        Self { content }
    }
}

pub struct ProtocolVersion;

impl ProtocolVersion {
    pub const LATEST: &'static str = "1";
    pub const V1: &'static str = "1";
    pub const DATE_FORMAT: &'static str = "2024-04-15";
}

pub struct MethodNames {
    pub initialize: &'static str,
    pub session_new: &'static str,
    pub session_prompt: &'static str,
}

pub const AGENT_METHOD_NAMES: MethodNames = MethodNames {
    initialize: "initialize",
    session_new: "session/new",
    session_prompt: "session/prompt",
};

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_session_notification_serialization() {
        let text = "Hello world";
        let content = ContentBlock::Text(TextContent::new(text));
        let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(content));
        let notification = SessionNotification::new(SessionId::new("session-123"), update);

        let json = serde_json::to_string_pretty(&notification).unwrap();
        println!("Serialized Notification: {}", json);

        let expected = json!({
            "sessionId": "session-123",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": {
                    "type": "text",
                    "text": "Hello world"
                }
            }
        });

        assert_eq!(serde_json::to_value(&notification).unwrap(), expected);
    }
}
