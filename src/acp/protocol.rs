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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCapabilities {
    /// Whether this agent supports Gemini-style permission requests before tool
    /// execution. Clients that understand `session/request_permission` should
    /// check this flag before enabling the three-button permission prompt UI.
    #[serde(
        default = "default_supports_permission_requests",
        rename = "supportsPermissionRequests"
    )]
    pub supports_permission_requests: bool,
}

impl Default for SessionCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionCapabilities {
    pub fn new() -> Self {
        Self {
            supports_permission_requests: true,
        }
    }
}

fn default_supports_permission_requests() -> bool {
    true
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
    /// Project root sent by the client (e.g. Zed) at initialization time.
    /// Accepted under multiple field names to maximise compatibility.
    #[serde(
        default,
        alias = "workspaceRoot",
        alias = "workspace_root",
        alias = "rootUri",
        alias = "rootPath"
    )]
    pub workspace_root: Option<String>,
    /// Alternative working-directory field some clients send.
    #[serde(default, alias = "workingDirectory")]
    pub working_directory: Option<String>,
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
    /// Sent after session creation (and any time the set changes) to advertise
    /// the slash commands this agent supports.
    #[serde(rename = "available_commands_update")]
    AvailableCommandsUpdate(AvailableCommandsUpdate),
    /// Notification that a new tool call has been initiated.
    #[serde(rename = "tool_call")]
    ToolCall(ToolCall),
    /// Update on the status or results of a tool call.
    #[serde(rename = "tool_call_update")]
    ToolCallUpdate(ToolCallUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Edit,
    Search,
    Execute,
    Think,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub tool_call_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<ToolCallLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolCallContent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallUpdate {
    pub tool_call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<ToolCallLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolCallContent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolCallContent {
    #[serde(rename = "text")]
    Text(TextContent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallLocation {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<ToolCallRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
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

// ---------------------------------------------------------------------------
// Slash-command advertisement types (ACP `available_commands_update`)
// ---------------------------------------------------------------------------

/// Input specification for a slash command — currently only unstructured text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCommandInput {
    /// Hint text shown in the client UI when no input has been typed yet.
    pub hint: String,
}

impl AvailableCommandInput {
    pub fn new(hint: impl Into<String>) -> Self {
        Self { hint: hint.into() }
    }
}

/// A single slash command that the agent advertises to ACP clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCommand {
    /// Command name without leading slash (e.g. `"web"`, `"plan"`).
    pub name: String,
    /// Human-readable description shown in the client command palette.
    pub description: String,
    /// Optional input specification. Omitted for commands that take no args.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AvailableCommandInput>,
}

impl AvailableCommand {
    /// Create a command that accepts no additional input.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input: None,
        }
    }

    /// Builder-style helper: attach a text-input hint to this command.
    pub fn with_input(mut self, hint: impl Into<String>) -> Self {
        self.input = Some(AvailableCommandInput::new(hint));
        self
    }
}

/// Payload carried inside a `SessionUpdate::AvailableCommandsUpdate` notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCommandsUpdate {
    #[serde(rename = "availableCommands")]
    pub available_commands: Vec<AvailableCommand>,
}

impl AvailableCommandsUpdate {
    pub fn new(commands: Vec<AvailableCommand>) -> Self {
        Self {
            available_commands: commands,
        }
    }
}

// ---------------------------------------------------------------------------
// ACP Gemini-style permission types (session/request_permission RPC)
// ---------------------------------------------------------------------------

/// Semantic intent of a permission option presented to the user.
///
/// Maps to the three standard Gemini permission button kinds:
///
/// | Variant      | `option_id`       | UI label       |
/// |--------------|-------------------|----------------|
/// | `AllowAlways`| `"proceed_always"`| "Always Allow" |
/// | `AllowOnce`  | `"proceed_once"`  | "Allow"        |
/// | `RejectOnce` | `"cancel"`        | "Reject"       |
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    /// Grant permission for this tool for the remainder of the session.
    AllowAlways,
    /// Grant permission for this single tool invocation only.
    AllowOnce,
    /// Deny permission for this tool invocation.
    RejectOnce,
}

/// A single button in the Gemini-style three-button permission prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    /// Stable identifier sent back by the client in [`PermissionOutcome`].
    /// Standard values: `"proceed_always"`, `"proceed_once"`, `"cancel"`.
    pub option_id: String,
    /// Human-readable label shown in the client UI button (e.g. `"Always Allow"`).
    pub name: String,
    /// Semantic kind so the client can colour or style the button appropriately.
    pub kind: PermissionKind,
}

impl PermissionOption {
    /// Construct a single permission option directly.
    pub fn new(
        option_id: impl Into<String>,
        name: impl Into<String>,
        kind: PermissionKind,
    ) -> Self {
        Self {
            option_id: option_id.into(),
            name: name.into(),
            kind,
        }
    }
}

/// Returns the canonical three permission options used by the Gemini-style UI.
///
/// These match the three buttons shown to the user:
///
/// ```text
/// [ Always Allow ]  [ Allow ]  [ Reject ]
/// ```
///
/// The client echoes back the chosen `option_id` inside [`PermissionOutcome`].
pub fn default_permission_options() -> Vec<PermissionOption> {
    vec![
        PermissionOption::new(
            "proceed_always",
            "Always Allow",
            PermissionKind::AllowAlways,
        ),
        PermissionOption::new("proceed_once", "Allow", PermissionKind::AllowOnce),
        PermissionOption::new("cancel", "Reject", PermissionKind::RejectOnce),
    ]
}

