//! Integration tests for `task_create`, `task_get`, and `task_update`
//! (subtask 148.6).
//!
//! All tests are sync (`#[test]`) because none of the task-tool functions
//! are async.  Each test creates an isolated [`TempDir`] so they are
//! fully independent and run safely in parallel.
//!
//! Run with:
//!   cargo test --test task_tools_tests -- --nocapture

#[path = "helpers.rs"]
mod helpers;

use grok_cli::tools::{task_create, task_get, task_update};
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// 1. Full lifecycle: task_create → task_get → task_update
// ─────────────────────────────────────────────────────────────────────────────

/// Create a task, read it back with task_get, and update its status.
/// This is the canonical happy-path end-to-end flow.
#[test]
fn lifecycle_create_get_update() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    // --- create ---------------------------------------------------------------
    let msg = task_create(
        "Lifecycle task",
        "Full lifecycle test",
        "high",
        vec![],
        "Implementation steps go here",
        "cargo test --test task_tools_tests",
        vec![],
        &sec,
    )
    .expect("task_create should succeed");

    assert!(
        msg.contains("Task 1"),
        "create message should mention 'Task 1': {msg}"
    );
    assert!(
        msg.contains("Lifecycle task"),
        "create message should echo the title: {msg}"
    );

    // --- get ------------------------------------------------------------------
    let json_str = task_get(1.0, &sec).expect("task_get should find task 1");
    let v: Value = serde_json::from_str(&json_str).expect("task_get must return valid JSON");

    assert_eq!(v["id"], 1, "id must be 1");
    assert_eq!(v["title"], "Lifecycle task");
    assert_eq!(v["description"], "Full lifecycle test");
    assert_eq!(v["priority"], "high");
    assert_eq!(v["status"], "pending", "initial status must be pending");
    assert_eq!(v["details"], "Implementation steps go here");
    assert_eq!(v["testStrategy"], "cargo test --test task_tools_tests");

    // --- update ---------------------------------------------------------------
    let upd = task_update(1.0, Some("in_progress"), None, None, None, &sec)
        .expect("task_update should succeed");

    assert!(
        upd.contains("status=in_progress"),
        "update message should confirm new status: {upd}"
    );

    // Read back and confirm the status was persisted.
    let after: Value = serde_json::from_str(&task_get(1.0, &sec).unwrap()).unwrap();
    assert_eq!(after["status"], "in_progress");
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Format A loading and searching
// ─────────────────────────────────────────────────────────────────────────────

