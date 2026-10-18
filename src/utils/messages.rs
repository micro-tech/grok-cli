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

/// Create a tool result message, tail-truncating `content` to `max_chars`
/// (Task 465).
///
/// This is the single point where tool results are appended to message
/// histories: every agent loop (ACP, CLI, interactive, HOH, explorer)
/// routes through here, so a large tool output can never dump unbounded
/// text into context.  The tail is kept because compiler/test errors live
/// at the end of the output.
#[inline]
pub fn tool_result_capped(
    tool_call_id: &str,
    content: impl Into<String>,
    max_chars: usize,
) -> Value {
    let content = content.into();
    // A cap of 0 disables truncation (matches the old per-loop sweep behaviour).
    let content = if max_chars == 0 {
        content
    } else {
        crate::acp::context_trim::truncate_tool_content(&content, max_chars)
    };
    tool_result(tool_call_id, content)
}

/// Create a tool result message (common in tool-using turns).
/// Prefer [`tool_result_capped`] so large outputs are truncated at the
/// append point (Task 465).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_capped_truncates_tail_at_append_point() {
        // Task 465: the single append point must truncate large tool results
        // (tail) so no loop can dump unbounded output into context.
        let big = "x".repeat(10_000) + "TAIL_MARKER";
        let msg = tool_result_capped("call-1", big, 1000);
        assert_eq!(msg.get("role").and_then(|r| r.as_str()), Some("tool"));
        assert_eq!(
            msg.get("tool_call_id").and_then(|i| i.as_str()),
            Some("call-1")
        );
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap();
        assert!(content.len() < 1100, "must be truncated, len={}", content.len());
        assert!(content.contains("TAIL_MARKER"), "tail must be kept");
        assert!(content.contains("earlier output truncated"));
    }

    #[test]
    fn test_tool_result_capped_zero_disables() {
        let big = "y".repeat(5000);
        let msg = tool_result_capped("call-2", big.clone(), 0);
        assert_eq!(msg.get("content").and_then(|c| c.as_str()), Some(big.as_str()));
    }

    #[test]
    fn test_tool_result_capped_small_passthrough() {
        let msg = tool_result_capped("call-3", "small", 1000);
        assert_eq!(
            msg.get("content").and_then(|c| c.as_str()),
            Some("small")
        );
    }
}
