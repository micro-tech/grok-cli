//! CoT (Chain-of-Thought) / Thinking Trace Guard
//!
//! **STRICT POLICY (radioactive isotope rule):**
//!
//! - Chain-of-thought / `reasoning_content` / `thinking_content` is **NEVER** stored.
//! - It is **NEVER** fed back into future prompts or conversation history.
//! - It is **NEVER** put into memory, context layers, session state, or logs that go to the model.
//! - It may **ONLY** be used for immediate one-shot UI display (e.g. ThinkingBlockUpdate) or local debug, then **discarded**.
//!
//! Every place that receives a response containing `thinking_content` or `reasoning_content`
//! MUST strip it before the message is added to any history that will be sent to the LLM again.

use serde_json::Value;

/// Strip any `reasoning_content` field from a JSON message object (in place).
/// Safe to call on any message Value.
#[inline]
pub fn strip_reasoning_content(msg: &mut Value) {
    if let Some(obj) = msg.as_object_mut() {
        obj.remove("reasoning_content");
        // Also strip nested assistant message if present
        if let Some(content) = obj.get_mut("message") {
            strip_reasoning_content(content);
        }
    }
}

/// Create a clean copy of an assistant message Value with CoT removed.
/// Use this before pushing any assistant response into conversation history.
#[inline]
pub fn clean_assistant_message(mut msg: Value) -> Value {
    strip_reasoning_content(&mut msg);
    msg
}

/// Returns true if the value contains a non-empty reasoning/thinking trace.
#[inline]
pub fn contains_reasoning_trace(msg: &Value) -> bool {
    if let Some(obj) = msg.as_object() {
        if let Some(rc) = obj.get("reasoning_content").and_then(|v| v.as_str()) {
            return !rc.trim().is_empty();
        }
        if let Some(msg_obj) = obj.get("message").and_then(|v| v.as_object()) {
            if let Some(rc) = msg_obj.get("reasoning_content").and_then(|v| v.as_str()) {
                return !rc.trim().is_empty();
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_reasoning_content() {
        let mut msg = json!({
            "role": "assistant",
            "content": "hello",
            "reasoning_content": "secret cot here"
        });
        strip_reasoning_content(&mut msg);
        assert!(msg.get("reasoning_content").is_none());
        assert_eq!(msg["content"], "hello");
    }

    #[test]
    fn clean_assistant_removes_cot() {
        let raw = json!({
            "role": "assistant",
            "content": "answer",
            "reasoning_content": "long thinking..."
        });
        let clean = clean_assistant_message(raw);
        assert!(clean.get("reasoning_content").is_none());
    }

    #[test]
    fn detects_cot() {
        let with_cot = json!({"reasoning_content": "foo"});
        let without = json!({"content": "bar"});
        assert!(contains_reasoning_trace(&with_cot));
        assert!(!contains_reasoning_trace(&without));
    }
}
