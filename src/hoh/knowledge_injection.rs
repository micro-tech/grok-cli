//! HOH Knowledge Injection (Task 297.16)
//!
//! Loads OKF bundles and past iteration summaries into the planning phase,
//! giving HOH rich historical context before deciding what to work on next.

use crate::hoh::state::{HOHPlan, IterationState};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A piece of knowledge to inject into the planning context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBundle {
    /// Source type: "okf", "iteration_history", "knowledge_dir", etc.
    pub source: String,
    /// Raw text content of the bundle.
    pub content: String,
    /// Relevance estimate (0.0–1.0).
    pub relevance_score: f32,
    pub timestamp: u64,
}

/// Loads knowledge from various sources and injects it into HOH planning.
pub struct KnowledgeInjector {
    pub project_root: PathBuf,
}

impl KnowledgeInjector {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Load `.md` and `.json` files from the `knowledge/` directory.
    pub async fn load_okf_bundles(&self) -> Vec<KnowledgeBundle> {
        let knowledge_dir = self.project_root.join("knowledge");
        if !knowledge_dir.exists() {
            return Vec::new();
        }

        let mut bundles = Vec::new();
        let read_dir = match std::fs::read_dir(&knowledge_dir) {
            Ok(rd) => rd,
            Err(_) => return bundles,
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "md" && ext != "json" {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                bundles.push(KnowledgeBundle {
                    source: "okf".to_string(),
                    content: content.chars().take(4096).collect(), // cap size
                    relevance_score: 0.8,
                    timestamp: now_secs(),
                });
            }
        }

        tracing::debug!(
            count = bundles.len(),
            "[HOH KnowledgeInjector] loaded OKF bundles"
        );
        bundles
    }

    /// Load `summary.md` files from the last N HOH iteration folders.
    pub async fn load_iteration_history(&self, last_n: usize) -> Vec<KnowledgeBundle> {
        let iter_dir = self.project_root.join(".grok/hoh/iterations");
        if !iter_dir.exists() {
            return Vec::new();
        }

        // Collect iteration directories, sort by name (numeric) descending
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&iter_dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect();

        dirs.sort_by(|a, b| {
            let an = a.file_name().and_then(|n| n.to_str()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            let bn = b.file_name().and_then(|n| n.to_str()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            bn.cmp(&an) // descending
        });

        let mut bundles = Vec::new();
        for dir in dirs.into_iter().take(last_n) {
            let summary_path = dir.join("summary.md");
            if let Ok(content) = std::fs::read_to_string(&summary_path) {
                let iter_id = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                bundles.push(KnowledgeBundle {
                    source: format!("iteration_history:{}", iter_id),
                    content: content.chars().take(4096).collect(),
                    relevance_score: 0.9,
                    timestamp: now_secs(),
                });
            }
        }

        tracing::debug!(
            count = bundles.len(),
            "[HOH KnowledgeInjector] loaded iteration history"
        );
        bundles
    }
}

/// Inject all available knowledge into the iteration state's plan.
///
/// Creates a default `HOHPlan` if `state.plan` is `None`.
pub async fn inject_knowledge(state: &mut IterationState, project_root: &Path) {
    let injector = KnowledgeInjector::new(project_root.to_path_buf());

    let okf = injector.load_okf_bundles().await;
    let history = injector.load_iteration_history(3).await;

    let total = okf.len() + history.len();
    let sources: Vec<&str> = okf
        .iter()
        .chain(history.iter())
        .map(|b| b.source.as_str())
        .take(5)
        .collect();

    // Ensure plan exists
    if state.plan.is_none() {
        state.plan = Some(HOHPlan::default());
    }

    if let Some(plan) = &mut state.plan {
        plan.goals.push(format!(
            "Knowledge injection: {} bundle(s) loaded from [{}{}]",
            total,
            sources.join(", "),
            if sources.len() < total { ", ..." } else { "" }
        ));

        // Add high-relevance content as experiment hints
        for bundle in okf.iter().chain(history.iter()).filter(|b| b.relevance_score >= 0.85) {
            let snippet: String = bundle.content.lines().take(2).collect::<Vec<_>>().join(" ");
            if !snippet.is_empty() {
                plan.experiments.push(format!("[{}] {}", bundle.source, snippet));
            }
        }
    }

    tracing::info!(
        iteration = state.iteration_id,
        total_bundles = total,
        "[HOH KnowledgeInjector] injection complete"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inject_does_not_panic_on_missing_dirs() {
        let tmp = std::env::temp_dir().join("hoh_test_inject");
        let mut state = IterationState::new(1);
        inject_knowledge(&mut state, &tmp).await;
        // Should have added at least the injection goal
        let plan = state.plan.expect("plan should be created");
        assert!(!plan.goals.is_empty());
        assert!(plan.goals[0].contains("Knowledge injection"));
    }

    #[tokio::test]
    async fn test_load_okf_empty_when_no_knowledge_dir() {
        let injector = KnowledgeInjector::new(std::env::temp_dir().join("no_such_dir_hoh"));
        let bundles = injector.load_okf_bundles().await;
        assert!(bundles.is_empty());
    }
}
