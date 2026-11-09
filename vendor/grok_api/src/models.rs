//! Data models for Grok API requests and responses

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents an image URL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageUrl {
    /// The URL of the image
    pub url: String,
    /// The detail level of the image (low, high, auto)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Represents a video URL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoUrl {
    /// The URL of the video
    pub url: String,
    /// The detail level (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A part of a message content (text, image, or video)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content
    Text { text: String },
    /// Image content
    ImageUrl { image_url: ImageUrl },
    /// Video content
    VideoUrl { video_url: VideoUrl },
}

/// The content of a message, which can be a simple string or a list of parts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Complex content with multiple parts
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Get the content as a text string if possible
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(s) => Some(s),
            MessageContent::Parts(parts) => {
                // Return the first text part, or None
                for part in parts {
                    if let ContentPart::Text { text } = part {
                        return Some(text);
                    }
                }
                None
            }
        }
    }

    /// Get the text content, returning empty string if none
    pub fn text(&self) -> &str {
        match self {
            MessageContent::Text(s) => s,
            MessageContent::Parts(parts) => {
                // Return the first text part, or empty string
                for part in parts {
                    if let ContentPart::Text { text } = part {
                        return text;
                    }
                }
                ""
            }
        }
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageContent::Text(s) => write!(f, "{}", s),
            MessageContent::Parts(parts) => {
                for part in parts {
                    match part {
                        ContentPart::Text { text } => write!(f, "{}", text)?,
                        ContentPart::ImageUrl { image_url } => {
                            write!(f, "[Image: {}]", image_url.url)?
                        }
                        ContentPart::VideoUrl { video_url } => {
                            write!(f, "[Video: {}]", video_url.url)?
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

/// Represents a chat message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// The role of the message sender
    pub role: String,

    /// The content of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,

    /// Tool calls made by the assistant (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Tool call ID (required for role: "tool")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    /// Create a system message
    pub fn system<S: Into<MessageContent>>(content: S) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message
    pub fn user<S: Into<MessageContent>>(content: S) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message with multiple parts
    pub fn user_parts(parts: Vec<ContentPart>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(MessageContent::Parts(parts)),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message
    pub fn assistant<S: Into<MessageContent>>(content: S) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message with tool calls
    pub fn assistant_with_tools<S: Into<MessageContent>>(
        content: Option<S>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.map(|c| c.into()),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(MessageContent::Text(content.into())),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// A chat completion request
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// The model to use for completion
    pub model: String,

    /// The conversation messages
    pub messages: Vec<ChatMessage>,

    /// Sampling temperature (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Tools available for the model to call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,

    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0).
    ///
    /// **⚠ Not supported by reasoning models** (Grok 4, Grok 4.20 variants).
    /// Sending this field to a reasoning model will result in an API error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (-2.0 to 2.0).
    ///
    /// **⚠ Not supported by reasoning models** (Grok 4, Grok 4.20 variants).
    /// Sending this field to a reasoning model will result in an API error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Reasoning effort level for models that support extended thinking.
    /// Valid values: `"low"`, `"medium"`, `"high"`.
    /// Only send this for models that support it (e.g. `grok-4.3`, `grok-3-mini`).
    /// Sending it to unsupported models returns an API error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    /// Prompt cache key (sent as `x-grok-conv-id` header on Chat Completions).
    ///
    /// Strongly recommended by xAI for Grok 4.7 and other models to get
    /// reliable prompt caching across turns in a conversation.
    ///
    /// See: https://docs.x.ai/developers/advanced-api-usage/prompt-caching
    #[serde(skip_serializing)]
    pub prompt_cache_key: Option<String>,
}

/// Response from a chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Unique identifier for the response
    pub id: String,

    /// Object type (always "chat.completion")
    pub object: String,

    /// Unix timestamp of when the response was created
    pub created: u64,

    /// The model used for completion
    pub model: String,

    /// The completion choices
    pub choices: Vec<Choice>,

    /// Token usage information
    pub usage: Usage,
}

impl ChatResponse {
    /// Get the primary response content (from first choice)
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .and_then(|content| content.as_text())
    }

    /// Get the primary response message
    pub fn message(&self) -> Option<&Message> {
        self.choices.first().map(|choice| &choice.message)
    }

    /// Get tool calls from the primary response
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        self.choices
            .first()
            .and_then(|choice| choice.message.tool_calls.as_deref())
    }

    /// Check if the response includes tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls().is_some_and(|calls| !calls.is_empty())
    }
}

/// A single completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Index of this choice
    pub index: u32,

    /// The message content
    pub message: Message,

    /// Reason why the completion finished
    pub finish_reason: Option<String>,
}

