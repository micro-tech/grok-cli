//! Task management tools — create and update tasks in `.zed/task_list.json`.
//!
//! Follows the **Task Builder** and **Task Runner** project rules:
//! - Every task has: id, title, description, status, dependencies, priority,
//!   details, testStrategy, subtasks.
//! - Status lifecycle: pending → in_progress → done | deferred.
//! - Subtask IDs are decimal: `{parent_id}.1`, `{parent_id}.2`, …
//! - Dependencies support both integer task IDs and decimal subtask IDs.

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

/// Create a new task in `.zed/task_list.json` following Task Builder rules.
///
/// # Fields
/// - `title`         — required; brief descriptive name.
/// - `description`   — concise overview of what the task involves.
/// - `priority`      — `"high"` | `"medium"` | `"low"`.
/// - `dependencies`  — IDs of tasks that must be `done` first (supports
///                     decimal subtask IDs such as `5.2`).
/// - `details`       — in-depth implementation instructions.
/// - `test_strategy` — verification approach (maps to `testStrategy` in JSON).
/// - `subtasks`      — optional initial subtasks; each must have at least
///                     a `"title"` key. IDs are auto-assigned as
///                     `{parent_id}.1`, `{parent_id}.2`, …
///
/// The new task is assigned `ID = floor(max_existing_id) + 1` and its
/// `status` is always initialised to `"pending"`.
pub fn task_create(
    title: &str,
    description: &str,
    priority: &str,
    dependencies: Vec<f64>,
    details: &str,
    test_strategy: &str,
    subtasks: Vec<Value>,
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

    // Validate every subtask has a title.
    for (i, st) in subtasks.iter().enumerate() {
        if st["title"].as_str().map(str::trim).unwrap_or("").is_empty() {
            return Err(anyhow!("Subtask {} is missing a 'title' field", i + 1));
        }
    }

    let (task_file, mut data) = load_task_file(security)?;

    let tasks = data["tasks"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("task_list.json is missing the 'tasks' array"))?;

    // Next ID = floor(max_id) + 1 (handles decimal subtask IDs safely)
    let max_id = tasks
        .iter()
        .filter_map(|t| t["id"].as_f64())
        .fold(0.0_f64, f64::max) as u64;
    let new_id = max_id + 1;

    // Build subtask list with auto-assigned decimal IDs: {parent}.1, {parent}.2, …
    let processed_subtasks: Vec<Value> = subtasks
        .into_iter()
        .enumerate()
        .map(|(i, subtask)| {
            // e.g. parent=85, i=0  =>  id=85.1
            let subtask_id: f64 = format!("{}.{}", new_id, i + 1).parse().unwrap_or(0.0);
            let subtask_title = subtask["title"]
                .as_str()
                .unwrap_or("Untitled subtask")
                .to_string();
            let subtask_deps = if subtask["dependencies"].is_array() {
                subtask["dependencies"].clone()
            } else {
                json!([])
            };
            json!({
                "id":           subtask_id,
                "title":        subtask_title,
                "status":       "pending",
                "dependencies": subtask_deps
            })
        })
        .collect();

    let subtask_count = processed_subtasks.len();

    let new_task = json!({
        "id":           new_id,
        "title":        title,
        "description":  description,
        "status":       "pending",
        "dependencies": dependencies,
        "priority":     priority,
        "details":      details,
        "testStrategy": test_strategy,
        "subtasks":     processed_subtasks
    });

    tasks.push(new_task);
    save_task_file(&task_file, &data)?;

    if subtask_count > 0 {
        Ok(format!(
            "Task {} created: \"{}\" [{}] with {} subtask(s) ({}.1 … {}.{})",
            new_id, title, priority, subtask_count, new_id, new_id, subtask_count
        ))
    } else {
        Ok(format!(
            "Task {} created: \"{}\" [{}]",
            new_id, title, priority
        ))
    }
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

    // ── helper to call task_create with all arguments ────────────────────
    fn create(dir: &TempDir, title: &str, desc: &str, priority: &str) -> Result<String> {
        let security = make_security(dir);
        task_create(title, desc, priority, vec![], "", "", vec![], &security)
    }

    #[test]
    fn create_task_in_empty_list() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        let result = create(&dir, "Test task", "A test", "high");
        assert!(result.is_ok(), "{:?}", result);
        assert!(result.unwrap().contains("Task 1"));
    }

    #[test]
    fn create_increments_id() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        create(&dir, "First", "", "medium").unwrap();
        let r = create(&dir, "Second", "", "low").unwrap();
        assert!(r.contains("Task 2"), "expected Task 2 in: {}", r);
    }

    #[test]
    fn create_stores_details_and_test_strategy() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        let security = make_security(&dir);

        task_create(
            "Task with details",
            "desc",
            "medium",
            vec![],
            "Do X then Y",
            "Run cargo test",
            vec![],
            &security,
        )
        .unwrap();

        let (_, data) = load_task_file(&security).unwrap();
        let task = &data["tasks"][0];
        assert_eq!(task["details"].as_str(), Some("Do X then Y"));
        assert_eq!(task["testStrategy"].as_str(), Some("Run cargo test"));
    }

    #[test]
    fn create_assigns_subtask_decimal_ids() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        let security = make_security(&dir);

        let subtasks = vec![
            json!({"title": "Design", "dependencies": []}),
            json!({"title": "Implement", "dependencies": [1.1]}),
        ];
        let r = task_create(
            "Feature",
            "desc",
            "high",
            vec![],
            "",
            "",
            subtasks,
            &security,
        )
        .unwrap();
        assert!(r.contains("2 subtask"), "expected subtask count in: {}", r);

        let (_, data) = load_task_file(&security).unwrap();
        let st = &data["tasks"][0]["subtasks"];
        assert_eq!(st[0]["id"].as_f64(), Some(1.1));
        assert_eq!(st[1]["id"].as_f64(), Some(1.2));
        assert_eq!(st[0]["status"].as_str(), Some("pending"));
    }

    #[test]
    fn create_rejects_subtask_missing_title() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let bad_subtasks = vec![json!({"dependencies": []})];
        let r = task_create("T", "", "low", vec![], "", "", bad_subtasks, &security);
        assert!(r.is_err());
    }

    #[test]
    fn create_supports_float_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        let security = make_security(&dir);

        // Create parent first so the file exists
        task_create("Parent", "", "high", vec![], "", "", vec![], &security).unwrap();
        let r = task_create(
            "Child",
            "",
            "medium",
            vec![1.0, 1.1],
            "",
            "",
            vec![],
            &security,
        )
        .unwrap();
        assert!(r.contains("Task 2"), "expected Task 2 in: {}", r);

        let (_, data) = load_task_file(&security).unwrap();
        let deps = &data["tasks"][1]["dependencies"];
        assert_eq!(deps[0].as_f64(), Some(1.0));
        assert_eq!(deps[1].as_f64(), Some(1.1));
    }

    #[test]
    fn update_task_status() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();

        create(&dir, "My task", "", "high").unwrap();
        let security = make_security(&dir);
        let r = task_update(1.0, Some("done"), None, None, None, &security).unwrap();
        assert!(r.contains("status=done"), "expected status=done in: {}", r);
    }

    #[test]
    fn update_nonexistent_task_returns_error() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        let security = make_security(&dir);

        let r = task_update(999.0, Some("done"), None, None, None, &security);
        assert!(r.is_err());
    }

    #[test]
    fn create_rejects_empty_title() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let r = task_create("", "desc", "high", vec![], "", "", vec![], &security);
        assert!(r.is_err());
    }
}
