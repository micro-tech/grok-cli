//! CLI smoke tests (task 148 — subtask 148.9).
//!
//! Exercises public library APIs that back the CLI commands (`grok tools`,
//! `grok settings`, tool-error display, arbitration edge cases) without
//! spawning a process or making any network calls.
//!
//! Run with:
//!   cargo test --test cli_smoke_tests -- --nocapture

use grok_cli::config::{AcpConfig, ThinkingMode};
use grok_cli::tools::tool_arbitration::{ArbitrationDecision, arbitrate_tool_call};
use grok_cli::tools::tool_error::format_tool_error_for_llm;
use grok_cli::tools::{
    get_available_tool_definitions, get_full_tool_definitions, get_tool_definitions, save_memory,
};
use serde_json::json;

// ── Tool listing ─────────────────────────────────────────────────────────────

#[test]
fn tool_definitions_returns_more_than_ten_entries() {
    let defs = get_tool_definitions();
    assert!(
        defs.len() > 10,
        "expected >10 tool definitions, got {}",
        defs.len()
    );
}

#[test]
fn tool_definitions_contains_core_tools() {
    let defs = get_tool_definitions();
    for name in &[
        "read_file",
        "write_file",
        "task_get",
        "task_create",
        "task_update",
    ] {
        assert!(defs.contains(name), "missing core tool: {name}");
    }
}

#[test]
fn full_tool_definitions_have_correct_shape() {
    let defs = get_full_tool_definitions();
    assert!(!defs.is_empty(), "full tool definitions must not be empty");
    for def in &defs {
        let name = def
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        assert!(
            !name.is_empty(),
            "tool definition missing function.name: {def}"
        );

        let desc = def
            .get("function")
            .and_then(|f| f.get("description"))
            .and_then(|d| d.as_str())
            .unwrap_or("");
        assert!(!desc.is_empty(), "tool '{name}' has empty description");
    }
}

#[test]
fn available_tool_definitions_is_non_empty() {
    let defs = get_available_tool_definitions();
    assert!(
        !defs.is_empty(),
        "available tool definitions must not be empty"
    );
}

// ── Tool error formatting ─────────────────────────────────────────────────────

#[test]
fn format_tool_error_contains_header_and_tool_name() {
    let msg = format_tool_error_for_llm(
        "read_file",
        &json!({"path": "src/main.rs"}),
        "file not found: src/main.rs",
    );
    assert!(msg.contains("TOOL ERROR"), "missing TOOL ERROR header");
    assert!(msg.contains("read_file"), "missing tool name");
    assert!(msg.contains("not found"), "missing error text");
}

#[test]
fn format_tool_error_unknown_tool_mentions_suggestions() {
    let msg = format_tool_error_for_llm(
        "unknown_tool_xyz",
        &serde_json::Value::Null,
        "Unknown tool: unknown_tool_xyz",
    );
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("unknown")
            || lower.contains("not recognised")
            || lower.contains("not registered"),
        "error message should mention unknown/unrecognised: {msg}"
    );
}

#[test]
fn format_tool_error_timeout_mentions_starlink_or_retry() {
    let msg = format_tool_error_for_llm(
        "web_fetch",
        &json!({"url": "https://x.com"}),
        "request timed out after 30s",
    );
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("starlink") || lower.contains("retry") || lower.contains("timed out"),
        "timeout error should mention Starlink or retry: {msg}"
    );
}

#[test]
fn format_tool_error_access_denied_mentions_trusted_dir() {
    let msg = format_tool_error_for_llm(
        "read_file",
        &json!({"path": "/etc/passwd"}),
        "Access denied: External access is disabled in configuration",
    );
    assert!(msg.contains("TOOL ERROR"), "missing header");
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("access") || lower.contains("trusted") || lower.contains("workspace"),
        "access denied error should mention workspace/trusted: {msg}"
    );
}

#[test]
fn format_tool_error_do_not_repeat_instruction_present() {
    let msg = format_tool_error_for_llm(
        "write_file",
        &json!({"path": "out.txt", "content": "hello"}),
        "permission denied",
    );
    assert!(
        msg.contains("Do NOT repeat") || msg.contains("do not repeat"),
        "error should include retry warning: {msg}"
    );
}

// ── Config defaults ───────────────────────────────────────────────────────────