/// Parameters sent from the agent to the client in a `session/request_permission`
/// JSON-RPC request.  The client must reply with a JSON-RPC *response* whose
/// `result` field deserialises as [`PermissionOutcome`].
///
/// The `request_id` field is distinct from the JSON-RPC message `id` — it is
/// carried *inside* the payload so that the outcome can be correlated even
/// after the raw JSON-RPC `id` has been consumed by the transport layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPermissionParams {
    /// The ACP session this permission request belongs to.
    pub session_id: SessionId,
    /// Unique identifier for this permission request.  Echoed back by the
    /// client in [`PermissionOutcome::request_id`] so the agent can correlate
    /// the response to the waiting tool call.
    pub request_id: String,
    /// The tool-call identifier that is gated by this permission request.
    pub tool_call_id: String,
    /// Short, human-readable title shown at the top of the permission prompt
    /// (e.g. `"Run shell command"`).
    pub title: String,
    /// Human-readable description of what the tool is about to do, shown as
    /// body text in the prompt (e.g. `"ls -la /tmp"`).
    pub message: String,
    /// Optional semantic kind that the client can use to style the prompt icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    /// The ordered list of buttons to present.  Callers should use
    /// [`default_permission_options()`] for the standard three-button layout.
    pub options: Vec<PermissionOption>,
}

impl RequestPermissionParams {
    /// Convenience constructor for the common case.
    pub fn new(
        session_id: SessionId,
        request_id: impl Into<String>,
        tool_call_id: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
        kind: Option<ToolKind>,
    ) -> Self {
        Self {
            session_id,
            request_id: request_id.into(),
            tool_call_id: tool_call_id.into(),
            title: title.into(),
            message: message.into(),
            kind,
            options: default_permission_options(),
        }
    }
}

/// The client's response to a `session/request_permission` request.
/// Deserialised from the `result` field of the matching JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOutcome {
    /// Echoed from [`RequestPermissionParams::request_id`] for correlation.
    pub request_id: String,
    /// The `option_id` of the button chosen by the user.
    /// Standard values: `"proceed_always"`, `"proceed_once"`, `"cancel"`.
    pub option_id: String,
}

impl PermissionOutcome {
    /// Build a cancel outcome — used as a safe fallback when the client
    /// disconnects (e.g. Starlink handover) before responding.
    pub fn cancel(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            option_id: "cancel".to_string(),
        }
    }

    /// Build a proceed-once outcome — used when the client does not support
    /// the `session/request_permission` protocol (e.g. Zed returning a
    /// JSON-RPC error for an unknown method) so that tools still execute
    /// rather than being silently blocked.
    pub fn proceed_once(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            option_id: "proceed_once".to_string(),
        }
    }

    /// Returns `true` when this outcome represents a denial of the tool call.
    pub fn is_cancelled(&self) -> bool {
        self.option_id == "cancel"
    }

    /// Returns `true` when this outcome means the tool should be allowed this
    /// invocation AND all future same-tool invocations in the session.
    pub fn is_always_allow(&self) -> bool {
        self.option_id == "proceed_always"
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
    /// Gemini-style pre-tool-execution permission prompt sent by the agent.
    pub session_request_permission: &'static str,
}

pub const AGENT_METHOD_NAMES: MethodNames = MethodNames {
    initialize: "initialize",
    session_new: "session/new",
    session_prompt: "session/prompt",
    session_request_permission: "session/request_permission",
};

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_permission_params_serialization() {
        let params = RequestPermissionParams::new(
            SessionId::new("sess-abc"),
            "req-001",
            "tc-1",
            "Run shell command",
            "ls -la /tmp",
            Some(ToolKind::Execute),
        );

        let json_val = serde_json::to_value(&params).unwrap();

        // Verify camelCase field names
        assert!(
            json_val.get("sessionId").is_some(),
            "sessionId field missing"
        );
        assert!(
            json_val.get("requestId").is_some(),
            "requestId field missing"
        );
        assert!(
            json_val.get("toolCallId").is_some(),
            "toolCallId field missing"
        );
        assert!(json_val.get("options").is_some(), "options field missing");

        // Verify the three standard option ids are present in order
        let options = json_val["options"].as_array().unwrap();
        assert_eq!(options.len(), 3);
        assert_eq!(options[0]["optionId"], "proceed_always");
        assert_eq!(options[1]["optionId"], "proceed_once");
        assert_eq!(options[2]["optionId"], "cancel");

        // Verify kind values serialise as snake_case
        assert_eq!(options[0]["kind"], "allow_always");
        assert_eq!(options[1]["kind"], "allow_once");
        assert_eq!(options[2]["kind"], "reject_once");

        // Round-trip
        let re: RequestPermissionParams = serde_json::from_value(json_val).unwrap();
        assert_eq!(re.request_id, "req-001");
        assert_eq!(re.options[0].option_id, "proceed_always");
    }

    #[test]
    fn test_permission_outcome_helpers() {
        let cancel = PermissionOutcome::cancel("req-001");
        assert!(cancel.is_cancelled());
        assert!(!cancel.is_always_allow());

        let always = PermissionOutcome {
            request_id: "req-002".to_string(),
            option_id: "proceed_always".to_string(),
        };
        assert!(always.is_always_allow());
        assert!(!always.is_cancelled());
    }

    #[test]
    fn test_session_capabilities_serializes_permission_flag() {
        let caps = SessionCapabilities::new();
        let json_val = serde_json::to_value(&caps).unwrap();
        assert_eq!(
            json_val["supportsPermissionRequests"], true,
            "supportsPermissionRequests must default to true"
        );
    }

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
