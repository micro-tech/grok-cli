//! Open Knowledge Format (OKF) support for Grok CLI.
//!
//! This implements support for Google's Open Knowledge Format (v0.1+):
//! https://github.com/google/okf (or the published spec).
//!
//! An OKF bundle is a directory tree of Markdown files with YAML frontmatter.
//! Each file represents one "concept".
//!
//! Key fields in frontmatter (per the spec):
//! - type:       Required (e.g. "BigQuery Table", "Metric", "Runbook", "API")
//! - title:      Human title
//! - description: Short summary
//! - resource:   Link to the real thing (BigQuery URL, doc, etc.)
//! - tags:       Array of strings
//! - timestamp:  ISO8601
//!
//! File path within the bundle acts as the stable identity.
//!
//! This module turns OKF bundles into first-class knowledge that can be:
//! - Loaded automatically at session start (becomes part of the "Knowledge OS")
//! - Queried via the `okf_lookup` tool (the "Knowledge API")
//!
//! We reuse the existing `[okf]` config section for bundle locations
//! (instead of creating yet another top-level key).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Parsed OKF concept (one .md file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkfConcept {
    /// Stable identity = relative path inside the bundle (without .md)
    pub id: String,

    /// Required type from frontmatter (e.g. "BigQuery Table")
    #[serde(default)]
    pub r#type: String,

    /// Title from frontmatter or derived from filename
    pub title: String,

    /// Short description
    #[serde(default)]
    pub description: String,

    /// Link to the canonical resource
    #[serde(default)]
    pub resource: Option<String>,

    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,

    /// Timestamp if present
    #[serde(default)]
    pub timestamp: Option<String>,

    /// Full markdown body (after stripping frontmatter)
    pub body: String,

    /// Original source file
    pub source_path: PathBuf,

    /// Which bundle this concept came from
    pub bundle_name: String,
}

/// A loaded OKF bundle (a directory of concepts).
#[derive(Debug, Clone)]
pub struct OkfBundle {
    pub name: String,
    pub root: PathBuf,
    pub concepts: Vec<OkfConcept>,
    /// Quick lookup by id
    pub by_id: HashMap<String, usize>,
}

