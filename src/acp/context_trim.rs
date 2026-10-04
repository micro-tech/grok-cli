//! Context trimming, token budgeting, and compression helpers for ACP sessions.
//!
//! Extracted from the monolithic `handle_chat_completion` as part of Task 280.1.
//! These functions handle per-message truncation, count-based trimming,
//! token-budget trimming, and smart compression (summarise + archive).

use crate::constants::{get_default_max_output_tokens, GROK4_CONTEXT_WINDOW, LEGACY_CONTEXT_WINDOW};

#[cfg(test)]
use crate::constants::{GROK4_CONTEXT_BUDGET, LEGACY_CONTEXT_BUDGET};
use crate::memory::context_archive::ContextChunk;
use serde_json::{json, Value};

/// Estimate token count for a list of messages.
/// Very rough approximation: ~4 chars per token.
pub fn estimate_tokens(messages: &[Value]) -> usize {
    (messages.iter().map(message_char_count).sum::<usize>() / 4) + (messages.len() * 2)
}

/// Char count backing [`estimate_tokens`] for a single message, factored out
/// so [`trim_to_token_budget`] can account removals incrementally instead of
/// re-scanning the whole history after every drop (Task 467).
fn message_char_count(m: &Value) -> usize {
    let mut total = 0usize;
    if let Some(content) = m.get("content") {
        if let Some(s) = content.as_str() {
            total += s.len();
        } else if let Some(arr) = content.as_array() {
            for item in arr {
                if let Some(s) = item.get("text").and_then(|t| t.as_str()) {
                    total += s.len();
                }
            }
        }
    }
    // Also count tool call / function call payloads
    if let Some(tool_calls) = m.get("tool_calls").and_then(|t| t.as_array()) {
        for tc in tool_calls {
            if let Some(func) = tc.get("function") {
                if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                    total += args.len();
                }
                if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                    total += name.len();
                }
            }
        }
    }
    total
}

/// Model context window information.
#[derive(Debug, Clone)]
pub struct ModelContextInfo {
    pub is_grok4_family: bool,
    pub context_window: usize,
}

static MODEL_CONTEXT_TABLE: &[(&str, ModelContextInfo)] = &[
    // Grok-4 family (1M context) - values now also live in crate::constants
    ("grok-4", ModelContextInfo { is_grok4_family: true, context_window: GROK4_CONTEXT_WINDOW }),
    ("grok-4.3", ModelContextInfo { is_grok4_family: true, context_window: GROK4_CONTEXT_WINDOW }),
    ("grok-4-latest", ModelContextInfo { is_grok4_family: true, context_window: GROK4_CONTEXT_WINDOW }),
    ("grok-4.5", ModelContextInfo { is_grok4_family: true, context_window: GROK4_CONTEXT_WINDOW }),
    ("grok-4.6", ModelContextInfo { is_grok4_family: true, context_window: GROK4_CONTEXT_WINDOW }),
    ("grok-4.7", ModelContextInfo { is_grok4_family: true, context_window: GROK4_CONTEXT_WINDOW }),
    ("grok-4.20", ModelContextInfo { is_grok4_family: true, context_window: GROK4_CONTEXT_WINDOW }),
    // Legacy / smaller models
    ("grok-3", ModelContextInfo { is_grok4_family: false, context_window: LEGACY_CONTEXT_WINDOW }),
    ("grok-3-mini", ModelContextInfo { is_grok4_family: false, context_window: LEGACY_CONTEXT_WINDOW }),
    ("grok-coder", ModelContextInfo { is_grok4_family: false, context_window: LEGACY_CONTEXT_WINDOW }),
];

/// Returns context info for a model (model-aware).
pub fn get_model_context_info(model: &str) -> ModelContextInfo {
    let m = model.to_ascii_lowercase();
    for (prefix, info) in MODEL_CONTEXT_TABLE {
        if m.starts_with(prefix) {
            return info.clone();
        }
    }
    // Default to legacy budget for unknown models
    ModelContextInfo {
        is_grok4_family: false,
        context_window: LEGACY_CONTEXT_WINDOW,
    }
}

/// Returns the appropriate context budget for the model.
/// grok-4.x family gets the high budget; everything else gets the legacy budget.
pub fn model_context_budget(model: &str, legacy_budget: usize, grok4_budget: usize) -> usize {
    let info = get_model_context_info(model);
    if info.is_grok4_family {
        grok4_budget
    } else {
        legacy_budget
    }
}

/// Public helper for external consumers (status bar, etc.).
pub fn get_model_context_window(model: &str) -> usize {
    get_model_context_info(model).context_window
}

/// Default max output tokens for a given model.
pub fn model_default_max_tokens(model: &str) -> u32 {
    get_default_max_output_tokens(model)
}

