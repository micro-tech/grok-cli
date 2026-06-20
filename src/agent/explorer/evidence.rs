//! Repository evidence structures returned by the Explorer agent.

use std::path::PathBuf;

/// Compact evidence collected by the Explorer agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoEvidence {
    pub items: Vec<RepoEvidenceItem>,
}

/// A single relevant snippet / file range discovered by the Explorer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoEvidenceItem {
    /// Relative path from the repository root.
    pub path: PathBuf,
    /// First line of the relevant range (1-based).
    pub line_start: u32,
    /// Last line of the relevant range (1-based, inclusive).
    pub line_end: u32,
    /// One-sentence summary of why this range matters.
    pub summary: String,
}
