//! Chat turn orchestration for ACP (Task 280).
//!
//! This module extracts the complex tool-using chat loop out of the
//! monolithic `handle_chat_completion` in `mod.rs`.
//!
//! Goals:
//! - Clear ownership of the message history during a turn (Task 269 style)
//! - Separated concerns: API call, tool execution, permission, emission
//! - Easier to test individual phases

use crate::acp::context_trim::{
    estimate_tokens, model_context_budget, trim_to_token_budget, truncate_tool_results,
};
use crate::acp::protocol::{SessionUpdate, ToolCall as ProtocolToolCall, ToolCallStatus, ToolCallUpdate};
use crate::acp::{PermissionBridge, GrokAcpAgent};
use crate::acp::status_bar::StatusBarState;
use crate::content_to_string;
use crate::tools;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{info, warn};

/// Outcome of a single chat turn (one model response + optional tool processing).
#[derive(Debug)]
pub struct TurnOutcome {
    pub response_text: String,
    pub had_tool_calls: bool,
    pub loop_count: u32,
    pub finished: bool,
}

/// Internal state for one chat turn. Owns the working message history.
pub struct ChatTurn {
    pub messages: Vec<Value>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub thinking_mode: crate::config::ThinkingMode,
    pub loop_count: u32,
    pub max_loops: u32,
    pub newly_always_allowed: Vec<String>,
    pub local_bayes: crate::bayes::BayesianEngine,
}

impl ChatTurn {
    pub fn new(
        messages: Vec<Value>,
        model: String,
        temperature: f32,
        max_tokens: u32,
        thinking_mode: crate::config::ThinkingMode,
        local_bayes: crate::bayes::BayesianEngine,
        max_loops: u32,
    ) -> Self {
        Self {
            messages,
            model,
            temperature,
            max_tokens,
            thinking_mode,
            loop_count: 0,
            max_loops,
            newly_always_allowed: Vec::new(),
            local_bayes,
        }
    }

    /// Re-apply trimming guards before an API call.
    pub fn reapply_trims(&mut self, config: &crate::config::Config) {
        let max_tc = config.acp.max_tool_result_chars;
        if max_tc > 0 {
            truncate_tool_results(&mut self.messages, max_tc);
        }

        let max_hist = config.acp.max_history_messages;
        if self.messages.len() > max_hist {
            let drop = self.messages.len() - max_hist;
            self.messages.drain(0..drop);
        }

        let limit = model_context_budget(
            &self.model,
            config.acp.max_context_tokens,
            config.acp.grok4_max_context_tokens,
        );
        let est = estimate_tokens(&self.messages);
        if est > limit {
            let before = self.messages.len();
            trim_to_token_budget(&mut self.messages, limit);
            warn!(
                "⚠️  Mid-loop context trim (iter {}): ~{} est. tokens > {} limit. Dropped {}.",
                self.loop_count,
                est,
                limit,
                before - self.messages.len()
            );
        }
    }

    /// Returns true if we should stop the tool loop.
    pub fn should_stop_loop(&self) -> bool {
        self.loop_count >= self.max_loops
    }

    /// Increment loop counter and return current count.
    pub fn next_loop(&mut self) -> u32 {
        self.loop_count += 1;
        self.loop_count
    }

