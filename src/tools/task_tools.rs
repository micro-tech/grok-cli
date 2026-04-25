//! Task management tools — create and update tasks in `.zed/task_list.json`.

use crate::acp::security::SecurityPolicy;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::fs;

// ── helpers ───────────────────────────────────────────────────────────────────

fn load_task_file(security: &SecurityPolicy) -> Result<(std::path::PathBuf, Value)> {
    let task_file = security
        .working_directory()
        .join(".zed")
        .join("task_list.json");

    // Verify the file exists and give a clear diagnostic if not.
    // An empty/missing task file is a CWD mismatch (Grok launched from the
    // wrong directory), not a valid "no tasks yet" state when updating.
    if !task_file.exists() {
        tracing::warn!(
            path = %task_file.display(),
            cwd  = %security.working_directory().display(),
            "task_tools::load_task_file: task_list.json not found — \
             the working directory may not be the project root"
        );
    }

    let data: Value = if task_file.exists() {
        let content = fs::read_to_string(&task_file).map_err(|e| {
            tracing::warn!(error = %e, "task_tools::load_task_file: failed to read task_list.json");
            anyhow!("Failed to read task_list.json: {}", e)
        })?;
        serde_json::from_str(&content).map_err(|e| {
            tracing::warn!(error = %e, "task_tools::load_task_file: invalid JSON in task_list.json");
            anyhow!("Invalid task_list.json: {}", e)
        })?
    } else {
        json!({ "tasks": [] })
    };

    Ok((task_file, data))
}

/// Write `data` to `path` atomically: write to a `.json.tmp` sibling first,
/// then rename over the real file so a mid-write crash never corrupts the store.
fn save_task_file(path: &std::path::Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            tracing::warn!(error = %e, "task_tools::save_task_file: failed to create .zed directory");
            anyhow!("Failed to create .zed directory: {}", e)
        })?;
    }

    let json_str = serde_json::to_string_pretty(data).map_err(|e| {
        tracing::warn!(error = %e, "task_tools::save_task_file: failed to serialise tasks");
        anyhow!("Failed to serialise tasks: {}", e)
    })?;

    // Atomic write: write to a temp file then rename.
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &json_str).map_err(|e| {
        tracing::warn!(error = %e, "task_tools::save_task_file: failed to write tmp file");
        anyhow::anyhow!("task_tools: failed to write tmp file: {}", e)
    })?;
    fs::rename(&tmp_path, path).map_err(|e| {
        tracing::warn!(error = %e, "task_tools::save_task_file: failed to rename tmp → task_list.json");
        anyhow::anyhow!("task_tools: failed to rename tmp → task_list.json: {}", e)
    })?;

    Ok(())
}

// ── public API ────────────────────────────────────────────────────────────────

/// Create a new task in `.zed/task_list.json`.
///
/// The new task is assigned an ID of `max_existing_id + 1`.
pub fn task_create(
    title: &str,
    description: &str,
    priority: &str,
    dependencies: Vec<u64>,
    security: &SecurityPolicy,
) -> Result<String> {
    if title.trim().is_empty() {
        tracing::warn!("task_tools::task_create: rejected — title is empty");
        return Err(anyhow!("Task title cannot be empty"));
    }

    let valid_priorities = ["high", "medium", "low"];
    if !valid_priorities.contains(&priority) {
        tracing::warn!(priority = %priority, "task_tools::task_create: invalid priority rejected");
        return Err(anyhow!(
            "Invalid priority '{}'. Use: high, medium, low",
            priority
        ));
    }

    let (task_file, mut data) = load_task_file(security)?;

    let tasks = data["tasks"].as_array_mut().ok_or_else(|| {
        tracing::warn!("task_tools::task_create: task_list.json is missing the 'tasks' array");
        anyhow!("task_list.json is missing the 'tasks' array")
    })?;

    // Next ID = floor(max_id) + 1 (works for both integer and decimal subtask IDs)
    let max_id = tasks
        .iter()
        .filter_map(|t| t["id"].as_f64())
        .fold(0.0_f64, f64::max) as u64;
    let new_id = max_id + 1;

    let new_task = json!({
        "id":           new_id,
        "title":        title,
        "description":  description,
        "status":       "pending",
        "dependencies": dependencies,
        "priority":     priority,
        "details":      "",
        "testStrategy": "",
        "subtasks":     []
    });

    tasks.push(new_task);
    save_task_file(&task_file, &data)?;

    Ok(format!(
        "Task {} created: \"{}\" [{}]",
        new_id, title, priority
    ))
}

