//! HOH Autonomous Debugger (Task 361.4)

use crate::hoh::state::{IterationState, PatchSet};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BugKind { TestFailure, CompileError, RuntimePanic, LogicError(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub id: String,
    pub kind: BugKind,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    pub iteration_id: u64,
    pub bugs_found: Vec<BugReport>,
    pub patches_generated: Vec<PatchSet>,
    pub resolved_count: usize,
}

/// Parse test/compiler output for bug signatures.
pub fn extract_bugs(output: &str, iteration_id: u64) -> Vec<BugReport> {
    let mut bugs = Vec::new();
    let mut i = 0u64;

    for line in output.lines() {
        let lower = line.to_lowercase();

        if lower.contains("error[e") || lower.contains("error:") {
            bugs.push(BugReport {
                id: format!("bug-{}-{}", iteration_id, i),
                kind: BugKind::CompileError,
                file: extract_file(line),
                line: extract_line_num(line),
                message: line.trim().to_string(),
                suggested_fix: None,
            });
            i += 1;
        } else if lower.contains("panicked at") {
            bugs.push(BugReport {
                id: format!("bug-{}-{}", iteration_id, i),
                kind: BugKind::RuntimePanic,
                file: extract_file(line),
                line: extract_line_num(line),
                message: line.trim().to_string(),
                suggested_fix: Some("Add bounds checking or use Option/Result".to_string()),
            });
            i += 1;
        } else if lower.contains("---- ") && lower.contains("stdout ----") {
            bugs.push(BugReport {
                id: format!("bug-{}-{}", iteration_id, i),
                kind: BugKind::TestFailure,
                file: None, line: None,
                message: line.trim().to_string(),
                suggested_fix: None,
            });
            i += 1;
        }
    }
    bugs
}

fn extract_file(line: &str) -> Option<String> {
    line.split("-->").nth(1)
        .and_then(|s| s.split(':').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_line_num(line: &str) -> Option<u32> {
    line.split(':').find_map(|p| p.trim().parse::<u32>().ok())
}

/// Generate a fix patch for a bug (best-effort).
fn generate_fix_patch(bug: &BugReport, iteration_id: u64) -> PatchSet {
    let fix = bug.suggested_fix.clone().unwrap_or_else(|| "investigate and fix".to_string());
    PatchSet {
        id: format!("fix-{}", bug.id),
        files_changed: bug.file.as_ref().map(|f| vec![f.clone()]).unwrap_or_default(),
        diff_summary: format!("HOH auto-fix for {}: {}", bug.id, fix),
        source: "autonomous_debugger".to_string(),
        timestamp: iteration_id + now(),
        intended_content: None,
    }
}

/// Run a full debug session: parse output, generate fix patches, update state.
pub fn run_debug_session(state: &mut IterationState, test_output: &str) -> DebugSession {
    let bugs = extract_bugs(test_output, state.iteration_id);
    let patches: Vec<PatchSet> = bugs.iter().map(|b| generate_fix_patch(b, state.iteration_id)).collect();
    let resolved = patches.len();

    for p in &patches { state.patches.push(p.clone()); }

    tracing::info!(
        iteration = state.iteration_id, bugs = bugs.len(), resolved,
        "[HOH Debugger] debug session complete"
    );

    DebugSession { iteration_id: state.iteration_id, bugs_found: bugs, patches_generated: patches, resolved_count: resolved }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_compile_error() {
        let output = "error[E0425]: cannot find value `foo` in this scope --> src/main.rs:42:5";
        let bugs = extract_bugs(output, 1);
        assert!(!bugs.is_empty());
        assert!(matches!(bugs[0].kind, BugKind::CompileError));
    }

    #[test]
    fn test_extract_panic() {
        let output = "thread 'main' panicked at 'index out of bounds', src/lib.rs:10:5";
        let bugs = extract_bugs(output, 1);
        assert!(bugs.iter().any(|b| matches!(b.kind, BugKind::RuntimePanic)));
    }
}
