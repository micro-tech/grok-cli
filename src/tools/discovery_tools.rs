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
        .ok_or_else(|| {
            let e = anyhow!("Cannot determine local data directory");
            tracing::warn!(
                error = %e,
                "discovery_tools: failed to locate local data directory"
            );
            e
        })?
        .join(".grok");
    fs::create_dir_all(&dir).map_err(|e| {
        let err = anyhow!("Failed to create .grok data directory: {}", e);
        tracing::warn!(
            path  = %dir.display(),
            error = %err,
            "discovery_tools: failed to create .grok data directory"
        );
        err
    })?;
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
        let e = anyhow!("query cannot be empty");
        tracing::warn!(
            error = %e,
            "discovery_tools: tool_search called with empty query"
        );
        return Err(e);
    }

    let query_lower = query.to_lowercase();
    let all_tools = crate::tools::registry::get_tool_definitions();

    let matches: Vec<String> = all_tools
        .iter()
        .filter(|t| {
            let name = t
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let desc = t
                .get("function")
                .and_then(|f| f.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or("");
            name.to_lowercase().contains(&query_lower) || desc.to_lowercase().contains(&query_lower)
        })
        .map(|t| {
            let name = t
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("?");
            let desc = t
                .get("function")
                .and_then(|f| f.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or("");
            format!("  {:<30} {}", name, desc)
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
        let e = anyhow!("name cannot be empty");
        tracing::warn!(error = %e, "discovery_tools: cron_create called with empty name");
        return Err(e);
    }
    if task.trim().is_empty() {
        let e = anyhow!("task cannot be empty");
        tracing::warn!(error = %e, "discovery_tools: cron_create called with empty task");
        return Err(e);
    }

    // Basic 5-field cron validation
    let parts: Vec<&str> = schedule.split_whitespace().collect();
    if parts.len() != 5 {
        let e = anyhow!(
            "Invalid cron expression '{}': must have exactly 5 fields \
             (minute hour day-of-month month day-of-week). Example: '0 9 * * 1-5'",
            schedule
        );
        tracing::warn!(
            schedule = schedule,
            fields   = parts.len(),
            error    = %e,
            "discovery_tools: invalid cron expression"
        );
        return Err(e);
    }

    let cron_file = grok_data_dir()?.join("crons.json");

    let mut data: Value = if cron_file.exists() {
        let content = fs::read_to_string(&cron_file).map_err(|e| {
            let err = anyhow!("Failed to read crons.json: {}", e);
            tracing::warn!(
                path  = %cron_file.display(),
                error = %err,
                "discovery_tools: failed to read crons.json"
            );
            err
        })?;
        serde_json::from_str(&content).unwrap_or(json!({ "crons": [] }))
    } else {
        json!({ "crons": [] })
    };

    let crons = data["crons"].as_array_mut().ok_or_else(|| {
        let e = anyhow!("Invalid crons.json: missing 'crons' array");
        tracing::warn!(
            path  = %cron_file.display(),
            error = %e,
            "discovery_tools: malformed crons.json — missing 'crons' array"
        );
        e
    })?;

    // Upsert: remove existing entry with same name
    crons.retain(|e| e["name"].as_str() != Some(name));

    crons.push(json!({
        "name":       name,
        "schedule":   schedule,
        "task":       task,
        "created_at": Utc::now().to_rfc3339(),
    }));

    fs::write(&cron_file, serde_json::to_string_pretty(&data)?).map_err(|e| {
        let err = anyhow!("Failed to write crons.json: {}", e);
        tracing::warn!(
            path  = %cron_file.display(),
            error = %err,
            "discovery_tools: failed to write crons.json"
        );
        err
    })?;

    Ok(format!(
        "Cron job '{}' registered: `{}` → {}",
        name, schedule, task
    ))
}

// ── remote_trigger ────────────────────────────────────────────────────────────

/// Fire an HTTP trigger to a remote endpoint.
///
/// `method` must be one of `"POST"`, `"GET"`, or `"PUT"` (case-insensitive).
/// Any other value returns an error immediately.
///
/// The `payload` is sent as a JSON body for POST / PUT; ignored for GET.
/// A 30-second per-attempt timeout is applied via the `reqwest` client.
///
/// On network drops (detected via [`crate::utils::network::detect_network_drop`])
/// the call is retried up to **3 times** with exponential back-off before the
/// error is surfaced. Non-2xx responses are surfaced as errors immediately
/// without retrying (the remote endpoint made a deliberate choice).
pub async fn remote_trigger(endpoint: &str, payload: Value, method: &str) -> Result<String> {
    if endpoint.trim().is_empty() {
        let e = anyhow!("endpoint cannot be empty");
        tracing::warn!(
            error = %e,
            "discovery_tools: remote_trigger called with empty endpoint"
        );
        return Err(e);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            let err = anyhow!("Failed to build HTTP client: {}", e);
            tracing::warn!(
                error = %err,
                "discovery_tools: failed to build reqwest client"
            );
            err
        })?;

<<<<<<< HEAD
    let response = match method.to_uppercase().as_str() {
        "GET" => client.get(endpoint).send().await,
        "PUT" => client.put(endpoint).json(&payload).send().await,
        _ => client.post(endpoint).json(&payload).send().await,
=======
    // ── method validation ─────────────────────────────────────────────────────
    // Reject unknown methods immediately, before spending any network budget.
    let method_upper = method.to_ascii_uppercase();
    match method_upper.as_str() {
        "GET" | "POST" | "PUT" => {}
        other => {
            let e = anyhow::anyhow!(
                "remote_trigger: unsupported HTTP method '{}'. \
                 Allowed: GET, POST, PUT.",
                other
            );
            tracing::warn!(
                method = other,
                error  = %e,
                "discovery_tools: remote_trigger — unsupported HTTP method"
            );
            return Err(e);
        }
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a
    }

    // ── retry loop ────────────────────────────────────────────────────────────
    // Up to MAX_RETRIES extra attempts on network drops; non-network errors and
    // non-2xx HTTP responses are returned immediately without retrying.
    const MAX_RETRIES: u32 = 3;

    for attempt in 0u32..=MAX_RETRIES {
        // Build a fresh RequestBuilder every iteration (RequestBuilder is not Clone).
        let request = match method_upper.as_str() {
            "GET" => client.get(endpoint),
            "POST" => client.post(endpoint).json(&payload),
            "PUT" => client.put(endpoint).json(&payload),
            // SAFETY: already validated above; this branch is unreachable.
            _ => unreachable!("HTTP method was validated before the retry loop"),
        };

        // Send and interpret the response inside an async block so we get a
        // single Result<String> to pattern-match against.
        let outcome: Result<String> = async {
            let response = request.send().await.map_err(|e| {
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
                let e = anyhow!("Trigger failed ({status}): {body}");
                tracing::warn!(
                    endpoint = endpoint,
                    status   = status.as_u16(),
                    error    = %e,
                    "discovery_tools: remote_trigger received non-2xx response"
                );
                Err(e)
            }
        }
        .await;

        match outcome {
            Ok(response_text) => return Ok(response_text),

            Err(e) if attempt < MAX_RETRIES && crate::utils::network::detect_network_drop(&e) => {
                let delay = crate::utils::network::calculate_retry_delay(attempt, false);
                tracing::warn!(
                    attempt  = attempt + 1,
                    delay_ms = delay.as_millis() as u64,
                    error    = %e,
                    "discovery_tools: remote_trigger — network error, retrying"
                );
                tokio::time::sleep(delay).await;
            }

            Err(e) => {
                tracing::warn!(
                    endpoint = endpoint,
                    attempt  = attempt + 1,
                    error    = %e,
                    "discovery_tools: remote_trigger — failed, no more retries"
                );
                return Err(e);
            }
        }
    }

    // The loop runs for attempt in 0..=MAX_RETRIES.  Every iteration either
    // returns Ok, sleeps and continues, or returns Err.  After MAX_RETRIES+1
    // iterations we must have returned, so this point is unreachable.
    unreachable!("retry loop exhausted without returning")
}

// ── list_tools ─────────────────────────────────────────────────────────────

/// Return a list of all available tools with their names and descriptions.
pub fn list_tools() -> Result<String> {
    let all_tools = crate::tools::registry::get_tool_definitions();
    let list: Vec<String> = all_tools
        .iter()
        .map(|t| {
            let name = t
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("?");
            let desc = t
                .get("function")
                .and_then(|f| f.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or("");
            format!("{}: {}", name, desc)
        })
        .collect();
    Ok(list.join("\n"))
}

// ── describe_tool ────────────────────────────────────────────────────────────

/// Return the full JSON schema for a specific tool.
pub fn describe_tool(name: &str) -> Result<String> {
    let all_tools = crate::tools::registry::get_tool_definitions();
    for t in all_tools {
        if let Some(n) = t
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            && n == name
        {
            return Ok(serde_json::to_string_pretty(&t).unwrap());
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

    #[tokio::test]
    async fn remote_trigger_unknown_method_returns_error() {
        let r = remote_trigger("http://localhost", json!({}), "PATCH").await;
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("unsupported HTTP method"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    async fn remote_trigger_empty_endpoint_returns_error() {
        let r = remote_trigger("", json!({}), "POST").await;
        assert!(r.is_err());
    }
}
