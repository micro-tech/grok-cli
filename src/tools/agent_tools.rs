//! Agent coordination tools — sub-agent spawning, inter-agent messaging,
//! and team management.

use anyhow::{Result, anyhow};
use chrono::Utc;
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

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
/// Reads the API key from the `GROK_API_KEY` or `XAI_API_KEY` environment
/// variable (set by the user's `.env` file or shell).  Calls the Grok API
/// with a tight system prompt that instructs the sub-agent to return a
/// concise, direct result.
///
/// `max_tokens` is clamped to the range 256–4096.
pub async fn spawn_agent(task: &str, context: &str, max_tokens: u32) -> Result<String> {
    if task.trim().is_empty() {
        return Err(anyhow!("task cannot be empty"));
    }

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

    router
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
}

// ── send_message ──────────────────────────────────────────────────────────────

/// Send a message to a named target (agent ID or channel).
///
/// Messages are appended to `{data_dir}/.grok/messages/{target}.jsonl` as
/// JSON Lines.  The target name is sanitised so it is safe as a file name.
pub fn send_message(target: &str, message: &str) -> Result<String> {
    if target.trim().is_empty() {
        return Err(anyhow!("target cannot be empty"));
    }
    if message.trim().is_empty() {
        return Err(anyhow!("message cannot be empty"));
    }

    let msg_dir = grok_data_dir()?.join("messages");
    fs::create_dir_all(&msg_dir)
        .map_err(|e| anyhow!("Failed to create messages directory: {}", e))?;

    // Sanitise target name for safe use as a filename
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

    let msg_file = msg_dir.join(format!("{}.jsonl", safe_target));

    let entry = json!({
        "timestamp": Utc::now().to_rfc3339(),
        "from":      "grok",
        "to":        target,
        "message":   message,
    });

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&msg_file)
        .map_err(|e| anyhow!("Failed to open message file: {}", e))?;

    writeln!(file, "{}", entry).map_err(|e| anyhow!("Failed to write message: {}", e))?;

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
        return Err(anyhow!("team name cannot be empty"));
    }

    let teams_file = grok_data_dir()?.join("teams.json");

    if !teams_file.exists() {
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
        return Err(anyhow!("Team '{}' not found.", name));
    }

    fs::write(&teams_file, serde_json::to_string_pretty(&data)?)
        .map_err(|e| anyhow!("Failed to write teams.json: {}", e))?;

    Ok(format!("Team '{}' deleted.", name))
}

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
        // Clean up
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
