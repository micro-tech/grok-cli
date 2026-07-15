//! Persistence helpers for WorkflowTrace (Task 234).
//!
//! Saves completed workflow traces as pretty-printed JSON under
//! `~/.grok-cli/workflows/`.
//!
//! Filenames are timestamped for easy sorting and replay:
//!   workflow-20250405-143022.json
//!
//! Provides:
//! - save_trace
//! - load_trace
//! - list_traces
//! - load_latest_trace (convenience)
//!
//! Traces are kept ephemeral by default (no automatic cleanup yet).

use crate::config::grok_data_dir;
use crate::workflow::WorkflowTrace;
use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// Returns the dedicated workflows directory: `~/.grok-cli/workflows/`
pub fn workflows_dir() -> PathBuf {
    grok_data_dir().join("workflows")
}

/// Ensures the workflows directory exists (creates it if needed).
pub fn ensure_workflows_dir() -> Result<PathBuf> {
    let dir = workflows_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create workflows directory at {:?}", dir))?;
    Ok(dir)
}

/// Save a completed WorkflowTrace to disk as JSON.
///
/// Returns the full path where it was written.
///
/// Example filename: `~/.grok-cli/workflows/workflow-20250405-143022.json`
pub fn save_trace(trace: &WorkflowTrace) -> Result<PathBuf> {
    let dir = ensure_workflows_dir()?;
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let filename = format!("workflow-{}.json", timestamp);
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(trace)
        .context("Failed to serialize WorkflowTrace to JSON")?;

    fs::write(&path, json)
        .with_context(|| format!("Failed to write workflow trace to {:?}", path))?;

    Ok(path)
}

/// Load a WorkflowTrace from a JSON file.
pub fn load_trace(path: &Path) -> Result<WorkflowTrace> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read trace file: {:?}", path))?;

    let trace: WorkflowTrace = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to deserialize WorkflowTrace from {:?}", path))?;

    Ok(trace)
}

/// List all saved workflow trace files (newest first).
///
/// Only returns `.json` files that look like workflow traces.
pub fn list_traces() -> Result<Vec<PathBuf>> {
    let dir = workflows_dir();

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("Failed to read workflows directory: {:?}", dir))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| {
            p.extension()
                .map_or(false, |ext| ext == "json")
                && p.file_name()
                    .map_or(false, |name| name.to_string_lossy().starts_with("workflow-"))
        })
        .collect();

    // Sort newest first by filename (timestamps are sortable)
    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    Ok(entries)
}

/// Convenience: load the most recently saved trace, if any.
pub fn load_latest_trace() -> Result<Option<WorkflowTrace>> {
    let traces = list_traces()?;
    match traces.first() {
        Some(path) => {
            let trace = load_trace(path)?;
            Ok(Some(trace))
        }
        None => Ok(None),
    }
}

/// Returns a human-readable summary for a trace file path (for /trace list etc.).
pub fn trace_file_summary(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{WorkflowStep, WorkflowTrace};
    use tempfile::tempdir;

    // We can't easily override grok_data_dir in tests without changing the API,
    // so we test the core save/load logic directly and use a temp dir for isolation.

    fn make_sample_trace() -> WorkflowTrace {
        let mut trace = WorkflowTrace::new();
        trace.push(WorkflowStep::UserPrompt("fix the tests".into()));
        trace.push(WorkflowStep::LlmGeneratedCode("fn main() { println!(\"hi\"); }".into()));
        trace.push(WorkflowStep::ToolRun {
            tool: "cargo check".into(),
            output: "error: ...".into(),
            success: false,
        });
        trace.push(WorkflowStep::Decision { passed: false });
        trace.push(WorkflowStep::ReturnedToLlm("please fix the compile error".into()));
        trace
    }

    #[test]
    fn save_and_load_roundtrip() {
        let trace = make_sample_trace();
        let original_len = trace.steps.len();

        // Use a real temp dir + write directly to test serialization roundtrip
        let temp = tempdir().unwrap();
        let path = temp.path().join("test-trace.json");

        let json = serde_json::to_string_pretty(&trace).unwrap();
        std::fs::write(&path, json).unwrap();

        let loaded = load_trace(&path).unwrap();
        assert_eq!(loaded.steps.len(), original_len);
        assert_eq!(loaded.last_decision_passed(), Some(false));
    }

    #[test]
    fn list_traces_filters_correctly() {
        // This test only exercises the filtering logic on a temp dir
        let temp = tempdir().unwrap();
        let dir = temp.path();

        // Create some files
        std::fs::write(dir.join("workflow-20250405-120000.json"), "{}").unwrap();
        std::fs::write(dir.join("workflow-20250405-130000.json"), "{}").unwrap();
        std::fs::write(dir.join("not-a-workflow.json"), "{}").unwrap();
        std::fs::write(dir.join("random.txt"), "hello").unwrap();

        // Manually test the filter logic (since we can't easily swap the global dir)
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map_or(false, |ext| ext == "json")
                    && p.file_name()
                        .map_or(false, |name| name.to_string_lossy().starts_with("workflow-"))
            })
            .collect();

        entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        assert_eq!(entries.len(), 2);
        assert!(entries[0].to_string_lossy().contains("130000"));
    }

    #[test]
    fn load_nonexistent_returns_error() {
        let result = load_trace(Path::new("/nonexistent/does-not-exist.json"));
        assert!(result.is_err());
    }
}
