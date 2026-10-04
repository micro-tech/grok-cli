//! Tool schema optimizer.
//!
//! Reduces token usage from tool schemas by pruning unused tools,
//! hashing schemas for cache keys, and applying light compression.

use crate::context::error::{ContextError, ContextResult};
use serde_json::Value;

/// Compute a simple hash of a tool schema for caching / deduplication.
pub fn schema_hash(schema: &Value) -> u64 {
    // Very lightweight hash — in production you'd use a proper hasher.
    let s = schema.to_string();
    s.bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

/// Prune tools that are not in the allowed list.
///
/// Fail-closed (Task 471.5): tools with a missing or unparseable name are
/// *dropped*, not kept — an unknown shape can't be usefully sent to the API
/// and keeping it would silently widen the tool surface.
pub fn prune_unused_tools(tools: Vec<Value>, keep: &[&str]) -> Vec<Value> {
    tools
        .into_iter()
        .filter(|t| {
            t.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|name| keep.contains(&name))
                .unwrap_or(false)
        })
        .collect()
}

/// Lightweight schema compression (removes verbose descriptions if present).
pub fn compress_schema(schema: &mut Value) -> ContextResult<()> {
    if let Some(desc) = schema.get_mut("description")
        && let Some(s) = desc.as_str()
        && s.len() > 120
    {
        if s.len() > 200_000 {
            return Err(ContextError::PromptTooLarge);
        }
        // Task 471.2: char-boundary-safe truncation — `&s[..117]` would panic
        // if byte 117 lands inside a multi-byte UTF-8 sequence.
        let mut end = 117.min(s.len());
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        *desc = Value::String(format!("{}\u{2026}", &s[..end]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_prune_unused() {
        let tools = vec![
            json!({"function": {"name": "read_file"}}),
            json!({"function": {"name": "write_file"}}),
        ];
        let pruned = prune_unused_tools(tools, &["read_file"]);
        assert_eq!(pruned.len(), 1);
    }

    #[test]
    fn test_schema_hash_stable() {
        let s = json!({"type": "object"});
        assert_eq!(schema_hash(&s), schema_hash(&s));
    }

    #[test]
    fn test_compress_schema_long_description() {
        let mut schema = json!({"description": "a".repeat(200)});
        compress_schema(&mut schema).unwrap();
        let desc = schema["description"].as_str().unwrap();
        assert!(desc.ends_with('…'));
        assert!(desc.len() < 130);
    }

    #[test]
    fn test_compress_schema_too_large() {
        let mut schema = json!({"description": "x".repeat(300_000)});
        assert!(compress_schema(&mut schema).is_err());
    }

    #[test]
    fn test_compress_schema_multibyte_boundary_no_panic() {
        // Task 471.2: byte 117 must not split a multi-byte char.
        // 116 ASCII chars + a 2-byte 'é' straddling the cut point.
        let mut schema = json!({"description": format!("{}é{}", "a".repeat(116), "b".repeat(100))});
        compress_schema(&mut schema).unwrap();
        let desc = schema["description"].as_str().unwrap();
        assert!(desc.ends_with('…'));
        assert!(desc.is_char_boundary(desc.len()));
    }

    #[test]
    fn test_prune_unused_drops_unknown_shapes() {
        // Task 471.5: fail-closed — tools with missing/unparseable names are dropped.
        let tools = vec![
            json!({"function": {"name": "read_file"}}),
            json!({"function": {}}),
            json!({"no_function": true}),
            json!({"function": "not-an-object"}),
        ];
        let pruned = prune_unused_tools(tools, &["read_file"]);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0]["function"]["name"], "read_file");
    }
}
