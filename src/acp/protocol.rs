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

/// The nested `toolCall` object inside `session/request_permission` params.
/// Mirrors the `ToolCallUpdate` shape from the ACP spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionToolCall {
    /// Identifier of the tool call awaiting permission.
    pub tool_call_id: String,
    /// Optional human-readable title for the permission dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional tool category hint for UI icons.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    /// Optional current status (typically `pending` before permission is granted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
}

/// Parameters sent from the agent to the client in a `session/request_permission`
/// JSON-RPC request.  Conforms to the official ACP spec:
/// https://agentclientprotocol.com/protocol/tool-calls#requesting-permission
///
/// The client must reply with a JSON-RPC *response* whose `result` field
/// deserialises as [`PermissionOutcome`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPermissionParams {
    /// The ACP session this permission request belongs to.
    pub session_id: SessionId,
    /// The tool call that requires permission before it can run.
    pub tool_call: PermissionToolCall,
    /// The ordered list of permission options for the user to choose from.
    /// Callers should use [`default_permission_options()`] for the standard layout.
    pub options: Vec<PermissionOption>,
}

impl RequestPermissionParams {
    /// Convenience constructor.
    pub fn new(
        session_id: SessionId,
        tool_call_id: impl Into<String>,
        title: Option<impl Into<String>>,
        kind: Option<ToolKind>,
    ) -> Self {
        Self {
            session_id,
            tool_call: PermissionToolCall {
                tool_call_id: tool_call_id.into(),
                title: title.map(|t| t.into()),
                kind,
                status: Some(ToolCallStatus::Pending),
            },
            options: default_permission_options(),
        }
    }
}

/// Inner detail of a `session/request_permission` outcome.
///
/// Serialises as:
/// - `{"outcome": "cancelled"}` — user dismissed or prompt turn was cancelled
/// - `{"outcome": "selected", "optionId": "..."}` — user clicked a button
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum OutcomeDetail {
    /// The prompt turn was cancelled before the user responded.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// The user selected one of the offered permission options.
    #[serde(rename = "selected")]
    Selected {
        #[serde(rename = "optionId")]
        option_id: String,
    },
}

/// The client's response to a `session/request_permission` JSON-RPC request.
/// Deserialised from the `result` field of the matching response.
///
/// Per ACP spec the result shape is:
/// `{"outcome": {"outcome": "selected", "optionId": "..."}}` or
/// `{"outcome": {"outcome": "cancelled"}}`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOutcome {
    pub outcome: OutcomeDetail,
}

impl PermissionOutcome {
    /// Build a cancelled outcome — used when the prompt turn is cancelled or
    /// when the client disconnects (e.g. Starlink handover) before responding.
    pub fn cancel() -> Self {
        Self {
            outcome: OutcomeDetail::Cancelled,
        }
    }

    /// Build a proceed-once outcome — used when the client does not support
    /// `session/request_permission` (returns a JSON-RPC error) so that tools
    /// still execute rather than being silently blocked.
    pub fn proceed_once() -> Self {
        Self {
            outcome: OutcomeDetail::Selected {
                option_id: "proceed_once".to_string(),
            },
        }
    }

    /// Returns `true` when this outcome represents a denial of the tool call.
    pub fn is_cancelled(&self) -> bool {
        matches!(self.outcome, OutcomeDetail::Cancelled)
    }

    /// Returns `true` when this outcome means the tool should be allowed for
    /// all future same-tool invocations in the session.
    pub fn is_always_allow(&self) -> bool {
        matches!(&self.outcome, OutcomeDetail::Selected { option_id } if option_id == "proceed_always")
    }

    /// Returns the selected `option_id`, or `None` if the outcome was cancelled.
    pub fn option_id(&self) -> Option<&str> {
        match &self.outcome {
            OutcomeDetail::Selected { option_id } => Some(option_id.as_str()),
            OutcomeDetail::Cancelled => None,
        }
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
            "tc-1",
            Some("Run shell command"),
            Some(ToolKind::Execute),
        );

        let json_val = serde_json::to_value(&params).unwrap();

        // Top-level ACP spec fields
        assert!(json_val.get("sessionId").is_some(), "sessionId missing");
        assert!(json_val.get("toolCall").is_some(), "toolCall missing");
        assert!(json_val.get("options").is_some(), "options missing");

        // requestId, title, message must NOT be top-level (spec does not have them)
        assert!(
            json_val.get("requestId").is_none(),
            "requestId must be absent per spec"
        );
        assert!(
            json_val.get("title").is_none(),
            "title must be absent per spec"
        );
        assert!(
            json_val.get("message").is_none(),
            "message must be absent per spec"
        );

        // toolCall nesting
        let tool_call = &json_val["toolCall"];
        assert_eq!(tool_call["toolCallId"], "tc-1");
        assert_eq!(tool_call["status"], "pending");

        // Three standard options
        let options = json_val["options"].as_array().unwrap();
        assert_eq!(options.len(), 3);
        assert_eq!(options[0]["optionId"], "proceed_always");
        assert_eq!(options[1]["optionId"], "proceed_once");
        assert_eq!(options[2]["optionId"], "cancel");

        // kind serialises as snake_case per spec
        assert_eq!(options[0]["kind"], "allow_always");
        assert_eq!(options[1]["kind"], "allow_once");
        assert_eq!(options[2]["kind"], "reject_once");

        // Round-trip
        let re: RequestPermissionParams = serde_json::from_value(json_val).unwrap();
        assert_eq!(re.tool_call.tool_call_id, "tc-1");
        assert_eq!(re.options[0].option_id, "proceed_always");
    }

    #[test]
    fn test_permission_outcome_helpers() {
        // Cancel outcome
        let cancel = PermissionOutcome::cancel();
        assert!(cancel.is_cancelled());
        assert!(!cancel.is_always_allow());
        assert_eq!(cancel.option_id(), None);

        // Proceed-once outcome
        let once = PermissionOutcome::proceed_once();
        assert!(!once.is_cancelled());
        assert!(!once.is_always_allow());
        assert_eq!(once.option_id(), Some("proceed_once"));

        // Always-allow outcome
        let always = PermissionOutcome {
            outcome: OutcomeDetail::Selected {
                option_id: "proceed_always".to_string(),
            },
        };
        assert!(always.is_always_allow());
        assert!(!always.is_cancelled());

        // ACP spec response shape: {"outcome":{"outcome":"selected","optionId":"proceed_always"}}
        let json_val = serde_json::to_value(&always).unwrap();
        let inner = &json_val["outcome"];
        assert_eq!(inner["outcome"], "selected");
        assert_eq!(inner["optionId"], "proceed_always");

        // Cancelled shape: {"outcome":{"outcome":"cancelled"}}
        let cancel_json = serde_json::to_value(&cancel).unwrap();
        assert_eq!(cancel_json["outcome"]["outcome"], "cancelled");
        assert!(cancel_json["outcome"].get("optionId").is_none());
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
