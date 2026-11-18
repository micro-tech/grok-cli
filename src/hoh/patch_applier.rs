//! HOH Patch Applier (Task 297.5 + 361.5 real application)
//!
//! Applies PatchSets to the filesystem in a controlled, safe way.
//! Respects simulation mode, AutonomyLevel, and basic safety.
//! This turns captured metadata into actual file changes.

use crate::hoh::state::{PatchSet, HOHError};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone, Default)]
pub struct ApplyResult {
    pub applied: bool,
    pub files_written: Vec<String>,
    pub errors: Vec<String>,
    pub backup_dir: Option<PathBuf>,
}

/// Configuration for patch application.
#[derive(Debug, Clone)]
pub struct ApplyConfig {
    pub simulation_mode: bool,
    pub dry_run: bool,
    pub create_backups: bool,
    pub backup_root: PathBuf,
    pub max_files_per_patch: usize,
    /// Jail for real writes: no file outside this directory is ever written,
    /// however the patch names it. The HOH manager sets this to
    /// `<project_root>/.grok/hoh` (its data dir); the default is the same
    /// path relative to the working directory. Relative entries are resolved
    /// against the current working directory (the HOH runs with cwd = the
    /// project root). `..` escapes are normalized away before the check, so a
    /// path that only *looks* inside the jail does not pass.
    pub jail: PathBuf,
}

impl Default for ApplyConfig {
    fn default() -> Self {
        Self {
            simulation_mode: true,
            dry_run: false,
            create_backups: true,
            backup_root: PathBuf::from(".grok/hoh/backups"),
            max_files_per_patch: 20,
            jail: PathBuf::from(".grok/hoh"),
        }
    }
}

