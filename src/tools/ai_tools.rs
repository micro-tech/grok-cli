use anyhow::Result;
use serde_json::Value;

/// AI-generated / dynamic tools scaffold.
/// The registry calls `run` for the generic "ai_tool" entry point.
pub async fn run(args: &Value, _policy: &crate::acp::security::SecurityPolicy) -> Result<String> {
    // TODO: Implement real dispatch for AI-generated tools here.
    // For now we just echo the arguments so the arbitration layer can be tested.
    Ok(format!("ai_tool received: {}", args))
}

/// Legacy name kept for compatibility.
pub async fn ai_tool(input: Value) -> Result<String> {
    // Use a default policy when called directly
    let default_policy = crate::acp::security::SecurityPolicy::new();
    run(&input, &default_policy).await
}
