//! Common types used across the API

use serde::{Deserialize, Serialize};

/// Role of a message in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message (instructions for the model)
    System,

    /// User message
    User,

    /// Assistant/model response
    Assistant,

    /// Tool/function response
    Tool,
}

impl Role {
    /// Convert role to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<Role> for String {
    fn from(role: Role) -> Self {
        role.as_str().to_string()
    }
}

/// Reason why a completion finished
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Natural completion
    Stop,

    /// Maximum tokens reached
    Length,

    /// Tool/function call
    ToolCalls,

    /// Content filtered
    ContentFilter,

    /// Other/unknown reason
    #[serde(other)]
    Other,
}

impl StopReason {
    /// Check if completion finished naturally
    pub fn is_complete(&self) -> bool {
        matches!(self, StopReason::Stop)
    }

    /// Check if more tokens were needed
    pub fn needs_more_tokens(&self) -> bool {
        matches!(self, StopReason::Length)
    }

    /// Check if tool calls were made
    pub fn has_tool_calls(&self) -> bool {
        matches!(self, StopReason::ToolCalls)
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReason::Stop => write!(f, "stop"),
            StopReason::Length => write!(f, "length"),
            StopReason::ToolCalls => write!(f, "tool_calls"),
            StopReason::ContentFilter => write!(f, "content_filter"),
            StopReason::Other => write!(f, "other"),
        }
    }
}

/// Controls the depth of internal reasoning for capable models.
///
/// Maps directly to the `reasoning_effort` API field sent to xAI:
///
/// | Variant  | API value  | Effect                                                   |
/// |----------|------------|----------------------------------------------------------|
/// | `None`   | `"none"`   | No reasoning; fastest & cheapest; zero reasoning tokens  |
/// | `Low`    | `"low"`    | Light reasoning; good for latency-sensitive tasks        |
/// | `Medium` | `"medium"` | Balanced; good for complex data analysis                 |
/// | `High`   | `"high"`   | Deep reasoning (API default when set); best quality      |
/// | `XHigh`  | `"xhigh"`  | Deepest level; Grok 4.6+ (incl. 4.7); error on older models |
///
/// ## Billing reality
///
/// `reasoning_tokens` are billed as output tokens **regardless** of whether
/// `reasoning_content` is returned in the response.  The model computes its
/// chain-of-thought, you pay for it — hiding the text does not zero that out.
///
/// The think_* builder methods ([`crate::client::ChatRequestBuilder::think`],
/// `think_low`, `think_medium`, `think_deep`, `think_xhigh`) strip
/// `reasoning_content` from the HTTP response.  This keeps CoT **off the
/// wire** and **out of your message history** (which is the real saving —
/// echoing CoT back as context in follow-up requests costs prompt tokens).
///
/// **To actually reduce the reasoning bill:**
/// - Use `Low` instead of `High` / `XHigh` — fewer reasoning_tokens consumed
/// - Use `None` (or a non-reasoning model) — zero reasoning_tokens
/// - Never echo `reasoning_content` back into subsequent request messages
///
/// Use [`crate::client::ChatRequestBuilder::with_reasoning`] when you *do*
/// want to read or log the chain-of-thought.
///
/// ## Example
///
/// ```no_run
/// use grok_api::{GrokClient, ChatMessage, ReasoningDepth, Result};
///
/// # async fn example() -> Result<()> {
/// let client = GrokClient::new("your-api-key")?;
/// let response = client
///     .chat_with_history(&[ChatMessage::user("Prove Fermat's Last Theorem in outline.")])
///     .model("grok-4.7")
///     .think_deep()           // reasoning_effort: "high", no chain-of-thought in response
///     .send()
///     .await?;
/// println!("{}", response.content().unwrap_or(""));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningDepth {
    /// Disable reasoning entirely — zero reasoning_tokens, fastest response.
    ///
    /// Supported on models that accept `reasoning_effort` (e.g. grok-4.3,
    /// grok-4.5, grok-4.6, grok-4.7).  Returns an API error on pure-reasoning models
    /// such as `grok-4.20-0309-reasoning`.
    None,
    /// Light reasoning — small reasoning_token budget; best for latency-sensitive tasks.
    Low,
    /// Balanced reasoning — moderate reasoning_token budget; good for complex analysis.
    Medium,
    /// Deep reasoning — larger reasoning_token budget; API default when `reasoning_effort`
    /// is set; best answer quality.
    High,
    /// Deepest reasoning — largest reasoning_token budget; Grok 4.6 and 4.7.
    /// Returns an API error on older models.
    #[serde(rename = "xhigh")]
    XHigh,
}

impl ReasoningDepth {
    /// Return the API string for this depth level.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None   => "none",
            Self::Low    => "low",
            Self::Medium => "medium",
            Self::High   => "high",
            Self::XHigh  => "xhigh",
        }
    }
}

impl std::fmt::Display for ReasoningDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ReasoningDepth {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none"   => Ok(Self::None),
            "low"    => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high"   => Ok(Self::High),
            "xhigh"  => Ok(Self::XHigh),
            other    => Err(format!("unknown reasoning depth: {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_conversion() {
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");

        let role_string: String = Role::User.into();
        assert_eq!(role_string, "user");
    }

    #[test]
    fn test_stop_reason_checks() {
        assert!(StopReason::Stop.is_complete());
        assert!(!StopReason::Length.is_complete());

        assert!(StopReason::Length.needs_more_tokens());
        assert!(!StopReason::Stop.needs_more_tokens());

        assert!(StopReason::ToolCalls.has_tool_calls());
        assert!(!StopReason::Stop.has_tool_calls());
    }

    #[test]
    fn test_role_serialization() {
        let json = serde_json::to_string(&Role::System).unwrap();
        assert_eq!(json, r#""system""#);

        let role: Role = serde_json::from_str(r#""user""#).unwrap();
        assert_eq!(role, Role::User);
    }

    #[test]
    fn test_reasoning_depth_as_str() {
        assert_eq!(ReasoningDepth::None.as_str(), "none");
        assert_eq!(ReasoningDepth::Low.as_str(), "low");
        assert_eq!(ReasoningDepth::Medium.as_str(), "medium");
        assert_eq!(ReasoningDepth::High.as_str(), "high");
        assert_eq!(ReasoningDepth::XHigh.as_str(), "xhigh");
    }

    #[test]
    fn test_reasoning_depth_from_str() {
        use std::str::FromStr;
        assert_eq!(ReasoningDepth::from_str("none").unwrap(), ReasoningDepth::None);
        assert_eq!(ReasoningDepth::from_str("low").unwrap(), ReasoningDepth::Low);
        assert_eq!(ReasoningDepth::from_str("medium").unwrap(), ReasoningDepth::Medium);
        assert_eq!(ReasoningDepth::from_str("high").unwrap(), ReasoningDepth::High);
        assert_eq!(ReasoningDepth::from_str("xhigh").unwrap(), ReasoningDepth::XHigh);
        assert!(ReasoningDepth::from_str("ultra").is_err());
    }

    #[test]
    fn test_reasoning_depth_display() {
        assert_eq!(format!("{}", ReasoningDepth::High), "high");
        assert_eq!(format!("{}", ReasoningDepth::XHigh), "xhigh");
    }

    #[test]
    fn test_reasoning_depth_serialization() {
        let json = serde_json::to_string(&ReasoningDepth::XHigh).unwrap();
        assert_eq!(json, r#""xhigh""#);

        let depth: ReasoningDepth = serde_json::from_str(r#""medium""#).unwrap();
        assert_eq!(depth, ReasoningDepth::Medium);
    }
}
