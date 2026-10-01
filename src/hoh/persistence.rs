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

/// Ensure the full recommended HOH directory structure exists.
/// This prevents scattered files and keeps all HOH-generated artifacts
/// (scratch, logs, patches, backups) safely under .grok/hoh/.
pub fn ensure_hoh_structure(base: &Path) -> Result<(), HOHError> {
    let root = hoh_data_dir(base);

    fs::create_dir_all(&root)?;
    fs::create_dir_all(root.join("scratch"))?;
    fs::create_dir_all(root.join("backups"))?;
    fs::create_dir_all(root.join("iterations"))?;
    fs::create_dir_all(root.join("versions"))?;   // for 327.13 task list versions
    fs::create_dir_all(root.join("logs"))?;

    Ok(())
}

/// Convenience: return the scratch directory for HOH-generated diagnostic / progress files.
/// All non-persistent, non-versioned HOH artifacts should live here.
pub fn scratch_dir(base: &Path) -> PathBuf {
    hoh_data_dir(base).join("scratch")
}

/// Produce a safe path inside scratch/ for a diagnostic / stub / progress artifact.
/// This sanitizes the name and guarantees it never lands in src/ or .zed/.
pub fn safe_diagnostic_path(base: &Path, name: &str) -> PathBuf {
    let safe = name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '.', "_");
    scratch_dir(base).join(safe)
}

/// Light context object that carries the project root and simulation flag.
/// Makes it harder to accidentally use the wrong root when writing HOH artifacts.
#[derive(Debug, Clone)]
pub struct HOHContext {
    pub project_root: PathBuf,
    pub simulation_mode: bool,
}

impl HOHContext {
    pub fn new(project_root: impl AsRef<Path>, simulation_mode: bool) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            simulation_mode,
        }
    }

    pub fn scratch(&self) -> PathBuf {
        scratch_dir(&self.project_root)
    }

    pub fn safe_diagnostic(&self, name: &str) -> PathBuf {
        safe_diagnostic_path(&self.project_root, name)
    }

    pub fn ensure_structure(&self) -> Result<(), HOHError> {
        ensure_hoh_structure(&self.project_root)
    }
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