impl OkfBundle {
    /// Load an OKF bundle from a directory.
    pub fn load(root: &Path, bundle_name: Option<&str>) -> Result<Self> {
        let name = bundle_name
            .map(|s| s.to_string())
            .or_else(|| {
                root.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unnamed".to_string());

        let mut concepts = Vec::new();
        let mut by_id = HashMap::new();

        Self::walk_dir(root, root, &mut concepts, &name)?;

        for (idx, c) in concepts.iter().enumerate() {
            by_id.insert(c.id.clone(), idx);
        }

        Ok(OkfBundle {
            name,
            root: root.to_path_buf(),
            concepts,
            by_id,
        })
    }

    fn walk_dir(base: &Path, dir: &Path, out: &mut Vec<OkfConcept>, bundle_name: &str) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::walk_dir(base, &path, out, bundle_name)?;
                continue;
            }

            if path.extension().map_or(false, |e| e == "md") {
                if let Ok(concept) = Self::parse_concept(&path, base, bundle_name) {
                    out.push(concept);
                }
            }
        }
        Ok(())
    }

    fn parse_concept(path: &Path, base: &Path, bundle_name: &str) -> Result<OkfConcept> {
        let raw = fs::read_to_string(path)?;

        let (frontmatter, body) = split_frontmatter(&raw);

        let fm: FrontMatter = if let Some(yaml) = frontmatter {
            serde_yaml::from_str(&yaml)
                .with_context(|| format!("Failed to parse YAML frontmatter in {}", path.display()))?
        } else {
            FrontMatter::default()
        };

        let rel_path = path.strip_prefix(base)
            .unwrap_or(path)
            .with_extension("");

        let id = rel_path
            .to_string_lossy()
            .replace('\\', "/")
            .to_string();

        let title = fm.title.clone().unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        });

        Ok(OkfConcept {
            id,
            r#type: fm.r#type.unwrap_or_default(),
            title,
            description: fm.description.unwrap_or_default(),
            resource: fm.resource,
            tags: fm.tags.unwrap_or_default(),
            timestamp: fm.timestamp,
            body: body.trim().to_string(),
            source_path: path.to_path_buf(),
            bundle_name: bundle_name.to_string(),
        })
    }

    pub fn get_by_id(&self, id: &str) -> Option<&OkfConcept> {
        self.by_id.get(id).and_then(|&i| self.concepts.get(i))
    }

    pub fn search(&self, query: &str) -> Vec<&OkfConcept> {
        let q = query.to_lowercase();
        let mut scored: Vec<(&OkfConcept, f32)> = self
            .concepts
            .iter()
            .filter_map(|c| {
                let score = score_concept(c, &q);
                if score > 0.0 {
                    Some((c, score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.into_iter().map(|(c, _)| c).collect()
    }
}

/// Minimal frontmatter we care about for OKF v0.1
#[derive(Debug, Default, Deserialize)]
struct FrontMatter {
    #[serde(rename = "type")]
    r#type: Option<String>,
    title: Option<String>,
    description: Option<String>,
    resource: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    timestamp: Option<String>,
}

/// Split YAML frontmatter from markdown body.
/// Supports the common --- ... --- format.
fn split_frontmatter(content: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 3 || lines[0].trim() != "---" {
        return (None, content.to_string());
    }

    let mut end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end = Some(i);
            break;
        }
    }

    match end {
        Some(end_idx) => {
            let yaml = lines[1..end_idx].join("\n");
            let body = lines[end_idx + 1..].join("\n");
            (Some(yaml), body)
        }
        None => (None, content.to_string()),
    }
}

/// Public scoring function used by MemoryStore and the `/okf` command.
/// Returns a relevance score (higher is better).
pub fn score_concept_for_search(c: &OkfConcept, query: &str) -> f32 {
    if query.is_empty() {
        return 0.1;
    }

    let q = query.to_lowercase();
    let mut score = 0.0f32;

    let hay = format!(
        "{} {} {} {} {}",
        c.title.to_lowercase(),
        c.description.to_lowercase(),
        c.r#type.to_lowercase(),
        c.body.to_lowercase(),
        c.tags.join(" ").to_lowercase()
    );

    // Simple but effective scoring (good enough for most OKF use cases)
    if hay.contains(&q) {
        score += 1.0;
    }

    for term in q.split_whitespace() {
        if hay.contains(term) {
            score += 0.6;
        }
        if c.title.to_lowercase().contains(term) {
            score += 1.5;
        }
        if c.description.to_lowercase().contains(term) {
            score += 0.8;
        }
        if c.tags.iter().any(|t| t.to_lowercase().contains(term)) {
            score += 1.2;
        }
    }

    // Strong bonus for exact type match
    if !c.r#type.is_empty() && c.r#type.to_lowercase().contains(&q) {
        score += 2.5;
    }

    // Bonus for ID match (very useful for `okf_get`)
    if c.id.to_lowercase().contains(&q) {
        score += 1.8;
    }

    score.min(15.0)
}

// Keep the old private name for backward compat inside the module
fn score_concept(c: &OkfConcept, query: &str) -> f32 {
    score_concept_for_search(c, query)
}

/// Load multiple OKF bundles from the paths configured in OkfConfig.
pub fn load_okf_bundles(bundle_dirs: &[PathBuf]) -> Result<Vec<OkfBundle>> {
    let mut bundles = Vec::new();

    for dir in bundle_dirs {
        if !dir.exists() {
            tracing::debug!("OKF bundle dir does not exist: {}", dir.display());
            continue;
        }

        match OkfBundle::load(dir, None) {
            Ok(bundle) => {
                tracing::info!(
                    "Loaded OKF bundle '{}' with {} concepts from {}",
                    bundle.name,
                    bundle.concepts.len(),
                    dir.display()
                );
                bundles.push(bundle);
            }
            Err(e) => {
                tracing::warn!("Failed to load OKF bundle from {}: {}", dir.display(), e);
            }
        }
    }

    Ok(bundles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn parses_frontmatter_and_body() {
        let raw = r#"---
type: BigQuery Table
title: Orders
description: One row per order
tags: [sales]
---

# Schema
..."#;

        let (fm, body) = split_frontmatter(raw);
        assert!(fm.is_some());
        assert!(body.contains("# Schema"));
    }

    #[test]
    fn loads_simple_bundle() {
        let dir = tempdir().unwrap();
        let concept = dir.path().join("sales/orders.md");
        fs::create_dir_all(concept.parent().unwrap()).unwrap();

        fs::write(
            &concept,
            r#"---
type: Table
title: Orders
---

Order data.
"#,
        )
        .unwrap();

        let bundle = OkfBundle::load(dir.path(), Some("test")).unwrap();
        assert_eq!(bundle.concepts.len(), 1);
        assert_eq!(bundle.concepts[0].title, "Orders");
        assert_eq!(bundle.concepts[0].r#type, "Table");
    }
}
