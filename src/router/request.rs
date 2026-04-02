use grok_api::Message;
use serde::{Deserialize, Serialize};

/// A function parameter schema, compatible with the OpenAI/xAI tool-calling spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A tool definition that can be passed to the Grok API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Always `"function"` for now.
    pub r#type: String,
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    /// Convenience constructor — sets `type` to `"function"` automatically.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

/// The unified request type handed to any backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterRequest {
    /// Model identifier, e.g. `"grok-3-mini"`.
    pub model: String,
    /// Conversation history in OpenAI-compatible message format.
    pub messages: Vec<Message>,
    /// Optional tool definitions to expose to the model.
    pub tools: Vec<ToolDefinition>,
    /// Cap on generated tokens (passed straight through to the backend).
    pub max_tokens: Option<u32>,
    /// Sampling temperature (0.0 – 2.0).
    pub temperature: Option<f32>,
}

impl RouterRequest {
    /// Create a minimal request with just a model and messages.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            tools: Vec::new(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// Attach tool definitions.
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the max-tokens limit.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the sampling temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Accept raw JSON tool definitions (the format returned by
    /// [`crate::acp::tools::get_available_tool_definitions`]) and convert
    /// them to typed [`ToolDefinition`] values.
    ///
    /// Any entry that cannot be deserialised is silently skipped so that
    /// unexpected fields from older code don't abort the request.
    pub fn with_json_tools(mut self, tools: Vec<serde_json::Value>) -> Self {
        self.tools = tools
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        self
    }
}
