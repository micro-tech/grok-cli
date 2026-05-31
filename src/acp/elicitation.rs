//! Elicitation support using official agent-client-protocol types (Task 111.6).
//!
//! This module provides the types and handler for the ACP `session/elicit`
//! (or `elicitation`) flow using the official schema where possible.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Official-style Elicitation Request (matches agent_client_protocol schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationRequest {
    pub session_id: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>, // JSON Schema for structured input
}

/// Official-style Elicitation Response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationResponse {
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
}

impl ElicitationResponse {
    pub fn cancelled() -> Self {
        Self {
            cancelled: true,
            content: None,
        }
    }

    pub fn with_content(content: Value) -> Self {
        Self {
            cancelled: false,
            content: Some(content),
        }
    }
}
