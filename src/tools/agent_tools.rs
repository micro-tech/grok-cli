//! Agent coordination tools — sub-agent spawning, inter-agent messaging,
//! and team management.

use crate::agent::config::SubAgentConfig;
use crate::agent::manager::AgentManager;
use anyhow::{Result, anyhow};
use chrono::Utc;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing::{error, info, warn};
use tracing::{error, info, warn};

// ── Global shared AgentManager (Task 127) ─────────────────────────────────────

static AGENT_MANAGER: once_cell::sync::Lazy<Arc<AgentManager>> =
    once_cell::sync::Lazy::new(AgentManager::new);

/// Returns the global shared AgentManager instance.
pub fn get_agent_manager() -> Arc<AgentManager> {
    AGENT_MANAGER.clone()
}

// ── constants ────────────────────────────────────────────────────────────────

/// System prompt injected into every sub-agent call.
const SUBAGENT_SYSTEM_PROMPT: &str = "You are a focused sub-agent. Complete the given task as concisely and \
     directly as possible. Return only the result — no preamble, no meta-commentary.";

/// Per-sub-agent API timeout in seconds (Starlink-safe: 3 min).
const SUBAGENT_TIMEOUT_SECS: u64 = 180;

/// Maximum number of Starlink-aware retries for a single sub-agent call.
const SUBAGENT_MAX_RETRIES: u32 = 3;

// ── helpers ───────────────────────────────────────────────────────────────────

fn grok_data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Cannot determine local data directory"))?
        .join(".grok");
    fs::create_dir_all(&dir).map_err(|e| anyhow!("Failed to create .grok directory: {}", e))?;
    Ok(dir)
}

// ── run_agent_session ──────────────────────────────────────────────────

