//! Discovery tools — tool search, cron scheduling, and remote triggers.
//!
//! All network calls are built with 30-second timeouts and retry-friendly
//! error messages to survive Starlink satellite handover drops.

use anyhow::{Result, anyhow};
use chrono::Utc;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

// ── helpers ───────────────────────────────────────────────────────────────────

fn grok_data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Cannot determine local data directory"))?
        .join(".grok");
    fs::create_dir_all(&dir)
        .map_err(|e| anyhow!("Failed to create .grok data directory: {}", e))?;
    Ok(dir)
}

// ── tool_search ───────────────────────────────────────────────────────────────

/// Search the tool registry for tools whose name or description matches the
/// query string (case-insensitive substring match).
///
/// Returns a formatted list of matching tools with their descriptions, or a
/// "no results" message when nothing matches.
pub fn tool_search(query: &str) -> Result<String> {
    if query.trim().is_empty() {
        return Err(anyhow!("query cannot be empty"));
    }

    let query_lower = query.to_lowercase();
    let all_tools = crate::tools::registry::get_full_tool_definitions();

    let matches: Vec<String> = all_tools
        .iter()
        .filter_map(|v| {
            let func = v.get("function")?;
            let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let desc = func
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            if name.to_lowercase().contains(&query_lower)
                || desc.to_lowercase().contains(&query_lower)
            {
                Some(format!("{}: {}", name, desc))
            } else {
                None
            }
        })
        .collect();

    if matches.is_empty() {
        Ok(format!(
            "No tools found matching '{}'.\nTry a broader keyword.",
            query
        ))
    } else {
        Ok(format!(
            "Found {} tool(s) matching '{}':\n{}",
            matches.len(),
            query,
            matches.join("\n")
        ))
    }
}

// ── cron_create ───────────────────────────────────────────────────────────────

/// Register a scheduled trigger in `{data_dir}/.grok/crons.json`.
///
/// `schedule` must be a valid 5-field cron expression
/// (minute hour day-of-month month day-of-week), e.g. `"0 9 * * 1-5"`.
/// An existing entry with the same name is replaced.
///
/// Note: Grok-CLI does not run a background scheduler; this function records
/// the intent.  An external runner (cron daemon, Task Scheduler, CI) must
/// read `crons.json` and invoke `grok` accordingly.
pub fn cron_create(name: &str, schedule: &str, task: &str) -> Result<String> {
    if name.trim().is_empty() {
        return Err(anyhow!("name cannot be empty"));
    }
    if task.trim().is_empty() {
        return Err(anyhow!("task cannot be empty"));
    }

    // Basic 5-field cron validation
    let parts: Vec<&str> = schedule.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(anyhow!(
            "Invalid cron expression '{}': must have exactly 5 fields \
             (minute hour day-of-month month day-of-week). Example: '0 9 * * 1-5'",
            schedule
        ));
    }

    let cron_file = grok_data_dir()?.join("crons.json");

    let mut data: Value = if cron_file.exists() {
        let content = fs::read_to_string(&cron_file)
            .map_err(|e| anyhow!("Failed to read crons.json: {}", e))?;
        serde_json::from_str(&content).unwrap_or(json!({ "crons": [] }))
    } else {
        json!({ "crons": [] })
    };

    let crons = data["crons"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("Invalid crons.json: missing 'crons' array"))?;

    // Upsert: remove existing entry with same name
    crons.retain(|e| e["name"].as_str() != Some(name));

    crons.push(json!({
        "name":       name,
        "schedule":   schedule,
        "task":       task,
        "created_at": Utc::now().to_rfc3339(),
    }));

    fs::write(&cron_file, serde_json::to_string_pretty(&data)?)
        .map_err(|e| anyhow!("Failed to write crons.json: {}", e))?;

    Ok(format!(
        "Cron job '{}' registered: `{}` → {}",
        name, schedule, task
    ))
}

// ── remote_trigger ────────────────────────────────────────────────────────────

