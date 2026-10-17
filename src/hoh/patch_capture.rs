//! Patch Capture System (Task 297.5)
//
//! Central place to create rich PatchSets.
//! Now supports carrying real intended content so the applier can do actual work.

use crate::hoh::state::PatchSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// Basic capture (backward compatible).
pub fn capture_patch(files: Vec<String>, diff: String, source: &str) -> PatchSet {
    capture_patch_with_content(files, diff, None, source)
}

/// Rich capture that can carry the actual content we intend to write.
pub fn capture_patch_with_content(
    files: Vec<String>,
    diff_summary: String,
    intended_content: Option<String>,
    source: &str,
) -> PatchSet {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    PatchSet {
        id: format!("patch-{}", now),
        files_changed: files,
        diff_summary,
        source: source.to_string(),
        timestamp: now,
        intended_content,
    }
}

/// Convenience: create a patch that is meant to write specific content to files.
pub fn capture_content_patch(
    files: Vec<String>,
    content: String,
    source: &str,
) -> PatchSet {
    let summary = if content.len() > 120 {
        format!("{}... ({} bytes)", &content[..120], content.len())
    } else {
        content.clone()
    };

    capture_patch_with_content(files, summary, Some(content), source)
}