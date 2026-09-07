//! Patch Capture System (Task 297.5)

use crate::hoh::state::PatchSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn capture_patch(files: Vec<String>, diff: String, source: &str) -> PatchSet {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    PatchSet {
        id: format!("patch-{}", now),
        files_changed: files,
        diff_summary: diff,
        source: source.to_string(),
        timestamp: now,
    }
}