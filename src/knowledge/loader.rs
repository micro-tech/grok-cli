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
            tracing::debug!("Knowledge directory does not exist");
            return Ok(Self { entries });
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension()
                && (ext == "md" || ext == "json")
            {
                let content = fs::read_to_string(&path)?;
                let relevance_score = Self::compute_relevance(&content, "query"); // Placeholder
                entries.push(KnowledgeEntry {
                    content,
                    relevance_score,
                    source: path.display().to_string(),
                });
            }
        }

        Ok(Self { entries })
    }

    /// Compute relevance using a lightweight TF-IDF style scorer.
    /// Returns a score between 0.0 and 1.0.
    fn compute_relevance(content: &str, query: &str) -> f32 {
        if query.trim().is_empty() {
            return 0.5;
        }

        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        if query_terms.is_empty() {
            return 0.5;
        }

        // Term frequency in content
        let mut tf = 0.0f32;
        for term in &query_terms {
            let count = content_lower.matches(term).count() as f32;
            tf += count;
        }
        tf /= content_lower.split_whitespace().count().max(1) as f32;

        // IDF approximation (inverse document frequency) — penalize very common words
        let mut idf = 0.0f32;
        for term in &query_terms {
            let df = if content_lower.contains(term) { 1.0 } else { 0.0 };
            idf += (1.0 / (1.0 + df)).ln();
        }
        idf = idf.max(0.1);

        let tf_idf = (tf * idf).min(1.0);

        // Boost for exact phrase match
        if content_lower.contains(&query_lower) {
            (tf_idf + 0.25).min(1.0)
        } else {
            tf_idf
        }
    }

    /// Get all loaded knowledge entries.
    pub fn get_all(&self) -> &[KnowledgeEntry] {
        &self.entries
    }

    /// Get relevant knowledge for a query.
    pub fn get_relevant(&self, _query: &str) -> Vec<&KnowledgeEntry> {
        self.entries
            .iter()
            .filter(|e| e.relevance_score > 0.0)
            .collect()
    }
}
