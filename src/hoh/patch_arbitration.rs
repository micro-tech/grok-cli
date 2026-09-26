//! HOH Multi-Agent Patch Arbitration (Task 361.38)

use crate::hoh::state::PatchSet;
use crate::hoh::patch_risk::score_risk;
use crate::hoh::patch_quality::score_patch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrationDecision {
    pub winner_id: String,
    pub rejected_ids: Vec<String>,
    pub reasoning: String,
    pub composite_score: f32,
}

/// Arbitrate between competing patches for the same files.
/// Returns the best patch to apply and rejects the rest.
pub fn arbitrate(competing: &[PatchSet]) -> Option<ArbitrationDecision> {
    if competing.is_empty() { return None; }
    if competing.len() == 1 {
        return Some(ArbitrationDecision {
            winner_id: competing[0].id.clone(),
            rejected_ids: Vec::new(),
            reasoning: "Only one candidate".to_string(),
            composite_score: 1.0,
        });
    }

    // Score each patch: quality bonus, risk penalty
    let mut scored: Vec<(&PatchSet, f32)> = competing.iter().map(|p| {
        let quality = score_patch(p).score;
        let risk = score_risk(p).risk_score;
        let composite = quality * 0.7 + (1.0 - risk) * 0.3;
        (p, composite)
    }).collect();

    // Sort best-first
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (winner, winner_score) = scored[0];
    let rejected: Vec<String> = scored[1..].iter().map(|(p, _)| p.id.clone()).collect();

    let reasoning = format!(
        "Selected '{}' (score={:.2}) over {} competitor(s). Ranked by quality×0.7 + safety×0.3.",
        winner.id, winner_score, rejected.len()
    );

    Some(ArbitrationDecision {
        winner_id: winner.id.clone(),
        rejected_ids: rejected,
        reasoning,
        composite_score: winner_score,
    })
}

/// Filter a patch set: keep only the winner for each set of conflicting patches.
pub fn arbitrate_conflicts(patches: Vec<PatchSet>) -> Vec<PatchSet> {
    use std::collections::HashMap;

    // Group patches by their set of affected files (sorted join as key)
    let mut groups: HashMap<String, Vec<PatchSet>> = HashMap::new();
    for patch in patches {
        let mut files = patch.files_changed.clone();
        files.sort();
        let key = files.join(",");
        groups.entry(key).or_default().push(patch);
    }

    // For each group, keep winner
    groups.into_values().filter_map(|group| {
        if group.len() == 1 { return Some(group.into_iter().next().unwrap()); }
        arbitrate(&group).and_then(|decision| {
            group.into_iter().find(|p| p.id == decision.winner_id)
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(id: &str, files: Vec<&str>, quality_hint: &str) -> PatchSet {
        PatchSet {
            id: id.to_string(),
            files_changed: files.into_iter().map(String::from).collect(),
            diff_summary: quality_hint.to_string(),
            source: "test".to_string(), timestamp: 0,
            intended_content: Some(format!("/// doc\nfn foo() {{}}\n#[test] fn test_foo() {{ assert!(true); }} {}", quality_hint)),
        }
    }

    #[test]
    fn test_arbitration_picks_winner() {
        let patches = vec![
            patch("good", vec!["src/a.rs"], "clean implementation with tests"),
            patch("bad", vec!["src/a.rs"], "unsafe { panic! }"),
        ];
        let decision = arbitrate(&patches).unwrap();
        assert_eq!(decision.winner_id, "good");
        assert_eq!(decision.rejected_ids, vec!["bad"]);
    }

    #[test]
    fn test_single_patch_always_wins() {
        let patches = vec![patch("only", vec!["src/a.rs"], "impl")];
        let d = arbitrate(&patches).unwrap();
        assert_eq!(d.winner_id, "only");
        assert!(d.rejected_ids.is_empty());
    }
}
