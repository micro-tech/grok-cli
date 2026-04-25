//! Integration tests: trace the full data-flow from tool execution through
//! message formatting to the LLM context window.
//!
//! These tests pin down the exact handoff points where data can be dropped:
//!
//!   1. tool result produced by `task_get`
//!   2. result placed into the ACP `session.messages` array
//!   3. history trimmer leaves the tool result in place
//!   4. system prompt survives trimming
//!   5. both task-list formats (grok-cli {"tasks":[…]} and bot {"0":{…},…})
//!   6. string subtask IDs ("60.1") found by `task_get`
//!   7. format_tool_error_for_llm produces a message the LLM can act on
//!
//! Run with:
//!   cargo test --test tool_data_flow -- --nocapture

use grok_cli::acp::security::SecurityPolicy;
use grok_cli::tools::tool_error::format_tool_error_for_llm;
use grok_cli::tools::{task_create, task_get};
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_security(dir: &TempDir) -> SecurityPolicy {
    SecurityPolicy::with_working_directory(dir.path().to_path_buf())
}

/// Build a `.zed/task_list.json` in the grok-cli format:
/// `{ "tasks": [ {…}, … ] }`
fn write_format_a(dir: &TempDir, tasks: Value) {
    let zed = dir.path().join(".zed");
    fs::create_dir_all(&zed).unwrap();
    fs::write(
        zed.join("task_list.json"),
        serde_json::to_string_pretty(&json!({ "tasks": tasks })).unwrap(),
    )
    .unwrap();
}

/// Build a `.zed/task_list.json` in the bot-project format:
/// `{ "0": {…}, "1": {…}, … }` — numeric string keys, task `id` is inside.
fn write_format_b(dir: &TempDir, tasks: &[Value]) {
    let zed = dir.path().join(".zed");
    fs::create_dir_all(&zed).unwrap();
    let mut obj = serde_json::Map::new();
    for (i, t) in tasks.iter().enumerate() {
        obj.insert(i.to_string(), t.clone());
    }
    fs::write(
        zed.join("task_list.json"),
        serde_json::to_string_pretty(&Value::Object(obj)).unwrap(),
    )
    .unwrap();
}

/// Simulate the ACP message-history that exists just before the second LLM
/// call (after one tool call has been executed).  This is the exact structure
/// `handle_chat_completion` builds before looping back to the API.
fn build_messages_with_tool_result(
    system_prompt: &str,
    user_question: &str,
    tool_name: &str,
    tool_call_id: &str,
    tool_result_json: &str,
) -> Vec<Value> {
    vec![
        // [0] system prompt — must never be evicted by the trimmer
        json!({ "role": "system", "content": system_prompt }),
        // [1] user turn
        json!({ "role": "user", "content": user_question }),
        // [2] assistant turn requesting the tool call
        json!({
            "role": "assistant",
            "tool_calls": [{
                "id": tool_call_id,
                "type": "function",
                "function": {
                    "name": tool_name,
                    "arguments": format!("{{\"id\":60}}")
                }
            }]
        }),
        // [3] tool result — this is what must reach the LLM on the next call
        json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": tool_result_json
        }),
    ]
}

