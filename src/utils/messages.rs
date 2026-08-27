//! Cheap message construction helpers (Task 267).
//!
//! Goal: Reduce repeated allocations from `json!({ "role": "...", "content": ... })`
//! in the per-turn hot paths.
//!
//! These helpers use small pre-sized maps and avoid some macro expansion overhead.

use serde_json::{Map, Value};

/// Create a minimal "user" message.
#[inline]
pub fn user(content: impl Into<String>) -> Value {
    let mut m = Map::with_capacity(2);
    m.insert("role".to_string(), Value::String("user".to_string()));
    m.insert("content".to_string(), Value::String(content.into()));
    Value::Object(m)
}

/// Create a minimal "assistant" message.
#[inline]
pub fn assistant(content: impl Into<String>) -> Value {
    let mut m = Map::with_capacity(2);
    m.insert("role".to_string(), Value::String("assistant".to_string()));
    m.insert("content".to_string(), Value::String(content.into()));
    Value::Object(m)
}

/// Create a minimal "system" message.
#[inline]
pub fn system(content: impl Into<String>) -> Value {
    let mut m = Map::with_capacity(2);
    m.insert("role".to_string(), Value::String("system".to_string()));
    m.insert("content".to_string(), Value::String(content.into()));
    Value::Object(m)
}

/// Create a tool result message (common in tool-using turns).
#[inline]
pub fn tool_result(tool_call_id: &str, content: impl Into<String>) -> Value {
    let mut m = Map::with_capacity(3);
    m.insert("role".to_string(), Value::String("tool".to_string()));
    m.insert(
        "tool_call_id".to_string(),
        Value::String(tool_call_id.to_string()),
    );
    m.insert("content".to_string(), Value::String(content.into()));
    Value::Object(m)
}

/// Create an assistant message that includes tool_calls (for function calling).
pub fn assistant_with_tool_calls(
    content: Option<String>,
    tool_calls: Vec<serde_json::Value>,
) -> Value {
    let mut m = Map::with_capacity(3);
    m.insert("role".to_string(), Value::String("assistant".to_string()));
    if let Some(c) = content {
        m.insert("content".to_string(), Value::String(c));
    } else {
        m.insert("content".to_string(), Value::Null);
    }
    m.insert("tool_calls".to_string(), Value::Array(tool_calls));
    Value::Object(m)
}