/// Run a full sub-agent session with per-agent tool permissions, persona,
/// sandbox boundaries, and context budget defined by [`SubAgentConfig`].
///
/// This is the core implementation used by `spawn_agent`, `fork_agent`, and
/// `delegate_plan_step`.  Call it directly when you need full control over
/// the agent's capabilities.
///
/// # How it works
///
/// 1. Builds a [`SecurityPolicy`] from `config.trusted_dirs` (CWD if empty).
/// 2. Filters the global tool registry down to `config.allowed_tools`.
/// 3. Constructs a `RouterRequest` with the persona system prompt + task.
/// 4. If no tools are allowed, falls back to a fast single-shot text completion.
/// 5. If tools are allowed, drives `AppRouter::route_with_tools` (the full
///    tool loop) inside the scoped security context.
pub async fn run_agent_session(
    task: String,
    context: String,
    config: SubAgentConfig,
) -> Result<String> {
    if task.trim().is_empty() {
        return Err(anyhow!("run_agent_session: task cannot be empty"));
    }

    let api_key = std::env::var("GROK_API_KEY")
        .or_else(|_| std::env::var("XAI_API_KEY"))
        .map_err(|_| anyhow!("No API key found. Set GROK_API_KEY or XAI_API_KEY."))?;

    // ── 1. Per-agent SecurityPolicy ──────────────────────────────────────────
    let mut policy = crate::acp::security::SecurityPolicy::new();
    if config.trusted_dirs.is_empty() {
        // Default: only trust CWD (most restrictive safe default).
        if let Ok(cwd) = std::env::current_dir() {
            policy.add_trusted_directory(&cwd);
        }
    } else {
        for dir in &config.trusted_dirs {
            policy.add_trusted_directory(dir);
        }
    }
    let tool_context = crate::tools::tool_context::ToolContext::new(policy);

    // ── 2. Tool whitelist filtering ──────────────────────────────────────────
    let filtered_tools: Vec<serde_json::Value> = match &config.allowed_tools {
        None => vec![],
        Some(whitelist) if whitelist.is_empty() => vec![],
        Some(whitelist) => crate::tools::registry::get_full_tool_definitions()
            .into_iter()
            .filter(|t| {
                t.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|name| whitelist.iter().any(|w| w == name))
                    .unwrap_or(false)
            })
            .collect(),
    };

    info!(
        model = %config.model,
        tools = filtered_tools.len(),
        max_tokens = config.max_tokens,
        has_persona = config.system_prompt.is_some(),
        trusted_dirs = config.trusted_dirs.len(),
        "run_agent_session: starting"
    );

    // ── 3. Build message history ──────────────────────────────────────────────
    let system = config
        .system_prompt
        .as_deref()
        .unwrap_or(SUBAGENT_SYSTEM_PROMPT)
        .to_string();

    let user_msg = if context.trim().is_empty() {
        task.to_string()
    } else {
        format!(
            "{}

## Context
{}",
            task, context
        )
    };

    let messages = vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user",   "content": user_msg}),
    ];

    let clamped = config.max_tokens.clamp(256, 8192);

    // ── 4. Build router ───────────────────────────────────────────────────────
    let router = crate::router::AppRouter::new(&api_key, SUBAGENT_TIMEOUT_SECS)
        .map_err(|e| anyhow!("Failed to initialise router: {}", e))?;

    // ── 5. Execute ─────────────────────────────────────────────────────────────
    if filtered_tools.is_empty() {
        // Fast path: pure text completion, no tool loop needed.
        let mwfr = router
            .chat_completion_with_history(
                &messages,
                config.temperature,
                clamped,
                &config.model,
                None,
                None,
            )
            .await
            .map_err(|e| anyhow!("Sub-agent text completion failed: {}", e))?;

        return Ok(match mwfr.message.content {
            Some(grok_api::MessageContent::Text(t)) => t,
            _ => String::new(),
        });
    }

    // Tool-enabled path: drive the full CpuRouter tool loop.
    let req = crate::router::RouterRequest::new(config.model.clone(), messages)
        .with_temperature(config.temperature)
        .with_max_tokens(clamped)
        .with_json_tools(filtered_tools);

    let resp = router
        .route_with_tools(req, &tool_context, config.max_tool_iterations)
        .await
        .map_err(|e| anyhow!("Sub-agent tool loop failed: {}", e))?;

    info!(
        "run_agent_session: completed ({} chars)",
        resp.text.as_deref().unwrap_or("").len()
    );
    Ok(resp.text.unwrap_or_default())
}

// ── call_subagent_api (private) ──────────────────────────────────────────────────