/// Apply the same trimming logic used in `handle_chat_completion`.
/// Returns the trimmed message slice.
fn apply_trimmer(messages: &mut Vec<Value>, max_history: usize) {
    if messages.len() > max_history {
        let has_system = messages
            .first()
            .and_then(|m| m.get("role"))
            .and_then(|r| r.as_str())
            == Some("system");

        let trim_from = usize::from(has_system);
        let available = messages.len().saturating_sub(trim_from);
        let need = messages.len() - max_history;
        let actual = need.min(available.saturating_sub(1));
        if actual > 0 {
            messages.drain(trim_from..trim_from + actual);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. task_get — Format A  {"tasks":[…]}
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_get_format_a_finds_task_by_id() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    write_format_a(
        &dir,
        json!([
            { "id": 59, "title": "Previous", "status": "done", "priority": "low",
              "description": "", "details": "", "subtasks": [] },
            { "id": 60, "title": "Design RPL Architecture", "status": "pending",
              "priority": "high", "description": "Define RPL structure.",
              "details": "Full details here.", "subtasks": [] },
            { "id": 61, "title": "Next", "status": "pending", "priority": "medium",
              "description": "", "details": "", "subtasks": [] },
        ]),
    );

    let json_str = task_get(60.0, &sec).unwrap();
    let v: Value = serde_json::from_str(&json_str).expect("task_get must return valid JSON");

    assert_eq!(v["id"], 60, "wrong id returned");
    assert_eq!(v["title"], "Design RPL Architecture", "wrong title");
    assert_eq!(v["status"], "pending", "wrong status");
    assert_eq!(v["priority"], "high", "wrong priority");

    // Verify no adjacent task bleeds in
    assert_ne!(v["title"], "Previous", "returned wrong (previous) task");
    assert_ne!(v["title"], "Next", "returned wrong (next) task");
}

#[test]
fn task_get_format_a_not_found_returns_err() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);
    write_format_a(
        &dir,
        json!([{ "id": 1, "title": "Only task", "status": "pending",
                                   "subtasks": [] }]),
    );

    let err = task_get(999.0, &sec).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("999"), "error should mention the requested id");
    assert!(msg.contains("not found"), "error should say not found");
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. task_get — Format B  {"0":{…}, "1":{…}, …}
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_get_format_b_finds_task_by_inner_id() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    write_format_b(
        &dir,
        &[
            json!({ "id": 58, "title": "Task 58", "status": "done",    "subtasks": [] }),
            json!({ "id": 59, "title": "Task 59", "status": "pending", "subtasks": [] }),
            json!({
                "id": 60,
                "title": "Design RPL Architecture",
                "status": "pending",
                "priority": "high",
                "description": "Define RPL structure.",
                "details": "Full details.",
                "subtasks": [
                    { "id": "60.1", "title": "Sub-alpha", "status": "pending",
                      "dependencies": [] },
                    { "id": "60.2", "title": "Sub-beta",  "status": "pending",
                      "dependencies": ["60.1"] }
                ]
            }),
            json!({ "id": 61, "title": "Task 61", "status": "pending", "subtasks": [] }),
        ],
    );

    let json_str = task_get(60.0, &sec).unwrap();
    let v: Value = serde_json::from_str(&json_str).expect("task_get must return valid JSON");

    assert_eq!(v["id"], 60, "wrong task id");
    assert_eq!(v["title"], "Design RPL Architecture", "wrong title");
    assert_eq!(v["status"], "pending", "wrong status");
}

#[test]
fn task_get_format_b_key_does_not_equal_id() {
    // In Format B the outer key ("2") does not match the inner id (60).
    // task_get must look at the inner "id" field, not the key.
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    // Only three tasks; task 60 is at key "2"
    write_format_b(
        &dir,
        &[
            json!({ "id": 100, "title": "Hundred",   "status": "done",    "subtasks": [] }),
            json!({ "id": 200, "title": "TwoHundred","status": "pending", "subtasks": [] }),
            json!({ "id": 60,  "title": "Found Me",  "status": "pending", "subtasks": [] }),
        ],
    );

    let json_str = task_get(60.0, &sec).unwrap();
    let v: Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(
        v["title"], "Found Me",
        "should find by inner id, not outer key"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Subtask lookup — string IDs ("60.1")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_get_format_b_finds_string_subtask_id() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    write_format_b(
        &dir,
        &[json!({
            "id": 60,
            "title": "Parent task",
            "status": "pending",
            "subtasks": [
                { "id": "60.1", "title": "Sub one",   "status": "pending" },
                { "id": "60.2", "title": "Sub two",   "status": "pending" },
                { "id": "60.3", "title": "Sub three", "status": "done"    }
            ]
        })],
    );

    // 60.1
    let j = task_get(60.1, &sec).unwrap();
    let v: Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["title"], "Sub one", "subtask 60.1 should be 'Sub one'");

    // 60.3
    let j3 = task_get(60.3, &sec).unwrap();
    let v3: Value = serde_json::from_str(&j3).unwrap();
    assert_eq!(
        v3["title"], "Sub three",
        "subtask 60.3 should be 'Sub three'"
    );
    assert_eq!(v3["status"], "done");
}