/// Trim messages until estimated tokens fit inside the budget.
/// Always keeps at least the system message (if present) + the last user message.
///
/// Task 467: the token estimate is computed once and decremented as messages
/// drop, instead of re-scanning all messages after every single removal
/// (which was O(n²)).  The arithmetic is identical to [`estimate_tokens`].
pub fn trim_to_token_budget(messages: &mut Vec<Value>, budget: usize) {
    let mut total_chars: usize = messages.iter().map(message_char_count).sum();
    while messages.len() > 1 && total_chars / 4 + messages.len() * 2 > budget {
        // Never drop the very first message if it's a system prompt
        let drop_idx =
            if messages.first().and_then(|m| m.get("role")).and_then(|r| r.as_str()) == Some("system")
                && messages.len() > 2
            {
                1
            } else {
                0
            };
        total_chars = total_chars.saturating_sub(message_char_count(&messages[drop_idx]));
        messages.remove(drop_idx);
    }
}

/// Tail-truncate a string to `max_chars` (char-boundary safe), keeping the
/// END of the string.  Returns the input unchanged when it already fits.
/// When truncation happens a marker records how much was cut so the model
/// knows it is seeing a tail, not the complete output.
///
/// Shared by [`truncate_tool_results`] (message sweep) and the shell tool's
/// source-level truncation (Task 464).
///
/// IMPORTANT for harness / cargo / build tools:
/// We keep the **tail** (end) of the output rather than the head.
/// Compiler errors, test failures, and the final status lines almost always appear
/// at the end of stderr/stdout. Keeping the head would hide the actual problem
/// from the LLM, leading to "tool call returned blank / no output" complaints.
pub fn truncate_tool_content(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    let suffix_overhead = 45; // a bit more room for the tail marker
    let target = max_chars.saturating_sub(suffix_overhead);

    // Keep the LAST `target` characters (tail) so errors at the end are visible.
    let mut start = s.len().saturating_sub(target);
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }

    let truncated = &s[start..];
    format!(
        "… [earlier output truncated, showing last {} of {} chars]\n{}",
        truncated.len(),
        s.len(),
        truncated
    )
}

/// Truncate the content of tool-result messages that are too long.
/// This is a cheap first-line defense against giant file reads / long command output.
pub fn truncate_tool_results(messages: &mut [Value], max_chars: usize) {
    for msg in messages.iter_mut() {
        if msg.get("role").and_then(|r| r.as_str()) != Some("tool") {
            continue;
        }

        if let Some(content) = msg.get_mut("content") {
            if let Some(s) = content.as_str() {
                if s.len() > max_chars {
                    *content = json!(truncate_tool_content(s, max_chars));
                }
            } else if let Some(arr) = content.as_array_mut() {
                for item in arr.iter_mut() {
                    let over = item
                        .get("text")
                        .and_then(|t| t.as_str())
                        .map(|t| t.len() > max_chars)
                        .unwrap_or(false);
                    if over {
                        let s = item
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let truncated = truncate_tool_content(&s, max_chars);
                        if let Some(slot) = item.get_mut("text") {
                            *slot = json!(truncated);
                        }
                    }
                }
            }
        }
    }
}