/// Update one or more fields of an existing task.
///
/// `id` supports decimal values (e.g. `85.2`) for subtasks.
pub fn task_update(
    id: f64,
    status: Option<&str>,
    title: Option<&str>,
    priority: Option<&str>,
    details: Option<&str>,
    security: &SecurityPolicy,
) -> Result<String> {
    // ── validate status ───────────────────────────────────────────────────────
    let valid_statuses = ["pending", "in_progress", "done", "deferred"];
    if let Some(s) = status {
        if !valid_statuses.contains(&s) {
            tracing::warn!(
                status = %s,
                "task_tools::task_update: invalid status rejected"
            );
            return Err(anyhow!(
                "Invalid status '{}'. Use: pending, in_progress, done, deferred",
                s
            ));
        }
    }

    // ── validate priority ─────────────────────────────────────────────────────
    if let Some(p) = priority {
        let valid = ["high", "medium", "low"];
        if !valid.contains(&p) {
            tracing::warn!(
                priority = %p,
                "task_tools::task_update: invalid priority rejected"
            );
            return Err(anyhow::anyhow!(
                "task_update: invalid priority '{}'. Must be one of: high, medium, low.",
                p
            ));
        }
    }

    let (task_file, mut data) = load_task_file(security)?;

    let tasks = data["tasks"].as_array_mut().ok_or_else(|| {
        tracing::warn!("task_tools::task_update: task_list.json is missing the 'tasks' array");
        anyhow!("task_list.json is missing the 'tasks' array")
    })?;

    // Find by ID (compare as f64 to support subtask IDs like 85.2)
    let task = tasks
        .iter_mut()
        .find(|t| t["id"].as_f64().map(|v| (v - id).abs() < f64::EPSILON) == Some(true))
        .ok_or_else(|| {
            tracing::warn!(id = %id, path = %task_file.display(), "task_tools::task_update: task not found");
            anyhow!(
                "Task {} not found in task_list.json at '{}'.\n\
                 If Grok is not running from the project root, the task file may be \
                 resolved to the wrong location. Current working directory: {}",
                id,
                task_file.display(),
                security.working_directory().display()
            )
        })?;

    let mut changed = Vec::new();
    if let Some(s) = status {
        task["status"] = json!(s);
        changed.push(format!("status={}", s));
    }
    if let Some(t) = title {
        task["title"] = json!(t);
        changed.push(format!("title=\"{}\"", t));
    }
    if let Some(p) = priority {
        task["priority"] = json!(p);
        changed.push(format!("priority={}", p));
    }
    if let Some(d) = details {
        task["details"] = json!(d);
        changed.push("details=<updated>".to_string());
    }

    if changed.is_empty() {
        return Ok(format!("Task {}: no fields changed.", id));
    }

    save_task_file(&task_file, &data)?;
    Ok(format!("Task {} updated: {}", id, changed.join(", ")))
}

