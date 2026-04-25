//! Plan mode and git worktree isolation tools.
//!
//! **Plan mode** is a lightweight state flag stored in `{data_dir}/.grok/state.json`.
//! When active, the agent is expected to outline a multi-step plan before
//! executing any changes.
//!
//! **Worktree tools** manage `git worktree` entries so that experimental
//! changes can be made in isolation without affecting the main branch.

use crate::acp::security::SecurityPolicy;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

// ── state file helpers ────────────────────────────────────────────────────────

fn grok_state_path() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Cannot determine local data directory"))?
        .join(".grok");
    fs::create_dir_all(&dir)
        .map_err(|e| anyhow!("Failed to create .grok state directory: {}", e))?;
    Ok(dir.join("state.json"))
}

fn load_state() -> Value {
    grok_state_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!({}))
}

fn save_state(state: &Value) -> Result<()> {
    let path = grok_state_path()?;
    fs::write(path, serde_json::to_string_pretty(state)?)
        .map_err(|e| anyhow!("Failed to save state: {}", e))
}

// ── plan mode ─────────────────────────────────────────────────────────────────

/// Enter plan mode.
///
/// Sets the `plan_mode` flag in the state file.  While in plan mode the
/// agent should outline every step before making any file or shell changes.
pub fn enter_plan_mode() -> Result<String> {
    let mut state = load_state();
    state["plan_mode"] = json!(true);
    state["plan_mode_entered_at"] = json!(chrono::Utc::now().to_rfc3339());
    save_state(&state)?;
    Ok("Plan mode ACTIVE. Outline your full plan before executing any changes.".to_string())
}

/// Exit plan mode and begin executing the current plan.
pub fn exit_plan_mode() -> Result<String> {
    let mut state = load_state();
    state["plan_mode"] = json!(false);
    state["plan_mode_entered_at"] = serde_json::Value::Null;
    save_state(&state)?;
    Ok("Plan mode INACTIVE. Proceeding with execution.".to_string())
}

// ── git worktree ──────────────────────────────────────────────────────────────

/// Create a git worktree at `path` on the given `branch`.
///
/// If the branch does not exist it is created with `-b`.  The worktree path
/// and branch are saved to the state file so [`exit_worktree`] can find them.
///
/// Branch names that start with `-` or contain `..` are rejected to prevent
/// argument injection and path traversal.
pub async fn enter_worktree(branch: &str, path: &str, security: &SecurityPolicy) -> Result<String> {
    if branch.trim().is_empty() {
        tracing::warn!("plan_tools::enter_worktree: rejected — branch name is empty");
        return Err(anyhow!("branch cannot be empty"));
    }

    // Sanitize: reject names that could be misinterpreted as git flags or
    // that contain path-traversal sequences.
    if branch.contains("..") || branch.starts_with('-') {
        tracing::warn!(
            branch = %branch,
            "plan_tools::enter_worktree: rejected unsafe branch name"
        );
        return Err(anyhow!(
            "enter_worktree: unsafe branch name '{}'. \
             Must not start with '-' or contain '..'.",
            branch
        ));
    }

    if path.trim().is_empty() {
        tracing::warn!("plan_tools::enter_worktree: rejected — worktree path is empty");
        return Err(anyhow!("path cannot be empty"));
    }

    let worktree_path = if std::path::Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        security.working_directory().join(path)
    };

    let cwd = security.working_directory().to_path_buf();

    // Try `git worktree add <path> <branch>` first; if the branch doesn't
    // exist fall back to `git worktree add -b <branch> <path>`.
    let output = timeout(
        Duration::from_secs(30),
        Command::new("git")
            .args([
                "worktree",
                "add",
                &worktree_path.display().to_string(),
                branch,
            ])
            .current_dir(&cwd)
            .output(),
    )
    .await
    .map_err(|_| {
        tracing::warn!(
            branch = %branch,
            "plan_tools::enter_worktree: git worktree add timed out"
        );
        anyhow!("git worktree add timed out")
    })?
    .map_err(|e| {
        tracing::warn!(error = %e, "plan_tools::enter_worktree: failed to spawn git");
        anyhow!("Failed to spawn git: {}", e)
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If the branch doesn't exist, try creating it
        if stderr.contains("invalid reference") || stderr.contains("not a valid") {
            let output2 = timeout(
                Duration::from_secs(30),
                Command::new("git")
                    .args([
                        "worktree",
                        "add",
                        "-b",
                        branch,
                        &worktree_path.display().to_string(),
                    ])
                    .current_dir(&cwd)
                    .output(),
            )
            .await
            .map_err(|_| {
                tracing::warn!(
                    branch = %branch,
                    "plan_tools::enter_worktree: git worktree add -b timed out"
                );
                anyhow!("git worktree add -b timed out")
            })?
            .map_err(|e| {
                tracing::warn!(error = %e, "plan_tools::enter_worktree: failed to spawn git (create branch)");
                anyhow!("Failed to spawn git: {}", e)
            })?;

            if !output2.status.success() {
                let err2 = String::from_utf8_lossy(&output2.stderr);
                tracing::warn!(
                    branch = %branch,
                    stderr = %err2,
                    "plan_tools::enter_worktree: git worktree add -b failed"
                );
                return Err(anyhow!("Failed to create worktree: {}", err2));
            }
        } else {
            tracing::warn!(
                branch = %branch,
                stderr = %stderr,
                "plan_tools::enter_worktree: git worktree add failed"
            );
            return Err(anyhow!("Failed to create worktree: {}", stderr));
        }
    }

    // Persist worktree state — failure is non-fatal but we log it so the
    // operator knows the next call may see stale state data.
    let mut state = load_state();
    state["worktree"] = json!({
        "branch": branch,
        "path":   worktree_path.display().to_string(),
    });
    if let Err(e) = save_state(&state) {
        tracing::warn!(
            error = %e,
            "plan_tools: failed to persist state — next call may see stale data"
        );
    }

    Ok(format!(
        "Worktree created at '{}' on branch '{}'.",
        worktree_path.display(),
        branch
    ))
}