    /// Execute the full tool-using chat turn loop.
    /// This is the extracted core of the former monolithic loop in handle_chat_completion (Task 280).
    /// `handle_chat_completion` is now a thin coordinator.
    pub async fn run(
        &mut self,
        agent: &GrokAcpAgent,
        session_id: &crate::acp::protocol::SessionId,
        event_sender: Option<&tokio::sync::mpsc::UnboundedSender<SessionUpdate>>,
        permission_bridge: Option<&Arc<PermissionBridge>>,
        local_always_allow: &std::collections::HashSet<String>,
        _start_time: std::time::Instant,
    ) -> Result<String> {
        let tool_defs = crate::tools::get_available_tool_definitions();

        loop {
            // Task 221: Cancellation check (still owned by agent for now)
            if agent.is_cancelled(&session_id.0).await {
                agent.clear_cancellation_flag(&session_id.0).await;
                info!("🛑 Prompt cancelled for session {}", session_id.0);
                return Ok("Request cancelled by user.".to_string());
            }

            if self.should_stop_loop() {
                return Err(anyhow!(
                    "Max tool loop iterations reached ({}). Consider increasing acp.max_tool_loop_iterations.",
                    self.max_loops
                ));
            }

            let current_loop = self.next_loop();
            if current_loop == (self.max_loops as f32 * 0.8) as u32 {
                warn!("Tool loop approaching limit: {}/{}", current_loop, self.max_loops);
            }

            let loop_start = std::time::Instant::now();
            info!("🔄 Tool loop iteration {}/{}", current_loop, self.max_loops);

            self.reapply_trims(&agent.config);

            // Use extracted retrying API caller (Task 280.2)
            let response_with_finish =
                perform_api_call_with_retries(agent, self, tool_defs).await?;

            let api_duration = std::time::Instant::now() - loop_start;
            info!("✅ Grok API responded in {:?}", api_duration);

            let response_msg = response_with_finish.message;
            let finish_reason = response_with_finish.finish_reason.as_deref();
            let thinking_content = response_with_finish.thinking_content;

            // Emit thinking if present (Task 280.4)
            if let Some(ref tc) = thinking_content
                && agent.config.acp.stream_thinking
                    && let Some(sender) = event_sender
                {
                    let blk = crate::acp::protocol::ThinkingBlockUpdate::new(tc, false);
                    let _ = sender.send(crate::acp::protocol::SessionUpdate::ThinkingBlockUpdate(blk));
                }

            self.messages.push(serde_json::to_value(&response_msg)?);

            let has_tool_calls = response_msg
                .tool_calls
                .as_ref()
                .map(|tc| !tc.is_empty())
                .unwrap_or(false);

            let response_text = content_to_string(response_msg.content.as_ref());

            if !has_tool_calls {
                info!(
                    "✨ Chat completion finished ({} loops)",
                    current_loop
                );

                let final_response = if let Some(tc) = thinking_content {
                    if agent.config.acp.stream_thinking
                        && let Some(sender) = event_sender
                    {
                        let blk = crate::acp::protocol::ThinkingBlockUpdate::new(&tc, true);
                        let _ = sender.send(crate::acp::protocol::SessionUpdate::ThinkingBlockUpdate(blk));
                    }
                    format!("<details><summary>🧠 Thinking…</summary>\n\n{}\n\n</details>\n\n{}", tc, response_text)
                } else {
                    response_text
                };

                // Final sync (Task 269 message ownership)
                {
                    let mut sessions = agent.sessions.write().await;
                    if let Some(s) = sessions.get_mut(&session_id.0) {
                        s.messages = std::mem::take(&mut self.messages);
                        s.bayes_engine = self.local_bayes.clone();
                        for name in &self.newly_always_allowed {
                            s.always_allow.insert(name.clone());
                        }
                        s.last_activity = std::time::Instant::now();
                    }
                }

                if let Some(sender) = event_sender {
                    emit_context_and_status(
                        agent,
                        sender,
                        &self.messages,
                        &self.model,
                        &self.thinking_mode,
                        false,
                    );
                }

                return Ok(final_response);
            }

            // Process tools using extracted function (Task 280.2 + 280.5)
            let Some(tool_calls) = response_msg.tool_calls.as_ref() else {
                // early return path
                {
                    let mut sessions = agent.sessions.write().await;
                    if let Some(s) = sessions.get_mut(&session_id.0) {
                        s.messages = std::mem::take(&mut self.messages);
                        s.bayes_engine = self.local_bayes.clone();
                        for name in &self.newly_always_allowed {
                            s.always_allow.insert(name.clone());
                        }
                    }
                }
                if let Some(sender) = event_sender {
                    emit_context_and_status(agent, sender, &self.messages, &self.model, &self.thinking_mode, false);
                }
                return Ok(response_text);
            };

            info!("🛠️  Processing {} tool calls", tool_calls.len());

            process_tool_calls(
                agent,
                session_id,
                tool_calls,
                self,
                event_sender,
                permission_bridge,
                local_always_allow,
            )
            .await?;

            // Emit live updates (Task 280.4)
            if let Some(sender) = event_sender {
                emit_context_and_status(
                    agent,
                    sender,
                    &self.messages,
                    &self.model,
                    &self.thinking_mode,
                    true,
                );
            }

            // Early stop if model said stop after tools
            if finish_reason == Some("stop") || finish_reason == Some("end_turn") {
                info!("✅ Model flagged stop after tools — returning");
                if let Some(sender) = event_sender {
                    emit_context_and_status(agent, sender, &self.messages, &self.model, &self.thinking_mode, false);
                }
                {
                    let mut sessions = agent.sessions.write().await;
                    if let Some(s) = sessions.get_mut(&session_id.0) {
                        s.messages = std::mem::take(&mut self.messages);
                        s.bayes_engine = self.local_bayes.clone();
                        for name in &self.newly_always_allowed {
                            s.always_allow.insert(name.clone());
                        }
                    }
                }
                return Ok(String::new());
            }
        }
    }
}

