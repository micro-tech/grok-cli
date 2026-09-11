//! HOH OKF ↔ TaskList Sync (Task 327.8)
//!
//! Keeps the task list in sync with structured knowledge stored in OKF bundles.
//!
//! # What it does
//!
//! | Situation | Proposal |
//! |-----------|----------|
//! | OKF concept with no matching task | `AddTask` — bring the concept into the backlog |
//! | OKF concept tagged `"deprecated"` / `"obsolete"` and a task matches it | `SetStatus("deferred")` |
//! | OKF concept tagged `"critical"` / `"high"` and the matching task is not high-priority | `SetPriority("high")` |
//! | Matching task has empty or short `description` | `SetDescription` — enrich from OKF |
//! | Matching task has no `testStrategy` and the concept is of type `Runbook` or `API` | `SetTestStrategy` |
//!
//! # Matching heuristic
//!
//! A task is considered to **match** an OKF concept when the combined relevance
//! score (keyword overlap between concept title/tags and task title/description)
//! exceeds [`MATCH_THRESHOLD`].  This avoids false positives when bundles contain
//! hundreds of concepts.
//!
//! # Graceful degradation
//!
//! When no OKF bundles are loaded or the bundle directories are empty the syncer
//! returns an empty proposal list silently — it never errors.

use crate::hoh::task_mutation::TaskMutation;
use crate::hoh::tasklist_adapter::{Task, TaskList};
use crate::knowledge::okf::OkfConcept;
use std::collections::HashSet;

// ─── Constants ──────────────────────────────────────────────────────────────────

/// Minimum relevance score for a task–concept pair to be considered a match.
const MATCH_THRESHOLD: f32 = 1.5;

/// IDs are generated from 90 000 000 upward to avoid collisions with existing tasks.
const NEW_TASK_ID_BASE: u64 = 90_000_000;

// ─── Types ──────────────────────────────────────────────────────────────────────

/// One proposal produced by the syncer.
#[derive(Debug, Clone)]
pub struct OkfSyncProposal {
    pub mutation: TaskMutation,
    /// OKF concept ID that triggered this proposal.
    pub okf_concept_id: String,
    /// Human-readable reasoning.
    pub reason: String,
}

// ─── Syncer ─────────────────────────────────────────────────────────────────────

/// Compares OKF bundles against the current task list and emits proposals.
#[derive(Debug, Default)]
pub struct OkfTaskListSyncer {
    /// Only generate `AddTask` proposals — never update existing tasks.
    pub add_only: bool,
    /// Maximum number of `AddTask` proposals per sync run (prevents flooding).
    pub max_new_tasks: usize,
}

impl OkfTaskListSyncer {
    pub fn new() -> Self {
        Self {
            add_only: false,
            max_new_tasks: 5,
        }
    }

    // ── Entry points ────────────────────────────────────────────────────────────

    /// Run a sync using the globally cached OKF bundles.
    ///
    /// This is the production path.  Returns an empty list when no bundles are
    /// loaded — never returns an error for missing OKF config.
    pub fn sync_from_global(&self, list: &TaskList) -> Vec<OkfSyncProposal> {
        let bundles = crate::tools::okf_tools::get_okf_bundles(None);
        let concepts: Vec<&OkfConcept> = bundles
            .iter()
            .flat_map(|b| b.concepts.iter())
            .collect();

        if concepts.is_empty() {
            tracing::debug!("OkfTaskListSyncer: no OKF concepts loaded — skipping sync");
            return vec![];
        }

        tracing::debug!(
            "OkfTaskListSyncer: syncing {} concepts against {} tasks",
            concepts.len(),
            list.tasks.len()
        );

        self.sync_with_concepts(&concepts, list)
    }

