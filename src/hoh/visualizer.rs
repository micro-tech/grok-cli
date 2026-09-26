//! HOH Iteration Visualizer (Task 297.17)
//!
//! Terminal-friendly views of current and past HOH iterations.
//! Uses plain ASCII — no external TUI crate required.

use crate::hoh::state::{IterationState, IterationStatus};

const WIDE: usize = 72;

fn divider(ch: char) -> String {
    std::iter::repeat(ch).take(WIDE).collect()
}

fn status_icon(s: &IterationStatus) -> &'static str {
    match s {
        IterationStatus::Planning   => "📋",
        IterationStatus::Executing  => "⚙️ ",
        IterationStatus::Testing    => "🧪",
        IterationStatus::Evaluating => "📊",
        IterationStatus::Completed  => "✅",
        IterationStatus::Failed     => "❌",
    }
}

/// Render a single iteration as a multi-line display string.
pub fn render_iteration(state: &IterationState) -> String {
    let mut out = String::new();

    out.push_str(&divider('='));
    out.push('\n');
    out.push_str(&format!(
        " {} HOH Iteration #{:>4}  [{:?}]\n",
        status_icon(&state.status),
        state.iteration_id,
        state.status
    ));
    out.push_str(&divider('='));
    out.push('\n');

    // Goals
    out.push_str("\nGOALS\n");
    out.push_str(&divider('-'));
    out.push('\n');
    if let Some(plan) = &state.plan {
        if plan.goals.is_empty() {
            out.push_str("  (no goals defined)\n");
        } else {
            for (i, g) in plan.goals.iter().enumerate() {
                out.push_str(&format!("  {}. {}\n", i + 1, g));
            }
        }
    } else {
        out.push_str("  (planning not yet started)\n");
    }

    // Patches
    out.push('\n');
    out.push_str("PATCHES\n");
    out.push_str(&divider('-'));
    out.push('\n');
    if state.patches.is_empty() {
        out.push_str("  (none generated)\n");
    } else {
        out.push_str(&format!("  {} patch(es) generated\n", state.patches.len()));
        for p in state.patches.iter().take(5) {
            let files = p.files_changed.join(", ");
            out.push_str(&format!("  • [{}] {}\n", p.id, files));
        }
        if state.patches.len() > 5 {
            out.push_str(&format!("  ... and {} more\n", state.patches.len() - 5));
        }
    }

    // Evaluations
    out.push('\n');
    out.push_str("EVALUATIONS\n");
    out.push_str(&divider('-'));
    out.push('\n');
    if state.evaluations.is_empty() {
        out.push_str("  (not yet evaluated)\n");
    } else {
        for (i, e) in state.evaluations.iter().enumerate() {
            let helix = e.helix_score.map(|s| format!("{:.3}", s)).unwrap_or_else(|| "N/A".to_string());
            let test = match e.test_passed {
                Some(true) => "PASS",
                Some(false) => "FAIL",
                None => "N/A",
            };
            out.push_str(&format!(
                "  [{}] helix={} tests={} patches={}\n",
                i + 1, helix, test, e.patch_count
            ));
        }
    }

    // Summary
    out.push('\n');
    out.push_str("SUMMARY\n");
    out.push_str(&divider('-'));
    out.push('\n');
    match &state.summary {
        Some(s) => {
            for line in s.lines() {
                out.push_str(&format!("  {}\n", line));
            }
        }
        None => out.push_str("  (no summary yet)\n"),
    }

    out.push_str(&divider('='));
    out.push('\n');
    out
}

/// Render a compact one-line-per-iteration history table.
pub fn render_history(states: &[IterationState]) -> String {
    let mut out = String::new();
    let header = format!(
        "{:<6} {:<12} {:>8} {:>7} {:>8}",
        "ID", "Status", "Score", "Patches", "Goals"
    );
    out.push_str(&divider('-'));
    out.push('\n');
    out.push_str(&format!(" {}\n", header));
    out.push_str(&divider('-'));
    out.push('\n');

    for s in states {
        let score = s
            .evaluations
            .iter()
            .filter_map(|e| e.helix_score)
            .reduce(f32::max)
            .map(|v| format!("{:.3}", v))
            .unwrap_or_else(|| "N/A".to_string());
        let goals = s.plan.as_ref().map(|p| p.goals.len()).unwrap_or(0);
        out.push_str(&format!(
            " {:<6} {:<12} {:>8} {:>7} {:>8}\n",
            s.iteration_id,
            format!("{:?}", s.status),
            score,
            s.patches.len(),
            goals,
        ));
    }

    out.push_str(&divider('-'));
    out.push('\n');
    out
}

/// Print a single iteration to stdout.
pub fn print_iteration(state: &IterationState) {
    print!("{}", render_iteration(state));
}

/// Print iteration history table to stdout.
pub fn print_history(states: &[IterationState]) {
    print!("{}", render_history(states));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_iteration_non_empty() {
        let state = IterationState::new(7);
        let out = render_iteration(&state);
        assert!(!out.is_empty());
        assert!(out.contains("HOH Iteration #"));
        assert!(out.contains("Planning"));
    }

    #[test]
    fn test_render_history_with_multiple_states() {
        let states: Vec<IterationState> = (1u64..=3).map(IterationState::new).collect();
        let out = render_history(&states);
        assert!(out.contains('1') && out.contains('2') && out.contains('3'));
    }
}
