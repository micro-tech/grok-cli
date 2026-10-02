//! HOH End-of-Iteration Summary (Task 297.29)
//!
//! Generates and saves human-readable summaries at the end of each HOH
//! iteration, covering what was attempted, results, metrics, and next steps.

use crate::hoh::state::IterationState;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Structured summary of one HOH iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationSummary {
    pub iteration_id: u64,
    pub status: String,
    pub goals_attempted: Vec<String>,
    pub patches_generated: usize,
    pub tests_passed: Option<bool>,
    pub helix_score: Option<f32>,
    pub improvements_proposed: Vec<String>,
    pub key_decisions: Vec<String>,
    pub next_recommended: Vec<String>,
    pub generated_at: u64,
}

/// Build an `IterationSummary` from a completed `IterationState`.
pub fn generate_summary(state: &IterationState) -> IterationSummary {
    let goals_attempted = state
        .plan
        .as_ref()
        .map(|p| p.goals.clone())
        .unwrap_or_default();

    let improvements_proposed = state
        .plan
        .as_ref()
        .map(|p| p.improvement_suggestions.clone())
        .unwrap_or_default();

    let helix_score = state
        .evaluations
        .iter()
        .filter_map(|e| e.helix_score)
        .reduce(f32::max);

    let tests_passed = state
        .evaluations
        .iter()
        .filter_map(|e| e.test_passed)
        .last();

    // Derive key decisions from evaluation notes
    let key_decisions: Vec<String> = state
        .evaluations
        .iter()
        .filter(|e| !e.notes.is_empty())
        .map(|e| e.notes.clone())
        .collect();

    // Recommend next steps based on test outcome
    let next_recommended = if tests_passed == Some(false) {
        vec![
            "Fix failing tests before next iteration".to_string(),
            "Review test output in evaluation report".to_string(),
        ]
    } else if improvements_proposed.is_empty() {
        vec!["Run evaluation to generate improvement proposals".to_string()]
    } else {
        improvements_proposed
            .iter()
            .take(3)
            .map(|s| format!("Consider: {}", s))
            .collect()
    };

    IterationSummary {
        iteration_id: state.iteration_id,
        status: format!("{:?}", state.status),
        goals_attempted,
        patches_generated: state.patches.len(),
        tests_passed,
        helix_score,
        improvements_proposed,
        key_decisions,
        next_recommended,
        generated_at: now_secs(),
    }
}

/// Format an `IterationSummary` as a readable markdown-like string.
pub fn format_summary(s: &IterationSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# HOH Iteration #{} Summary\n\n",
        s.iteration_id
    ));
    out.push_str(&format!("**Status:** {}\n", s.status));
    out.push_str(&format!(
        "**Generated:** {}\n\n",
        chrono::DateTime::from_timestamp(s.generated_at as i64, 0)
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| s.generated_at.to_string())
    ));

    out.push_str("## Goals Attempted\n");
    if s.goals_attempted.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for g in &s.goals_attempted {
            out.push_str(&format!("- {}\n", g));
        }
    }
    out.push('\n');

    out.push_str(&format!("## Patches Generated\n{}\n\n", s.patches_generated));

    out.push_str("## Test Results\n");
    match s.tests_passed {
        Some(true) => out.push_str("✅ Tests passed\n\n"),
        Some(false) => out.push_str("❌ Tests failed\n\n"),
        None => out.push_str("⬜ Tests not run\n\n"),
    }

    out.push_str("## Helix Score\n");
    match s.helix_score {
        Some(score) => out.push_str(&format!("{:.3}\n\n", score)),
        None => out.push_str("N/A\n\n"),
    }

    out.push_str("## Improvements Proposed\n");
    if s.improvements_proposed.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for imp in &s.improvements_proposed {
            out.push_str(&format!("- {}\n", imp));
        }
    }
    out.push('\n');

    out.push_str("## Key Decisions\n");
    if s.key_decisions.is_empty() {
        out.push_str("- (none recorded)\n");
    } else {
        for d in &s.key_decisions {
            out.push_str(&format!("- {}\n", d));
        }
    }
    out.push('\n');

    out.push_str("## Next Steps\n");
    for n in &s.next_recommended {
        out.push_str(&format!("- {}\n", n));
    }

    out
}

/// Generate, format, and save the summary for an iteration.
///
/// Writes to:
/// - `.grok/hoh/iterations/<id>/summary.md`
/// - Appends one-liner to `.grok/hoh/logs/summary_log.txt`
pub async fn save_summary(state: &IterationState, project_root: &Path) -> io::Result<()> {
    let summary = generate_summary(state);
    let text = format_summary(&summary);

    // Write full summary file
    let iter_dir = project_root
        .join(".grok/hoh/iterations")
        .join(state.iteration_id.to_string());
    tokio::fs::create_dir_all(&iter_dir).await?;
    tokio::fs::write(iter_dir.join("summary.md"), &text).await?;

    // Append one-liner to log
    let logs_dir = project_root.join(".grok/hoh/logs");
    tokio::fs::create_dir_all(&logs_dir).await?;
    let line = format!(
        "[{}] Iteration #{}: status={} patches={} helix={}\n",
        summary.generated_at,
        summary.iteration_id,
        summary.status,
        summary.patches_generated,
        summary
            .helix_score
            .map(|s| format!("{:.3}", s))
            .unwrap_or_else(|| "N/A".to_string()),
    );
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(logs_dir.join("summary_log.txt"))
        .await?;
    file.write_all(line.as_bytes()).await?;

    tracing::info!(
        iteration = state.iteration_id,
        "[HOH Summary] saved to {:?}",
        iter_dir
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_summary_non_empty() {
        let state = IterationState::new(42);
        let s = generate_summary(&state);
        assert_eq!(s.iteration_id, 42);
        assert!(!s.status.is_empty());
    }

    #[test]
    fn test_format_summary_contains_sections() {
        let state = IterationState::new(1);
        let s = generate_summary(&state);
        let text = format_summary(&s);
        assert!(text.contains("HOH Iteration #1"));
        assert!(text.contains("Goals Attempted"));
        assert!(text.contains("Next Steps"));
    }
}