    /// Pure version: sync a pre-built concept slice against a task list.
    ///
    /// Used directly in unit tests (no filesystem or global state needed).
    pub fn sync_with_concepts(
        &self,
        concepts: &[&OkfConcept],
        list: &TaskList,
    ) -> Vec<OkfSyncProposal> {
        let mut proposals = Vec::new();
        let mut new_task_counter: u64 = 0;

        // Collect all tasks (flat including subtasks) for matching
        let flat_tasks = Self::flatten_tasks(&list.tasks);
        let existing_ids: HashSet<u64> = flat_tasks.iter().map(|t| t.id).collect();

        for concept in concepts {
            // Find the best-matching existing task (if any)
            let best_match = self.best_match(concept, &flat_tasks);

            match best_match {
                Some((task, score)) => {
                    tracing::trace!(
                        "OKF concept '{}' matched task #{} (score {:.2})",
                        concept.id,
                        task.id,
                        score
                    );

                    if !self.add_only {
                        // Propose mutations based on concept metadata vs task state
                        self.propose_status_updates(concept, task, &mut proposals);
                        self.propose_priority_elevation(concept, task, &mut proposals);
                        self.propose_description_enrichment(concept, task, &mut proposals);
                        self.propose_test_strategy(concept, task, &mut proposals);
                    }
                }
                None => {
                    // No matching task — suggest adding one from the OKF concept
                    if new_task_counter < self.max_new_tasks as u64
                        && !self.concept_is_deprecated(concept)
                    {
                        let new_id =
                            NEW_TASK_ID_BASE + new_task_counter + existing_ids.len() as u64;
                        new_task_counter += 1;

                        let new_task = self.concept_to_task(concept, new_id);
                        proposals.push(OkfSyncProposal {
                            mutation: TaskMutation::AddTask { new_task },
                            okf_concept_id: concept.id.clone(),
                            reason: format!(
                                "OKF concept '{}' (type: {}) has no matching task — added to backlog",
                                concept.title, concept.r#type
                            ),
                        });
                    }
                }
            }
        }

        proposals
    }

    // ── Proposal generators ─────────────────────────────────────────────────────

    fn propose_status_updates(
        &self,
        concept: &OkfConcept,
        task: &Task,
        out: &mut Vec<OkfSyncProposal>,
    ) {
        if task.status != "pending" && task.status != "in_progress" {
            return; // only act on active tasks
        }

        if self.concept_is_deprecated(concept) {
            out.push(OkfSyncProposal {
                mutation: TaskMutation::SetStatus {
                    task_id: task.id,
                    new_status: "deferred".to_string(),
                },
                okf_concept_id: concept.id.clone(),
                reason: format!(
                    "OKF concept '{}' is tagged deprecated/obsolete — deferring task #{}",
                    concept.title, task.id
                ),
            });
        }
    }

    fn propose_priority_elevation(
        &self,
        concept: &OkfConcept,
        task: &Task,
        out: &mut Vec<OkfSyncProposal>,
    ) {
        if task.priority == "high" {
            return; // already high, nothing to do
        }

        let tags_lower: Vec<String> = concept.tags.iter().map(|t| t.to_lowercase()).collect();
        let is_critical = tags_lower
            .iter()
            .any(|t| t == "critical" || t == "high" || t == "high-priority" || t == "urgent");

        if is_critical {
            out.push(OkfSyncProposal {
                mutation: TaskMutation::SetPriority {
                    task_id: task.id,
                    new_priority: "high".to_string(),
                },
                okf_concept_id: concept.id.clone(),
                reason: format!(
                    "OKF concept '{}' is tagged as critical/high — elevating task #{} priority",
                    concept.title, task.id
                ),
            });
        }
    }

