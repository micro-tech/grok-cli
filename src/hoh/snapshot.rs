//! HOH Project Snapshot System (Task 297.19)
//!
//! Takes snapshots of project state before and after iterations for
//! rollback and comparison. Uses git commit SHA as the primary anchor.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Why a snapshot was taken.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotKind {
    PreIteration,
    PostIteration,
    Manual(String),
}

/// Metadata for one project snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique id: `"<iteration_id>-<timestamp>"`.
    pub id: String,
    pub kind: SnapshotKind,
    pub iteration_id: u64,
    pub timestamp: u64,
    /// Git HEAD commit at snapshot time, if available.
    pub git_commit: Option<String>,
    /// Representative file list at snapshot time (top-level src/ files).
    pub files_snapshotted: Vec<String>,
    /// Optional path to a tarball (not created by default; future enhancement).
    pub tarball_path: Option<PathBuf>,
}

/// Manages snapshots stored under `.grok/hoh/snapshots/`.
pub struct SnapshotManager {
    pub project_root: PathBuf,
    pub snapshot_dir: PathBuf,
}

impl SnapshotManager {
    pub fn new(project_root: PathBuf) -> Self {
        let snapshot_dir = project_root.join(".grok/hoh/snapshots");
        Self { project_root, snapshot_dir }
    }

    fn meta_path(&self, snapshot_id: &str) -> PathBuf {
        self.snapshot_dir.join(snapshot_id).join("meta.json")
    }

    /// Capture current git HEAD (best-effort; returns None if not a git repo).
    fn git_head(&self) -> Option<String> {
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.project_root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// List representative files under `src/` (best-effort).
    fn list_src_files(&self) -> Vec<String> {
        let src_dir = self.project_root.join("src");
        if !src_dir.exists() {
            return Vec::new();
        }
        walkdir::WalkDir::new(&src_dir)
            .max_depth(2)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                e.path()
                    .strip_prefix(&self.project_root)
                    .unwrap_or(e.path())
                    .to_string_lossy()
                    .to_string()
            })
            .take(50)
            .collect()
    }

    /// Take a snapshot and persist its metadata.
    pub async fn take_snapshot(
        &self,
        iteration_id: u64,
        kind: SnapshotKind,
    ) -> io::Result<Snapshot> {
        let ts = now_secs();
        let id = format!("{}-{}", iteration_id, ts);

        let snap_dir = self.snapshot_dir.join(&id);
        tokio::fs::create_dir_all(&snap_dir).await?;

        let snapshot = Snapshot {
            id: id.clone(),
            kind,
            iteration_id,
            timestamp: ts,
            git_commit: self.git_head(),
            files_snapshotted: self.list_src_files(),
            tarball_path: None,
        };

        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        tokio::fs::write(self.meta_path(&id), json).await?;

        tracing::info!(
            iteration_id,
            snapshot_id = id,
            git_commit = snapshot.git_commit.as_deref().unwrap_or("N/A"),
            "[HOH Snapshot] taken"
        );
        Ok(snapshot)
    }

    /// Restore the project to a previous snapshot's git commit (warn-only on failure).
    pub async fn restore_snapshot(&self, snapshot_id: &str) -> io::Result<()> {
        let meta_bytes = tokio::fs::read(self.meta_path(snapshot_id)).await?;
        let snap: Snapshot = serde_json::from_slice(&meta_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if let Some(commit) = &snap.git_commit {
            let status = Command::new("git")
                .args(["checkout", commit])
                .current_dir(&self.project_root)
                .status();
            match status {
                Ok(s) if s.success() => {
                    tracing::info!(commit, snapshot_id, "[HOH Snapshot] restored to commit");
                }
                _ => {
                    tracing::warn!(
                        commit,
                        snapshot_id,
                        "[HOH Snapshot] git checkout failed — manual restore may be needed"
                    );
                }
            }
        } else {
            tracing::warn!(snapshot_id, "[HOH Snapshot] no git commit recorded — cannot restore");
        }
        Ok(())
    }

    /// List all snapshots, sorted by timestamp descending.
    pub fn list_snapshots(&self) -> Vec<Snapshot> {
        if !self.snapshot_dir.exists() {
            return Vec::new();
        }
        let mut snaps: Vec<Snapshot> = std::fs::read_dir(&self.snapshot_dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                let meta = e.path().join("meta.json");
                std::fs::read_to_string(meta)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
            })
            .collect();
        snaps.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        snaps
    }

    /// Delete all but the most recent `keep_last_n` snapshots.
    /// Returns the number of snapshots deleted.
    pub async fn cleanup_old_snapshots(&self, keep_last_n: usize) -> io::Result<usize> {
        let all = self.list_snapshots();
        let to_delete = if all.len() > keep_last_n {
            &all[keep_last_n..]
        } else {
            return Ok(0);
        };

        let mut deleted = 0;
        for snap in to_delete {
            let dir = self.snapshot_dir.join(&snap.id);
            if let Ok(()) = tokio::fs::remove_dir_all(&dir).await {
                deleted += 1;
                tracing::debug!(snapshot_id = snap.id, "[HOH Snapshot] cleaned up old snapshot");
            }
        }
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_manager_new() {
        let tmp = std::env::temp_dir().join("hoh_snapshot_test");
        let mgr = SnapshotManager::new(tmp.clone());
        assert_eq!(mgr.project_root, tmp);
        assert!(mgr.snapshot_dir.ends_with(".grok/hoh/snapshots"));
    }

    #[tokio::test]
    async fn test_take_snapshot_creates_meta_file() {
        let tmp = std::env::temp_dir().join("hoh_snapshot_take_test");
        let mgr = SnapshotManager::new(tmp.clone());
        let snap = mgr.take_snapshot(1, SnapshotKind::PreIteration).await.unwrap();
        assert_eq!(snap.iteration_id, 1);
        assert_eq!(snap.kind, SnapshotKind::PreIteration);
        let meta = mgr.meta_path(&snap.id);
        assert!(meta.exists());
        // cleanup
        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }

    #[tokio::test]
    async fn test_cleanup_removes_old_snapshots() {
        let tmp = std::env::temp_dir().join("hoh_snapshot_cleanup_test");
        let mgr = SnapshotManager::new(tmp.clone());
        // Create 3 snapshots
        for i in 1u64..=3 {
            mgr.take_snapshot(i, SnapshotKind::Manual(format!("test-{}", i))).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        let deleted = mgr.cleanup_old_snapshots(1).await.unwrap();
        assert_eq!(deleted, 2);
        let remaining = mgr.list_snapshots();
        assert_eq!(remaining.len(), 1);
        // cleanup
        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }
}