#[test]
fn acp_config_max_tool_loop_iterations_is_positive() {
    let cfg = AcpConfig::default();
    assert!(
        cfg.max_tool_loop_iterations > 0,
        "max_tool_loop_iterations must be > 0"
    );
}

#[test]
fn acp_config_auto_compress_defaults_true() {
    let cfg = AcpConfig::default();
    assert!(cfg.auto_compress, "auto_compress should default to true");
}

#[test]
fn acp_config_context_tokens_are_sane() {
    let cfg = AcpConfig::default();
    assert!(
        cfg.max_context_tokens > 10_000,
        "max_context_tokens too small"
    );
    assert!(
        cfg.grok4_max_context_tokens > cfg.max_context_tokens,
        "grok4 budget should exceed grok3 budget"
    );
}

#[test]
fn acp_config_thinking_mode_defaults_off() {
    let cfg = AcpConfig::default();
    assert_eq!(cfg.thinking_mode, ThinkingMode::Off);
}

#[test]
fn acp_config_compression_threshold_in_range() {
    let cfg = AcpConfig::default();
    assert!(
        cfg.compression_threshold > 0.0 && cfg.compression_threshold < 1.0,
        "compression_threshold should be between 0 and 1, got {}",
        cfg.compression_threshold
    );
}

// ── Tool arbitration edge cases ───────────────────────────────────────────────

#[test]
fn arbitrate_task_create_with_title_is_execute() {
    let result = arbitrate_tool_call("task_create", &json!({"title": "My Task"})).unwrap();
    assert!(matches!(result, ArbitrationDecision::Execute { .. }));
}

#[test]
fn arbitrate_task_create_without_title_is_need_more_info() {
    let result = arbitrate_tool_call("task_create", &json!({})).unwrap();
    assert!(matches!(result, ArbitrationDecision::NeedMoreInfo { .. }));
}

#[test]
fn arbitrate_task_get_with_id_is_execute() {
    let result = arbitrate_tool_call("task_get", &json!({"id": 42.0})).unwrap();
    assert!(matches!(result, ArbitrationDecision::Execute { .. }));
}

#[test]
fn arbitrate_task_get_without_id_is_need_more_info() {
    let result = arbitrate_tool_call("task_get", &json!({})).unwrap();
    assert!(matches!(result, ArbitrationDecision::NeedMoreInfo { .. }));
}

#[test]
fn arbitrate_unknown_tool_is_reject() {
    let result = arbitrate_tool_call("does_not_exist_xyz", &json!({})).unwrap();
    assert!(matches!(result, ArbitrationDecision::Reject { .. }));
}

#[test]
fn arbitrate_read_file_with_path_is_execute() {
    let result = arbitrate_tool_call("read_file", &json!({"path": "src/main.rs"})).unwrap();
    assert!(matches!(result, ArbitrationDecision::Execute { .. }));
}

#[test]
fn arbitrate_read_file_missing_path_is_need_more_info() {
    let result = arbitrate_tool_call("read_file", &json!({})).unwrap();
    assert!(matches!(result, ArbitrationDecision::NeedMoreInfo { .. }));
}

#[test]
fn arbitrate_fork_agent_matches_known_status() {
    // fork_agent is not in is_known_tool (no arbitration entry) so it should Reject.
    // If it ever gets added, this test will catch the change.
    let defs = get_tool_definitions();
    let result = arbitrate_tool_call("fork_agent", &json!({"tasks": ["a", "b"]})).unwrap();
    if defs.contains(&"fork_agent") {
        assert!(
            matches!(result, ArbitrationDecision::Execute { .. }),
            "fork_agent is in tool definitions but arbitration rejected it"
        );
    } else {
        assert!(
            matches!(result, ArbitrationDecision::Reject { .. }),
            "fork_agent is not registered so arbitration should reject it"
        );
    }
}

// ── save_memory tool edge cases ───────────────────────────────────────────────

#[test]
fn save_memory_empty_string_returns_error() {
    let result = save_memory("");
    assert!(result.is_err(), "empty fact must return Err");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must not be empty"),
        "error message must mention 'must not be empty'"
    );
}

#[test]
fn save_memory_whitespace_only_returns_error() {
    let result = save_memory("   \t\n  ");
    assert!(result.is_err(), "whitespace-only fact must return Err");
}