/// Performs one model call with Starlink-aware retries.
/// Now uses the unified `RetryPolicy` (Task 287) for consistent backoff.
pub async fn perform_api_call_with_retries(
    agent: &GrokAcpAgent,
    turn: &ChatTurn,
    tool_defs: &[Value],
) -> Result<crate::MessageWithFinishReason> {
    use crate::utils::network::RetryPolicy;

    // Build policy from config when possible; fall back to Starlink defaults.
    let policy = if agent.config.network.starlink_optimizations {
        RetryPolicy::from_config(&agent.config.network)
    } else {
        RetryPolicy::default_starlink()
    };

    let mut attempt = 0u32;

    loop {
        attempt += 1;

        match agent
            .get_router()?
            .chat_completion_with_history(
                &turn.messages,
                turn.temperature,
                turn.max_tokens,
                &turn.model,
                Some(tool_defs.to_vec()),
                turn.thinking_mode.as_api_str(),
            )
            .await
        {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                // Prompt-too-long is not retriable.
                let raw = e.to_string();
                let lower = raw.to_lowercase();
                if lower.contains("maximum prompt length")
                    || lower.contains("prompt length")
                    || (lower.contains("invalid argument") && lower.contains("token"))
                {
                    let current_est = estimate_tokens(&turn.messages);
                    return Err(anyhow!(
                        "Context window overflow — estimated ~{} tokens.\n\
                         Use /clear or reduce max_history_messages / max_tool_result_chars.",
                        current_est
                    ));
                }

                let is_retriable = policy.should_retry(attempt.saturating_sub(1), &e)
                    || lower.contains("timeout")
                    || lower.contains("timed out")
                    || lower.contains("reset")
                    || lower.contains("connection")
                    || lower.contains("network error")
                    || lower.contains("error sending request");

                if is_retriable && attempt <= policy.max_retries {
                    let delay = policy.delay_for_attempt(attempt.saturating_sub(1));

                    let err_kind = if lower.contains("timeout") || lower.contains("timed out") {
                        format!("TIMEOUT (real={})", agent.config.timeout_secs)
                    } else {
                        "NETWORK DROP".to_string()
                    };

                    warn!(
                        "API retry {}/{} [{}]: {}. Waiting {:?}...",
                        attempt, policy.max_retries, err_kind, e, delay
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    let tip = if lower.contains("timeout") {
                        format!("\n(real timeout={}s; grok_api bug reports '30s')", agent.config.timeout_secs)
                    } else {
                        String::new()
                    };
                    return Err(anyhow!("{}{}", e, tip));
                }
            }
        }
    }
}

