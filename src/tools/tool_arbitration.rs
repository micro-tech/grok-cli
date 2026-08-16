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

/// High-level arbitration entry point (Task 271 optimized).
/// - Validates tool name (O(1) via static set)
/// - Validates required arguments (O(1) via static map)
/// - Only clones on the success path (and only once).
pub fn arbitrate_tool_call(name: &str, args: &Value) -> Result<ArbitrationDecision> {
    // 1) Validate tool name against the known set (fast O(1) path).
    if !is_known_tool(name) {
        return Ok(ArbitrationDecision::Reject {
            message: format!(
                "I don't have a tool named `{}`. Use /tools or /help to see available tools.",
                name
            ),
        });
    }

    // 2) Tool-specific argument validation (fast O(1) path).
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

    // 3) Normalization hook (Task 271).
    // We perform one clone on the happy path here.
    // normalize_args takes ownership so the common no-op case does not clone again.
    let normalized_args = normalize_args(name, args.clone())?;

    Ok(ArbitrationDecision::Execute {
        name: name.to_string(),
        args: normalized_args,
    })
}

/// Minimal known-tool check (Task 271 optimized).
///
/// Uses a pre-computed static HashSet for O(1) lookup instead of O(n) Vec::contains.
/// This is on the hot path for every tool call (arbitration before dispatch).
pub(crate) fn is_known_tool(name: &str) -> bool {
    crate::tools::registry::is_known_tool_fast(name)
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

/// Hook for argument normalization / correction (Task 271).
///
/// Takes ownership so callers can pass a clone they already paid for.
/// Current implementation is a no-op (returns the value as-is).
/// This is intentionally cheap on the hot path.
fn normalize_args(_name: &str, args: Value) -> Result<Value> {
    // Future: coerce numeric strings, trim, canonicalize paths, etc.
    // For now we keep it zero-cost.
    Ok(args)
}
