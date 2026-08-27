//! Context trimming, token budgeting, and compression helpers for ACP sessions.
//!
//! Extracted from the monolithic `handle_chat_completion` as part of Task 280.1.
//! These functions handle per-message truncation, count-based trimming,
//! token-budget trimming, and smart compression (summarise + archive).

use crate::constants::{
    get_context_budget, get_context_window, get_default_max_output_tokens,
    GROK4_CONTEXT_BUDGET, GROK4_CONTEXT_WINDOW, LEGACY_CONTEXT_BUDGET, LEGACY_CONTEXT_WINDOW,
    MAX_TOOL_RESULT_CHARS,
};
use crate::memory::context_archive::ContextChunk;
use serde_json::{json, Value};

/// Estimate token count for a list of messages.
/// Very rough approximation: ~4 chars per token.
pub fn estimate_tokens(messages: &[Value]) -> usize {
    let mut total = 0usize;
    for m in messages {
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
    }
    // 4 chars ≈ 1 token, plus a small overhead per message
    (total / 4) + (messages.len() * 2)
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
pub fn trim_to_token_budget(messages: &mut Vec<Value>, budget: usize) {
    while messages.len() > 1 && estimate_tokens(messages) > budget {
        // Never drop the very first message if it's a system prompt
        if messages.first().and_then(|m| m.get("role")).and_then(|r| r.as_str()) == Some("system")
            && messages.len() > 2
        {
            messages.remove(1);
        } else {
            messages.remove(0);
        }
    }
}

/// Truncate the content of tool-result messages that are too long.
/// This is a cheap first-line defense against giant file reads.
pub fn truncate_tool_results(messages: &mut [Value], max_chars: usize) {
    for msg in messages.iter_mut() {
        if msg.get("role").and_then(|r| r.as_str()) != Some("tool") {
            continue;
        }

        if let Some(content) = msg.get_mut("content") {
            if let Some(s) = content.as_str() {
                if s.len() > max_chars {
                    let mut end = max_chars;
                    // Try to respect UTF-8 boundaries
                    while end > 0 && !s.is_char_boundary(end) {
                        end -= 1;
                    }
                    let truncated = &s[..end];
                    *content = json!(format!("{}… [truncated {} chars]", truncated, s.len() - end));
                }
            } else if let Some(arr) = content.as_array_mut() {
                for item in arr.iter_mut() {
                    if let Some(text) = item.get_mut("text").and_then(|t| t.as_str())
                        && text.len() > max_chars {
                            let mut end = max_chars;
                            while end > 0 && !text.is_char_boundary(end) {
                                end -= 1;
                            }
                            let truncated = &text[..end];
                            *item.get_mut("text").unwrap() = json!(format!(
                                "{}… [truncated {} chars]",
                                truncated,
                                text.len() - end
                            ));
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
        assert!(content.len() <= 30020); // small overhead for truncation marker
        assert!(content.contains("truncated"));
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
        assert!(text.contains("truncated"));
    }
}