/// Lexically normalize a path (resolve `.` and `..` without touching the
/// filesystem), so the jail check works for files that do not exist yet.
/// A `..` that would escape a relative path is kept visible (so the jail
/// check rejects it); a `..` at the root of an absolute path is a no-op.
fn normalize_lexically(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() && !path.is_absolute() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Resolve `path` to an absolute, lexically-normalized path. Relative paths
/// are resolved against the current working directory.
fn resolve_for_jail(path: &Path) -> PathBuf {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    normalize_lexically(&abs)
}

/// True when `path` (resolved as above) lies inside `jail`.
fn is_path_in_jail(path: &Path, jail: &Path) -> bool {
    resolve_for_jail(path).starts_with(resolve_for_jail(jail))
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
            // H9: do not even back up a file outside the jail — it will be
            // refused by the write loop below, so there is nothing to preserve.
            if !is_path_in_jail(Path::new(file), &config.jail) {
                continue;
            }
            if let Ok(content) = fs::read_to_string(file) {
                let backup_path = backup_dir.join(file.replace(['/', '\\'], "_"));
                if let Err(e) = fs::write(&backup_path, content) {
                    result.errors.push(format!("backup failed for {}: {}", file, e));
                }
            }
        }
    }

    for file_path in &patch.files_changed {
        // Resolve the content for THIS file. A single `intended_content` blob is
        // only applied to single-file patches; a multi-file patch without
        // per-file content is skipped rather than overwriting every file with a
        // shared blob (which would silently corrupt all but one of them).
        let Some(target_content) = patch.content_for(file_path) else {
            let msg = format!(
                "no per-file content for '{}' in multi-file patch {} — skipped to avoid overwriting files with a shared blob",
                file_path, patch.id
            );
            warn!("apply_patch: {}", msg);
            result.errors.push(msg);
            continue;
        };

        if target_content.trim().is_empty() {
            warn!("apply_patch: patch {} has empty content for {}", patch.id, file_path);
            result.errors.push(format!("empty content for {}", file_path));
            continue;
        }

        let path = Path::new(file_path);

        // H9: jail all real writes under the HOH data dir. A patch may name
        // any file (including a real src/ path as an "intended target"), but
        // the applier only ever writes inside the jail. `..` escapes and
        // absolute paths outside the jail are rejected here, before any
        // directory is created or file is written.
        if !is_path_in_jail(path, &config.jail) {
            let msg = format!(
                "refusing to write '{}' in patch {}: outside the HOH jail ({})",
                file_path,
                patch.id,
                config.jail.display()
            );
            warn!("apply_patch: {}", msg);
            result.errors.push(msg);
            continue;
        }

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
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn real_config(tmp: &Path) -> ApplyConfig {
        ApplyConfig {
            simulation_mode: false,
            dry_run: false,
            create_backups: false,
            backup_root: tmp.join("backups"),
            max_files_per_patch: 20,
            // The test temp dir is the jail, so writes to it succeed and
            // writes anywhere else are rejected.
            jail: tmp.to_path_buf(),
        }
    }

    #[tokio::test]
    async fn single_file_patch_writes_intended_content() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("out.txt");
        let patch = PatchSet {
            id: "p1".into(),
            files_changed: vec![target.to_string_lossy().to_string()],
            diff_summary: "summary".into(),
            source: "test".into(),
            timestamp: 0,
            intended_content: Some("hello world".into()),
        };

        let result = apply_patch(&patch, &real_config(tmp.path()))
            .await
            .unwrap();

        assert!(result.applied, "errors: {:?}", result.errors);
        assert_eq!(result.files_written.len(), 1);
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello world");
    }

    #[tokio::test]
    async fn multi_file_patch_does_not_fan_out_shared_blob() {
        // A patch listing two files but carrying only a single `intended_content`
        // blob must NOT write that same blob to both files. Both writes should be
        // skipped and reported as errors, leaving no files created.
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let patch = PatchSet {
            id: "p2".into(),
            files_changed: vec![
                a.to_string_lossy().to_string(),
                b.to_string_lossy().to_string(),
            ],
            diff_summary: "shared".into(),
            source: "test".into(),
            timestamp: 0,
            intended_content: Some("SHARED-BLOB".into()),
        };

        let result = apply_patch(&patch, &real_config(tmp.path()))
            .await
            .unwrap();

        assert!(!result.applied);
        assert!(result.files_written.is_empty());
        assert_eq!(result.errors.len(), 2);
        assert!(!a.exists(), "first file must not be written");
        assert!(!b.exists(), "second file must not be written");
    }

    #[test]
    fn content_for_resolves_only_single_file_patches() {
        let single = PatchSet {
            id: "s".into(),
            files_changed: vec!["only.rs".into()],
            diff_summary: String::new(),
            source: "test".into(),
            timestamp: 0,
            intended_content: Some("body".into()),
        };
        assert_eq!(single.content_for("only.rs"), Some("body".into()));

        let multi = PatchSet {
            id: "m".into(),
            files_changed: vec!["a.rs".into(), "b.rs".into()],
            diff_summary: String::new(),
            source: "test".into(),
            timestamp: 0,
            intended_content: Some("body".into()),
        };
        assert_eq!(multi.content_for("a.rs"), None);
        assert_eq!(multi.content_for("b.rs"), None);
    }

    #[test]
    fn content_for_never_returns_diff_summary() {
        // H6: diff_summary is prose describing the patch, not file content.
        // A single-file patch with only diff_summary set (no intended_content)
        // must resolve to no content, so the applier skips it instead of
        // writing the summary into the target file.
        let patch = PatchSet {
            id: "d".into(),
            files_changed: vec!["only.rs".into()],
            diff_summary: "a human-readable summary of the change".into(),
            source: "test".into(),
            timestamp: 0,
            intended_content: None,
        };
        assert_eq!(patch.content_for("only.rs"), None);
        assert_eq!(patch.content_to_apply(), "");
    }

    #[tokio::test]
    async fn write_outside_jail_is_rejected() {
        // H9: a patch naming a file outside the jail must not be written,
        // even when it carries real intended_content.
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("evil.txt");
        let patch = PatchSet {
            id: "p3".into(),
            files_changed: vec![target.to_string_lossy().to_string()],
            diff_summary: "summary".into(),
            source: "test".into(),
            timestamp: 0,
            intended_content: Some("evil".into()),
        };

        let result = apply_patch(&patch, &real_config(tmp.path()))
            .await
            .unwrap();

        assert!(!result.applied);
        assert!(result.files_written.is_empty());
        assert!(!target.exists(), "file outside the jail must not be written");
        assert!(
            result.errors.iter().any(|e| e.contains("jail")),
            "errors: {:?}",
            result.errors
        );
    }

    #[tokio::test]
    async fn dotdot_escape_out_of_jail_is_rejected() {
        // H9: a path that only *looks* inside the jail but escapes via `..`
        // must be rejected after lexical normalization.
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp
            .path()
            .join("sub")
            .join("..")
            .join("..")
            .join("escape.txt");
        let patch = PatchSet {
            id: "p4".into(),
            files_changed: vec![target.to_string_lossy().to_string()],
            diff_summary: "summary".into(),
            source: "test".into(),
            timestamp: 0,
            intended_content: Some("escape".into()),
        };

        let result = apply_patch(&patch, &real_config(tmp.path()))
            .await
            .unwrap();

        assert!(!result.applied);
        assert!(result.files_written.is_empty());
        assert!(!target.exists(), "escaped file must not be written");
        assert!(
            result.errors.iter().any(|e| e.contains("jail")),
            "errors: {:?}",
            result.errors
        );
    }
}