/// Remove the active git worktree and optionally merge its branch.
///
/// Reads the worktree location from the state file written by
/// [`enter_worktree`].
pub async fn exit_worktree(merge: bool, security: &SecurityPolicy) -> Result<String> {
    let state = load_state();
    let worktree_path = state["worktree"]["path"]
        .as_str()
        .ok_or_else(|| {
            tracing::warn!("plan_tools::exit_worktree: no active worktree found in state");
            anyhow!("No active worktree found. Call enter_worktree first.")
        })?
        .to_string();
    let branch = state["worktree"]["branch"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let cwd = security.working_directory().to_path_buf();
    let mut messages = Vec::new();

    // Remove worktree
    let rm = timeout(
        Duration::from_secs(30),
        Command::new("git")
            .args(["worktree", "remove", &worktree_path, "--force"])
            .current_dir(&cwd)
            .output(),
    )
    .await
    .map_err(|_| {
        tracing::warn!(
            path = %worktree_path,
            "plan_tools::exit_worktree: git worktree remove timed out"
        );
        anyhow!("git worktree remove timed out")
    })?
    .map_err(|e| {
        tracing::warn!(error = %e, "plan_tools::exit_worktree: failed to spawn git for remove");
        anyhow!("Failed to spawn git: {}", e)
    })?;

    if rm.status.success() {
        messages.push(format!("Worktree at '{}' removed.", worktree_path));
    } else {
        let stderr = String::from_utf8_lossy(&rm.stderr);
        messages.push(format!("Warning: could not remove worktree: {}", stderr));
    }

    // Optional merge
    if merge {
        let mg = timeout(
            Duration::from_secs(60),
            Command::new("git")
                .args(["merge", &branch])
                .current_dir(&cwd)
                .output(),
        )
        .await
        .map_err(|_| {
            tracing::warn!(
                branch = %branch,
                "plan_tools::exit_worktree: git merge timed out"
            );
            anyhow!("git merge timed out")
        })?
        .map_err(|e| {
            tracing::warn!(error = %e, "plan_tools::exit_worktree: failed to spawn git for merge");
            anyhow!("Failed to spawn git: {}", e)
        })?;

        if mg.status.success() {
            messages.push(format!("Branch '{}' merged successfully.", branch));
        } else {
            let stderr = String::from_utf8_lossy(&mg.stderr);
            messages.push(format!("Merge of '{}' failed: {}", branch, stderr));
        }
    }

    // Clear state — failure is non-fatal but logged.
    let mut state = load_state();
    state["worktree"] = serde_json::Value::Null;
    if let Err(e) = save_state(&state) {
        tracing::warn!(
            error = %e,
            "plan_tools: failed to persist state — next call may see stale data"
        );
    }

    Ok(messages.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_plan_mode_succeeds() {
        // Just verify it doesn't panic / returns Ok
        let result = enter_plan_mode();
        assert!(result.is_ok());
    }

    #[test]
    fn exit_plan_mode_after_enter() {
        enter_plan_mode().unwrap();
        let result = exit_plan_mode();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("INACTIVE"));
    }

    #[test]
    fn exit_worktree_without_enter_returns_error() {
        // Clear any existing state first
        if let Ok(p) = grok_state_path() {
            let _ = fs::write(&p, r#"{}"#);
        }
        let policy = crate::acp::security::SecurityPolicy::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(exit_worktree(false, &policy));
        assert!(result.is_err());
    }

    #[test]
    fn enter_worktree_rejects_dotdot_branch() {
        let policy = crate::acp::security::SecurityPolicy::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(enter_worktree("../../evil", "some/path", &policy));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unsafe branch name"),
            "unexpected message: {}",
            msg
        );
    }

    #[test]
    fn enter_worktree_rejects_dash_prefix_branch() {
        let policy = crate::acp::security::SecurityPolicy::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(enter_worktree("-upstream", "some/path", &policy));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unsafe branch name"),
            "unexpected message: {}",
            msg
        );
    }

    #[test]
    fn enter_worktree_rejects_empty_branch() {
        let policy = crate::acp::security::SecurityPolicy::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(enter_worktree("", "some/path", &policy));
        assert!(result.is_err());
    }

    #[test]
    fn enter_worktree_rejects_empty_path() {
        let policy = crate::acp::security::SecurityPolicy::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(enter_worktree("main", "", &policy));
        assert!(result.is_err());
    }
}
