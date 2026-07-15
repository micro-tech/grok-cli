//! Basic workflow runner for Task 232.
//!
//! Demonstrates recording a full UserPrompt → LLM code → validation (cargo check/clippy/test) → Decision flow.
//! This is intentionally simple; real usage will come from the CpuRouter + tool loop or a dedicated code workflow command.

use super::{WorkflowStep, WorkflowTrace};
use std::process::Command;

/// Run a simple local validation workflow on generated Rust code.
///
/// This writes the code to a temporary directory, runs cargo check/clippy/test,
/// records every step into a `WorkflowTrace`, and returns the trace.
///
/// Note: This is a demonstration for Task 232. Production use should integrate
/// with the existing shell tool security policy and the main tool loop.
pub fn run_cargo_validation_workflow(
    user_prompt: &str,
    generated_code: &str,
) -> WorkflowTrace {
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

    // Helper to run a cargo command and record the result
    let run_and_record = |trace: &mut WorkflowTrace, cmd: &str, args: &[&str]| {
        let output = Command::new("cargo")
            .current_dir(&temp_dir)
            .args(args)
            .output();

        let (success, stdout) = match output {
            Ok(o) => {
                let combined = format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                (o.status.success(), combined)
            }
            Err(e) => (false, format!("Failed to run {}: {}", cmd, e)),
        };

        trace.push(WorkflowStep::ToolRun {
            tool: cmd.to_string(),
            output: stdout,
            success,
        });

        success
    };

    // 1. cargo check
    let check_ok = run_and_record(&mut trace, "cargo check", &["check"]);

    // 2. cargo clippy (only if check passed, to save time)
    let mut clippy_ok = true;
    if check_ok {
        clippy_ok = run_and_record(&mut trace, "cargo clippy", &["clippy", "--", "-D", "warnings"]);
    } else {
        trace.push(WorkflowStep::ToolRun {
            tool: "cargo clippy".to_string(),
            output: "Skipped because cargo check failed.".to_string(),
            success: false,
        });
    }

    // 3. cargo test (only if previous steps passed)
    let mut test_ok = true;
    if check_ok && clippy_ok {
        test_ok = run_and_record(&mut trace, "cargo test", &["test"]);
    } else {
        trace.push(WorkflowStep::ToolRun {
            tool: "cargo test".to_string(),
            output: "Skipped due to earlier failure.".to_string(),
            success: false,
        });
    }

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
        assert!(trace.steps.iter().any(|s| matches!(s, WorkflowStep::UserPrompt(_))));
        assert!(trace.steps.iter().any(|s| matches!(s, WorkflowStep::LlmGeneratedCode(_))));
        assert!(trace.steps.iter().any(|s| matches!(s, WorkflowStep::ToolRun { .. })));
        assert!(trace.steps.iter().any(|s| matches!(s, WorkflowStep::Decision { .. })));

        // The last decision should be recorded
        assert!(trace.last_decision_passed().is_some());
    }
}