    fn propose_description_enrichment(
        &self,
        concept: &OkfConcept,
        task: &Task,
        out: &mut Vec<OkfSyncProposal>,
    ) {
        // Only enrich if the task description is missing or very short
        if task.description.len() >= 60 {
            return;
        }

        // Build a richer description from the OKF concept
        let new_desc = if concept.description.is_empty() {
            format!("[OKF: {}] {}", concept.r#type, concept.title)
        } else {
            format!(
                "[OKF: {}] {}",
                concept.r#type,
                concept.description.trim()
            )
        };

        if new_desc.len() > task.description.len() + 10 {
            out.push(OkfSyncProposal {
                mutation: TaskMutation::SetDescription {
                    task_id: task.id,
                    new_description: new_desc,
                },
                okf_concept_id: concept.id.clone(),
                reason: format!(
                    "Enriched task #{} description from OKF concept '{}'",
                    task.id, concept.title
                ),
            });
        }
    }

    fn propose_test_strategy(
        &self,
        concept: &OkfConcept,
        task: &Task,
        out: &mut Vec<OkfSyncProposal>,
    ) {
        if !task.test_strategy.trim().is_empty() {
            return; // already has one
        }

        let concept_type = concept.r#type.to_lowercase();
        let strategy = if concept_type.contains("runbook") {
            format!(
                "Follow the runbook steps in OKF concept '{}'. Verify each step completes without error.",
                concept.title
            )
        } else if concept_type.contains("api") {
            format!(
                "Verify the API described in OKF concept '{}': test happy path, error cases, and auth.",
                concept.title
            )
        } else if concept_type.contains("metric") {
            format!(
                "Verify metric '{}': confirm the value is emitted, within expected range, and queryable.",
                concept.title
            )
        } else {
            return; // only inject for known concept types
        };

        out.push(OkfSyncProposal {
            mutation: TaskMutation::SetTestStrategy {
                task_id: task.id,
                strategy,
            },
            okf_concept_id: concept.id.clone(),
            reason: format!(
                "OKF concept '{}' (type: {}) provides a testStrategy template for task #{}",
                concept.title, concept.r#type, task.id
            ),
        });
    }

    // ── Matching ────────────────────────────────────────────────────────────────

    /// Find the best-matching task for an OKF concept.
    /// Returns `(task, score)` when score ≥ [`MATCH_THRESHOLD`].
    fn best_match<'t>(
        &self,
        concept: &OkfConcept,
        tasks: &'t [Task],
    ) -> Option<(&'t Task, f32)> {
        let mut best: Option<(&Task, f32)> = None;

        for task in tasks {
            let score = Self::relevance(concept, task);
            if score >= MATCH_THRESHOLD {
                if best.map_or(true, |(_, s)| score > s) {
                    best = Some((task, score));
                }
            }
        }

        best
    }

    /// Compute a relevance score between a concept and a task.
    ///
    /// Higher = better match. Threshold is [`MATCH_THRESHOLD`].
    pub fn relevance(concept: &OkfConcept, task: &Task) -> f32 {
        let mut score = 0.0_f32;

        let task_text = format!(
            "{} {} {}",
            task.title.to_lowercase(),
            task.description.to_lowercase(),
            task.details.to_lowercase()
        );

        // Concept ID match (strongest signal)
        let concept_id_slug = concept.id.replace('/', " ").replace('_', " ").to_lowercase();
        for word in concept_id_slug.split_whitespace().filter(|w| w.len() > 3) {
            if task_text.contains(word) {
                score += 1.2;
            }
        }

        // Title word overlap
        let title_words: Vec<&str> = concept
            .title
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();
        for word in &title_words {
            let w = word.to_lowercase();
            if task.title.to_lowercase().contains(&w) {
                score += 1.5; // title↔title match is strongest
            } else if task_text.contains(&w) {
                score += 0.6;
            }
        }

        // Tag overlap
        for tag in &concept.tags {
            let t = tag.to_lowercase();
            if task_text.contains(&t) {
                score += 0.8;
            }
        }

        // Description word overlap (less weight)
        let desc_words: Vec<&str> = concept
            .description
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .collect();
        let desc_hits = desc_words
            .iter()
            .filter(|w| task_text.contains(w.to_lowercase().as_str()))
            .count();
        score += desc_hits.min(3) as f32 * 0.4;

        score
    }

    // ── Helpers ─────────────────────────────────────────────────────────────────

    fn concept_is_deprecated(&self, concept: &OkfConcept) -> bool {
        let tags_lower: Vec<String> = concept.tags.iter().map(|t| t.to_lowercase()).collect();
        tags_lower
            .iter()
            .any(|t| t == "deprecated" || t == "obsolete" || t == "retired")
            || concept.description.to_lowercase().contains("deprecated")
            || concept.body.to_lowercase().starts_with("deprecated")
    }

    fn concept_to_task(&self, concept: &OkfConcept, id: u64) -> Task {
        let details = if concept.body.is_empty() {
            format!(
                "Implement work related to OKF concept: {} (type: {}, bundle: {})",
                concept.title, concept.r#type, concept.bundle_name
            )
        } else {
            // Use first 600 chars of the OKF body as task details
            let body_preview: String = concept.body.chars().take(600).collect();
            format!(
                "[OKF: {}]\n\n{}",
                concept.id,
                body_preview.trim()
            )
        };

        let test_strategy = match concept.r#type.to_lowercase().as_str() {
            t if t.contains("runbook") => format!(
                "Follow all steps in the runbook '{}'. Each step must complete without error.",
                concept.title
            ),
            t if t.contains("api") => format!(
                "Test the API '{}': happy path, error handling, and authentication.",
                concept.title
            ),
            _ => String::new(),
        };

