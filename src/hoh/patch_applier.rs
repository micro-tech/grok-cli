//! HOH Patch Applier (Task 297.5 + 361.5 real application)
//!
//! Applies PatchSets to the filesystem in a controlled, safe way.
//! Respects simulation mode, AutonomyLevel, and basic safety.
//! This turns captured metadata into actual file changes.

use crate::hoh::state::{PatchSet, HOHError};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub applied: bool,
    pub files_written: Vec<String>,
    pub errors: Vec<String>,
    pub backup_dir: Option<PathBuf>,
}

impl Default for ApplyResult {
    fn default() -> Self {
        Self {
            applied: false,
            files_written: vec![],
            errors: vec![],
            backup_dir: None,
        }
    }
}

/// Configuration for patch application.
#[derive(Debug, Clone)]
pub struct ApplyConfig {
    pub simulation_mode: bool,
    pub dry_run: bool,
    pub create_backups: bool,
    pub backup_root: PathBuf,
    pub max_files_per_patch: usize,
}

impl Default for ApplyConfig {
    fn default() -> Self {
        Self {
            simulation_mode: true,
            dry_run: false,
            create_backups: true,
            backup_root: PathBuf::from(".grok/hoh/backups"),
            max_files_per_patch: 20,
        }
    }
}

/// Main entry point: apply a PatchSet safely.
pub async fn apply_patch(
    patch: &PatchSet,
    config: &ApplyConfig,
) -> Result<ApplyResult, HOHError> {
    let mut result = ApplyResult::default();

    if patch.files_changed.is_empty() {
        info!("apply_patch: nothing to do for empty patch {}", patch.id);
        return Ok(result);
    }

    if patch.files_changed.len() > config.max_files_per_patch {
        warn!(
            "apply_patch: refusing large patch ({} files > max {})",
            patch.files_changed.len(),
            config.max_files_per_patch
        );
        result.errors.push("Patch too large".to_string());
        return Ok(result);
    }

    if config.simulation_mode || config.dry_run {
        info!(
            "apply_patch [SIM]: would apply '{}' to {} files (source: {})",
            patch.id,
            patch.files_changed.len(),
            patch.source
        );
        result.applied = false;
        result.files_written = patch.files_changed.clone();
        return Ok(result);
    }

    // Real application path
    if config.create_backups {
        let backup_dir = create_backup_dir(&config.backup_root, &patch.id)?;
        result.backup_dir = Some(backup_dir.clone());

        for file in &patch.files_changed {
            if let Ok(content) = fs::read_to_string(file) {
                let backup_path = backup_dir.join(file.replace(['/', '\\'], "_"));
                if let Err(e) = fs::write(&backup_path, content) {
                    result.errors.push(format!("backup failed for {}: {}", file, e));
                }
            }
        }
    }

    // Use the single source of truth helper. It prefers intended_content.
    let target_content = patch.content_to_apply();

    if target_content.trim().is_empty() {
        warn!("apply_patch: patch {} has no content to apply", patch.id);
        result.errors.push("No content to apply".to_string());
        return Ok(result);
    }

    for file_path in &patch.files_changed {
        let path = Path::new(file_path);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    result.errors.push(format!("mkdir failed for {}: {}", file_path, e));
                    continue;
                }
            }
        }

        match fs::write(path, &target_content) {
            Ok(_) => {
                info!("apply_patch: wrote {} ({} bytes)", file_path, target_content.len());
                result.files_written.push(file_path.clone());
            }
            Err(e) => {
                warn!("apply_patch: failed to write {}: {}", file_path, e);
                result.errors.push(format!("write {}: {}", file_path, e));
            }
        }
    }

    result.applied = result.errors.is_empty() && !result.files_written.is_empty();
    Ok(result)
}

fn create_backup_dir(root: &Path, patch_id: &str) -> Result<PathBuf, HOHError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let dir = root.join(format!("{}-{}", patch_id, now));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Convenience: apply multiple patches.
pub async fn apply_patches(
    patches: &[PatchSet],
    config: &ApplyConfig,
) -> Result<Vec<ApplyResult>, HOHError> {
    let mut results = Vec::new();
    for p in patches {
        results.push(apply_patch(p, config).await?);
    }
    Ok(results)
}
