//! Shared test helpers for all integration tests.
//!
//! Each integration test file includes this module via:
//! ```rust,ignore
//! #[path = "helpers.rs"] mod helpers;
//! ```
//! (when the test source is inside `tests/integration/`)

use grok_cli::acp::security::SecurityPolicy;
use grok_cli::tools::ToolContext;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// Security / context helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Create a [`SecurityPolicy`] rooted at the temp dir so all path operations
/// stay within the isolated sandbox.
pub fn make_security(dir: &TempDir) -> SecurityPolicy {
    SecurityPolicy::with_working_directory(dir.path().to_path_buf())
}

/// Alias of [`make_security`] — preferred name in file-tools tests.
pub fn make_policy(dir: &TempDir) -> SecurityPolicy {
    make_security(dir)
}

/// Create a [`ToolContext`] rooted at the temp dir.
pub fn make_ctx(dir: &TempDir) -> ToolContext {
    ToolContext::new(make_security(dir))
}

// ─────────────────────────────────────────────────────────────────────────────
// Task-list fixture helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Write `.zed/task_list.json` in **Format A**: `{"tasks": [...]}`.
///
/// This is the canonical grok-cli task format.
pub fn write_task_list_a(dir: &TempDir, tasks: Value) {
    let zed = dir.path().join(".zed");
    fs::create_dir_all(&zed).unwrap();
    fs::write(
        zed.join("task_list.json"),
        serde_json::to_string_pretty(&json!({ "tasks": tasks })).unwrap(),
    )
    .unwrap();
}

/// Write `.zed/task_list.json` as a **plain JSON array** (Format C).
///
/// `load_task_file` normalises this into `{"tasks":[…]}` on read, so
/// callers using `task_get` / `task_update` should work transparently.
pub fn write_task_list_c(dir: &TempDir, tasks: &[Value]) {
    let zed = dir.path().join(".zed");
    fs::create_dir_all(&zed).unwrap();
    let arr = Value::Array(tasks.to_vec());
    fs::write(
        zed.join("task_list.json"),
        serde_json::to_string_pretty(&arr).unwrap(),
    )
    .unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Generic file fixture helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Write `content` to `relative_path` inside `dir`, creating any needed parent
/// directories first.  Panics on I/O errors — only intended for test setup.
///
/// Returns the absolute [`std::path::PathBuf`] of the created file.
pub fn write_fixture(dir: &TempDir, relative_path: &str, content: &str) -> std::path::PathBuf {
    let full = dir.path().join(relative_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create fixture parent dirs");
    }
    fs::write(&full, content).expect("write fixture file");
    full
}
