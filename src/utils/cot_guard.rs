//! CoT (Chain-of-Thought) / Thinking Trace Guard
//!
//! **STRICT PROJECT RULE (MONEY SAVER - radioactive isotope policy):**
//!
//! - Chain-of-thought / `reasoning_content` / `thinking_content` is **NEVER** stored.
//! - It is **NEVER** fed back into future prompts or conversation history.
//! - It is **NEVER** put into memory, context layers, session state, or logs that go to the model.
//! - It may **ONLY** be used for immediate one-shot UI display (e.g. ThinkingBlockUpdate) or local debug, then **discarded**.
//!
//! **Every place** that receives a response containing `thinking_content` or `reasoning_content`
//! MUST strip it before the message is added to any history that will be sent to the LLM again.
//!
//! This rule exists to avoid wasting tokens and real money on sending internal reasoning traces
//! back to Grok/xAI on every turn.

use serde_json::Value;

/// Strip any `reasoning_content` or `thinking_content` field from a JSON message object (in place).
/// Safe to call on any message Value.
#[inline]
pub fn strip_reasoning_content(msg: &mut Value) {
    if let Some(obj) = msg.as_object_mut() {
        obj.remove("reasoning_content");
        obj.remove("thinking_content");

        // Also strip nested assistant message if present
        if let Some(content) = obj.get_mut("message") {
            strip_reasoning_content(content);
        }

        // Check inside "content" array (some formats)
        if let Some(content_arr) = obj.get_mut("content").and_then(|c| c.as_array_mut()) {
            for item in content_arr {
                if let Some(item_obj) = item.as_object_mut() {
                    item_obj.remove("reasoning_content");
                    item_obj.remove("thinking_content");
                }
            }
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
            if !rc.trim().is_empty() { return true; }
        }
        if let Some(tc) = obj.get("thinking_content").and_then(|v| v.as_str()) {
            if !tc.trim().is_empty() { return true; }
        }
        if let Some(msg_obj) = obj.get("message").and_then(|v| v.as_object()) {
            if let Some(rc) = msg_obj.get("reasoning_content").and_then(|v| v.as_str()) {
                if !rc.trim().is_empty() { return true; }
            }
            if let Some(tc) = msg_obj.get("thinking_content").and_then(|v| v.as_str()) {
                if !tc.trim().is_empty() { return true; }
            }
        }
    }
    false
}

/// **Strong runtime guard (debug builds only).**
///
/// In `debug` / `dev` builds (`cfg(debug_assertions)`), this **panics** if any message
/// in the slice still contains CoT / thinking_content.
///
/// In release builds this is a complete no-op (zero cost).
///
/// Use this as a safety net right before sending messages to the LLM.
///
/// Example:
/// ```ignore
/// let clean_history: Vec<_> = history.iter().map(|m| clean_assistant_message(m.clone())).collect();
/// debug_assert_no_cot_in_messages(&clean_history);
/// router.chat_completion_with_history(&clean_history, ...).await?;
/// ```
#[inline]
pub fn debug_assert_no_cot_in_messages(_messages: &[Value]) {
    #[cfg(debug_assertions)]
    {
        for (i, m) in messages.iter().enumerate() {
            if contains_reasoning_trace(m) {
                panic!(
                    "🚨 CoT LEAK DETECTED (debug guard) — message index {} still contains reasoning_content/thinking_content!\n\
                     This would have been sent to the LLM and wasted money.\n\
                     Message: {}\n\n\
                     Always call clean_assistant_message() (or strip_reasoning_content) before pushing to history.",
                    i,
                    serde_json::to_string_pretty(m).unwrap_or_default()
                );
            }
        }
    }
}

/// Convenience: clean the message **and** run the debug assertion on it.
/// Recommended for push sites in dev.
#[inline]
pub fn clean_and_assert_no_cot(mut msg: Value) -> Value {
    strip_reasoning_content(&mut msg);
    debug_assert_no_cot_in_messages(std::slice::from_ref(&msg));
    msg
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