#[test]
fn task_get_format_a_finds_numeric_subtask_id() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    write_format_a(
        &dir,
        json!([{
            "id": 60,
            "title": "Parent",
            "status": "pending",
            "priority": "high",
            "subtasks": [
                { "id": 60.1, "title": "First sub",  "status": "pending" },
                { "id": 60.2, "title": "Second sub", "status": "in_progress" }
            ]
        }]),
    );

    let j = task_get(60.2, &sec).unwrap();
    let v: Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["title"], "Second sub");
    assert_eq!(v["status"], "in_progress");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Tool result message format — what the LLM actually receives
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn tool_result_message_has_correct_role_and_content() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);
    write_format_a(
        &dir,
        json!([{
            "id": 60,
            "title": "Design RPL Architecture",
            "status": "pending",
            "priority": "high",
            "description": "Define RPL.",
            "details": "Detailed instructions.",
            "subtasks": []
        }]),
    );

    let result_json = task_get(60.0, &sec).unwrap();

    // Simulate what handle_chat_completion pushes to session.messages
    let tool_msg = json!({
        "role": "tool",
        "tool_call_id": "call_abc123",
        "content": result_json
    });

    // Structural checks
    assert_eq!(
        tool_msg["role"], "tool",
        "role must be 'tool' for the LLM to recognise the function result"
    );
    assert_eq!(
        tool_msg["tool_call_id"], "call_abc123",
        "tool_call_id must match the assistant's call"
    );

    // Content must be parseable JSON and contain the right data
    let content_str = tool_msg["content"]
        .as_str()
        .expect("content must be a JSON string");
    let content: Value =
        serde_json::from_str(content_str).expect("tool result content must be valid JSON");

    assert_eq!(content["id"], 60, "id in content");
    assert_eq!(
        content["title"], "Design RPL Architecture",
        "title in content"
    );
    assert_eq!(content["status"], "pending", "status in content");
}

