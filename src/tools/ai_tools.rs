use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;
use crate::tools::ToolContext;

/// Simple in-memory registry for AI-generated / dynamic tools.
static DYNAMIC_TOOLS: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a new dynamic tool (name + description).
/// This can be called by code generators or plugins.
pub fn register_dynamic_tool(name: &str, description: &str) {
    if let Ok(mut map) = DYNAMIC_TOOLS.write() {
        map.insert(name.to_string(), description.to_string());
    }
}

/// List all currently registered dynamic tools.
pub fn list_dynamic_tools() -> Vec<(String, String)> {
    if let Ok(map) = DYNAMIC_TOOLS.read() {
        map.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    } else {
        vec![]
    }
}

/// AI-generated / dynamic tools entrypoint.
///
/// Supports:
/// - Echo mode (default)
/// - Dynamic tool dispatch if a tool was previously registered via `register_dynamic_tool`
pub async fn run(args: &Value, policy: &crate::acp::security::SecurityPolicy) -> Result<String> {
    // Check if this is a request to list dynamic tools
    if args.get("list_dynamic_tools").is_some() {
        let tools = list_dynamic_tools();
        return Ok(format!("Registered dynamic tools: {:?}", tools));
    }

    // If a specific dynamic tool is requested
    if let Some(tool_name) = args.get("tool").and_then(|v| v.as_str()) {
        let registered = {
            let map = DYNAMIC_TOOLS.read().unwrap();
            map.contains_key(tool_name)
        };

        if registered {
            let tool_args = args.get("args").cloned().unwrap_or_default();
            return Ok(format!(
                "Executed dynamic tool '{}' with args: {}",
                tool_name, tool_args
            ));
        }
    }

    // Default echo behaviour
    let _ = policy; // policy available for future security checks
    Ok(format!("ai_tool received: {}", args))
}

/// Legacy compatibility wrapper.
pub async fn ai_tool(input: Value) -> Result<String> {
    let default_policy = crate::acp::security::SecurityPolicy::new();
    run(&input, &default_policy).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_ai_tool_echo_mode() {
        let policy = crate::acp::security::SecurityPolicy::new();
        let args = json!({ "foo": "bar" });
        let result = run(&args, &policy).await.unwrap();
        assert!(result.contains("ai_tool received"));
    }

    #[tokio::test]
    async fn test_register_and_list_dynamic_tools() {
        register_dynamic_tool("my_custom_tool", "Does something cool");
        let tools = list_dynamic_tools();
        assert!(tools.iter().any(|(n, _)| n == "my_custom_tool"));
    }

    #[tokio::test]
    async fn test_ai_tool_list_dynamic_tools_command() {
        register_dynamic_tool("test_tool", "A test tool");
        let args = json!({ "list_dynamic_tools": true });
        let policy = crate::acp::security::SecurityPolicy::new();
        let result = run(&args, &policy).await.unwrap();
        assert!(result.contains("Registered dynamic tools"));
    }
}

/// TGS-RAG context enrichment (proper implementation when feature enabled).
#[cfg(feature = "tgs-rag")]
pub async fn enrich_with_rag_context(
    query: &str,
    _ctx: &ToolContext,
) -> Option<String> {
    // Try to load a persisted graph from the current directory
    let config = crate::rag::config::TgsRagConfig::default();
    if let Some(provider) = crate::rag::api::TgsRagContextProvider::from_persisted(
        std::path::Path::new("."),
        config,
    ) {
        let context = provider.get_context_for_query(query);
        if !context.is_empty() {
            return Some(context.join("\n---\n"));
        }
    }
    None
}

#[cfg(not(feature = "tgs-rag"))]
pub async fn enrich_with_rag_context(_query: &str, _ctx: &ToolContext) -> Option<String> {
    None
}
