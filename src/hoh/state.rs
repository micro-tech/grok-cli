//! HOH Iteration State and related data structures (Task 297.2)

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IterationState {
    pub iteration_id: u64,
    pub started_at: u64,
    pub status: IterationStatus,
    pub plan: Option<HOHPlan>,
    pub patches: Vec<PatchSet>,
    pub evaluations: Vec<EvaluationReport>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum IterationStatus {
    #[default]
    Planning,
    Executing,
    Testing,
    Evaluating,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HOHPlan {
    pub goals: Vec<String>,
    pub selected_tasks: Vec<u64>,
    pub experiments: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatchSet {
    pub id: String,
    pub files_changed: Vec<String>,
    pub diff_summary: String,
    pub source: String, // "inner_harness", "hoh", etc.
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationReport {
    pub iteration_id: u64,
    pub helix_score: Option<f32>,
    pub internal_metrics: std::collections::HashMap<String, f32>,
    pub notes: String,
}

impl IterationState {
    pub fn new(iteration_id: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            iteration_id,
            started_at: now,
            status: IterationStatus::Planning,
            ..Default::default()
        }
    }

    pub fn mark_completed(&mut self, summary: String) {
        self.status = IterationStatus::Completed;
        self.summary = Some(summary);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HOHError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Other: {0}")]
    Other(String),
}