/// Pre-write a Format-A file (the canonical `{"tasks":[…]}` shape) and verify
/// that `task_get` can locate tasks by their numeric IDs.
#[test]
fn format_a_load_and_search() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    helpers::write_task_list_a(
        &dir,
        json!([
            { "id": 10, "title": "Alpha",  "status": "done",    "subtasks": [] },
            { "id": 20, "title": "Beta",   "status": "pending", "subtasks": [] },
            { "id": 30, "title": "Gamma",  "status": "pending", "priority": "low", "subtasks": [] }
        ]),
    );

    // Find by each ID.
    let a: Value = serde_json::from_str(&task_get(10.0, &sec).unwrap()).unwrap();
    assert_eq!(a["title"], "Alpha");
    assert_eq!(a["status"], "done");

    let b: Value = serde_json::from_str(&task_get(20.0, &sec).unwrap()).unwrap();
    assert_eq!(b["title"], "Beta");

    let g: Value = serde_json::from_str(&task_get(30.0, &sec).unwrap()).unwrap();
    assert_eq!(g["title"], "Gamma");
    assert_eq!(g["priority"], "low");

    // Searching for a non-existent ID must return an error.
    let err = task_get(99.0, &sec).unwrap_err().to_string();
    assert!(
        err.contains("99"),
        "error must mention the requested ID: {err}"
    );
    assert!(err.contains("not found"), "error must say not found: {err}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Format C (plain array) normalisation
// ─────────────────────────────────────────────────────────────────────────────

/// Write a plain JSON array file and confirm that `task_get` transparently
/// handles it via the normalisation that wraps it in `{"tasks":[…]}`.
#[test]
fn format_c_normalised_task_get_works() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    helpers::write_task_list_c(
        &dir,
        &[
            json!({ "id": 1, "title": "First",  "status": "pending", "subtasks": [] }),
            json!({ "id": 2, "title": "Second", "status": "pending", "subtasks": [] }),
        ],
    );

    // task_get must work even though the file is a bare array.
    let v1: Value = serde_json::from_str(&task_get(1.0, &sec).unwrap()).unwrap();
    assert_eq!(v1["title"], "First");
    assert_eq!(v1["status"], "pending");

    let v2: Value = serde_json::from_str(&task_get(2.0, &sec).unwrap()).unwrap();
    assert_eq!(v2["title"], "Second");

    // Searching for something that doesn't exist must still return an error.
    assert!(task_get(99.0, &sec).is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. .bak recovery — corrupt the live file, verify load falls back to .bak
// ─────────────────────────────────────────────────────────────────────────────

/// After two successful writes a `.bak` file exists (snapshot of the first
/// write).  Corrupting the live file must not panic; `task_get` must recover
/// from `.bak` and still return the task that was present after the first write.
#[test]
fn bak_recovery_on_corrupt_live_file() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    // First write — creates `task_list.json` (no .bak yet).
    task_create("GoodTask", "desc", "medium", vec![], "", "", vec![], &sec).unwrap();

    // Second write — snapshots the first write to `.bak`, then replaces the
    // live file with a 2-task version.
    task_create("SecondTask", "desc", "low", vec![], "", "", vec![], &sec).unwrap();

    // Confirm .bak exists.
    let bak_path = dir.path().join(".zed").join("task_list.json.bak");
    assert!(bak_path.exists(), ".bak must exist after the second write");

    // Corrupt the live file.
    fs::write(
        dir.path().join(".zed").join("task_list.json"),
        "{{{ NOT VALID JSON !!!",
    )
    .unwrap();

    // task_get must silently recover from .bak.
    // .bak contains the snapshot taken just *before* the second write,
    // so it holds exactly 1 task.
    let result = task_get(1.0, &sec);
    assert!(
        result.is_ok(),
        "task_get must recover from .bak, got: {:?}",
        result
    );

    let v: Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(v["title"], "GoodTask");

    // task ID 2 (from the second write) was never in .bak.
    let err = task_get(2.0, &sec).unwrap_err().to_string();
    assert!(
        err.contains("not found"),
        "task 2 should be absent from recovered .bak: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Atomic save — .tmp file must not exist after save
// ─────────────────────────────────────────────────────────────────────────────

/// `save_task_file` writes to a `.tmp` sibling first, then renames it over
/// the live file.  After a successful save the `.tmp` file must be gone.
#[test]
fn atomic_save_leaves_no_tmp_file() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    task_create(
        "Atomic save task",
        "desc",
        "medium",
        vec![],
        "",
        "",
        vec![],
        &sec,
    )
    .unwrap();

    let tmp_path = dir.path().join(".zed").join("task_list.json.tmp");
    assert!(
        !tmp_path.exists(),
        ".tmp file must be cleaned up after a successful save"
    );

    // The live file must exist and be valid JSON.
    let live_path = dir.path().join(".zed").join("task_list.json");
    assert!(live_path.exists(), "task_list.json must exist after save");

    let raw = fs::read_to_string(&live_path).unwrap();
    let parsed: Value = serde_json::from_str(&raw).expect("task_list.json must be valid JSON");
    assert!(
        parsed["tasks"].is_array(),
        "top-level tasks key must be an array"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. task_create rejects empty title
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_create_rejects_empty_title() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    let err = task_create("", "some description", "high", vec![], "", "", vec![], &sec)
        .unwrap_err()
        .to_string();

    assert!(
        err.to_lowercase().contains("title"),
        "error must mention 'title': {err}"
    );
}

/// Whitespace-only title must also be rejected.
#[test]
fn task_create_rejects_whitespace_only_title() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    let result = task_create("   \t  ", "desc", "low", vec![], "", "", vec![], &sec);
    assert!(result.is_err(), "whitespace-only title must be rejected");
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. task_create rejects invalid priority
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_create_rejects_invalid_priority() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    let err = task_create(
        "Valid title",
        "desc",
        "URGENT",
        vec![],
        "",
        "",
        vec![],
        &sec,
    )
    .unwrap_err()
    .to_string();

    assert!(
        err.to_lowercase().contains("priority"),
        "error must mention 'priority': {err}"
    );
    assert!(
        err.contains("URGENT"),
        "error must echo the invalid value: {err}"
    );
}

/// Verify all three valid priority values are accepted.
#[test]
fn task_create_accepts_all_valid_priorities() {
    for priority in &["high", "medium", "low"] {
        let dir = TempDir::new().unwrap();
        let sec = helpers::make_security(&dir);

        let result = task_create(
            &format!("Task with priority {priority}"),
            "desc",
            priority,
            vec![],
            "",
            "",
            vec![],
            &sec,
        );
        assert!(
            result.is_ok(),
            "priority '{priority}' must be accepted, got: {:?}",
            result
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. task_update rejects invalid status
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_update_rejects_invalid_status() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    // Create a task to update.
    task_create(
        "Update target",
        "desc",
        "medium",
        vec![],
        "",
        "",
        vec![],
        &sec,
    )
    .unwrap();

    let err = task_update(1.0, Some("completed"), None, None, None, &sec)
        .unwrap_err()
        .to_string();

    assert!(
        err.to_lowercase().contains("status"),
        "error must mention 'status': {err}"
    );
    assert!(
        err.contains("completed"),
        "error must echo the invalid value: {err}"
    );
}

/// Verify all four valid status values are accepted.
#[test]
fn task_update_accepts_all_valid_statuses() {
    for status in &["pending", "in_progress", "done", "deferred"] {
        let dir = TempDir::new().unwrap();
        let sec = helpers::make_security(&dir);
        task_create(
            "Status test task",
            "desc",
            "low",
            vec![],
            "",
            "",
            vec![],
            &sec,
        )
        .unwrap();

        let result = task_update(1.0, Some(status), None, None, None, &sec);
        assert!(
            result.is_ok(),
            "status '{status}' must be accepted, got: {:?}",
            result
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. task_get returns descriptive error for missing file
// ─────────────────────────────────────────────────────────────────────────────

/// When `.zed/task_list.json` does not exist at all, `task_get` must return an
/// `Err` whose message mentions the path (not panic or produce a confusing error).
#[test]
fn task_get_missing_file_returns_descriptive_error() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);
    // Deliberately do NOT create `.zed/` or `task_list.json`.

    let err = task_get(1.0, &sec).unwrap_err().to_string();

    assert!(
        err.contains("task_list.json"),
        "error must mention the missing file: {err}"
    );
    // Must not be a raw panic trace.
    assert!(!err.contains("panicked"), "must not panic: {err}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. task_get returns descriptive error for unknown ID
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_get_unknown_id_returns_descriptive_error() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    helpers::write_task_list_a(
        &dir,
        json!([
            { "id": 1, "title": "Only task", "status": "pending", "subtasks": [] }
        ]),
    );

    let err = task_get(42.0, &sec).unwrap_err().to_string();

    assert!(
        err.contains("42"),
        "error must mention the requested ID: {err}"
    );
    assert!(
        err.contains("not found"),
        "error must say 'not found': {err}"
    );
    // Must include a hint to help the caller discover valid IDs.
    assert!(
        err.contains("task_list.json"),
        "error must reference the file: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. Subtask decimal IDs — create with subtasks, get by 1.1 ID
// ─────────────────────────────────────────────────────────────────────────────

/// `task_create` must auto-assign decimal IDs (`parent.1`, `parent.2`, …) to
/// inline subtasks, and `task_get` must be able to retrieve them individually.
#[test]
fn subtask_decimal_ids_created_and_retrievable() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    let subtasks = vec![
        json!({ "title": "Design the schema",   "dependencies": [] }),
        json!({ "title": "Implement the logic", "dependencies": [] }),
        json!({ "title": "Write tests",         "dependencies": [] }),
    ];

    let msg = task_create(
        "Feature with subtasks",
        "A feature that needs sub-work",
        "high",
        vec![],
        "Detailed implementation plan",
        "cargo test",
        subtasks,
        &sec,
    )
    .expect("task_create with subtasks must succeed");

    // The success message should report the subtask count.
    assert!(
        msg.contains("3 subtask"),
        "message must report 3 subtasks: {msg}"
    );
    assert!(
        msg.contains("1.1"),
        "message must mention 1.1 subtask range: {msg}"
    );

    // Retrieve the parent task and check subtask IDs.
    let parent_json = task_get(1.0, &sec).expect("must find parent task 1");
    let parent: Value = serde_json::from_str(&parent_json).unwrap();
    let subs = parent["subtasks"]
        .as_array()
        .expect("subtasks must be an array");

    assert_eq!(subs.len(), 3, "must have 3 subtasks");
    assert_eq!(
        subs[0]["id"].as_f64().unwrap(),
        1.1,
        "first subtask ID must be 1.1"
    );
    assert_eq!(
        subs[1]["id"].as_f64().unwrap(),
        1.2,
        "second subtask ID must be 1.2"
    );
    assert_eq!(
        subs[2]["id"].as_f64().unwrap(),
        1.3,
        "third subtask ID must be 1.3"
    );
    assert_eq!(subs[0]["status"], "pending");

    // Retrieve individual subtasks by their decimal ID.
    let st1: Value = serde_json::from_str(&task_get(1.1, &sec).unwrap()).unwrap();
    assert_eq!(st1["title"], "Design the schema");

    let st2: Value = serde_json::from_str(&task_get(1.2, &sec).unwrap()).unwrap();
    assert_eq!(st2["title"], "Implement the logic");

    let st3: Value = serde_json::from_str(&task_get(1.3, &sec).unwrap()).unwrap();
    assert_eq!(st3["title"], "Write tests");
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. task_update sets status=done
// ─────────────────────────────────────────────────────────────────────────────

/// The primary task-runner lifecycle ends with marking a task `done`.
/// Verify that the status is persisted and is readable back via `task_get`.
#[test]
fn task_update_sets_status_done() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    task_create(
        "Task to finish",
        "desc",
        "high",
        vec![],
        "",
        "",
        vec![],
        &sec,
    )
    .unwrap();

    // Confirm initial state.
    let before: Value = serde_json::from_str(&task_get(1.0, &sec).unwrap()).unwrap();
    assert_eq!(before["status"], "pending");

    // Mark done.
    let upd = task_update(1.0, Some("done"), None, None, None, &sec)
        .expect("update to done must succeed");

    assert!(
        upd.contains("status=done"),
        "update message must confirm status=done: {upd}"
    );

    // Read back and confirm persistence.
    let after: Value = serde_json::from_str(&task_get(1.0, &sec).unwrap()).unwrap();
    assert_eq!(after["status"], "done", "status must be persisted as done");
    // Other fields must be unchanged.
    assert_eq!(after["title"], "Task to finish");
    assert_eq!(after["priority"], "high");
}

// ─────────────────────────────────────────────────────────────────────────────
// Bonus: task_update on non-existent ID returns an error (not a panic)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_update_nonexistent_id_returns_error() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    // Create a task so the file exists, but update a different ID.
    task_create("Real task", "desc", "low", vec![], "", "", vec![], &sec).unwrap();

    let err = task_update(999.0, Some("done"), None, None, None, &sec)
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("999"),
        "error must mention the missing ID: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bonus: task_update can change title, priority, and details simultaneously
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_update_multiple_fields_persisted() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    task_create(
        "Original title",
        "desc",
        "low",
        vec![],
        "old details",
        "",
        vec![],
        &sec,
    )
    .unwrap();

    let upd = task_update(
        1.0,
        Some("in_progress"),
        Some("Updated title"),
        Some("high"),
        Some("new details"),
        &sec,
    )
    .expect("multi-field update must succeed");

    // All four changes must be confirmed in the message.
    assert!(upd.contains("status=in_progress"), "status change: {upd}");
    assert!(upd.contains("Updated title"), "title change: {upd}");
    assert!(upd.contains("priority=high"), "priority change: {upd}");
    assert!(upd.contains("details=<updated>"), "details change: {upd}");

    // Verify persistence.
    let v: Value = serde_json::from_str(&task_get(1.0, &sec).unwrap()).unwrap();
    assert_eq!(v["status"], "in_progress");
    assert_eq!(v["title"], "Updated title");
    assert_eq!(v["priority"], "high");
    assert_eq!(v["details"], "new details");
}

// ─────────────────────────────────────────────────────────────────────────────
// Bonus: second write produces a .bak snapshot of the previous state
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn second_save_creates_bak_snapshot() {
    let dir = TempDir::new().unwrap();
    let sec = helpers::make_security(&dir);

    // First save — no .bak yet.
    task_create("First task", "desc", "high", vec![], "", "", vec![], &sec).unwrap();
    let bak_path = dir.path().join(".zed").join("task_list.json.bak");
    assert!(
        !bak_path.exists(),
        ".bak must not exist after the first write"
    );

    // Second save — should snapshot the first write into .bak.
    task_create(
        "Second task",
        "desc",
        "medium",
        vec![],
        "",
        "",
        vec![],
        &sec,
    )
    .unwrap();
    assert!(bak_path.exists(), ".bak must exist after the second write");

    // .bak must be valid JSON containing exactly one task (the first write).
    let bak_raw = fs::read_to_string(&bak_path).unwrap();
    let bak_v: Value = serde_json::from_str(&bak_raw).expect(".bak must be valid JSON");
    let bak_tasks = bak_v["tasks"].as_array().unwrap();
    assert_eq!(
        bak_tasks.len(),
        1,
        ".bak snapshot must hold exactly one task"
    );
    assert_eq!(bak_tasks[0]["title"], "First task");
}