        Task {
            id,
            title: format!("[OKF] {}", concept.title),
            description: concept.description.clone(),
            status: "pending".to_string(),
            priority: if concept
                .tags
                .iter()
                .any(|t| t.to_lowercase() == "critical" || t.to_lowercase() == "high")
            {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            details,
            test_strategy,
            dependencies: vec![],
            subtasks: vec![],
        }
    }

    fn flatten_tasks(tasks: &[Task]) -> Vec<Task> {
        let mut out = Vec::new();
        for t in tasks {
            out.push(t.clone());
            for sub in &t.subtasks {
                out.push(sub.clone());
            }
        }
        out
    }
}

// ─── Convenience: extract just the mutations from proposals ───────────────────

/// Extract only the `TaskMutation`s from a proposal list.
pub fn proposals_to_mutations(proposals: Vec<OkfSyncProposal>) -> Vec<TaskMutation> {
    proposals.into_iter().map(|p| p.mutation).collect()
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoh::task_mutation::TaskMutation;
    use crate::hoh::tasklist_adapter::Task;
    use crate::knowledge::okf::OkfConcept;
    use std::path::PathBuf;

    // ── Helpers ──────────────────────────────────────────────────────────────────

    fn concept(id: &str, title: &str, typ: &str, description: &str, tags: &[&str]) -> OkfConcept {
        OkfConcept {
            id: id.to_string(),
            r#type: typ.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            resource: None,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            timestamp: None,
            body: String::new(),
            source_path: PathBuf::from(format!("{}.md", id)),
            bundle_name: "test-bundle".to_string(),
        }
    }

    fn concept_with_body(
        id: &str,
        title: &str,
        typ: &str,
        description: &str,
        tags: &[&str],
        body: &str,
    ) -> OkfConcept {
        let mut c = concept(id, title, typ, description, tags);
        c.body = body.to_string();
        c
    }

    fn task(id: u64, title: &str, priority: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            priority: priority.to_string(),
            status: "pending".to_string(),
            description: String::new(),
            details: "Some details.".to_string(),
            test_strategy: String::new(),
            dependencies: vec![],
            subtasks: vec![],
        }
    }

    fn task_list(tasks: Vec<Task>) -> TaskList {
        TaskList { tasks }
    }

    fn syncer() -> OkfTaskListSyncer {
        OkfTaskListSyncer::new()
    }

    // ── Matching ────────────────────────────────────────────────────────────────

    #[test]
    fn relevance_high_for_title_overlap() {
        let c = concept("auth/oauth", "OAuth Integration", "API", "OAuth 2.0 flow", &[]);
        let t = task(1, "Implement OAuth integration for login", "medium");
        let score = OkfTaskListSyncer::relevance(&c, &t);
        assert!(
            score >= MATCH_THRESHOLD,
            "title overlap should produce score >= {MATCH_THRESHOLD}, got {score}"
        );
    }

    #[test]
    fn relevance_low_for_unrelated() {
        let c = concept("db/orders", "Orders Table", "BigQuery Table", "Order data", &[]);
        let t = task(1, "Write release notes", "low");
        let score = OkfTaskListSyncer::relevance(&c, &t);
        assert!(
            score < MATCH_THRESHOLD,
            "unrelated concept/task should score < {MATCH_THRESHOLD}, got {score}"
        );
    }

    #[test]
    fn relevance_id_slug_contributes() {
        let c = concept("hoh/tasklist_adapter", "TaskList Adapter", "Module", "", &[]);
        let t = task(1, "Improve the tasklist adapter load path", "medium");
        let score = OkfTaskListSyncer::relevance(&c, &t);
        assert!(
            score >= MATCH_THRESHOLD,
            "ID slug words should contribute to score, got {score}"
        );
    }

    // ── AddTask proposals ───────────────────────────────────────────────────────

    #[test]
    fn proposes_add_task_when_no_match() {
        let s = syncer();
        let c = concept(
            "infra/cache-layer",
            "Cache Layer",
            "Module",
            "Redis-backed cache for hot data",
            &[],
        );
        let list = task_list(vec![task(1, "Write unit tests", "medium")]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        assert_eq!(proposals.len(), 1);
        assert!(
            matches!(proposals[0].mutation, TaskMutation::AddTask { .. }),
            "should propose AddTask for an unmatched concept"
        );
    }

    #[test]
    fn add_task_uses_concept_title_and_description() {
        let s = syncer();
        let c = concept(
            "monitoring/sla-tracker",
            "SLA Tracker",
            "Metric",
            "Tracks SLA compliance per service",
            &[],
        );
        let list = task_list(vec![]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        let TaskMutation::AddTask { new_task } = &proposals[0].mutation else {
            panic!("expected AddTask");
        };
        assert!(new_task.title.contains("SLA Tracker"));
        assert_eq!(new_task.description, "Tracks SLA compliance per service");
    }

    #[test]
    fn deprecated_concept_does_not_become_a_new_task() {
        let s = syncer();
        let c = concept(
            "old/legacy-api",
            "Legacy API",
            "API",
            "Deprecated endpoint",
            &["deprecated"],
        );
        let list = task_list(vec![]);
        let proposals = s.sync_with_concepts(&[&c], &list);
        assert!(
            proposals.is_empty(),
            "deprecated concepts should not generate AddTask proposals"
        );
    }

    #[test]
    fn max_new_tasks_is_respected() {
        let mut s = syncer();
        s.max_new_tasks = 2;

        let concepts: Vec<OkfConcept> = (1..=10)
            .map(|i| concept(&format!("new/concept-{}", i), &format!("Concept {}", i), "Module", "", &[]))
            .collect();
        let refs: Vec<&OkfConcept> = concepts.iter().collect();
        let list = task_list(vec![]);
        let proposals = s.sync_with_concepts(&refs, &list);

        let add_task_count = proposals
            .iter()
            .filter(|p| matches!(p.mutation, TaskMutation::AddTask { .. }))
            .count();
        assert_eq!(add_task_count, 2, "should cap at max_new_tasks=2");
    }

    // ── Status / priority updates ────────────────────────────────────────────────

    #[test]
    fn proposes_deferred_for_deprecated_concept_with_matching_task() {
        let s = syncer();
        let c = concept(
            "auth/old-oauth",
            "Old OAuth Flow",
            "API",
            "Deprecated OAuth flow",
            &["deprecated"],
        );
        // Task closely matches the concept
        let t = task(42, "Implement old OAuth flow authentication", "medium");
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        assert!(
            proposals
                .iter()
                .any(|p| matches!(&p.mutation, TaskMutation::SetStatus { task_id: 42, new_status } if new_status == "deferred")),
            "should propose SetStatus(deferred) for the matching task"
        );
    }

    #[test]
    fn elevates_priority_for_critical_concept() {
        let s = syncer();
        let c = concept(
            "security/auth-hardening",
            "Auth Hardening",
            "Runbook",
            "Critical security procedure",
            &["critical", "security"],
        );
        let t = task(7, "Implement auth hardening security procedure", "low");
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        assert!(
            proposals.iter().any(|p| matches!(
                &p.mutation,
                TaskMutation::SetPriority { task_id: 7, new_priority }
                if new_priority == "high"
            )),
            "should propose SetPriority(high) for critical concept"
        );
    }

    #[test]
    fn no_priority_elevation_when_already_high() {
        let s = syncer();
        let c = concept("sec/crit", "Crit Module", "Module", "", &["critical"]);
        let mut t = task(8, "Crit module implementation", "medium");
        t.priority = "high".to_string();
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        assert!(
            !proposals.iter().any(|p| matches!(p.mutation, TaskMutation::SetPriority { .. })),
            "no priority elevation when task is already high"
        );
    }

    // ── Description enrichment ──────────────────────────────────────────────────

    #[test]
    fn enriches_empty_task_description() {
        let s = syncer();
        let c = concept(
            "data/pipeline",
            "Data Pipeline",
            "Module",
            "Real-time data processing pipeline for events",
            &[],
        );
        let t = task(10, "Build the data pipeline processing module", "medium");
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        assert!(
            proposals.iter().any(|p| matches!(p.mutation, TaskMutation::SetDescription { task_id: 10, .. })),
            "should propose SetDescription for task with empty description"
        );
    }

    #[test]
    fn does_not_enrich_long_description() {
        let s = syncer();
        let c = concept("data/pipeline", "Data Pipeline", "Module", "Short", &[]);
        let mut t = task(10, "Build the data pipeline processing module", "medium");
        t.description =
            "This task implements a comprehensive real-time data processing pipeline that handles events from multiple sources with backpressure support.".to_string();
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        assert!(
            !proposals.iter().any(|p| matches!(p.mutation, TaskMutation::SetDescription { .. })),
            "should not enrich a task that already has a long description"
        );
    }

    // ── Test strategy injection ─────────────────────────────────────────────────

    #[test]
    fn injects_test_strategy_for_runbook_concept() {
        let s = syncer();
        let c = concept(
            "ops/deploy",
            "Deployment Runbook",
            "Runbook",
            "Steps to deploy the service",
            &[],
        );
        let t = task(20, "Follow deployment runbook procedure", "medium");
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        assert!(
            proposals.iter().any(|p| matches!(p.mutation, TaskMutation::SetTestStrategy { task_id: 20, .. })),
            "should inject testStrategy for Runbook concept"
        );
    }

    #[test]
    fn no_strategy_injection_for_plain_concept() {
        let s = syncer();
        let c = concept("misc/notes", "Meeting Notes", "Document", "", &[]);
        let t = task(21, "Review meeting notes", "low");
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);
        assert!(
            !proposals.iter().any(|p| matches!(p.mutation, TaskMutation::SetTestStrategy { .. })),
            "should not inject testStrategy for non-actionable concept types"
        );
    }

    // ── add_only mode ───────────────────────────────────────────────────────────

    #[test]
    fn add_only_skips_mutation_proposals() {
        let mut s = syncer();
        s.add_only = true;

        let c = concept(
            "auth/oauth",
            "OAuth Integration",
            "API",
            "OAuth 2.0",
            &["critical"],
        );
        // Task closely matches → would normally get priority + description proposals
        let t = task(1, "Implement OAuth integration authentication", "low");
        let list = task_list(vec![t]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        // In add_only mode, no mutation proposals for matched tasks
        assert!(
            !proposals.iter().any(|p| matches!(
                p.mutation,
                TaskMutation::SetPriority { .. } | TaskMutation::SetDescription { .. }
            )),
            "add_only mode should suppress mutation proposals for matched tasks"
        );
    }

    // ── proposals_to_mutations ──────────────────────────────────────────────────

    #[test]
    fn proposals_to_mutations_extracts_correctly() {
        let proposals = vec![
            OkfSyncProposal {
                mutation: TaskMutation::SetPriority {
                    task_id: 1,
                    new_priority: "high".to_string(),
                },
                okf_concept_id: "test/concept".to_string(),
                reason: "test".to_string(),
            },
            OkfSyncProposal {
                mutation: TaskMutation::SetStatus {
                    task_id: 2,
                    new_status: "deferred".to_string(),
                },
                okf_concept_id: "test/concept2".to_string(),
                reason: "test".to_string(),
            },
        ];
        let mutations = proposals_to_mutations(proposals);
        assert_eq!(mutations.len(), 2);
        assert!(matches!(mutations[0], TaskMutation::SetPriority { .. }));
        assert!(matches!(mutations[1], TaskMutation::SetStatus { .. }));
    }

    // ── concept_with_body becomes task details ──────────────────────────────────

    #[test]
    fn okf_body_used_as_task_details() {
        let s = syncer();
        let c = concept_with_body(
            "arch/cqrs",
            "CQRS Pattern",
            "Pattern",
            "Command-Query Responsibility Segregation",
            &[],
            "## Overview\nSeparate read and write models for scalability.",
        );
        let list = task_list(vec![]);
        let proposals = s.sync_with_concepts(&[&c], &list);

        let TaskMutation::AddTask { new_task } = &proposals[0].mutation else {
            panic!("expected AddTask");
        };
        assert!(
            new_task.details.contains("Separate read and write"),
            "OKF body should appear in new task details"
        );
    }
}
