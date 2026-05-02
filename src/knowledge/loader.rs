//! Local Knowledge Pack Loader
//!
//! Loads project-specific knowledge from `knowledge/` directory.
//! Supports .md and .json files with relevance scoring.

use std::fs;
use std::path::Path;

/// Knowledge entry with content and metadata.
#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub content: String,
    pub relevance_score: f32,
    pub source: String,
}

/// Knowledge loader.
pub struct KnowledgeLoader {
    entries: Vec<KnowledgeEntry>,
}

impl KnowledgeLoader {
    /// Load all knowledge from `knowledge/` directory.
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        let dir = Path::new("knowledge");

        if !dir.exists() {
            tracing::warn!("Knowledge directory does not exist");
            return Ok(Self { entries });
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "md" || ext == "json" {
                    let content = fs::read_to_string(&path)?;
                    let relevance_score = Self::compute_relevance(&content, "query"); // Placeholder
                    entries.push(KnowledgeEntry {
                        content,
                        relevance_score,
                        source: path.display().to_string(),
                    });
                }
            }
        }

        Ok(Self { entries })
    }

    /// Compute relevance score (placeholder: simple keyword match).
    fn compute_relevance(content: &str, query: &str) -> f32 {
        if content.contains(query) { 1.0 } else { 0.5 }
    }

    /// Get relevant knowledge for a query.
    pub fn get_relevant(&self, _query: &str) -> Vec<&KnowledgeEntry> {
        self.entries
            .iter()
            .filter(|e| e.relevance_score > 0.0)
            .collect()
    }
}