#[test]
fn tool_error_message_contains_actionable_guidance() {
    let error_msg = format_tool_error_for_llm(
        "task_get",
        &json!({"id": 60}),
        "task_list.json is missing the 'tasks' array",
    );

    // Must have the TOOL ERROR header so the LLM knows this is a failure
    assert!(
        error_msg.contains("TOOL ERROR"),
        "error must start with TOOL ERROR header"
    );

    // Must name the tool
    assert!(
        error_msg.contains("task_get"),
        "error must name the failing tool"
    );

    // Must carry the raw error so the LLM can reason about it
    assert!(
        error_msg.contains("missing the 'tasks' array"),
        "raw error text must be present"
    );

    // Must include the "Do NOT repeat" guard to stop retry loops
    assert!(
        error_msg.contains("Do NOT repeat"),
        "error must include the do-not-repeat guard"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. History trimmer — system prompt and tool results survive
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn trimmer_never_removes_system_prompt() {
    let system = "System instructions";
    let mut messages = build_messages_with_tool_result(
        system,
        "What is task 60?",
        "task_get",
        "call_001",
        r#"{"id":60,"title":"Design RPL Architecture"}"#,
    );
    // 4 messages in, max_history = 3 → one must be evicted
    apply_trimmer(&mut messages, 3);

    assert_eq!(
        messages[0]["role"], "system",
        "system prompt must remain at index 0 after trimming"
    );
    assert_eq!(
        messages[0]["content"], system,
        "system prompt content must be unchanged"
    );
}

#[test]
fn trimmer_preserves_tool_result_after_evicting_user_turn() {
    let mut messages = build_messages_with_tool_result(
        "System",
        "What is task 60?",
        "task_get",
        "call_001",
        r#"{"id":60,"title":"Design RPL Architecture"}"#,
    );
    // We have [system, user, assistant, tool_result] = 4 messages.
    // max_history = 3 → evict the user turn (oldest non-system message).
    apply_trimmer(&mut messages, 3);

    assert_eq!(
        messages.len(),
        3,
        "should have exactly 3 messages after trim"
    );

    // System at [0]
    assert_eq!(messages[0]["role"], "system");

    // Tool result must still be present
    let has_tool_result = messages.iter().any(|m| m["role"] == "tool");
    assert!(
        has_tool_result,
        "tool result must not be evicted: {:?}",
        messages
            .iter()
            .map(|m| m["role"].as_str())
            .collect::<Vec<_>>()
    );

    // And the tool result content must be intact
    let tool_msg = messages.iter().find(|m| m["role"] == "tool").unwrap();
    let content: Value = serde_json::from_str(tool_msg["content"].as_str().unwrap()).unwrap();
    assert_eq!(
        content["title"], "Design RPL Architecture",
        "tool result content must be intact after trimming"
    );
}

#[test]
fn trimmer_preserves_all_messages_when_within_limit() {
    let mut messages = build_messages_with_tool_result(
        "System",
        "What is task 60?",
        "task_get",
        "call_001",
        r#"{"id":60,"title":"RPL"}"#,
    );
    let original_len = messages.len();
    apply_trimmer(&mut messages, original_len + 5); // plenty of headroom
    assert_eq!(
        messages.len(),
        original_len,
        "no messages should be evicted"
    );
}

#[test]
fn trimmer_handles_no_system_prompt() {
    let mut messages = vec![
        json!({"role": "user",      "content": "Hello"}),
        json!({"role": "assistant", "content": "Hi"}),
        json!({"role": "user",      "content": "Task 60?"}),
        json!({"role": "tool",      "tool_call_id": "c1",
               "content": r#"{"id":60,"title":"RPL"}"#}),
    ];
    apply_trimmer(&mut messages, 3);
    // Should keep the last 3 messages (trim from index 0)
    assert!(messages.len() <= 3);
    // Tool result must survive
    assert!(
        messages.iter().any(|m| m["role"] == "tool"),
        "tool result must survive when there is no system prompt"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Full pipeline snapshot: task_get → message → trimmer → content intact
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn full_pipeline_format_a_task60_survives_to_llm_context() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);
    write_format_a(
        &dir,
        json!([
            {"id": 58, "title": "T58", "status": "done",    "subtasks": []},
            {"id": 59, "title": "T59", "status": "done",    "subtasks": []},
            { "id": 60,
              "title": "Design RPL Architecture",
              "status": "pending",
              "priority": "high",
              "description": "Define RPL structure.",
              "details": "Implement according to spec.",
              "subtasks": [
                {"id": 60.1, "title": "Sub A", "status": "pending"},
                {"id": 60.2, "title": "Sub B", "status": "pending"}
              ]
            },
            {"id": 61, "title": "T61", "status": "pending", "subtasks": []},
        ]),
    );

    // Step 1: call the tool
    let result_json = task_get(60.0, &sec).expect("task_get must succeed for Format A");

    // Step 2: verify the raw result is correct
    let result: Value = serde_json::from_str(&result_json).unwrap();
    assert_eq!(result["id"], 60);
    assert_eq!(result["title"], "Design RPL Architecture");

    // Step 3: build the full message history as the ACP would
    let mut messages = build_messages_with_tool_result(
        "You are a coding assistant. Tasks are in .zed/task_list.json. \
         Use task_get to retrieve a specific task.",
        "What is task 60?",
        "task_get",
        "call_xyz",
        &result_json,
    );

    // Step 4: apply trimmer at a realistic limit (e.g. 20 messages)
    apply_trimmer(&mut messages, 20);

    // Step 5: verify the LLM would see the correct data
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool result message must be present");

    let tool_content: Value = serde_json::from_str(tool_msg["content"].as_str().unwrap())
        .expect("tool result must be valid JSON");

    assert_eq!(tool_content["id"], 60, "PIPELINE: id must be 60");
    assert_eq!(
        tool_content["title"], "Design RPL Architecture",
        "PIPELINE: title correct"
    );
    assert_eq!(
        tool_content["status"], "pending",
        "PIPELINE: status correct"
    );
    assert_eq!(
        tool_content["priority"], "high",
        "PIPELINE: priority correct"
    );
}

#[test]
fn full_pipeline_format_b_task60_survives_to_llm_context() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    // Replicate a subset of the actual bot project structure
    write_format_b(
        &dir,
        &[
            json!({"id": 58, "title": "T58", "status": "done",    "subtasks": []}),
            json!({"id": 59, "title": "T59", "status": "done",    "subtasks": []}),
            json!({
                "id": 60,
                "title": "Design RPL Architecture",
                "status": "pending",
                "priority": "high",
                "description": "Define RPL.",
                "details": "Details.",
                "subtasks": [
                    {"id": "60.1", "title": "Sub A", "status": "pending",
                     "dependencies": []},
                    {"id": "60.2", "title": "Sub B", "status": "pending",
                     "dependencies": ["60.1"]}
                ]
            }),
            json!({"id": 61, "title": "T61", "status": "pending", "subtasks": []}),
        ],
    );

    let result_json = task_get(60.0, &sec).expect("task_get must succeed for Format B");

    let result: Value = serde_json::from_str(&result_json).unwrap();
    assert_eq!(
        result["title"], "Design RPL Architecture",
        "FORMAT B: title must be correct before entering message pipeline"
    );

    let mut messages = build_messages_with_tool_result(
        "System prompt",
        "Task 60?",
        "task_get",
        "call_format_b",
        &result_json,
    );
    apply_trimmer(&mut messages, 20);

    let tool_msg = messages.iter().find(|m| m["role"] == "tool").unwrap();
    let content: Value = serde_json::from_str(tool_msg["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["id"], 60);
    assert_eq!(content["title"], "Design RPL Architecture");
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. task_create round-trip — written then read back correctly
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_create_then_task_get_round_trip() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);
    fs::create_dir_all(dir.path().join(".zed")).unwrap();

    task_create("Round-trip task", "desc", "high", vec![], &sec).unwrap();
    task_create("Second task", "desc", "low", vec![], &sec).unwrap();

    // task_get should find the first task by its auto-assigned id=1
    let json_str = task_get(1.0, &sec).unwrap();
    let v: Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["id"], 1);
    assert_eq!(v["title"], "Round-trip task");
    assert_eq!(v["status"], "pending");

    // And the second
    let j2 = task_get(2.0, &sec).unwrap();
    let v2: Value = serde_json::from_str(&j2).unwrap();
    assert_eq!(v2["title"], "Second task");
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Missing file — clear diagnostic, not a panic
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn task_get_missing_file_returns_descriptive_error() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);
    // No .zed directory created → file missing

    let err = task_get(60.0, &sec).unwrap_err().to_string();
    assert!(
        err.contains("task_list.json"),
        "error must mention the missing file: {err}"
    );
    // Must not contain a panic or unwrap trace
    assert!(!err.contains("panicked"), "must not panic: {err}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Format C — plain JSON array  [{…}, {…}, …]
// ─────────────────────────────────────────────────────────────────────────────

/// Write a `.zed/task_list.json` as a plain top-level JSON array (Format C).
/// This is the format used by the bot project.
fn write_format_c(dir: &TempDir, tasks: &[Value]) {
    let zed = dir.path().join(".zed");
    fs::create_dir_all(&zed).unwrap();
    fs::write(
        zed.join("task_list.json"),
        serde_json::to_string_pretty(&Value::Array(tasks.to_vec())).unwrap(),
    )
    .unwrap();
}

#[test]
fn task_get_format_c_finds_task_by_id() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    write_format_c(
        &dir,
        &[
            json!({ "id": 58, "title": "T58", "status": "done",    "subtasks": [] }),
            json!({ "id": 59, "title": "T59", "status": "pending", "subtasks": [] }),
            json!({
                "id": 60,
                "title": "Design RPL Architecture",
                "status": "pending",
                "priority": "high",
                "description": "Define RPL.",
                "details": "Full details.",
                "subtasks": []
            }),
            json!({ "id": 61, "title": "T61", "status": "pending", "subtasks": [] }),
        ],
    );

    let json_str = task_get(60.0, &sec).unwrap();
    let v: Value = serde_json::from_str(&json_str).expect("must be valid JSON");

    assert_eq!(v["id"], 60, "wrong id");
    assert_eq!(v["title"], "Design RPL Architecture", "wrong title");
    assert_eq!(v["status"], "pending");
    assert_eq!(v["priority"], "high");

    // Must not bleed into adjacent tasks
    assert_ne!(v["title"], "T59");
    assert_ne!(v["title"], "T61");
}

#[test]
fn task_get_format_c_not_found_returns_err() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);
    write_format_c(
        &dir,
        &[json!({ "id": 1, "title": "Only", "status": "pending", "subtasks": [] })],
    );

    let err = task_get(999.0, &sec).unwrap_err().to_string();
    assert!(err.contains("999"), "error must mention the requested id");
    assert!(err.contains("not found"));
}

#[test]
fn task_get_format_c_finds_string_subtask_id() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    write_format_c(
        &dir,
        &[json!({
            "id": 60,
            "title": "Parent",
            "status": "pending",
            "subtasks": [
                { "id": "60.1", "title": "Sub one",   "status": "pending" },
                { "id": "60.2", "title": "Sub two",   "status": "pending" },
                { "id": "60.3", "title": "Sub three", "status": "done"    }
            ]
        })],
    );

    let j = task_get(60.1, &sec).unwrap();
    let v: Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["title"], "Sub one");

    let j3 = task_get(60.3, &sec).unwrap();
    let v3: Value = serde_json::from_str(&j3).unwrap();
    assert_eq!(v3["title"], "Sub three");
    assert_eq!(v3["status"], "done");
}

