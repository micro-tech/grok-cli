//! Agent coordination tools — sub-agent spawning, inter-agent messaging,
//! and team management.

use crate::agent::manager::AgentManager;
use anyhow::{Result, anyhow};
use chrono::Utc;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;

// ── Global shared AgentManager (Task 127) ─────────────────────────────────────

static AGENT_MANAGER: once_cell::sync::Lazy<Arc<AgentManager>> =
    once_cell::sync::Lazy::new(AgentManager::new);

/// Returns the global shared AgentManager instance.
pub fn get_agent_manager() -> Arc<AgentManager> {
    AGENT_MANAGER.clone()
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn grok_data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Cannot determine local data directory"))?
        .join(".grok");
    fs::create_dir_all(&dir).map_err(|e| anyhow!("Failed to create .grok directory: {}", e))?;
    Ok(dir)
}

// ── spawn_agent ───────────────────────────────────────────────────────────────

/// Spawn a focused sub-agent to complete a well-scoped task.
///
/// This version (Task 127) also registers the sub-agent with the global
/// `AgentManager` so the system can track, list, and merge results from
/// multiple concurrent sub-agents.
pub async fn spawn_agent(task: &str, context: &str, max_tokens: u32) -> Result<String> {
    if task.trim().is_empty() {
        warn!("spawn_agent: task is empty");
        return Err(anyhow!("task cannot be empty"));
    }

    // Register with AgentManager first
    let manager = get_agent_manager();
    let agent_id = manager.spawn(task, None, Some("grok-3-mini".to_string()), None).await;

    // Task 128.3 — emit activity
    crate::agent::activity::emit_agent_activity(
        &agent_id,
        None,
        crate::acp::protocol::AgentActivityStatus::Spawned,
        format!("Spawned for task: {}", task),
    );

    let api_key = std::env::var("GROK_API_KEY")
        .or_else(|_| std::env::var("XAI_API_KEY"))
        .map_err(|_| {
            anyhow!("No API key found. Set the GROK_API_KEY or XAI_API_KEY environment variable.")
        })?;

    let router = crate::router::AppRouter::new(&api_key, 60)
        .map_err(|e| anyhow!("Failed to initialise router: {}", e))?;

    let prompt = if context.trim().is_empty() {
        task.to_string()
    } else {
        format!("{}\n\n## Context\n{}", task, context)
    };

    let clamped_tokens = max_tokens.clamp(256, 4096);

    const MAX_RETRIES: u32 = 3;
    for attempt in 0..=MAX_RETRIES {
        match router
            .chat_completion(
                &prompt,
                Some(
                    "You are a focused sub-agent. Complete the given task as concisely and \
                     directly as possible. Return only the result — no preamble, no meta-commentary.",
                ),
                0.7,
                clamped_tokens,
                "grok-3-mini",
            )
            .await
            .map_err(|e| anyhow!("Sub-agent call failed: {}", e))
        {
            Ok(result) => {
                // Mark as completed in the manager
                manager.complete(&agent_id, result.clone()).await;
                return Ok(result);
            }
            Err(e) if attempt < MAX_RETRIES && crate::utils::network::detect_network_drop(&e) => {
                let delay = crate::utils::network::calculate_retry_delay(attempt, false);
                warn!(
                    attempt = attempt + 1,
                    max_attempts = MAX_RETRIES + 1,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "spawn_agent: network error — retrying after delay"
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                manager.fail(&agent_id, e.to_string()).await;
                warn!(error = %e, "spawn_agent: sub-agent call failed — no more retries");
                return Err(e);
            }
        }
    }
    unreachable!()
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

/// Fork / spawn multiple sub-agents in parallel for different parts of a task.
pub async fn fork_agent(tasks: Vec<String>) -> Result<String> {
    let manager = get_agent_manager();
    let mut ids = Vec::new();

    for task in tasks {
        let id = manager.spawn(&task, None, None, None).await;
        ids.push(id.clone());

        // Task 128.3
        crate::agent::activity::emit_agent_activity(
            &id,
            None,
            crate::acp::protocol::AgentActivityStatus::Forked,
            format!("Forked task: {}", task),
        );
    }

    Ok(format!("Forked {} sub-agents: {:?}", ids.len(), ids))
}

/// Join results from multiple sub-agents (waits conceptually by checking status).
pub async fn join_agents(agent_ids: Vec<String>) -> Result<String> {
    let manager = get_agent_manager();
    let mut results = Vec::new();

    for id in agent_ids {
        if let Some(agent) = manager.get(&id).await {
            if let Some(res) = agent.result {
                results.push(res);
            }
        }

        // Task 128.3 — treat join as a lifecycle event
        crate::agent::activity::emit_agent_activity(
            &id,
            None,
            crate::acp::protocol::AgentActivityStatus::Joined,
            "Joined results".to_string(),
        );
    }

    if results.is_empty() {
        return Ok("No completed results found yet.".to_string());
    }

    merge_agent_results(results)
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
