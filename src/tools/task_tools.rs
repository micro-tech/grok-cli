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

    let data: Value = if task_file.exists() {
        let content = fs::read_to_string(&task_file)
            .map_err(|e| anyhow!("Failed to read task_list.json: {}", e))?;
        serde_json::from_str(&content).map_err(|e| anyhow!("Invalid task_list.json: {}", e))?
    } else {
        json!({ "tasks": [] })
    };

    Ok((task_file, data))
}

fn save_task_file(path: &std::path::Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| anyhow!("Failed to create .zed directory: {}", e))?;
    }
    let json_str = serde_json::to_string_pretty(data)
        .map_err(|e| anyhow!("Failed to serialise tasks: {}", e))?;
    fs::write(path, json_str).map_err(|e| anyhow!("Failed to write task_list.json: {}", e))?;
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
        return Err(anyhow!("Task title cannot be empty"));
    }

    let valid_priorities = ["high", "medium", "low"];
    if !valid_priorities.contains(&priority) {
        return Err(anyhow!(
            "Invalid priority '{}'. Use: high, medium, low",
            priority
        ));
    }

    let (task_file, mut data) = load_task_file(security)?;

    let tasks = data["tasks"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("task_list.json is missing the 'tasks' array"))?;

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
    let valid_statuses = ["pending", "in_progress", "done", "deferred"];
    if let Some(s) = status {
        if !valid_statuses.contains(&s) {
            return Err(anyhow!(
                "Invalid status '{}'. Use: pending, in_progress, done, deferred",
                s
            ));
        }
    }

    let (task_file, mut data) = load_task_file(security)?;

    let tasks = data["tasks"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("task_list.json is missing the 'tasks' array"))?;

    // Find by ID (compare as f64 to support subtask IDs like 85.2)
    let task = tasks
        .iter_mut()
        .find(|t| t["id"].as_f64().map(|v| (v - id).abs() < f64::EPSILON) == Some(true))
        .ok_or_else(|| anyhow!("Task {} not found in task_list.json", id))?;

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
}
