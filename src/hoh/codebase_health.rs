//! HOH Codebase Health Analyzer (Task 361.14)

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub clippy_warnings: usize,
    pub test_count: usize,
    pub todo_count: usize,
    pub unwrap_count: usize,
    pub dead_code_warnings: usize,
    pub health_score: f32,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Run cargo clippy and extract warning count (best-effort).
fn run_clippy(project_root: &Path) -> usize {
    Command::new("cargo")
        .args(["clippy", "--message-format=short", "--quiet"])
        .current_dir(project_root)
        .output()
        .ok()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stderr);
            out.lines().filter(|l| l.contains("warning:")).count()
        })
        .unwrap_or(0)
}

/// Count occurrences of patterns across all .rs files.
fn count_pattern(project_root: &Path, pattern: &str) -> usize {
    let src = project_root.join("src");
    walkdir::WalkDir::new(&src)
        .into_iter()
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("rs"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .map(|c| c.matches(pattern).count())
        .sum()
}

/// Produce a comprehensive health report.
pub fn analyze_health(project_root: &Path, run_tools: bool) -> HealthReport {
    let clippy_warnings = if run_tools { run_clippy(project_root) } else { 0 };
    let todo_count = count_pattern(project_root, "TODO");
    let unwrap_count = count_pattern(project_root, ".unwrap()");

    // Estimate test count from #[test] attributes
    let test_count = count_pattern(project_root, "#[test]")
        + count_pattern(project_root, "#[tokio::test]");

    let dead_code_warnings = if run_tools {
        Command::new("cargo")
            .args(["check", "--message-format=short", "--quiet"])
            .current_dir(project_root)
            .output()
            .ok()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stderr);
                out.lines().filter(|l| l.contains("dead_code")).count()
            })
            .unwrap_or(0)
    } else { 0 };

    let mut issues = Vec::new();
    let mut recommendations = Vec::new();

    if clippy_warnings > 10 { issues.push(format!("{} clippy warnings", clippy_warnings)); }
    if todo_count > 20       { issues.push(format!("{} TODO comments", todo_count)); }
    if unwrap_count > 50     { issues.push(format!("{} .unwrap() calls", unwrap_count));
                               recommendations.push("Replace .unwrap() with ? or proper error handling".to_string()); }
    if test_count < 10       { issues.push("Low test count".to_string());
                               recommendations.push("Add more unit and integration tests".to_string()); }

    // Score: start at 1.0, subtract penalties
    let score = (1.0
        - (clippy_warnings as f32 * 0.005).min(0.3)
        - (todo_count as f32 * 0.002).min(0.2)
        - (unwrap_count as f32 * 0.001).min(0.2)
        - if test_count < 10 { 0.2 } else { 0.0 }
    ).max(0.0);

    HealthReport {
        clippy_warnings, test_count, todo_count, unwrap_count,
        dead_code_warnings, health_score: score, issues, recommendations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_report_without_tools() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let report = analyze_health(&root, false);
        // Should have at least some test annotations
        assert!(report.test_count > 0);
        assert!(report.health_score >= 0.0);
    }
}
