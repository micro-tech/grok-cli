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

    /// Compute relevance score using TF-IDF style scoring + length normalization.
    /// Returns a score in [0.0, 1.0] where higher = more relevant.
    fn compute_relevance(content: &str, query: &str) -> f32 {
        if query.trim().is_empty() {
            return 0.5; // neutral when no query
        }

        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        if query_terms.is_empty() {
            return 0.5;
        }

        let content_words: Vec<&str> = content_lower.split_whitespace().collect();
        let content_len = content_words.len() as f32;

        if content_len == 0.0 {
            return 0.0;
        }

        // Term frequency scoring with length normalization
        let mut score = 0.0f32;
        for term in &query_terms {
            let term_count = content_words.iter().filter(|w| w.contains(term)).count() as f32;
            if term_count > 0.0 {
                // TF with log dampening + length normalization
                let tf = (1.0 + term_count.ln()).min(3.0);
                let norm = (tf / (content_len.ln() + 1.0)).min(1.0);
                score += norm;
            }
        }

        // Average over query terms, cap at 1.0
        let avg = (score / query_terms.len() as f32).min(1.0);

        // Boost exact phrase matches
        if content_lower.contains(&query_lower) {
            (avg * 1.3).min(1.0)
        } else {
            avg
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
