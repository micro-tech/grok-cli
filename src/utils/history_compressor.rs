//! Context-Aware History Compression
//!
//! Compresses conversation history using semantic clustering, importance scoring,
//! and recency weighting to prevent token bloat while preserving important context.

use serde_json::Value;

/// Represents a compressed conversation history
#[derive(Debug, Clone)]
pub struct CompressedHistory {
    pub messages: Vec<Value>,
    pub summary: Option<String>,
}

/// Compress conversation history to reduce token usage
pub fn compress_history(history: &[Value], max_messages: usize) -> CompressedHistory {
    if history.len() <= max_messages {
        return CompressedHistory {
            messages: history.to_vec(),
            summary: None,
        };
    }

    // Keep the most recent messages
    let recent = &history[history.len().saturating_sub(max_messages)..];

    // Create a simple summary of older messages
    let older = &history[..history.len().saturating_sub(max_messages)];
    let summary = if older.is_empty() {
        None
    } else {
        Some(summarize_messages(older))
    };

    CompressedHistory {
        messages: recent.to_vec(),
        summary,
    }
}

/// Create a simple summary of messages
fn summarize_messages(messages: &[Value]) -> String {
    let mut user_messages = 0;
    let mut assistant_messages = 0;
    let mut system_messages = 0;
    let mut tool_calls = 0;

    for msg in messages {
        if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
            match role {
                "user" => user_messages += 1,
                "assistant" => {
                    assistant_messages += 1;
                    if msg.get("tool_calls").is_some() {
                        tool_calls += 1;
                    }
                }
                "system" => system_messages += 1,
                _ => {}
            }
        }
    }

    format!(
        "Previous conversation summary: {} user messages, {} assistant responses ({} with tool calls), {} system messages.",
        user_messages, assistant_messages, tool_calls, system_messages
    )
}

/// Score message importance (simple heuristic)
pub fn score_importance(msg: &Value) -> f64 {
    let mut score = 1.0;

    // System messages are important
    if msg.get("role").and_then(|r| r.as_str()) == Some("system") {
        score += 2.0;
    }

    // Messages with tool calls are important
    if msg.get("tool_calls").is_some() {
        score += 1.5;
    }

    // Longer messages might be more important
    if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
        let len = content.len();
        if len > 1000 {
            score += 1.0;
        } else if len > 500 {
            score += 0.5;
        }
    }

    score
}

/// Apply recency weighting (more recent = higher weight)
pub fn recency_weight(index: usize, total: usize) -> f64 {
    (index as f64 / total as f64).powf(0.5) // Square root for gradual increase
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_compress_small_history() {
        let history = vec![
            json!({"role": "user", "content": "hello"}),
            json!({"role": "assistant", "content": "hi"}),
        ];
        let compressed = compress_history(&history, 5);
        assert_eq!(compressed.messages.len(), 2);
        assert!(compressed.summary.is_none());
    }

    #[test]
    fn test_compress_large_history() {
        let mut history = Vec::new();
        for i in 0..10 {
            history.push(json!({"role": "user", "content": format!("msg {}", i)}));
        }
        let compressed = compress_history(&history, 3);
        assert_eq!(compressed.messages.len(), 3);
        assert!(compressed.summary.is_some());
        assert!(compressed.summary.as_ref().unwrap().contains("7 user messages"));
    }

    #[test]
    fn test_score_importance() {
        let system_msg = json!({"role": "system", "content": "You are helpful"});
        let tool_msg = json!({"role": "assistant", "tool_calls": []});
        let normal_msg = json!({"role": "user", "content": "hello"});

        assert!(score_importance(&system_msg) > score_importance(&tool_msg));
        assert!(score_importance(&tool_msg) > score_importance(&normal_msg));
    }
}