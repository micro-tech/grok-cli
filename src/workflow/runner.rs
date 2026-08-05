//! Basic workflow runner for Task 232.
//!
//! Demonstrates recording a full UserPrompt → LLM code → validation (cargo check/clippy/test) → Decision flow.
//!
//! Task 237: The validation portion can now be expressed as a TaskGraph for
//! parallel execution of independent steps (cargo check + cargo clippy).
//! The graph engine (`crate::task_graph`) runs independent nodes concurrently
//! via tokio JoinSet.

use super::{WorkflowStep, WorkflowTrace};
use crate::task_graph::{TaskGraph, TaskNode, ToolCall};

/// Run a simple local validation workflow on generated Rust code.
///
/// This writes the code to a temporary directory, runs cargo check/clippy/test,
/// records every step into a `WorkflowTrace`, and returns the trace.
///
/// Note: This is a demonstration for Task 232. Production use should integrate
/// with the existing shell tool security policy and the main tool loop.
pub fn run_cargo_validation_workflow(user_prompt: &str, generated_code: &str) -> WorkflowTrace {
    let mut trace = WorkflowTrace::new();

    trace.push(WorkflowStep::UserPrompt(user_prompt.to_string()));
    trace.push(WorkflowStep::LlmGeneratedCode(generated_code.to_string()));

    // Create a temp project for validation (very lightweight)
    let temp_dir = std::env::temp_dir().join(format!("grok-workflow-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);

    // Write a minimal Cargo.toml + src/main.rs or lib.rs
    let cargo_toml = r#"[package]
name = "grok-workflow-temp"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

    let is_lib = generated_code.contains("pub fn") || generated_code.contains("mod ");
    let src_file = if is_lib { "src/lib.rs" } else { "src/main.rs" };

    let _ = std::fs::create_dir_all(temp_dir.join("src"));
    let _ = std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml);
    let _ = std::fs::write(temp_dir.join(src_file), generated_code);

    // ── Task 237: Build and execute validation as a TaskGraph for parallelism ──
    // Independent steps (check + clippy) run concurrently.
    // test depends on both.
    //
    // We use the registered "run_shell_command" tool via the TaskGraph so that
    // execution goes through the normal security + tracing paths when used
    // from the main agent loop.
    //
    // For the standalone demo runner we create a minimal ToolContext and
    // run the graph inside a temporary tokio runtime.

    let temp_dir_str = temp_dir.display().to_string();

    // Helper to build a shell command node that cds into the temp project
    let make_cargo_node = |id: &str, cargo_args: &str| -> TaskNode {
        let full_cmd = format!("cd \"{}\" && cargo {}", temp_dir_str, cargo_args);
        TaskNode {
            id: id.to_string(),
            action: ToolCall {
                tool_name: "run_shell_command".to_string(),
                arguments: serde_json::json!({
                    "command": full_cmd
                }),
            },
            dependencies: vec![],
        }
    };

    let mut graph = TaskGraph::new();

    // Independent nodes (will run in parallel)
    graph.add_node(make_cargo_node("check", "check"));
    graph.add_node(make_cargo_node("clippy", "clippy -- -D warnings"));

    // test depends on both check and clippy succeeding at graph level
    // (we still record individual outcomes)
    let mut test_node = make_cargo_node("test", "test");
    test_node.dependencies = vec!["check".to_string(), "clippy".to_string()];
    graph.add_node(test_node);

    // Record that we are entering the parallel validation phase
    trace.push(WorkflowStep::ToolRun {
        tool: "task_graph:validation".to_string(),
        output: "Starting parallel cargo validation (check || clippy) → test".to_string(),
        success: true,
    });

    // Execute the graph (parallel where possible)
    let ctx = crate::tools::ToolContext::default_for_cwd();

    let graph_results: std::collections::HashMap<String, String> = {
        // We need a runtime because the demo runner is synchronous.
        // In real agent paths (CpuRouter / route_with_workflow_trace) this
        // would already be inside an async context.
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime for task graph");
        rt.block_on(async { graph.execute(&ctx).await })
            .unwrap_or_else(|e| {
                // On graph error, return empty results; individual steps will be marked failed below
                tracing::warn!("Task graph execution error (falling back to recorded failure): {}", e);
                std::collections::HashMap::new()
            })
    };

    // Map graph results back into WorkflowTrace as individual ToolRun steps.
    // This keeps the trace complete and human-readable even though execution was parallel.
    let check_out = graph_results.get("check").cloned().unwrap_or_else(|| "Graph node did not produce output (may have failed early)".to_string());
    let clippy_out = graph_results.get("clippy").cloned().unwrap_or_else(|| "Graph node did not produce output (may have failed early)".to_string());
    let test_out = graph_results.get("test").cloned().unwrap_or_else(|| "Graph node did not produce output (may have failed early or skipped)".to_string());

    // We consider success if the output does not contain obvious failure markers.
    // The real cargo exit code is reflected in whether the tool succeeded.
    let check_ok = !check_out.to_lowercase().contains("error") && !check_out.contains("aborting");
    let clippy_ok = !clippy_out.to_lowercase().contains("error") && !clippy_out.contains("aborting");
    let test_ok = !test_out.to_lowercase().contains("error") && !test_out.contains("aborting") && graph_results.contains_key("test");

    trace.push(WorkflowStep::ToolRun {
        tool: "cargo check".to_string(),
        output: check_out,
        success: check_ok,
    });

    trace.push(WorkflowStep::ToolRun {
        tool: "cargo clippy".to_string(),
        output: clippy_out,
        success: clippy_ok,
    });

    trace.push(WorkflowStep::ToolRun {
        tool: "cargo test".to_string(),
        output: test_out,
        success: test_ok,
    });

    let all_passed = check_ok && clippy_ok && test_ok;
    trace.push(WorkflowStep::Decision { passed: all_passed });

    if all_passed {
        trace.push(WorkflowStep::ReturnedToUser(
            "All validation steps passed. Code is ready.".to_string(),
        ));
    } else {
        trace.push(WorkflowStep::ReturnedToLlm(
            "Validation failed. Please review the tool outputs and provide a fix.".to_string(),
        ));
    }

    // Best-effort cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Task 234: Persist the completed trace to ~/.grok-cli/workflows/
    // We do this best-effort so a persistence failure never breaks the caller.
    if let Err(e) = crate::workflow::save_trace(&trace) {
        // In a real run we might log this, but for now we silently continue.
        // The trace is still returned in memory for the TUI viewer and /trace.
        let _ = e; // silence unused warning in some builds
    }

    trace
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_workflow_records_steps() {
        // Use a minimal valid Rust program
        let code = r#"
fn main() {
    println!("Hello from workflow test");
}
"#;

        let trace = run_cargo_validation_workflow("make a hello program", code);

        // Should have recorded the main stages
        assert!(
            trace
                .steps
                .iter()
                .any(|s| matches!(s, WorkflowStep::UserPrompt(_)))
        );
        assert!(
            trace
                .steps
                .iter()
                .any(|s| matches!(s, WorkflowStep::LlmGeneratedCode(_)))
        );
        assert!(
            trace
                .steps
                .iter()
                .any(|s| matches!(s, WorkflowStep::ToolRun { .. }))
        );
        assert!(
            trace
                .steps
                .iter()
                .any(|s| matches!(s, WorkflowStep::Decision { .. }))
        );

        // The last decision should be recorded
        assert!(trace.last_decision_passed().is_some());
    }
}