/// Fire an HTTP trigger to a remote endpoint.
///
/// `method` must be one of `"POST"`, `"GET"`, or `"PUT"` (case-insensitive).
/// The `payload` is sent as a JSON body for POST / PUT requests and is ignored
/// for GET.  A 30-second timeout is applied; returns an error on non-2xx
/// responses so the agent can react.
pub async fn remote_trigger(endpoint: &str, payload: Value, method: &str) -> Result<String> {
    if endpoint.trim().is_empty() {
        return Err(anyhow!("endpoint cannot be empty"));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let response = match method.to_uppercase().as_str() {
        "GET" => client.get(endpoint).send().await,
        "PUT" => client.put(endpoint).json(&payload).send().await,
        _ => client.post(endpoint).json(&payload).send().await,
    }
    .map_err(|e| {
        anyhow!(
            "Failed to reach '{}': {}\n\
            This may be a Starlink handover drop — retry in a few seconds.",
            endpoint,
            e
        )
    })?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if status.is_success() {
        Ok(format!("Trigger succeeded ({status}): {body}"))
    } else {
        Err(anyhow!("Trigger failed ({status}): {body}"))
    }
}

// ── list_tools ─────────────────────────────────────────────────────────────

/// Return a list of all available tools with their names and descriptions.
pub fn list_tools() -> Result<String> {
    let all_tools = crate::tools::registry::get_full_tool_definitions();
    let list: Vec<String> = all_tools
        .iter()
        .filter_map(|v| {
            let func = v.get("function")?;
            let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("?");
            let desc = func
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            Some(format!("{}: {}", name, desc))
        })
        .collect();
    Ok(list.join("\n"))
}

// ── describe_tool ────────────────────────────────────────────────────────────

/// Return the full JSON schema for a specific tool.
pub fn describe_tool(name: &str) -> Result<String> {
    let all_tools = crate::tools::registry::get_full_tool_definitions();
    for v in all_tools {
        if let Some(n) = v
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            && n == name
        {
            return Ok(serde_json::to_string_pretty(&v).unwrap());
        }
    }
    Err(anyhow!("Tool '{}' not found", name))
}

// ── tool_examples ────────────────────────────────────────────────────────────

/// Return usage examples for a specific tool.
pub fn tool_examples(name: &str) -> Result<String> {
    // Hardcoded examples for some tools
    match name {
        "read_file" => Ok(r##"Examples for read_file:
1. Read a configuration file: {"path": "config.toml"}
2. Read a source code file: {"path": "src/main.rs"}"##
            .to_string()),
        "write_file" => Ok(r##"Examples for write_file:
1. Create a new file: {"path": "notes.txt", "content": "My notes"}
2. Update an existing file: {"path": "README.md", "content": "# Updated Title\n\nNew content"}"##
            .to_string()),
        "run_shell_command" => Ok(r##"Examples for run_shell_command:
1. List files: {"command": "ls -la"}
2. Check git status: {"command": "git status"}"##
            .to_string()),
        "web_search" => Ok(r##"Examples for web_search:
1. Search for Rust documentation: {"query": "rust async tutorial"}
2. Search for news: {"query": "latest AI news"}"##
            .to_string()),
        _ => Ok(format!("No examples available for tool '{}'", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_search_finds_read_file() {
        let result = tool_search("read").unwrap();
        assert!(
            result.contains("read_file"),
            "expected read_file in: {result}"
        );
    }

    #[test]
    fn tool_search_empty_query_returns_error() {
        assert!(tool_search("").is_err());
    }

    #[test]
    fn tool_search_no_match() {
        let result = tool_search("zzz_no_such_tool_xyz").unwrap();
        assert!(result.contains("No tools found"));
    }

    #[test]
    fn cron_create_invalid_expression_returns_error() {
        let r = cron_create("test", "not a valid cron", "echo hi");
        assert!(r.is_err());
    }

    #[test]
    fn cron_create_valid_expression_succeeds() {
        let r = cron_create("test_daily", "0 9 * * *", "grok chat 'daily report'");
        assert!(r.is_ok(), "{:?}", r);
        assert!(r.unwrap().contains("test_daily"));
    }

    #[tokio::test]
    async fn remote_trigger_invalid_url_returns_error() {
        let r = remote_trigger("not-a-url", json!({}), "POST").await;
        assert!(r.is_err());
    }
}