/// Backward-compatible raw API call for a sub-agent task.
///
/// Now delegates to [`run_agent_session`] with a default [`SubAgentConfig`]
/// (no tools, default model, default persona).  Retains the same Starlink-aware
/// retry behaviour by wrapping in a retry loop.
async fn call_subagent_api(task: &str, context: &str, max_tokens: u32) -> Result<String> {
    let config = SubAgentConfig::builder().max_tokens(max_tokens).build();

    for attempt in 0..=SUBAGENT_MAX_RETRIES {
        match run_agent_session(task, context, &config).await {
            Ok(result) => return Ok(result),
            Err(e)
                if attempt < SUBAGENT_MAX_RETRIES
                    && crate::utils::network::detect_network_drop(&e) =>
            {
                let delay = crate::utils::network::calculate_retry_delay(attempt, false);
                warn!(
                    attempt = attempt + 1,
                    max_attempts = SUBAGENT_MAX_RETRIES + 1,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "call_subagent_api: network error — retrying after delay"
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                warn!(error = %e, "call_subagent_api: failed — no more retries");
                return Err(e);
            }
        }
    }
    unreachable!()
}

// ── spawn_agent ───────────────────────────────────────────────────────────────

/// Spawn a focused sub-agent with default configuration (no tools, grok-3-mini).
///
/// This is the backward-compatible entry point.  For per-agent tool permissions,
/// personas, and sandbox rules use [`spawn_agent_configured`] or call
/// [`run_agent_session`] directly.
pub async fn spawn_agent(task: &str, context: &str, max_tokens: u32) -> Result<String> {
    let config = SubAgentConfig::builder().max_tokens(max_tokens).build();
    spawn_agent_configured(task, context, None, config).await
}

/// Spawn a focused sub-agent with a full [`SubAgentConfig`].
///
/// - `parent_id` — optional parent agent UUID for tree tracking.
/// - `config`    — per-agent model, persona, tool whitelist, sandbox dirs, budgets.
///
/// Registers in `AgentManager`, runs `run_agent_session`, stores result, and
/// emits Spawned/Joined activity events visible in Zed.
pub async fn spawn_agent_configured(
    task: &str,
    context: &str,
    parent_id: Option<String>,
    config: SubAgentConfig,
) -> Result<String> {
    if task.trim().is_empty() {
        warn!("spawn_agent_configured: task is empty");
        return Err(anyhow!("task cannot be empty"));
    }

    let manager = get_agent_manager();
    let agent_id = manager
        .spawn(task, parent_id, Some(config.model.clone()), None)
        .await;

    crate::agent::activity::emit_agent_activity(
        &agent_id,
        None,
        crate::acp::protocol::AgentActivityStatus::Spawned,
        format!(
            "Spawned (model={}, tools={}, dirs={}): {}",
            config.model,
            config.tool_count(),
            config.trusted_dirs.len(),
            &task.chars().take(60).collect::<String>()
        ),
    );
    info!(
        agent_id = %agent_id,
        model = %config.model,
        tools = config.tool_count(),
        task = %task,
        "spawn_agent_configured: starting"
    );

    match run_agent_session(task, context, &config).await {
        Ok(result) => {
            manager.complete(&agent_id, result.clone()).await;
            crate::agent::activity::emit_agent_activity(
                &agent_id,
                None,
                crate::acp::protocol::AgentActivityStatus::Joined,
                "Completed successfully",
            );
            info!(agent_id = %agent_id, chars = result.len(), "spawn_agent_configured: completed");
            Ok(result)
        }
        Err(e) => {
            manager.fail(&agent_id, e.to_string()).await;
            error!(agent_id = %agent_id, error = %e, "spawn_agent_configured: failed");
            Err(e)
        }
    }
}

// ── send_message ──────────────────────────────────────────────────────────────

/// Send a message to a named target (agent ID or channel).
///
/// Messages are appended to `{data_dir}/.grok/messages/{target}.jsonl` as
/// JSON Lines using an atomic write (`.tmp` → rename) so a Starlink drop
/// mid-write cannot corrupt the log file.  The target name is sanitised so
/// it is safe as a file name.
pub fn send_message(target: &str, message: &str) -> Result<String> {
    if target.trim().is_empty() {
        warn!("send_message: target is empty");
        return Err(anyhow!("target cannot be empty"));
    }
    if message.trim().is_empty() {
        warn!("send_message: message is empty");
        return Err(anyhow!("message cannot be empty"));
    }

    let msg_dir = grok_data_dir()?.join("messages");
    fs::create_dir_all(&msg_dir)
        .map_err(|e| anyhow!("Failed to create messages directory: {}", e))?;

    // Sanitise target name for safe use as a filename.
    let safe_target: String = target
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let path = msg_dir.join(format!("{}.jsonl", safe_target));

    let entry = json!({
        "timestamp": Utc::now().to_rfc3339(),
        "from":      "grok",
        "to":        target,
        "message":   message,
    });

    // Read existing content so we can append and then write atomically.
    let existing = if path.exists() {
        fs::read_to_string(&path)
            .map_err(|e| anyhow!("send_message: failed to read existing messages: {}", e))?
    } else {
        String::new()
    };
    let content = format!("{}{}\n", existing, entry);

    // Atomic write: write to .tmp then rename so a mid-write Starlink drop
    // cannot leave a partial / corrupt JSONL file.
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &content).map_err(|e| anyhow!("send_message: write tmp failed: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| anyhow!("send_message: rename failed: {}", e))?;

    Ok(format!(
        "Message delivered to '{}' ({} chars).",
        target,
        message.len()
    ))
}

// ── team_create ───────────────────────────────────────────────────────────────

/// Create a named team configuration in `{data_dir}/.grok/teams.json`.
///
/// Returns an error if a team with the same name already exists — call
/// [`team_delete`] first if you need to recreate it.
pub fn team_create(name: &str, members: Vec<String>, description: &str) -> Result<String> {
    if name.trim().is_empty() {
        warn!("team_create: team name is empty");
        return Err(anyhow!("team name cannot be empty"));
    }

    let teams_file = grok_data_dir()?.join("teams.json");

    let mut data: Value = if teams_file.exists() {
        let content = fs::read_to_string(&teams_file)
            .map_err(|e| anyhow!("Failed to read teams.json: {}", e))?;
        serde_json::from_str(&content).unwrap_or(json!({ "teams": [] }))
    } else {
        json!({ "teams": [] })
    };

    let teams = data["teams"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("Invalid teams.json: missing 'teams' array"))?;

    if teams.iter().any(|t| t["name"].as_str() == Some(name)) {
        warn!(team = %name, "team_create: team already exists");
        return Err(anyhow!(
            "Team '{}' already exists. Call team_delete first to recreate it.",
            name
        ));
    }

    teams.push(json!({
        "name":        name,
        "description": description,
        "members":     members,
        "created_at":  Utc::now().to_rfc3339(),
    }));

    fs::write(&teams_file, serde_json::to_string_pretty(&data)?)
        .map_err(|e| anyhow!("Failed to write teams.json: {}", e))?;

    Ok(format!(
        "Team '{}' created with {} member(s).",
        name,
        data["teams"]
            .as_array()
            .map(|a| a
                .last()
                .and_then(|t| t["members"].as_array())
                .map(|m| m.len())
                .unwrap_or(0))
            .unwrap_or(0)
    ))
}

// ── team_delete ───────────────────────────────────────────────────────────────

/// Delete a named team from `{data_dir}/.grok/teams.json`.
pub fn team_delete(name: &str) -> Result<String> {
    if name.trim().is_empty() {
        warn!("team_delete: team name is empty");
        return Err(anyhow!("team name cannot be empty"));
    }

    let teams_file = grok_data_dir()?.join("teams.json");

    if !teams_file.exists() {
        warn!("team_delete: teams file does not exist — no teams have been created");
        return Err(anyhow!("No teams file found — no teams have been created."));
    }

    let content =
        fs::read_to_string(&teams_file).map_err(|e| anyhow!("Failed to read teams.json: {}", e))?;
    let mut data: Value =
        serde_json::from_str(&content).map_err(|e| anyhow!("Invalid teams.json: {}", e))?;

    let teams = data["teams"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("Invalid teams.json: missing 'teams' array"))?;

    let before = teams.len();
    teams.retain(|t| t["name"].as_str() != Some(name));

    if teams.len() == before {
        warn!(team = %name, "team_delete: team not found");
        return Err(anyhow!("Team '{}' not found.", name));
    }

    fs::write(&teams_file, serde_json::to_string_pretty(&data)?)
        .map_err(|e| anyhow!("Failed to write teams.json: {}", e))?;

    Ok(format!("Team '{}' deleted.", name))
}

// ── merge_agent_results ───────────────────────────────────────────────────────

/// Merge results from multiple sub-agents using simple arbitration.
///
/// Returns `Ok("No agent results to merge.")` immediately when `results` is
/// empty.  Otherwise prefers longer, more detailed responses and returns a
/// formatted summary of the top three candidates.
pub fn merge_agent_results(results: Vec<String>) -> Result<String> {
    if results.is_empty() {
        return Ok("No agent results to merge.".to_string());
    }
    if results.len() == 1 {
        return Ok(results[0].clone());
    }

    // Simple scoring: prefer longer responses as more detailed.
    let mut scored: Vec<(String, usize)> = results
        .into_iter()
        .map(|r| {
            let len = r.len();
            (r, len)
        })
        .collect();
    scored.sort_by_key(|item| std::cmp::Reverse(item.1));

    // Take top 3 and format a merged summary.
    let top: Vec<String> = scored.into_iter().take(3).map(|(r, _)| r).collect();

    Ok(format!(
        "Merged results from {} agents:\n\n{}",
        top.len(),
        top.join("\n\n---\n\n")
    ))
}

// ── Task 127: New orchestration tools using AgentManager ─────────────────────

/// List all currently tracked sub-agents (or only those belonging to a parent).
pub async fn list_agents(parent_id: Option<&str>) -> Result<String> {
    let manager = get_agent_manager();
    let agents = manager.list(parent_id).await;

    if agents.is_empty() {
        return Ok("No sub-agents found.".to_string());
    }

    let mut output = String::from("## Active Sub-Agents\n\n");
    for a in agents {
        let status = match a.status {
            crate::agent::manager::AgentStatus::Running => "🟡 Running",
            crate::agent::manager::AgentStatus::Completed => "🟢 Completed",
            crate::agent::manager::AgentStatus::Failed => "🔴 Failed",
            crate::agent::manager::AgentStatus::Cancelled => "⚫ Cancelled",
        };
        output.push_str(&format!(
            "- **{}** | {} | Task: {}\n",
            a.id,
            status,
            a.task.chars().take(60).collect::<String>()
        ));
    }
    Ok(output)
}

/// Get the current status and result (if available) of a specific sub-agent.
pub async fn get_agent_status(agent_id: &str) -> Result<String> {
    let manager = get_agent_manager();
    match manager.get(agent_id).await {
        Some(a) => {
            let status = match a.status {
                crate::agent::manager::AgentStatus::Running => "Running",
                crate::agent::manager::AgentStatus::Completed => "Completed",
                crate::agent::manager::AgentStatus::Failed => "Failed",
                crate::agent::manager::AgentStatus::Cancelled => "Cancelled",
            };
            let result = a.result.unwrap_or_else(|| "(no result yet)".to_string());
            Ok(format!(
                "Agent {} | Status: {} | Task: {}\nResult:\n{}",
                a.id, status, a.task, result
            ))
        }
        None => Ok(format!("No agent found with ID '{}'", agent_id)),
    }
}

/// Cancel a running sub-agent.
pub async fn cancel_agent(agent_id: &str) -> Result<String> {
    let manager = get_agent_manager();
    manager.cancel(agent_id).await;

    // Task 128.3
    crate::agent::activity::emit_agent_activity(
        agent_id,
        None,
        crate::acp::protocol::AgentActivityStatus::Cancelled,
        "Agent cancelled".to_string(),
    );

    Ok(format!("Agent {} cancelled (if it was running).", agent_id))
}

// ── In-memory messaging (Task 127) ────────────────────────────────────────────

/// Send a message using the fast in-memory bus (preferred over file-based).
pub async fn send_message_in_memory(from: &str, to: &str, message: &str) -> Result<String> {
    let bus = crate::agent::message_bus::MESSAGE_BUS.clone();
    Ok(bus.send(from, to, message).await)
}

/// Receive pending messages for an agent from the in-memory bus.
pub async fn receive_messages(target: &str) -> Result<String> {
    let bus = crate::agent::message_bus::MESSAGE_BUS.clone();
    let msgs = bus.receive(target).await;

    if msgs.is_empty() {
        return Ok("No pending messages.".to_string());
    }

    let formatted: Vec<String> = msgs
        .iter()
        .map(|m| format!("[{}] {} → {}: {}", m.timestamp, m.from, m.to, m.content))
        .collect();

    Ok(formatted.join("\n"))
}

// ── Advanced orchestration tools ─────────────────────────────────────────────

/// Fork multiple sub-agents in parallel and wait for all results.
///
/// Each task is registered in the `AgentManager` and run concurrently via
/// `tokio::spawn`.  The function waits for **all** tasks to finish (or time
/// out) before returning a structured summary.  Use `join_agents` only when
/// you need to re-query results from agents that were spawned earlier.
pub async fn fork_agent(tasks: Vec<String>) -> Result<String> {
    if tasks.is_empty() {
        return Ok("No tasks provided to fork.".to_string());
    }

    let manager = get_agent_manager();
    let total = tasks.len();
    info!(count = total, "fork_agent: launching parallel sub-agents");

    // ── Phase 1: register all agents + launch tokio tasks ────────────────────────
    let mut handles: Vec<(String, tokio::task::JoinHandle<(String, Result<String>)>)> =
        Vec::with_capacity(total);

    for task in &tasks {
        let agent_id = manager
            .spawn(task, None, Some("grok-3-mini".to_string()), None)
            .await;

        crate::agent::activity::emit_agent_activity(
            &agent_id,
            None,
            crate::acp::protocol::AgentActivityStatus::Forked,
            format!("Forked: {}", &task.chars().take(80).collect::<String>()),
        );
        info!(agent_id = %agent_id, task = %task, "fork_agent: spawning task");

        let task_clone = task.clone();
        let agent_id_clone = agent_id.clone();
        let manager_clone = Arc::clone(&manager);

        let handle = tokio::spawn(async move {
            // Wrap with per-task timeout so one slow/stuck task can't block all.
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(SUBAGENT_TIMEOUT_SECS),
                call_subagent_api(&task_clone, "", 2048),
            )
            .await
            .unwrap_or_else(|_| {
                Err(anyhow!(
                    "sub-agent timed out after {}s",
                    SUBAGENT_TIMEOUT_SECS
                ))
            });

            match &result {
                Ok(r) => {
                    manager_clone.complete(&agent_id_clone, r.clone()).await;
                    crate::agent::activity::emit_agent_activity(
                        &agent_id_clone,
                        None,
                        crate::acp::protocol::AgentActivityStatus::Joined,
                        "Completed successfully",
                    );
                }
                Err(e) => {
                    manager_clone.fail(&agent_id_clone, e.to_string()).await;
                    crate::agent::activity::emit_agent_activity(
                        &agent_id_clone,
                        None,
                        crate::acp::protocol::AgentActivityStatus::Cancelled,
                        format!("Failed: {}", e),
                    );
                }
            }

            (agent_id_clone, result)
        });

        handles.push((agent_id, handle));
    }

    // ── Phase 2: collect results as tasks complete ────────────────────────────
    let mut ok_results: Vec<(String, String)> = Vec::new(); // (agent_id, result)
    let mut err_results: Vec<(String, String)> = Vec::new(); // (agent_id, error)

    for (agent_id, handle) in handles {
        let short = agent_id.chars().take(8).collect::<String>();
        match handle.await {
            Ok((_id, Ok(result))) => {
                info!(agent_id = %agent_id, "fork_agent: task succeeded");
                ok_results.push((short, result));
            }
            Ok((_id, Err(e))) => {
                error!(agent_id = %agent_id, error = %e, "fork_agent: task failed");
                err_results.push((short, e.to_string()));
            }
            Err(join_err) => {
                // tokio task panicked
                error!(agent_id = %agent_id, "fork_agent: task panicked: {}", join_err);
                err_results.push((short, format!("task panicked: {}", join_err)));
            }
        }
    }

    // ── Phase 3: format structured summary ─────────────────────────────────────
    let success_count = ok_results.len();
    let fail_count = err_results.len();
    info!(
        succeeded = success_count,
        failed = fail_count,
        "fork_agent: all tasks complete"
    );

    if success_count == 0 {
        // All failed — surface as an error so the LLM can react.
        let details: Vec<String> = err_results
            .into_iter()
            .map(|(id, e)| format!("[{}] {}", id, e))
            .collect();
        return Err(anyhow!(
            "All {} forked sub-agents failed:\n{}",
            fail_count,
            details.join("\n")
        ));
    }

    let mut output = format!(
        "## Fork Results ({}/{} succeeded)\n\n",
        success_count, total
    );
    for (short_id, result) in &ok_results {
        output.push_str(&format!(
            "### Agent `{}` ✅\n{}\n\n---\n\n",
            short_id, result
        ));
    }
    for (short_id, err) in &err_results {
        output.push_str(&format!(
            "### Agent `{}` ❌\nError: {}\n\n---\n\n",
            short_id, err
        ));
    }

    Ok(output)
}

/// Collect and merge results from previously spawned/forked sub-agents.
///
/// Looks up each `agent_id` in the `AgentManager`.  Reports results for
/// completed agents, errors for failed ones, and "still running" for any that
/// haven't finished yet.  If all requested agents are still running, returns
/// the status table so the LLM knows to wait or retry.
pub async fn join_agents(agent_ids: Vec<String>) -> Result<String> {
    if agent_ids.is_empty() {
        return Ok("No agent IDs provided to join.".to_string());
    }

    let manager = get_agent_manager();
    let mut completed: Vec<String> = Vec::new();
    let mut still_running: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for id in &agent_ids {
        match manager.get(id).await {
            Some(agent) => match agent.status {
                crate::agent::manager::AgentStatus::Completed => {
                    crate::agent::activity::emit_agent_activity(
                        id,
                        None,
                        crate::acp::protocol::AgentActivityStatus::Joined,
                        "Results collected",
                    );
                    let result = agent.result.unwrap_or_else(|| "(empty result)".to_string());
                    let short = id.chars().take(8).collect::<String>();
                    completed.push(format!("### Agent `{}` ✅\n{}\n", short, result));
                }
                crate::agent::manager::AgentStatus::Failed => {
                    let err = agent.result.unwrap_or_else(|| "unknown error".to_string());
                    let short = id.chars().take(8).collect::<String>();
                    failed.push(format!("### Agent `{}` ❌\nError: {}\n", short, err));
                }
                crate::agent::manager::AgentStatus::Running => {
                    let short = id.chars().take(8).collect::<String>();
                    still_running.push(format!("`{}` ⏳ still running", short));
                }
                crate::agent::manager::AgentStatus::Cancelled => {
                    let short = id.chars().take(8).collect::<String>();
                    failed.push(format!("### Agent `{}` ⚫\nCancelled.\n", short));
                }
            },
            None => {
                let short = id.chars().take(8).collect::<String>();
                failed.push(format!(
                    "### Agent `{}` ❓\nNot found in registry.\n",
                    short
                ));
            }
        }
    }

    if completed.is_empty() && failed.is_empty() {
        return Ok(format!(
            "All {} requested agents are still running: {}\n\
             Call `join_agents` again when they finish, or `get_agent_status` to poll individually.",
            still_running.len(),
            still_running.join(", ")
        ));
    }

    let total = agent_ids.len();
    let ok_count = completed.len();
    let mut output = format!("## Join Results ({}/{} completed)\n\n", ok_count, total);
    for r in &completed {
        output.push_str(r);
        output.push_str("\n---\n\n");
    }
    for r in &failed {
        output.push_str(r);
        output.push_str("\n---\n\n");
    }
    if !still_running.is_empty() {
        output.push_str(&format!(
            "**Still running:** {}\n",
            still_running.join(", ")
        ));
    }

    Ok(output)
}

/// Delegate a plan step to a child sub-agent with proper parent tracking.
///
/// Registers the child agent under `parent_id` in the `AgentManager` (so
/// `list_agents(Some(parent_id))` correctly returns it), then calls
/// `call_subagent_api` directly instead of `spawn_agent` to avoid creating
/// a second untracked entry in the registry.
pub async fn delegate_plan_step(task: &str, parent_id: Option<&str>) -> Result<String> {
    if task.trim().is_empty() {
        return Err(anyhow!("delegate_plan_step: task cannot be empty"));
    }

    let manager = get_agent_manager();
    let agent_id = manager
        .spawn(
            task,
            parent_id.map(|s| s.to_string()),
            Some("grok-3-mini".to_string()),
            None,
        )
        .await;

    crate::agent::activity::emit_agent_activity(
        &agent_id,
        parent_id.map(|s| s.to_string()),
        crate::acp::protocol::AgentActivityStatus::Spawned,
        format!(
            "Delegated plan step: {}",
            &task.chars().take(80).collect::<String>()
        ),
    );
    info!(
        agent_id = %agent_id,
        parent_id = ?parent_id,
        task = %task,
        "delegate_plan_step: starting"
    );

    // Use run_agent_session directly so the AgentManager only has one entry
    // for this task (the one we just created above with the correct parent_id).
    let config = SubAgentConfig::builder().max_tokens(1024).build();
    match run_agent_session(task, "", &config).await {
        Ok(result) => {
            manager.complete(&agent_id, result.clone()).await;
            crate::agent::activity::emit_agent_activity(
                &agent_id,
                parent_id.map(|s| s.to_string()),
                crate::acp::protocol::AgentActivityStatus::Joined,
                "Plan step completed",
            );
            info!(agent_id = %agent_id, "delegate_plan_step: completed");
            Ok(result)
        }
        Err(e) => {
            manager.fail(&agent_id, e.to_string()).await;
            error!(agent_id = %agent_id, error = %e, "delegate_plan_step: failed");
            Err(e)
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn spawn_agent_requires_non_empty_task() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(spawn_agent("", "", 512));
        assert!(r.is_err());
    }

    #[test]
    fn send_message_requires_non_empty_target() {
        let r = send_message("", "hello");
        assert!(r.is_err());
    }

    #[test]
    fn send_message_requires_non_empty_message() {
        let r = send_message("target_agent", "");
        assert!(r.is_err());
    }

    #[test]
    #[serial]
    fn send_message_writes_to_file() {
        let r = send_message("test_agent_unit", "ping");
        assert!(r.is_ok(), "{:?}", r);
    }

    #[test]
    fn merge_agent_results_empty_returns_ok() {
        let r = merge_agent_results(vec![]);
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), "No agent results to merge.");
    }

    #[test]
    fn merge_agent_results_single_passthrough() {
        let r = merge_agent_results(vec!["only one".to_string()]);
        assert_eq!(r.unwrap(), "only one");
    }

    #[test]
    fn merge_agent_results_prefers_longer() {
        let short = "hi".to_string();
        let long = "a".repeat(500);
        let r = merge_agent_results(vec![short, long.clone()]);
        assert!(r.unwrap().contains(&long));
    }

    #[test]
    #[serial]
    fn team_create_and_delete_roundtrip() {
        let name = format!("test_team_{}", Utc::now().timestamp_millis());
        team_create(
            &name,
            vec!["alice".to_string(), "bob".to_string()],
            "Test team",
        )
        .unwrap();
        let r = team_delete(&name);
        assert!(r.is_ok(), "delete failed: {:?}", r);
    }

    #[test]
    #[serial]
    fn team_create_duplicate_returns_error() {
        let name = format!("dup_team_{}", Utc::now().timestamp_millis());
        team_create(&name, vec![], "first").unwrap();
        let r = team_create(&name, vec![], "second");
        // Clean up regardless of assertion outcome.
        let _ = team_delete(&name);
        assert!(r.is_err());
    }

    #[test]
    #[serial]
    fn team_delete_nonexistent_returns_error() {
        let r = team_delete("no_such_team_xyz_abc_123");
        assert!(r.is_err());
    }
}