/// Return a single task (or subtask) by numeric ID as pretty-printed JSON.
///
/// Handles two common `task_list.json` layouts automatically:
///
/// * **Format A** — `{"tasks": [{…}, …]}` (grok-cli standard)
/// * **Format B** — `{"0": {…}, "1": {…}, …}` (numeric-indexed object where
///   the actual task `id` lives *inside* each value, not in the key)
///
/// In Format B the subtask `id` fields may be strings (`"60.1"`) rather than
/// numbers, so both representations are compared.
pub fn task_get(id: f64, security: &SecurityPolicy) -> Result<String> {
    let (task_file, data) = load_task_file(security)?;

    if !task_file.exists() {
        tracing::warn!(
            path = %task_file.display(),
            cwd  = %security.working_directory().display(),
            "task_tools::task_get: task_list.json not found"
        );
        return Err(anyhow!(
            "task_list.json not found at '{}' (working dir: '{}'). \
             Launch Grok from the project root that contains the '.zed' folder.",
            task_file.display(),
            security.working_directory().display()
        ));
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Return true when a JSON value's "id" field (number *or* string) matches
    /// the requested float id within a small epsilon.
    fn id_matches(val: &serde_json::Value, target: f64) -> bool {
        if let Some(n) = val["id"].as_f64() {
            return (n - target).abs() < 0.001;
        }
        if let Some(s) = val["id"].as_str() {
            if let Ok(n) = s.parse::<f64>() {
                return (n - target).abs() < 0.001;
            }
        }
        false
    }

    /// Search `tasks` slice (top-level + subtasks) for id `target`.
    fn search_slice(tasks: &[serde_json::Value], target: f64) -> Option<serde_json::Value> {
        for t in tasks {
            if id_matches(t, target) {
                return Some(t.clone());
            }
            if let Some(subs) = t["subtasks"].as_array() {
                for st in subs {
                    if id_matches(st, target) {
                        return Some(st.clone());
                    }
                }
            }
        }
        None
    }

    // ── Format A: {"tasks": [...]} ────────────────────────────────────────────
    let found = if let Some(tasks) = data["tasks"].as_array() {
        search_slice(tasks, id)

    // ── Format C: [{…}, {…}, …] plain JSON array ──────────────────────────────
    // Some projects store the task list as a top-level array with no wrapper key.
    } else if let Some(arr) = data.as_array() {
        search_slice(arr, id)

    // ── Format B: {"0": {…}, "1": {…}, …} numeric-indexed object ─────────────
    // The key is a 0-based index; the real task id lives inside the object.
    } else if let Some(obj) = data.as_object() {
        let values: Vec<serde_json::Value> =
            obj.values().filter(|v| v.is_object()).cloned().collect();
        search_slice(&values, id)
    } else {
        None
    };

    // ── result ────────────────────────────────────────────────────────────────
    match found {
        Some(task) => serde_json::to_string_pretty(&task).map_err(|e| {
            tracing::warn!(error = %e, "task_tools::task_get: serialisation failed");
            anyhow!("Failed to serialise task {}: {}", id, e)
        }),
        None => {
            let total = data["tasks"]
                .as_array()
                .map(|a| a.len())
                .or_else(|| data.as_array().map(|a| a.len()))
                .or_else(|| data.as_object().map(|o| o.len()))
                .unwrap_or(0);
            tracing::warn!(
                id = %id, total, path = %task_file.display(),
                "task_tools::task_get: task not found"
            );
            Err(anyhow!(
                "Task {} not found in task_list.json \
                 ({} entries in '{}').\n\
                 Tip: call read_file with '.zed/task_list.json' to list all IDs.",
                id,
                total,
                task_file.display()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::security::SecurityPolicy;
    use std::fs;
    use tempfile::TempDir;

    fn make_security(dir: &TempDir) -> SecurityPolicy {
        SecurityPolicy::with_working_directory(dir.path().to_path_buf())
    }

    #[test]
    fn create_task_in_empty_list() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        let result = task_create("Test task", "A test", "high", vec![], &security);
        assert!(result.is_ok(), "{:?}", result);
        assert!(result.unwrap().contains("Task 1"));
    }

    #[test]
    fn create_increments_id() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        task_create("First", "", "medium", vec![], &security).unwrap();
        let r = task_create("Second", "", "low", vec![], &security).unwrap();
        assert!(r.contains("Task 2"), "expected Task 2 in: {}", r);
    }

    #[test]
    fn update_task_status() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        task_create("My task", "", "high", vec![], &security).unwrap();
        let r = task_update(1.0, Some("done"), None, None, None, &security).unwrap();
        assert!(r.contains("status=done"), "expected status=done in: {}", r);
    }

    #[test]
    fn update_nonexistent_task_returns_error() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        let r = task_update(999.0, Some("done"), None, None, None, &security);
        assert!(r.is_err());
    }

    #[test]
    fn create_rejects_empty_title() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let r = task_create("", "desc", "high", vec![], &security);
        assert!(r.is_err());
    }

    #[test]
    fn update_rejects_invalid_priority() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        task_create("A task", "", "high", vec![], &security).unwrap();
        let r = task_update(1.0, None, None, Some("critical"), None, &security);
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("high, medium, low"),
            "error should mention valid options, got: {}",
            msg
        );
    }

    #[test]
    fn task_get_returns_correct_task() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        task_create("Alpha", "first", "high", vec![], &security).unwrap();
        task_create("Beta", "second", "low", vec![], &security).unwrap();
        let json = task_get(1.0, &security).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["title"], "Alpha");
        assert_eq!(v["id"], 1);
    }

    #[test]
    fn task_get_missing_returns_err() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        let r = task_get(999.0, &security);
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn update_rejects_invalid_status() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        task_create("A task", "", "high", vec![], &security).unwrap();
        let r = task_update(1.0, Some("finished"), None, None, None, &security);
        assert!(r.is_err());
    }
}
