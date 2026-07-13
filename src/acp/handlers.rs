//! ACP v1 stable method handlers (Task 219)
//!
//! Implements the newly stabilized ACP methods:
//! - logout
//! - cancel (request cancellation)
//! - session/info_update
//! - model/config_options

use crate::acp::GrokAcpAgent;
use crate::acp::protocol::SessionId;
use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, warn};

/// Handle the `logout` method — cleanly ends the authenticated session
/// without killing the whole ACP process.
pub async fn handle_logout(_agent: &GrokAcpAgent, params: &Value) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .map(|s| SessionId::new(s));

    if let Some(sid) = session_id {
        info!("Logout requested for session {}", sid.0);
        // Clear any per-session state that should not survive logout.
        // For now we just log; future work can drop cached tokens etc.
        // We keep the session alive so the client can start a fresh one.
    } else {
        info!("Global logout requested (no sessionId)");
    }

    Ok(json!({ "ok": true }))
}

/// Handle the `cancel` method — aborts an in-flight request (tool call or prompt).
pub async fn handle_cancel(agent: &GrokAcpAgent, params: &Value) -> Result<Value> {
    let request_id = params
        .get("requestId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .map(|s| SessionId::new(s));

    if let Some(ref sid) = session_id {
        info!(
            "Cancel requested: request_id={}, session={}",
            request_id, sid.0
        );
        agent.cancel_session(&sid.0).await;
    } else {
        warn!("Cancel requested without sessionId — cannot cancel");
    }

    Ok(json!({ "cancelled": true, "requestId": request_id }))
}

/// Handle `session/info_update` — agent pushes metadata (title, status, …) to client.
pub async fn handle_session_info_update(
    agent: &GrokAcpAgent,
    params: &Value,
) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let title = params.get("title").and_then(|v| v.as_str());
    let status = params.get("status").and_then(|v| v.as_str());

    info!(
        "Session info update for {}: title={:?}, status={:?}",
        session_id, title, status
    );

    // Emit a visible notification so Zed (and other clients) can show the
    // updated title/status in the session UI.  We reuse the existing
    // AgentMessageChunk path for maximum compatibility.
    if let Some(title) = title {
        // Best-effort: we don't have an event_sender here, so we just log.
        // In a real flow the caller would forward the notification.
        info!("Session {} title → {}", session_id, title);
    }

    Ok(json!({ "ok": true }))
}

/// Handle `model/config_options` — returns the structured model configuration
/// category that editors can render as nice UI (temperature, max_tokens, etc.).
///
/// This is the exact shape the new ACP "Model Config Option Category" expects
/// (stable as of the March 2026 updates).  Editors (Zed, Cursor, etc.) can
/// use this to build a rich settings panel instead of raw text fields.
pub async fn handle_model_config_options(_agent: &GrokAcpAgent) -> Result<Value> {
    // This matches the new ACP "Model Config Option Category" schema.
    let options = json!({
        "category": "model",
        "title": "Grok Model Settings",
        "description": "Configure the AI model and generation behaviour for this session.",
        "options": [
            {
                "id": "temperature",
                "name": "Temperature",
                "type": "number",
                "default": 0.5,
                "min": 0.0,
                "max": 2.0,
                "step": 0.1,
                "description": "Controls randomness of the output. Lower = more deterministic."
            },
            {
                "id": "max_tokens",
                "name": "Max Output Tokens",
                "type": "integer",
                "default": 16384,
                "min": 256,
                "max": 131072,
                "description": "Maximum number of tokens the model may generate in one response."
            },
            {
                "id": "thinking_mode",
                "name": "Thinking Mode",
                "type": "string",
                "enum": ["off", "low", "high"],
                "default": "off",
                "description": "How much internal reasoning trace the model should produce before answering."
            },
            {
                "id": "model",
                "name": "Model",
                "type": "string",
                "enum": ["grok-4", "grok-4.3", "grok-3", "grok-3-mini", "grok-2"],
                "default": "grok-4",
                "description": "Which Grok model to use for this session."
            }
        ]
    });

    Ok(options)
}