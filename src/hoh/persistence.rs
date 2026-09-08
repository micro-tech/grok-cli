//! HOH Iteration Persistence (Task 297.10 + multi-day support)
//!
//! Saves and loads full IterationState (plan, patches, evaluations, summaries)
//! into organized folders under .grok/hoh/iterations/<id>/
//!
//! This enables:
//! - History across days
//! - Resume capability
//! - Post-run inspection of patches/evals
//! - Feeding real history into 361.5 meta loops

use crate::hoh::state::{IterationState, HOHError};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// Root directory for all HOH data.
pub fn hoh_data_dir(base: &Path) -> PathBuf {
    base.join(".grok/hoh")
}

/// Directory for a specific iteration.
pub fn iteration_dir(base: &Path, iteration_id: u64) -> PathBuf {
    hoh_data_dir(base).join("iterations").join(format!("{:04}", iteration_id))
}

/// Ensure the folder structure exists for an iteration.
pub fn ensure_iteration_dirs(base: &Path, iteration_id: u64) -> Result<PathBuf, HOHError> {
    let dir = iteration_dir(base, iteration_id);
    fs::create_dir_all(dir.join("patches"))?;
    fs::create_dir_all(dir.join("evals"))?;
    fs::create_dir_all(dir.join("logs"))?;
    fs::create_dir_all(dir.join("plans"))?;
    Ok(dir)
}

/// Save a complete IterationState to disk.
pub fn save_iteration(base: &Path, state: &IterationState) -> Result<PathBuf, HOHError> {
    let dir = ensure_iteration_dirs(base, state.iteration_id)?;

    // Main state file
    let state_path = dir.join("state.json");
    let json = serde_json::to_string_pretty(state)?;
    fs::write(&state_path, json)?;
    info!("HOH persistence: saved state for iteration {} -> {}", state.iteration_id, state_path.display());

    // Save patches separately (easier to inspect)
    if !state.patches.is_empty() {
        let patches_path = dir.join("patches").join("patches.json");
        let pjson = serde_json::to_string_pretty(&state.patches)?;
        fs::write(&patches_path, pjson)?;
    }

    // Save evaluations
    if !state.evaluations.is_empty() {
        let evals_path = dir.join("evals").join("evaluations.json");
        let ejson = serde_json::to_string_pretty(&state.evaluations)?;
        fs::write(&evals_path, ejson)?;
    }

    // Save plan if present
    if let Some(plan) = &state.plan {
        let plan_path = dir.join("plans").join("plan.json");
        let plan_json = serde_json::to_string_pretty(plan)?;
        fs::write(&plan_path, plan_json)?;
    }

    // Write a human-readable summary
    let summary_path = dir.join("summary.txt");
    let summary_text = format!(
        "HOH Iteration {}\nStatus: {:?}\nStarted: {}\nSummary: {}\nPatches: {}\nEvals: {}\n",
        state.iteration_id,
        state.status,
        state.started_at,
        state.summary.as_deref().unwrap_or("N/A"),
        state.patches.len(),
        state.evaluations.len()
    );
    fs::write(&summary_path, summary_text)?;

    Ok(dir)
}

/// Load the latest completed iteration (highest id with Completed status).
pub fn load_latest_iteration(base: &Path) -> Result<Option<IterationState>, HOHError> {
    let root = hoh_data_dir(base).join("iterations");
    if !root.exists() {
        return Ok(None);
    }

    let mut highest: Option<(u64, IterationState)> = None;

    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if let Ok(id) = entry.file_name().to_string_lossy().parse::<u64>() {
            let state_path = entry.path().join("state.json");
            if state_path.exists() {
                if let Ok(json) = fs::read_to_string(&state_path) {
                    if let Ok(state) = serde_json::from_str::<IterationState>(&json) {
                        if state.status == crate::hoh::state::IterationStatus::Completed {
                            if highest.as_ref().map_or(true, |(h, _)| id > *h) {
                                highest = Some((id, state));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(highest.map(|(_, s)| s))
}

/// Load a specific iteration by id.
pub fn load_iteration(base: &Path, iteration_id: u64) -> Result<Option<IterationState>, HOHError> {
    let state_path = iteration_dir(base, iteration_id).join("state.json");
    if !state_path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(state_path)?;
    let state: IterationState = serde_json::from_str(&json)?;
    Ok(Some(state))
}

/// List all known iteration IDs (sorted).
pub fn list_iterations(base: &Path) -> Result<Vec<u64>, HOHError> {
    let root = hoh_data_dir(base).join("iterations");
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut ids: Vec<u64> = vec![];
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if let Ok(id) = entry.file_name().to_string_lossy().parse::<u64>() {
            ids.push(id);
        }
    }
    ids.sort();
    Ok(ids)
}