/// A message in a chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: String,

    /// Content of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,

    /// Tool/function calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Reasoning / chain-of-thought content produced by the model when
    /// `reasoning_effort` was set on the request.  Only present for
    /// reasoning-capable models (e.g. grok-4.3, grok-3-mini).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

/// A tool/function call made by the assistant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique identifier for the tool call
    pub id: String,

    /// Type of tool call (usually "function")
    #[serde(rename = "type")]
    pub call_type: String,

    /// The function being called
    pub function: FunctionCall,
}

/// Details of a function call
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionCall {
    /// Name of the function
    pub name: String,

    /// JSON string of function arguments
    pub arguments: String,
}

impl FunctionCall {
    /// Parse the arguments as JSON
    pub fn parse_arguments(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_str(&self.arguments)
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,

    /// Number of tokens in the completion
    pub completion_tokens: u32,

    /// Total number of tokens used
    pub total_tokens: u32,

    /// Number of prompt tokens served from cache (reduces cost on repeated prompts).
    /// Only present when prompt caching was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_prompt_tokens: Option<u32>,

    /// Tokens used internally by the model for reasoning (not in the final response).
    /// Only present for reasoning models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

/// Available Grok models
///
/// ## Grok 4.7 — Current Flagship (August 2026)
/// - SpaceXAI's frontier model for coding, agentic tasks, and knowledge work.
/// - Context window: **500,000 tokens**. Knowledge cutoff: **May 2026**.
/// - Supports `reasoning_effort`: `low`, `medium`, `high` (default), `xhigh`.
/// - Modalities: text + image input → text output; no output token limit.
/// - Pricing (< 200 k prompt tokens): **$2.00 / 1 M input · $6.00 / 1 M output**.
/// - Pricing (≥ 200 k prompt tokens): **$4.00 / 1 M input · $12.00 / 1 M output**.
/// - Cached input: **$0.50 / 1 M** (< 200 k) · **$1.00 / 1 M** (≥ 200 k).
/// - Use for: **code, chat, agentic tasks, STEM, knowledge work** — everything.
///
/// ## Grok 4.6 — Previous Flagship (August 2026)
/// - Same 500 K context and pricing as 4.7. Knowledge cutoff: **February 1, 2026**.
/// - Prefer [`Model::Grok4_7`] for new work.
///
/// ## Grok Build 0.1 — Agentic Coding (May 2026)
/// - Fast coding model trained specifically for **agentic coding workflows**.
/// - Accepts text and image input; returns text output.
/// - Context window: **256,000 tokens**.
/// - Reasoning is **built-in** (not user-configurable).
/// - Priced at $1.00 / 1 M input tokens, $2.00 / 1 M output tokens.
/// - Aliases: `grok-code-fast-1`, `grok-code-fast`.
///
/// ## Grok 4.20 Notes
/// - Context window: **2,000,000 tokens**.
/// - Does **not** support the `logprobs` field (ignored if sent).
/// - `Grok4_20_0309Reasoning` / `Grok4_20MultiAgent0309` do **not** support
///   `presence_penalty`, `frequency_penalty`, or `stop`.
/// - No `reasoning_effort` parameter on these variants (returns error if sent).
///
/// ## Retirements — effective May 15, 2026
/// The following variants are **deprecated** and will stop working on
/// 2026-05-15 12:00 PT. Compiler deprecation warnings guide you to the
/// recommended replacement.
///
/// | Deprecated variant | Retires | Use instead |
/// |---|---|---|
/// | `Grok4_1FastReasoning` | 2026-05-15 | [`Model::Grok4_7`] |
/// | `Grok4_1FastNonReasoning` | 2026-05-15 | [`Model::Grok4_20NonReasoning`] |
/// | `Grok4_0709` | 2026-05-15 | [`Model::Grok4_7`] |
/// | `Grok3` | 2026-05-15 | [`Model::Grok4_7`] |
/// | `GrokCodeFast1` | 2026-05-15 | [`Model::GrokBuild0_1`] |
/// | `GrokImagineImagePro` | 2026-05-15 | [`Model::GrokImagineImage`] |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    // ── Grok 4.7 — Current flagship (August 2026, 500 K context) ─────────────────
    /// Grok 4.7 — SpaceXAI's frontier model for coding, agentic tasks, and
    /// knowledge work. **Recommended for all new work.**
    ///
    /// | Property | Value |
    /// |---|---|
    /// | Model name | `grok-4.7` |
    /// | Context window | 500,000 tokens |
    /// | Knowledge cutoff | May 2026 |
    /// | Modalities | Text + image input; text output |
    /// | Output limit | None |
    /// | Reasoning effort | `low`, `medium`, `high` (default), `xhigh` |
    /// | Input price | $2.00 / 1M tokens (< 200k prompt) |
    /// | Cached input | $0.50 / 1M tokens (< 200k prompt) |
    /// | Output price | $6.00 / 1M tokens (< 200k prompt) |
    /// | Long context (≥ 200k) | $4.00 in / $1.00 cached / $12.00 out per 1M |
    /// | Tools | Function calling, web search, X search, code execution |
    /// | APIs | Responses API and Chat Completions |
    ///
    /// Grok 4.7 Fast is the same model on faster infrastructure (2× token
    /// rates) and is **not** on the public API — Cursor and Grok Build only.
    ///
    /// Aliases recognised by [`Model::parse`]: `grok-4.7-latest`
    ///
    /// See the [official Grok 4.7 page](https://docs.x.ai/developers/grok-4-7).
    Grok4_7,

    // ── Grok 4.6 — Previous flagship (August 2026, 500 K context) ────────────
    /// Grok 4.6 — previous flagship for coding, agentic tasks, and knowledge
    /// work. Prefer [`Model::Grok4_7`] for new work.
    ///
    /// | Property | Value |
    /// |---|---|
    /// | Model name | `grok-4.6` |
    /// | Context window | 500,000 tokens |
    /// | Knowledge cutoff | February 1, 2026 |
    /// | Modalities | Text + image input; text output |
    /// | Output limit | None |
    /// | Reasoning effort | `low`, `medium`, `high` (default), `xhigh` |
    /// | Input price | $2.00 / 1M tokens (< 200k prompt) |
    /// | Cached input | $0.50 / 1M tokens (< 200k prompt) |
    /// | Output price | $6.00 / 1M tokens (< 200k prompt) |
    /// | Long context (≥ 200k) | $4.00 in / $1.00 cached / $12.00 out per 1M |
    /// | Tools | Function calling, web search, X search, code execution |
    ///
    /// Aliases recognised by [`Model::parse`]: `grok-4.6-latest`
    Grok4_6,

    // ── Grok 4.5 (500 K context, July 2026) ─────────────────────────────────────
    /// Grok 4.5 — SpaceXAI model for coding, agentic tasks, and knowledge work.
    /// Released July 2026; superseded by [`Model::Grok4_6`], then [`Model::Grok4_7`].
    ///
    /// - **Context window**: 500,000 tokens
    /// - **Reasoning effort**: `low`, `medium`, `high` (default); **no `xhigh`**
    /// - **Modalities**: text + image input → text output
    /// - **Input price**: $2.00 / 1M tokens · **Output**: $6.00 / 1M tokens
    /// - **Cached input**: $0.30 / 1M tokens (< 200k prompt)
    /// - **Aliases**: `grok-4.5-latest`, `grok-build-latest`
    ///
    /// Prefer [`Model::Grok4_7`] for new work.
    Grok4_5,

    // ── Grok 5 (PLACEHOLDER — not yet released) ────────────────────────────────
    /// **Placeholder** for the upcoming Grok 5 model family.
    ///
    /// ⚠️ **All capability values are speculative** and will be updated when xAI
    /// officially announces Grok 5. **Do not use in production** until the real
    /// specs are confirmed and this notice is removed.
    ///
    /// ### Assumed API string
    /// `"grok-5"` (best guess — subject to change).
    ///
    /// ### Current placeholder values
    /// | Method | Placeholder value | Rationale |
    /// |---|---|---|
    /// | `context_window()` | `Some(1_000_000)` | conservative floor based on 4.3 |
    /// | `is_reasoning_model()` | `false` | assumed flexible like 4.3 |
    /// | `supports_reasoning_effort()` | `true` | assumed continues 4.3 pattern |
    /// | `supports_logprobs()` | `false` | assumed new-gen behaviour |
    ///
    /// ### When Grok 5 drops
    /// 1. Update `as_str()` with the confirmed API string.
    /// 2. Verify and correct every capability method below.
    /// 3. Check whether new `ChatRequest` fields are needed.
    /// 4. Add `#[deprecated]` to any models being retired.
    /// 5. Remove this warning block.
    /// 6. Bump crate version and publish.
    #[deprecated(
        since = "0.2.1",
        note = "Grok 5 has not been officially released. This variant is a speculative placeholder with unconfirmed API strings and capabilities. Do not use in production."
    )]
    Grok5,

    // ── Grok 4.3 (1 M context, May 2026) ──────────────────────────────────────
    /// Grok 4.3 — 1 M token context window; configurable reasoning effort
    /// (`low` / `medium` / `high`; `"none"` to disable).
    ///
    /// Good choice when you need the full 1 M context window or EU availability.
    /// For everything else, prefer [`Model::Grok4_7`].
    Grok4_3,

    // ── Grok Build 0.1 (agentic coding, 256 K context, May 2026) ───────────────────
    /// Grok Build 0.1 — fast agentic coding model; 256 K token context window;
    /// accepts text + image input; built-in reasoning (no configurable effort).
    ///
    /// **Recommended default for code generation, agentic coding, and multi-step
    /// development tasks.** Successor to the retired `grok-code-fast-1`.
    ///
    /// Aliases recognised by [`Model::parse`]: `grok-code-fast-1`, `grok-code-fast`.
    GrokBuild0_1,

    // ── Grok 4.20 (flagship, 2M context) ──────────────────────────────────────
    /// Grok 4.20 Reasoning (0309) — flagship reasoning model, 2M context window
    Grok4_20_0309Reasoning,

    /// Grok 4.20 Non-Reasoning — stable alias for the 4.20 fast standard model,
    /// 2M context window. Recommended replacement for `grok-4-1-fast-non-reasoning`.
    Grok4_20NonReasoning,

    /// Grok 4.20 Non-Reasoning (0309) — dated variant of the flagship standard model, 2M context window
    Grok4_20_0309NonReasoning,

    /// Grok 4.20 Multi-Agent (0309) — optimised for agentic tool-calling pipelines
    Grok4_20MultiAgent0309,

    // ── Grok 4.1 Fast (DEPRECATED — retires 2026-05-15) ───────────────────────
    /// ⚠️ **Deprecated** — retires 2026-05-15. Use [`Model::Grok4_3`] instead.
    #[deprecated(
        since = "0.1.7",
        note = "Retires 2026-05-15. Migrate to Model::Grok4_3 (grok-4.3)."
    )]
    Grok4_1FastReasoning,

    /// ⚠️ **Deprecated** — retires 2026-05-15. Use [`Model::Grok4_20NonReasoning`] instead.
    #[deprecated(
        since = "0.1.7",
        note = "Retires 2026-05-15. Migrate to Model::Grok4_20NonReasoning (grok-4.20-non-reasoning)."
    )]
    Grok4_1FastNonReasoning,

    // ── Grok 4 (DEPRECATED — retires 2026-05-15) ──────────────────────────────
    /// ⚠️ **Deprecated** — retires 2026-05-15. Use [`Model::Grok4_3`] instead.
    #[deprecated(
        since = "0.1.7",
        note = "Retires 2026-05-15. Migrate to Model::Grok4_3 (grok-4.3)."
    )]
    Grok4_0709,

    // ── Grok 3 ────────────────────────────────────────────────────────────────
    /// ⚠️ **Deprecated** — retires 2026-05-15. Use [`Model::Grok4_3`] instead.
    #[deprecated(
        since = "0.1.7",
        note = "Retires 2026-05-15. Migrate to Model::Grok4_3 (grok-4.3)."
    )]
    Grok3,

    /// Grok 3 Mini — efficient smaller model (not retiring May 2026)
    Grok3Mini,

    // ── Code (DEPRECATED — retires 2026-05-15) ────────────────────────────────
    /// ⚠️ **Deprecated** — retires 2026-05-15. Use [`Model::Grok4_3`] instead.
    #[deprecated(
        since = "0.1.7",
        note = "Retires 2026-05-15. Migrate to Model::GrokBuild0_1 (grok-build-0.1) for coding workloads."
    )]
    GrokCodeFast1,

    // ── Image generation ──────────────────────────────────────────────────────
    /// ⚠️ **Deprecated** — retires 2026-05-15. Use [`Model::GrokImagineImage`] instead.
    #[deprecated(
        since = "0.1.7",
        note = "Retires 2026-05-15. Migrate to Model::GrokImagineImage (grok-imagine-image)."
    )]
    GrokImagineImagePro,

    /// Grok Imagine Image — standard image generation model
    GrokImagineImage,

    // ── Video generation ──────────────────────────────────────────────────────
    /// Grok Imagine Video — video generation model
    GrokImagineVideo,

    // ── Voice (real-time Speech-to-Speech via WebSocket) ──────────────────────
    /// Grok Voice Think Fast 2.0 — current flagship real-time voice model.
    ///
    /// Connect via `wss://api.x.ai/v1/realtime?model=grok-voice-think-fast-2.0`.
    /// NOT usable with the chat completions API or [`crate::client::GrokClient`].
    GrokVoiceThinkFast2_0,

    /// Grok Voice Think Fast 1.0 — previous-generation real-time voice model.
    ///
    /// Connect via `wss://api.x.ai/v1/realtime?model=grok-voice-think-fast-1.0`.
    GrokVoiceThinkFast1_0,

    /// `grok-voice-latest` — rolling alias; currently points to
    /// [`Model::GrokVoiceThinkFast2_0`]. Will update to the next flagship on
    /// **August 5, 2026**.
    ///
    /// Use a pinned variant (`GrokVoiceThinkFast2_0`) in production for stability.
    GrokVoiceLatest,
}