/// Process a batch of tool calls for one assistant response.
/// This is the heart of the tool loop (Task 280.2 + 280.5).
pub async fn process_tool_calls(
    agent: &GrokAcpAgent,
    session_id: &crate::acp::protocol::SessionId,
    tool_calls: &[grok_api::ToolCall],
    turn: &mut ChatTurn,
    event_sender: Option<&tokio::sync::mpsc::UnboundedSender<SessionUpdate>>,
    permission_bridge: Option<&Arc<PermissionBridge>>,
    local_always_allow: &std::collections::HashSet<String>,
) -> Result<()> {
    for (idx, tool_call) in tool_calls.iter().enumerate() {
        let tool_start = std::time::Instant::now();
        let function_name = &tool_call.function.name;
        let args: Value = serde_json::from_str(&tool_call.function.arguments)?;

        info!("Tool {}/{}: {}", idx + 1, tool_calls.len(), function_name);

        // Emit ACP protocol ToolCall start (different struct from grok_api::ToolCall)
        if let Some(sender) = event_sender {
            let evt = ProtocolToolCall {
                tool_call_id: tool_call.id.clone(),
                title: format!("Running tool: {}", function_name),
                kind: Some(crate::acp::protocol::ToolKind::Execute),
                status: Some(ToolCallStatus::InProgress),
                raw_input: Some(args.clone()),
                raw_output: None,
                locations: None,
                content: None,
            };
            let _ = sender.send(SessionUpdate::ToolCall(evt));
        }

        // before_tool hooks
        {
            let hooks = agent.get_hook_manager().read().await;
            if !hooks.execute_before_tool(function_name, &args)? {
                turn.messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": "Tool execution blocked by hook."
                }));
                continue;
            }
        }

        // === PERMISSION GATE (Task 280.5) ===
        let needs_permission = agent.config.acp.require_permission
            && !local_always_allow.contains(function_name.as_str())
            && !turn.newly_always_allowed.contains(function_name);

        if needs_permission
            && let Some(bridge) = permission_bridge {
                let req_id = uuid::Uuid::new_v4().to_string();
                let params = crate::acp::protocol::RequestPermissionParams::new(
                    session_id.clone(),
                    tool_call.id.clone(),
                    Some(format!("Run {}", function_name)),
                    Some(crate::acp::protocol::ToolKind::Execute),
                );

                let (tx, rx) = oneshot::channel();
                if bridge.outbound.send((req_id, params, tx)).is_ok() {
                    let timeout = std::time::Duration::from_secs(agent.config.acp.permission_timeout_secs);
                    match tokio::time::timeout(timeout, rx).await {
                        Ok(Ok(outcome)) => {
                            if outcome.is_cancelled() {
                                turn.messages.push(json!({
                                    "role": "tool",
                                    "tool_call_id": tool_call.id,
                                    "content": "User rejected the tool execution."
                                }));
                                continue;
                            }
                            if outcome.is_always_allow() {
                                turn.newly_always_allowed.push(function_name.clone());
                            }
                        }
                        Ok(Err(_)) => return Err(anyhow!("Permission bridge closed")),
                        Err(_) => return Err(anyhow!("Permission timeout")),
                    }
                }
            }
        // === END PERMISSION GATE ===

        // Execute via unified registry
        let policy = agent.get_security().get_policy();
        let ctx = tools::ToolContext::new(policy);

        let mut augmented = args.clone();
        if function_name == "run_shell_command" {
            let shell_timeout = agent.config.tools.shell.command_timeout_secs;
            if shell_timeout > 0 && augmented.get("timeout_secs").is_none() {
                augmented["timeout_secs"] = json!(shell_timeout);
            }
        }

        let result = tools::execute_tool(function_name, &augmented, &ctx).await;

        let (content, status) = match result {
            Ok(s) => {
                info!("Tool {} completed in {:?}", function_name, tool_start.elapsed());
                {
                    let mut guard = agent.sessions.write().await;
                    if let Some(s) = guard.get_mut(&session_id.0) {
                        s.dna.update_from_tool_result(true, function_name);
                    }
                }
                (s, ToolCallStatus::Completed)
            }
            Err(e) => {
                warn!("Tool {} failed: {}", function_name, e);
                turn.local_bayes.update_from_tool_failure();

                {
                    let mut guard = agent.sessions.write().await;
                    if let Some(s) = guard.get_mut(&session_id.0) {
                        s.dna.update_from_tool_result(false, function_name);
                    }
                }

                let mut err = format!("Error executing {}: {}", function_name, e);
                if turn.local_bayes.is_low_confidence() {
                    err.push_str("\n\n[System Note: Low confidence after failure. Consider asking user or changing approach.]");
                }
                (err, ToolCallStatus::Failed)
            }
        };

        // Emit update
        if let Some(sender) = event_sender {
            let update = ToolCallUpdate {
                tool_call_id: tool_call.id.clone(),
                kind: None,
                status: Some(status),
                locations: None,
                content: Some(vec![crate::acp::protocol::ToolCallContent::Content(
                    crate::acp::protocol::ToolCallContentInner {
                        content: crate::acp::protocol::ContentBlock::Text(
                            crate::acp::protocol::TextContent::new(content.clone()),
                        ),
                    },
                )]),
            };
            let _ = sender.send(SessionUpdate::ToolCallUpdate(update));
        }

        // after_tool hooks
        {
            let hooks = agent.get_hook_manager().read().await;
            hooks.execute_after_tool(function_name, &args, &content)?;
        }

        turn.messages.push(json!({
            "role": "tool",
            "tool_call_id": tool_call.id,
            "content": content
        }));

        // Final-answer guard (helps prevent max-loop)
        turn.messages.push(json!({
            "role": "system",
            "content": "Tool result received. Produce your final answer now. Do NOT call any more tools unless the user explicitly asks for additional actions."
        }));
    }

    Ok(())
}

/// Emit context + status bar updates (Task 280.4)
pub fn emit_context_and_status(
    agent: &GrokAcpAgent,
    sender: &tokio::sync::mpsc::UnboundedSender<SessionUpdate>,
    messages: &[Value],
    model: &str,
    thinking_mode: &crate::config::ThinkingMode,
    is_generating: bool,
) {
    if !agent.config.acp.show_context_usage {
        return;
    }

    let usage = crate::acp::protocol::ContextUsageUpdate::new(
        estimate_tokens(messages),
        model_context_budget(
            model,
            agent.config.acp.max_context_tokens,
            agent.config.acp.grok4_max_context_tokens,
        ),
        messages.len(),
    );
    let _ = sender.send(SessionUpdate::ContextUsageUpdate(usage));

    let state = StatusBarState {
        model: model.to_string(),
        thinking_mode: thinking_mode.as_api_str().unwrap_or("off").to_string(),
        current_tokens: estimate_tokens(messages),
        max_tokens: model_context_budget(
            model,
            agent.config.acp.max_context_tokens,
            agent.config.acp.grok4_max_context_tokens,
        ),
        context_percent: (estimate_tokens(messages) as f32)
            / (model_context_budget(
                model,
                agent.config.acp.max_context_tokens,
                agent.config.acp.grok4_max_context_tokens,
            ) as f32),
        is_generating,
    };
    agent.emit_status_bar(Some(sender), &state);
}
