use anyhow::Result;
use serde_json::Value;

/// What the arbiter decided to do with the tool call.
pub enum ArbitrationDecision {
    /// Tool call is valid; proceed with execution.
    Execute { name: String, args: Value },
    /// Tool call is invalid; return a user-facing message instead of executing.
    Reject { message: String },
    /// Tool call is incomplete; ask the LLM/user for more info.
    NeedMoreInfo {
        message: String,
        missing_fields: Vec<String>,
    },
}

/// High-level arbitration entry point.
/// - Validates tool name
/// - Validates required arguments
/// - Optionally normalizes / corrects args
pub fn arbitrate_tool_call(name: &str, args: &Value) -> Result<ArbitrationDecision> {
    // 1) Validate tool name against the known set.
    if !is_known_tool(name) {
        return Ok(ArbitrationDecision::Reject {
            message: format!(
                "I don't have a tool named `{}`. Use /tools or /help to see available tools.",
                name
            ),
        });
    }

    // 2) Tool-specific argument validation.
    let missing = missing_required_fields(name, args);

    if !missing.is_empty() {
        return Ok(ArbitrationDecision::NeedMoreInfo {
            message: format!(
                "The `{}` tool is missing required fields: {}. \
                 Please provide these and try again.",
                name,
                missing.join(", ")
            ),
            missing_fields: missing,
        });
    }

    // 3) (Optional) Argument normalization / correction hook.
    let normalized_args = normalize_args(name, args)?;

    Ok(ArbitrationDecision::Execute {
        name: name.to_string(),
        args: normalized_args,
    })
}

/// Minimal known-tool check.
///
/// Delegates to the single source of truth in `registry::get_tool_definitions()`.
/// This is part of ARCH-2 (unified tool registry).
pub(crate) fn is_known_tool(name: &str) -> bool {
    crate::tools::registry::get_tool_definitions().contains(&name)
}

/// Return a list of missing required fields for a given tool.
///
/// Now driven **entirely** from the JSON schemas in registry (via `get_required_parameters`).
/// This removes the duplicated per-tool match (ARCH-2 progress).
fn missing_required_fields(name: &str, args: &Value) -> Vec<String> {
    let required = crate::tools::registry::get_required_parameters(name);

    required
        .into_iter()
        .filter(|field| args.get(field).is_none() || args[field].is_null())
        .collect()
}

/// Hook for argument normalization / correction.
/// Right now it's a no-op; you can grow this over time.
fn normalize_args(_name: &str, args: &Value) -> Result<Value> {
    // Example: coerce numeric strings, trim whitespace, etc.
    // For now, just clone.
    Ok(args.clone())
}