#[allow(deprecated)]
impl Model {
    /// Get the model identifier string used in API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            // Grok 4.7 (current flagship, August 2026)
            Model::Grok4_7 => "grok-4.7",
            // Grok 4.6
            Model::Grok4_6 => "grok-4.6",
            // Grok 4.5
            Model::Grok4_5 => "grok-4.5",
            // Grok 5 (placeholder — API string not yet confirmed)
            Model::Grok5 => "grok-5",
            // Grok 4.3
            Model::Grok4_3 => "grok-4.3",
            // Grok Build 0.1
            Model::GrokBuild0_1 => "grok-build-0.1",
            // Grok 4.20
            Model::Grok4_20_0309Reasoning => "grok-4.20-0309-reasoning",
            Model::Grok4_20NonReasoning => "grok-4.20-non-reasoning",
            Model::Grok4_20_0309NonReasoning => "grok-4.20-0309-non-reasoning",
            Model::Grok4_20MultiAgent0309 => "grok-4.20-multi-agent-0309",
            // Grok 4.1 Fast (deprecated)
            Model::Grok4_1FastReasoning => "grok-4-1-fast-reasoning",
            Model::Grok4_1FastNonReasoning => "grok-4-1-fast-non-reasoning",
            // Grok 4 (deprecated)
            Model::Grok4_0709 => "grok-4-0709",
            // Grok 3
            Model::Grok3 => "grok-3",
            Model::Grok3Mini => "grok-3-mini",
            // Code (deprecated)
            Model::GrokCodeFast1 => "grok-code-fast-1",
            // Image generation
            Model::GrokImagineImagePro => "grok-imagine-image-pro",
            Model::GrokImagineImage => "grok-imagine-image",
            // Video generation
            Model::GrokImagineVideo => "grok-imagine-video",
            // Voice (real-time Speech-to-Speech)
            Model::GrokVoiceThinkFast2_0 => "grok-voice-think-fast-2.0",
            Model::GrokVoiceThinkFast1_0 => "grok-voice-think-fast-1.0",
            Model::GrokVoiceLatest => "grok-voice-latest",
        }
    }

    /// Parse a model from its API identifier string.
    ///
    /// Returns `None` for unknown strings. Deprecated model strings are still
    /// recognised so that existing config files and env vars continue to work
    /// until the xAI API retires them on 2026-05-15.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            // Grok 4.7 (+ its rolling aliases)
            "grok-4.7" | "grok-4.7-latest" => Some(Model::Grok4_7),
            // Grok 4.6 (+ its rolling aliases)
            "grok-4.6" | "grok-4.6-latest" => Some(Model::Grok4_6),
            // Grok 4.5 (+ its rolling aliases)
            "grok-4.5" | "grok-4.5-latest" | "grok-build-latest" => Some(Model::Grok4_5),
            // Grok 5 (placeholder — recognise the most likely API string)
            "grok-5" => Some(Model::Grok5),
            // Grok 4.3
            "grok-4.3" => Some(Model::Grok4_3),
            // Grok Build 0.1 (+ its legacy aliases)
            "grok-build-0.1" | "grok-code-fast" | "grok-code-fast-1-0825" => {
                Some(Model::GrokBuild0_1)
            }
            // Grok 4.20
            "grok-4.20-0309-reasoning" => Some(Model::Grok4_20_0309Reasoning),
            "grok-4.20-non-reasoning" => Some(Model::Grok4_20NonReasoning),
            "grok-4.20-0309-non-reasoning" => Some(Model::Grok4_20_0309NonReasoning),
            "grok-4.20-multi-agent-0309" => Some(Model::Grok4_20MultiAgent0309),
            // Grok 4.1 Fast (deprecated — recognised until 2026-05-15)
            #[allow(deprecated)]
            "grok-4-1-fast-reasoning" => Some(Model::Grok4_1FastReasoning),
            #[allow(deprecated)]
            "grok-4-1-fast-non-reasoning" => Some(Model::Grok4_1FastNonReasoning),
            // Grok 4 (deprecated — recognised until 2026-05-15)
            #[allow(deprecated)]
            "grok-4-0709" => Some(Model::Grok4_0709),
            // Grok 3
            #[allow(deprecated)]
            "grok-3" => Some(Model::Grok3),
            "grok-3-mini" => Some(Model::Grok3Mini),
            // Code (deprecated — recognised until 2026-05-15)
            #[allow(deprecated)]
            "grok-code-fast-1" => Some(Model::GrokCodeFast1),
            // Image generation
            #[allow(deprecated)]
            "grok-imagine-image-pro" => Some(Model::GrokImagineImagePro),
            "grok-imagine-image" => Some(Model::GrokImagineImage),
            // Video generation
            "grok-imagine-video" => Some(Model::GrokImagineVideo),
            // Voice (real-time Speech-to-Speech)
            "grok-voice-think-fast-2.0" => Some(Model::GrokVoiceThinkFast2_0),
            "grok-voice-think-fast-1.0" => Some(Model::GrokVoiceThinkFast1_0),
            "grok-voice-latest" => Some(Model::GrokVoiceLatest),
            _ => None,
        }
    }

    /// All **active, released** models, ordered newest → oldest.
    ///
    /// Returns a `'static` slice — no heap allocation on each call.
    /// Deprecated and unreleased variants are excluded so that UI lists,
    /// help text, and auto-complete only surface models that actually work.
    pub fn all() -> &'static [Self] {
        &[
            // Grok 4.7 — current flagship (August 2026)
            Model::Grok4_7,
            // Grok 4.6 — previous flagship
            Model::Grok4_6,
            // Grok 4.5
            Model::Grok4_5,
            // Grok 4.3 (1 M context, EU-available)
            Model::Grok4_3,
            // Grok Build 0.1 — agentic coding
            Model::GrokBuild0_1,
            // Grok 4.20 — active variants
            Model::Grok4_20_0309Reasoning,
            Model::Grok4_20NonReasoning,
            Model::Grok4_20_0309NonReasoning,
            Model::Grok4_20MultiAgent0309,
            // Grok 3 Mini (Grok3 itself is deprecated)
            Model::Grok3Mini,
            // Image generation (GrokImagineImagePro is deprecated)
            Model::GrokImagineImage,
            // Video generation
            Model::GrokImagineVideo,
            // Voice (real-time Speech-to-Speech)
            Model::GrokVoiceThinkFast2_0,
            Model::GrokVoiceThinkFast1_0,
            Model::GrokVoiceLatest,
        ]
    }

    /// All models including deprecated ones — useful for migration tooling
    /// or exhaustive API-string lookup tables.
    ///
    /// Returns a `'static` slice — no heap allocation on each call.
    #[allow(deprecated)]
    pub fn all_including_deprecated() -> &'static [Self] {
        &[
            // Grok 4.7 (current flagship, August 2026)
            Model::Grok4_7,
            // Grok 4.6
            Model::Grok4_6,
            // Grok 4.5
            Model::Grok4_5,
            // Grok 5 (placeholder)
            Model::Grok5,
            // Grok 4.3
            Model::Grok4_3,
            // Grok Build 0.1
            Model::GrokBuild0_1,
            // Grok 4.20
            Model::Grok4_20_0309Reasoning,
            Model::Grok4_20NonReasoning,
            Model::Grok4_20_0309NonReasoning,
            Model::Grok4_20MultiAgent0309,
            // Grok 4.1 Fast (deprecated)
            Model::Grok4_1FastReasoning,
            Model::Grok4_1FastNonReasoning,
            // Grok 4 (deprecated)
            Model::Grok4_0709,
            // Grok 3
            Model::Grok3,
            Model::Grok3Mini,
            // Code (deprecated)
            Model::GrokCodeFast1,
            // Image generation
            Model::GrokImagineImagePro,
            Model::GrokImagineImage,
            // Video generation
            Model::GrokImagineVideo,
            // Voice
            Model::GrokVoiceThinkFast2_0,
            Model::GrokVoiceThinkFast1_0,
            Model::GrokVoiceLatest,
        ]
    }

    /// Returns `true` if this model is a **pure reasoning model** that does
    /// **not** support `presence_penalty`, `frequency_penalty`, `stop`, or
    /// `reasoning_effort` parameters. Sending those fields will cause an API error.
    ///
    /// Note: [`Model::Grok4_3`] is **not** a pure reasoning model — it
    /// supports the `reasoning_effort` parameter (low / medium / high) but
    /// also works without it.
    pub fn is_reasoning_model(&self) -> bool {
        matches!(
            self,
            // Note: GrokBuild0_1 has built-in reasoning but is NOT listed here
            // because it behaves like a standard model for API parameters
            // (presence/frequency penalty are accepted).
            Model::Grok4_20_0309Reasoning
                | Model::Grok4_20MultiAgent0309
                | Model::Grok4_1FastReasoning
                | Model::Grok4_0709
        )
    }

    /// Returns `true` if this model supports `presence_penalty` and
    /// `frequency_penalty` request fields.
    ///
    /// Pure reasoning models (Grok 4.20 reasoning variants, Grok 4-0709)
    /// do not support these parameters.
    pub fn supports_frequency_presence_penalty(&self) -> bool {
        !self.is_reasoning_model()
    }

    /// Returns `true` if this model supports the `reasoning_effort` request
    /// field.
    ///
    /// Valid effort values for these models:
    /// - `"none"` — disable reasoning entirely (zero reasoning_tokens)
    /// - `"low"` — small reasoning_token budget
    /// - `"medium"` — moderate reasoning_token budget
    /// - `"high"` — large reasoning_token budget (API default when set)
    /// - `"xhigh"` — **Grok 4.6 and 4.7**; maximum reasoning_token budget
    ///
    /// Pure-reasoning models (e.g. `grok-4.20-0309-reasoning`) do **not**
    /// accept `reasoning_effort`; sending it to them returns an API error.
    ///
    /// Use the typed [`crate::ReasoningDepth`] enum with the builder instead
    /// of raw strings when possible.
    pub fn supports_reasoning_effort(&self) -> bool {
        matches!(
            self,
            Model::Grok4_7 | Model::Grok4_6 | Model::Grok4_5 | Model::Grok4_3
        )
    }

    /// Returns `true` if this model supports the `logprobs` response field.
    ///
    /// Grok 4.20 family and Grok 4.3 do not support `logprobs`.
    pub fn supports_logprobs(&self) -> bool {
        // Grok5: assumed false (new-gen, likely no logprobs) — UPDATE WHEN CONFIRMED
        !matches!(
            self,
            Model::Grok4_7
                | Model::Grok4_6
                | Model::Grok4_5
                | Model::Grok5
                | Model::Grok4_3
                | Model::GrokBuild0_1
                | Model::Grok4_20_0309Reasoning
                | Model::Grok4_20NonReasoning
                | Model::Grok4_20_0309NonReasoning
                | Model::Grok4_20MultiAgent0309
        )
    }

    /// Returns the context window size in tokens, if known.
    ///
    /// Returns `None` for image/video generation models which do not use a
    /// token-based context window.
    pub fn context_window(&self) -> Option<u32> {
        match self {
            // Current flagships (500 K context)
            Model::Grok4_7 | Model::Grok4_6 | Model::Grok4_5 => Some(500_000),

            // Grok 5 placeholder — conservative 1 M floor (update when real specs are released)
            Model::Grok5 => Some(1_000_000),

            // Grok 4.3 (1 M context)
            Model::Grok4_3 => Some(1_000_000),

            // Grok Build 0.1 (agentic coding)
            Model::GrokBuild0_1 => Some(256_000),

            // Grok 4.20 family (2 M context)
            Model::Grok4_20_0309Reasoning
            | Model::Grok4_20NonReasoning
            | Model::Grok4_20_0309NonReasoning
            | Model::Grok4_20MultiAgent0309 => Some(2_000_000),

            // Deprecated 4.x / 4.1 models that still report 2 M in some docs (keep for compatibility)
            #[allow(deprecated)]
            Model::Grok4_1FastReasoning | Model::Grok4_1FastNonReasoning | Model::Grok4_0709 => {
                Some(2_000_000)
            }

            // Grok 3 family
            Model::Grok3 | Model::Grok3Mini => Some(131_072),

            // Legacy code model
            #[allow(deprecated)]
            Model::GrokCodeFast1 => Some(131_072),

            // Image / video / voice models have no token context window
            Model::GrokImagineImagePro
            | Model::GrokImagineImage
            | Model::GrokImagineVideo
            | Model::GrokVoiceThinkFast2_0
            | Model::GrokVoiceThinkFast1_0
            | Model::GrokVoiceLatest => None,
        }
    }

    /// Returns `true` if this is a text / language model (chat completions API).
    ///
    /// Returns `false` for image, video, **and voice** generation models, which
    /// use separate API endpoints and cannot be used with [`crate::client::GrokClient`].
    pub fn is_language_model(&self) -> bool {
        !matches!(
            self,
            Model::GrokImagineImagePro
                | Model::GrokImagineImage
                | Model::GrokImagineVideo
                | Model::GrokVoiceThinkFast2_0
                | Model::GrokVoiceThinkFast1_0
                | Model::GrokVoiceLatest
        )
    }

    /// Returns `true` if this model can generate images.
    pub fn is_image_model(&self) -> bool {
        matches!(self, Model::GrokImagineImagePro | Model::GrokImagineImage)
    }

    /// Returns `true` if this model can generate video.
    pub fn is_video_model(&self) -> bool {
        matches!(self, Model::GrokImagineVideo)
    }

    /// Returns `true` if this is a real-time **voice** model (Speech-to-Speech).
    ///
    /// Voice models connect via WebSocket (`wss://api.x.ai/v1/realtime?model=...`)
    /// and are **not** usable with [`crate::client::GrokClient`] or the chat
    /// completions API. Use the variant’s [`Model::as_str`] to get the model name
    /// for the WebSocket query parameter.
    pub fn is_voice_model(&self) -> bool {
        matches!(
            self,
            Model::GrokVoiceThinkFast2_0 | Model::GrokVoiceThinkFast1_0 | Model::GrokVoiceLatest
        )
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<Model> for String {
    fn from(model: Model) -> Self {
        model.as_str().to_string()
    }
}

#[cfg(test)]
mod tests {
    // ... (rest of the tests remain unchanged)
}