#[test]
fn full_pipeline_format_c_task60_survives_to_llm_context() {
    let dir = TempDir::new().unwrap();
    let sec = make_security(&dir);

    write_format_c(
        &dir,
        &[
            json!({"id": 59, "title": "T59", "status": "done",    "subtasks": []}),
            json!({
                "id": 60,
                "title": "Design RPL Architecture",
                "status": "pending",
                "priority": "high",
                "description": "Define the RPL.",
                "details": "Implementation details.",
                "subtasks": [
                    {"id": "60.1", "title": "Sub A", "status": "pending"},
                    {"id": "60.2", "title": "Sub B", "status": "pending"}
                ]
            }),
            json!({"id": 61, "title": "T61", "status": "pending", "subtasks": []}),
        ],
    );

    // Step 1: tool call
    let result_json =
        task_get(60.0, &sec).expect("task_get must succeed for Format C (plain array)");

    let result: Value = serde_json::from_str(&result_json).unwrap();
    assert_eq!(result["id"], 60);
    assert_eq!(result["title"], "Design RPL Architecture");

    // Step 2: place into ACP message history
    let mut messages = build_messages_with_tool_result(
        "System prompt",
        "What is task 60?",
        "task_get",
        "call_format_c",
        &result_json,
    );
    apply_trimmer(&mut messages, 20);

    // Step 3: verify the LLM context contains the correct data
    let tool_msg = messages.iter().find(|m| m["role"] == "tool").unwrap();
    let content: Value = serde_json::from_str(tool_msg["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["id"], 60, "PIPELINE FORMAT C: id");
    assert_eq!(
        content["title"], "Design RPL Architecture",
        "PIPELINE FORMAT C: title"
    );
    assert_eq!(content["priority"], "high", "PIPELINE FORMAT C: priority");
}
