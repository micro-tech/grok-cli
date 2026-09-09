//! HOH Testing Phase (Task 297.6 + rich feedback)
//!
//! Returns structured results (raw output + failure hints) so 361.5 meta loops
//! and planners can see real errors instead of blank or just a bool.

use crate::hoh::state::IterationState;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct TestRunResult {
    pub passed: bool,
    pub summary: String,
    pub raw_output: String,
    pub failing_items: Vec<String>,
    pub exit_code: Option<i32>,
}

pub async fn run_testing(state: &mut IterationState, data_dir: &std::path::Path) -> Result<TestRunResult, String> {
    state.status = crate::hoh::state::IterationStatus::Testing;

    let output = Command::new("cargo")
        .args(["test", "--quiet"])
        .current_dir(data_dir)
        .output();

    let res = match output {
        Ok(out) => {
            let success = out.status.success();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let combined = format!("{}\n{}", stderr, stdout);

            let mut fails = vec![];
            for line in combined.lines() {
                let l = line.trim();
                if l.contains("FAILED") || l.contains("error[") || l.contains("test ") && l.contains("FAILED") {
                    fails.push(l.chars().take(120).collect());
                    if fails.len() > 5 { break; }
                }
            }

            let summary = if success { "cargo test: PASSED".into() } else {
                let tail = if combined.len() > 450 { &combined[combined.len()-450..] } else { &combined };
                format!("cargo test: FAILED — {}", tail.chars().take(220).collect::<String>())
            };

            let full = format!("=== HOH test ===\nSTDOUT:\n{}\nSTDERR:\n{}\n=== end ===", 
                if stdout.is_empty() { "(empty)" } else { &stdout }, 
                if stderr.is_empty() { "(empty)" } else { &stderr });

            TestRunResult {
                passed: success,
                summary,
                raw_output: full,
                failing_items: fails,
                exit_code: out.status.code(),
            }
        }
        Err(e) => TestRunResult {
            passed: false,
            summary: format!("cargo unavailable: {}", e),
            raw_output: e.to_string(),
            failing_items: vec![e.to_string()],
            exit_code: None,
        }
    };

    state.summary = Some(format!("{} | Tests: {}", state.summary.clone().unwrap_or_default(), if res.passed { "PASSED" } else { "FAILED" }));
    Ok(res)
}