/// Build a compact system message announcing that older context was archived.
pub fn build_archive_notice(chunk: &ContextChunk) -> Value {
    let ts = chunk.created_at.format("%Y-%m-%d %H:%M UTC").to_string();
    let facts = if chunk.key_facts.is_empty() {
        String::new()
    } else {
        let bullets: String = chunk
            .key_facts
            .iter()
            .map(|f| format!("- {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\nKey facts:\n{}", bullets)
    };

    let preview: String = chunk.summary.chars().take(200).collect();
    let preview = if chunk.summary.len() > 200 {
        format!("{}…", preview)
    } else {
        preview
    };

    let content = format!(
        "[Context Archive #{} | {}]\n\
         {} messages summarised (~{} tokens saved).\n\
         Summary: {}\n\
         {}\n\n\
         Type `/recall {}` or say \"recall archive {}\" to restore the original messages.",
        chunk.chunk_id,
        ts,
        chunk.message_count,
        chunk.estimated_tokens_saved,
        preview,
        facts,
        chunk.chunk_id,
        chunk.chunk_id
    );

    json!({
        "role": "system",
        "content": content
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_archive_notice_has_correct_role_and_chunk_id() {
        let chunk = ContextChunk {
            chunk_id: 7,
            session_id: "sess-123".into(),
            created_at: chrono::Utc::now(),
            message_count: 12,
            estimated_tokens_saved: 3400,
            summary: "We discussed the new auth module and decided to use JWT.".into(),
            key_facts: vec!["Use JWT".into(), "Rotate keys every 30 days".into()],
            raw_messages: vec![],
        };

        let notice = build_archive_notice(&chunk);
        assert_eq!(notice["role"], "system");
        let content = notice["content"].as_str().unwrap();
        assert!(content.contains("Archive #7"));
        assert!(content.contains("recall archive 7"));
        assert!(content.contains("Use JWT"));
    }

    #[test]
    fn test_model_context_budget_grok4_uses_grok4_budget() {
        assert_eq!(model_context_budget("grok-4.3", LEGACY_CONTEXT_BUDGET, GROK4_CONTEXT_BUDGET), GROK4_CONTEXT_BUDGET);
        assert_eq!(
            model_context_budget("grok-4-latest", LEGACY_CONTEXT_BUDGET, GROK4_CONTEXT_BUDGET),
            GROK4_CONTEXT_BUDGET
        );
        assert_eq!(model_context_budget("grok-4", LEGACY_CONTEXT_BUDGET, GROK4_CONTEXT_BUDGET), GROK4_CONTEXT_BUDGET);
    }

    #[test]
    fn test_model_context_budget_legacy_models_use_legacy_budget() {
        assert_eq!(model_context_budget("grok-3", LEGACY_CONTEXT_BUDGET, GROK4_CONTEXT_BUDGET), LEGACY_CONTEXT_BUDGET);
        assert_eq!(model_context_budget("grok-3-mini", LEGACY_CONTEXT_BUDGET, GROK4_CONTEXT_BUDGET), LEGACY_CONTEXT_BUDGET);
        assert_eq!(model_context_budget("grok-2-latest", LEGACY_CONTEXT_BUDGET, GROK4_CONTEXT_BUDGET), LEGACY_CONTEXT_BUDGET);
        assert_eq!(model_context_budget("grok-beta", LEGACY_CONTEXT_BUDGET, GROK4_CONTEXT_BUDGET), LEGACY_CONTEXT_BUDGET);
    }

    #[test]
    fn test_truncate_tool_results_utf8_boundary() {
        let long_string = "A".repeat(29998) + "─" + &"B".repeat(10);
        let mut messages = vec![json!({
            "role": "tool",
            "content": long_string
        })];

        truncate_tool_results(&mut messages, 30000);

        let content = messages[0]["content"].as_str().unwrap();
        assert!(content.contains("truncated"), "should indicate truncation");
        // We now keep the TAIL, not the head. The last ~30k chars should be present.
        assert!(content.contains("B"), "tail (the B's) must be preserved");
        assert!(content.len() <= 30050);
        assert!(!content.starts_with("AAAAA"), "should NOT start with the original head anymore");
    }

    #[test]
    fn test_truncate_tool_results_array_utf8_boundary() {
        let long_string = "A".repeat(29998) + "─" + &"B".repeat(10);
        let mut messages = vec![json!({
            "role": "tool",
            "content": [{"type": "text", "text": long_string}]
        })];

        truncate_tool_results(&mut messages, 30000);

        let text = messages[0]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("truncated"), "should indicate truncation");
        assert!(text.contains("B"), "tail must be preserved");
        assert!(text.len() <= 30050);
    }
}
#[cfg(test)]
mod trim_tests_467 {
    use super::*;
    use serde_json::json;

    fn msg(role: &str, body_len: usize) -> Value {
        json!({"role": role, "content": "x".repeat(body_len)})
    }

    #[test]
    fn test_trim_to_token_budget_matches_estimate_arithmetic() {
        // Task 467: incremental accounting must agree with estimate_tokens.
        let mut messages = vec![
            msg("system", 100),
            msg("user", 4000),
            msg("assistant", 4000),
            msg("user", 4000),
        ];
        let before = estimate_tokens(&messages);
        assert!(before > 3000);
        trim_to_token_budget(&mut messages, 3000);
        let after = estimate_tokens(&messages);
        assert!(after <= 3000, "after trim: {} > 3000", after);
        // System message pinned, last user message kept.
        assert_eq!(messages.first().unwrap()["role"], "system");
        assert_eq!(messages.last().unwrap()["role"], "user");
        assert!(messages.len() >= 2);
    }

    #[test]
    fn test_trim_noop_when_under_budget() {
        let mut messages = vec![msg("system", 50), msg("user", 50)];
        let len = messages.len();
        trim_to_token_budget(&mut messages, 100_000);
        assert_eq!(messages.len(), len);
    }

    #[test]
    fn test_estimate_tokens_refactor_parity() {
        // estimate_tokens must equal the old formula: total_chars/4 + 2 per message.
        let messages = vec![
            json!({"role": "user", "content": "abcdefgh"}), // 8 chars
            json!({"role": "assistant", "content": [{"type": "text", "text": "ijklmnop"}]}), // 8 chars
        ];
        assert_eq!(estimate_tokens(&messages), 16 / 4 + 2 * 2);
    }
}
