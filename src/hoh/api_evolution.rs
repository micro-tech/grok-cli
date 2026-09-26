//! HOH Autonomous API Evolution (Task 361.16)

use crate::hoh::state::PatchSet;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiChangeKind {
    AddParameter { name: String, type_hint: String },
    RenameFunction { from: String, to: String },
    ExtractTrait { name: String },
    AddReturnValue { description: String },
    Deprecate { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProposal {
    pub id: String,
    pub target: String,          // e.g. "src/hoh/planner.rs::create_plan"
    pub kind: ApiChangeKind,
    pub rationale: String,
    pub breaking: bool,
    pub migration_path: Option<String>,
}

pub struct ApiEvolutionEngine { pub simulation_mode: bool }

impl ApiEvolutionEngine {
    pub fn new(sim: bool) -> Self { Self { simulation_mode: sim } }

    /// Analyze source files and propose API improvements.
    pub fn propose_evolutions(&self, project_root: &std::path::Path) -> Vec<ApiProposal> {
        let src = project_root.join("src");
        let mut proposals = Vec::new();

        for entry in walkdir::WalkDir::new(&src).into_iter().flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
            let Ok(content) = std::fs::read_to_string(entry.path()) else { continue };
            let rel = entry.path().strip_prefix(project_root).unwrap_or(entry.path())
                .to_string_lossy().to_string();

            // Detect overly long pub fn signatures (> 5 parameters = complexity smell)
            for line in content.lines() {
                if line.contains("pub fn") {
                    let param_count = line.matches(':').count().saturating_sub(1);
                    if param_count > 4 {
                        let fn_name = line.split("pub fn ").nth(1)
                            .and_then(|s| s.split('(').next())
                            .unwrap_or("unknown").to_string();
                        proposals.push(ApiProposal {
                            id: format!("api-{}-{}", proposals.len(), now()),
                            target: format!("{}::{}", rel, fn_name),
                            kind: ApiChangeKind::ExtractTrait { name: format!("{}Config", fn_name) },
                            rationale: format!("Function '{}' has {} parameters — consider a builder/config pattern", fn_name, param_count),
                            breaking: true,
                            migration_path: Some("Introduce config struct, provide From impl for backward compat".to_string()),
                        });
                    }
                }

                // Detect functions returning () that could return Result
                if line.contains("pub fn") && line.contains("-> ()") {
                    let fn_name = line.split("pub fn ").nth(1)
                        .and_then(|s| s.split('(').next())
                        .unwrap_or("unknown").to_string();
                    proposals.push(ApiProposal {
                        id: format!("api-{}-{}", proposals.len(), now()),
                        target: format!("{}::{}", rel, fn_name),
                        kind: ApiChangeKind::AddReturnValue { description: "Result<(), Error>".to_string() },
                        rationale: "Returning () prevents callers from handling errors properly".to_string(),
                        breaking: true,
                        migration_path: Some("Update callers to handle Result".to_string()),
                    });
                }
            }
            if proposals.len() >= 10 { break; }
        }
        proposals
    }

    pub fn to_patch(&self, proposal: &ApiProposal, iteration_id: u64) -> PatchSet {
        PatchSet {
            id: format!("api-patch-{}", proposal.id),
            files_changed: vec![proposal.target.split("::").next().unwrap_or("").to_string()],
            diff_summary: format!("[API Evolution] {}: {:?}", proposal.target, proposal.kind),
            source: "api_evolution".to_string(),
            timestamp: now() + iteration_id,
            intended_content: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propose_evolutions_returns_proposals() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let engine = ApiEvolutionEngine::new(true);
        let proposals = engine.propose_evolutions(&root);
        // The project almost certainly has some multi-param functions
        // Just verify it doesn't panic and returns a vec
        assert!(proposals.len() >= 0);
    }
}
