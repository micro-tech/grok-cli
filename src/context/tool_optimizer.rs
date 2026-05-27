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
    s.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

/// Prune tools that are not in the allowed list.
pub fn prune_unused_tools(tools: Vec<Value>, keep: &[&str]) -> Vec<Value> {
    tools
        .into_iter()
        .filter(|t| {
            t.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|name| keep.contains(&name))
                .unwrap_or(true)
        })
        .collect()
}

/// Lightweight schema compression (removes verbose descriptions if present).
pub fn compress_schema(schema: &mut Value) -> ContextResult<()> {
    if let Some(desc) = schema.get_mut("description") {
        if let Some(s) = desc.as_str() {
            if s.len() > 120 {
                if s.len() > 200_000 {
                    return Err(ContextError::PromptTooLarge);
                }
                *desc = Value::String(format!("{}…", &s[..117]));
            }
        }
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
}
