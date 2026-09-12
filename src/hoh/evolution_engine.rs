//! HOH Task Evolution Engine (Task 327.3)
//!
//! Uses the dependency graph + mutation rules to intelligently evolve the task list.
//! This is the "brain" that proposes and applies safe improvements to tasks over time.

use crate::hoh::okf_tasklist_sync::OkfTaskListSyncer;
use crate::hoh::state::HOHError;
use crate::hoh::task_dependency_graph::TaskDependencyGraph;
use crate::hoh::task_mutation::{MutationResult, TaskMutation, TaskMutationRules};
use crate::hoh::tasklist_adapter::{Task, TaskList, TaskListAdapter};

/// The main evolution engine.
#[derive(Debug)]
pub struct TaskEvolutionEngine {
    adapter: TaskListAdapter,
    rules: TaskMutationRules,
}

impl TaskEvolutionEngine {
    pub fn new(adapter: TaskListAdapter) -> Self {
        Self {
            adapter,
            rules: TaskMutationRules::new(),
        }
    }

    /// Analyze the current task list and propose a set of safe mutations.
    /// Enhanced with OKF awareness (327.8), Helix feedback hooks (327.9), and better heuristics.
    pub async fn propose_evolutions(&self, recent_helix_score: Option<f32>) -> Result<Vec<TaskMutation>, HOHError> {
        let list = self.adapter.load().await?;
        let _graph = self.adapter.build_dependency_graph().await?;
        let mut proposals = Vec::new();

        // 327.8: OKF Sync — real implementation via OkfTaskListSyncer
        let okf_syncer = OkfTaskListSyncer::new();
        let okf_proposals = okf_syncer.sync_from_global(&list);
        if !okf_proposals.is_empty() {
            tracing::info!(
                "HOH Evolution (327.8): OKF sync produced {} proposals",
                okf_proposals.len()
            );
            for p in okf_proposals {
                proposals.push(p.mutation);
            }
        }

        // 327.9: Sync TaskList with Helix Evaluations
        // Map evaluation results back to affected tasks: adjust priorities/status based on helix_score.
        // High score → modest promotion of well-defined / high-impact pending work (momentum).
        // Low score → elevate remediation-focused tasks (test, fix, debug, quality).
        if let Some(score) = recent_helix_score {
            let clamped = score.clamp(0.0, 1.0);
            tracing::info!("HOH Evolution (327.9): applying Helix score {:.2} to task list sync", clamped);

            if clamped < 0.5 {
                // Poor evaluation: boost tasks that look like they can remediate issues
                for task in &list.tasks {
                    if task.status == "pending" && task.priority != "high" {
                        let text = format!(
                            "{} {} {}",
                            task.title.to_lowercase(),
                            task.description.to_lowercase(),
                            task.details.to_lowercase()
                        );
                        if text.contains("test") || text.contains("fix") || text.contains("debug")
                            || text.contains("quality") || text.contains("fail") || text.contains("error")
                            || text.contains("regression")
                        {
                            proposals.push(TaskMutation::SetPriority {
                                task_id: task.id,
                                new_priority: "high".to_string(),
                            });
                            // also ensure it has a basic test strategy if missing (makes it actionable)
                            if task.test_strategy.trim().is_empty() {
                                proposals.push(TaskMutation::SetTestStrategy {
                                    task_id: task.id,
                                    strategy: "Re-run relevant tests and verify fix for recent low Helix score.".to_string(),
                                });
                            }
                        }
                    }
                }
                tracing::info!("HOH Evolution (327.9): low helix — proposed priority boosts for remediation tasks");
            } else if clamped >= 0.75 {
                // Strong evaluation: promote a few well-specified pending tasks to keep momentum
                let mut promoted = 0usize;
                for task in &list.tasks {
                    if promoted >= 2 { break; }
                    if task.status == "pending" && task.priority == "medium" {
                        let has_good_definition = !task.test_strategy.trim().is_empty() && task.details.len() > 60;
                        if has_good_definition {
                            proposals.push(TaskMutation::SetPriority {
                                task_id: task.id,
                                new_priority: "high".to_string(),
                            });
                            promoted += 1;
                        }
                    }
                }
                if promoted > 0 {
                    tracing::info!("HOH Evolution (327.9): high helix — promoted {} well-defined tasks", promoted);
                }
            }
        }

        // 1. Promote high-value pending tasks that have good test strategies
        for task in &list.tasks {
            if task.status == "pending" && task.priority != "high" {
                if !task.test_strategy.is_empty() && task.test_strategy.len() > 30 {
                    proposals.push(TaskMutation::SetPriority {
                        task_id: task.id,
                        new_priority: "high".to_string(),
                    });
                }
            }
        }

        // 2. Split overly large tasks - 327.10 Auto-Expansion (SKILL.md style)
        // Produces phased, actionable subtasks with proper testStrategy and sequential dependencies.
        for task in &list.tasks {
            if task.details.len() > 1800 && task.subtasks.is_empty() && task.status == "pending" {
                let subs = create_skilled_auto_expansion(task);
                if !subs.is_empty() {
                    proposals.push(TaskMutation::SplitTask {
                        task_id: task.id,
                        new_subtasks: subs,
                    });
                }
            }
        }

        // 3. Add missing dependencies for tasks that mention other task IDs in details (327.4)
        for task in &list.tasks {
            if task.dependencies.is_empty() {
                if let Some(dep_id) = self.extract_mentioned_task_id(&task.details, &list) {
                    if dep_id != task.id {
                        proposals.push(TaskMutation::AddDependency {
                            task_id: task.id,
                            depends_on: dep_id,
                        });
                    }
                }
            }
        }

        // 4. Defer or cancel very old low-priority tasks (kept for compatibility; main logic moved to 327.12)
        for task in &list.tasks {
            if task.priority == "low" && task.status == "pending" {
                let title_lower = task.title.to_lowercase();
                if title_lower.contains("deprecated") || title_lower.contains("obsolete") || title_lower.contains("legacy") {
                    proposals.push(TaskMutation::SetStatus {
                        task_id: task.id,
                        new_status: "deferred".to_string(),
                    });
                }
            }
        }

        // 5. 327.11 Auto-Refinement: Improve vague tasks (descriptions + details + testStrategy)
        // Uses title + existing weak content to generate more specific, actionable content.
        // Follows SKILL.md: make tasks specific, with clear testStrategy and rich details.
        for task in &list.tasks {
            if task.status != "pending" {
                continue;
            }
            let is_vague = task.details.len() < 100
                || task.test_strategy.trim().is_empty()
                || task.details.trim().len() < 40
                || task.description.trim().len() < 20;

            if is_vague {
                let refinements = propose_auto_refinements(task);
                for r in refinements {
                    if self.rules.validate_mutation(&r, &list.tasks).is_ok() {
                        proposals.push(r);
                    }
                }
            }
        }

        // 6. 327.8 OKF Sync hint (lightweight): if task mentions knowledge-related terms, suggest OKF-related detail
        // (We don't mutate OKF directly here; we just improve the task description when relevant)
        for task in &list.tasks {
            let combined = format!("{} {}", task.title, task.details).to_lowercase();
            if (combined.contains("okf") || combined.contains("knowledge") || combined.contains("bundle"))
                && task.details.len() < 300
                && task.status == "pending"
            {
                // Propose a small refinement to call out OKF usage
                proposals.push(TaskMutation::SetDescription {
                    task_id: task.id,
                    new_description: format!(
                        "{}\n\n[HOH Evolution] Consider using okf_lookup / okf_get tools for structured knowledge.",
                        task.description
                    ),
                });
            }
        }

        // 7. 327.12 Auto-Pruning: identify and propose deferral of obsolete, stale, duplicate or low-value tasks
        // Conservative: only SetStatus → "deferred" (never hard delete). Always safe + reviewable.
        // Multiple signals required for a proposal (keywords + low substance).
        let prunings = propose_auto_prunings(&list);
        for p in prunings {
            if self.rules.validate_mutation(&p, &list.tasks).is_ok() {
                proposals.push(p);
            }
        }

        // Filter proposals through the rule engine
        let mut valid_proposals = Vec::new();
        for proposal in proposals {
            if self.rules.validate_mutation(&proposal, &list.tasks).is_ok() {
                valid_proposals.push(proposal);
            }
        }

        Ok(valid_proposals)
    }

    /// Apply a list of mutations (after validation).
    /// 327.13: If `create_version` is true, a versioned snapshot is saved before writing the new list.
    pub async fn apply_mutations(
        &self,
        mutations: &[TaskMutation],
        create_version: bool,
    ) -> Result<Vec<MutationResult>, HOHError> {
        let mut list = self.adapter.load().await?;
        let mut results = Vec::new();

        for mutation in mutations {
            match self.rules.apply_mutation(mutation, &mut list.tasks) {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    tracing::warn!("Mutation failed: {}", e);
                    results.push(MutationResult {
                        success: false,
                        message: e,
                        task_id: 0,
                    });
                }
            }
        }

        if create_version && !self.adapter.is_simulation() {
            let _ = self
                .adapter
                .save_versioned(&list, "post-mutation", "evolution_engine")
                .await;
        }

        // Save the evolved task list (normal path also creates .bak)
        self.adapter.save(&list).await?;
        Ok(results)
    }

    /// Full evolution cycle: propose + optionally apply.
    /// Now accepts optional recent_helix_score (327.9) to influence priority/status mutations.
    pub async fn run_evolution_cycle(
        &self,
        auto_apply: bool,
        recent_helix_score: Option<f32>,
    ) -> Result<(Vec<TaskMutation>, Vec<MutationResult>), HOHError> {
        let proposals = self.propose_evolutions(recent_helix_score).await?;

        if auto_apply && !proposals.is_empty() {
            // 327.13: create a versioned snapshot for the mutations
            let results = self.apply_mutations(&proposals, true).await?;
            Ok((proposals, results))
        } else {
            Ok((proposals, vec![]))
        }
    }

    /// Simple heuristic to extract a task ID mentioned in text (e.g. "see task 123")
    fn extract_mentioned_task_id(&self, text: &str, list: &TaskList) -> Option<u64> {
        let text_lower = text.to_lowercase();

        for task in &list.tasks {
            let id_str = task.id.to_string();
            if text_lower.contains(&format!("task {}", id_str))
                || text_lower.contains(&format!("#{}", id_str))
                || text_lower.contains(&format!("t{}", id_str))
            {
                return Some(task.id);
            }
        }
        None
    }

    /// Get the current dependency graph (convenience)
    pub async fn get_graph(&self) -> Result<TaskDependencyGraph, HOHError> {
        self.adapter.build_dependency_graph().await
    }
}

/// 327.11: Auto-Refinement — turn vague tasks into high-quality, actionable ones.
/// Uses title + existing (weak) description/details to produce:
/// - richer description
/// - expanded details (approach, scope, files, risks)
/// - strong test_strategy
///
/// This is the core of 327.11. It is deliberately heuristic + template-based
/// so it stays deterministic and safe inside the evolution loop.
fn propose_auto_refinements(task: &Task) -> Vec<TaskMutation> {
    let mut mutations = Vec::new();
    let title_lower = task.title.to_lowercase();

    // 1. Refine description if it's weak or missing
    let current_desc = task.description.trim();
    if current_desc.len() < 25 || current_desc == task.title {
        let better_desc = generate_better_description(task);
        if better_desc.len() > current_desc.len() + 10 {
            mutations.push(TaskMutation::SetDescription {
                task_id: task.id,
                new_description: better_desc,
            });
        }
    }

    // 2. Expand / improve details (the "how" and acceptance criteria)
    let current_details = task.details.trim();
    if current_details.len() < 80 {
        let better_details = generate_better_details(task, &title_lower);
        if better_details.len() > current_details.len() + 30 {
            mutations.push(TaskMutation::SetDetails {
                task_id: task.id,
                new_details: better_details,
            });
        }
    }

    // 3. Always try to strengthen test_strategy if missing or generic
    let current_ts = task.test_strategy.trim();
    let needs_better_ts = current_ts.is_empty()
        || current_ts.len() < 25
        || current_ts.contains("run tests")
        || current_ts.contains("verify");

    if needs_better_ts {
        let better_ts = generate_better_test_strategy(task, &title_lower);
        if better_ts.len() > current_ts.len() {
            mutations.push(TaskMutation::SetTestStrategy {
                task_id: task.id,
                strategy: better_ts,
            });
        }
    }

    mutations
}

/// Generate a more specific, SKILL.md-style description from the title + context.
fn generate_better_description(task: &Task) -> String {
    let title = &task.title;
    let base = if task.description.trim().is_empty() {
        format!("Implement / complete: {}", title)
    } else {
        task.description.clone()
    };

    // Add outcome-oriented language
    if title.to_lowercase().contains("implement") || title.to_lowercase().contains("add") {
        format!("{}. Deliver working, tested functionality that matches the acceptance criteria.", base)
    } else if title.to_lowercase().contains("fix") || title.to_lowercase().contains("bug") {
        format!("{}. Root-cause the issue, implement a minimal correct fix, and prevent regression.", base)
    } else if title.to_lowercase().contains("refactor") {
        format!("{}. Improve structure and readability while preserving all observable behavior.", base)
    } else {
        base
    }
}

/// Generate richer details (implementation approach + scope).
fn generate_better_details(task: &Task, title_lower: &str) -> String {
    let mut out = String::new();

    if !task.details.trim().is_empty() {
        out.push_str(&task.details);
        out.push_str("\n\n");
    }

    out.push_str("### Refined Scope (HOH 327.11 Auto-Refinement)\n");

    if title_lower.contains("implement") || title_lower.contains("add") {
        out.push_str("- Identify affected modules and entry points.\n");
        out.push_str("- Define clear interfaces / data shapes.\n");
        out.push_str("- Implement core logic + error handling.\n");
        out.push_str("- Add or update unit + integration tests.\n");
    } else if title_lower.contains("fix") {
        out.push_str("- Reproduce the failure case.\n");
        out.push_str("- Locate root cause (logs, tests, static analysis).\n");
        out.push_str("- Implement targeted fix + regression test.\n");
    } else if title_lower.contains("refactor") {
        out.push_str("- Analyze current structure and coupling.\n");
        out.push_str("- Propose minimal safe refactoring steps.\n");
        out.push_str("- Preserve all public behavior and test coverage.\n");
    } else {
        out.push_str("- Break work into small, verifiable steps.\n");
        out.push_str("- Update relevant documentation and tests.\n");
    }

    out.push_str("\nAcceptance criteria will be validated by the task's testStrategy.");

    out
}

/// 327.12: Auto-Pruning — propose safe deferral of low-value, obsolete, stale or duplicate tasks.
///
/// Conservative policy (per spec):
/// - Only ever proposes `SetStatus { "deferred" }`
/// - Never deletes tasks
/// - Never auto-applies destructive changes
/// - Suggestions should be reviewable via diff + human approval
///
/// Signals used:
/// - Keyword matches for obsolete/deprecated/superseded
/// - Very low information content (short details + weak description)
/// - Duplicate or near-duplicate titles among pending tasks
/// - "todo", "investigate", "hack", "temp", "wip", "placeholder" patterns
fn propose_auto_prunings(list: &TaskList) -> Vec<TaskMutation> {
    let mut proposals = Vec::new();
    let pending_tasks: Vec<&Task> = list
        .tasks
        .iter()
        .filter(|t| t.status == "pending")
        .collect();

    for task in &pending_tasks {
        if should_prune(task, &pending_tasks) {
            proposals.push(TaskMutation::SetStatus {
                task_id: task.id,
                new_status: "deferred".to_string(),
            });
        }
    }

    proposals
}

/// Core decision for 327.12: should this pending task be proposed for deferral?
fn should_prune(task: &Task, all_pending: &[&Task]) -> bool {
    if task.priority == "high" {
        return false; // never auto-prune high priority
    }

    let title_l = task.title.to_lowercase();
    let combined = format!(
        "{} {} {}",
        task.title, task.description, task.details
    )
    .to_lowercase();

    // Strong obsolete / superseded signals
    if title_l.contains("deprecated")
        || title_l.contains("obsolete")
        || title_l.contains("legacy")
        || title_l.contains("superseded")
        || title_l.contains("replaced by")
        || combined.contains("no longer needed")
        || combined.contains("use instead")
    {
        return true;
    }

    // Very low-substance pending tasks (stale / placeholder)
    let is_placeholder = title_l.contains("todo")
        || title_l.contains("investigate")
        || title_l.contains("hack")
        || title_l.contains("temp ")
        || title_l.contains("wip")
        || title_l.contains("placeholder")
        || title_l.contains("fixme");

    let low_substance = task.details.trim().len() < 60
        && task.description.trim().len() < 40
        && task.subtasks.is_empty();

    if is_placeholder && low_substance && task.priority != "high" {
        return true;
    }

    // Duplicate detection (simple exact title match among other pending tasks)
    let duplicate_count = all_pending
        .iter()
        .filter(|t| t.id != task.id && t.title.trim().eq_ignore_ascii_case(&task.title.trim()))
        .count();

    if duplicate_count > 0 && low_substance {
        return true;
    }

    // Additional low-value heuristic: pending + low priority + almost no content at all
    if task.priority == "low"
        && task.details.trim().len() < 30
        && task.subtasks.is_empty()
        && !title_l.contains("implement") // avoid pruning real work that just hasn't been detailed yet
    {
        return true;
    }

    false
}

/// Produce a concrete, measurable test strategy.
fn generate_better_test_strategy(task: &Task, title_lower: &str) -> String {
    if title_lower.contains("implement") || title_lower.contains("add") {
        "All new code has unit tests. Integration points exercised. cargo test + cargo clippy pass. Behavior matches description.".to_string()
    } else if title_lower.contains("fix") {
        "Failing case is reproduced before the fix. After fix the case passes and no existing tests regress. cargo test passes.".to_string()
    } else if title_lower.contains("refactor") {
        "All existing tests still pass at 100%. No behavior change observable. cargo clippy clean.".to_string()
    } else if task.test_strategy.trim().is_empty() {
        "Execute relevant tests for the modified area. Verify against the task description. No new warnings from clippy.".to_string()
    } else {
        // upgrade a weak existing one
        format!("{}. Additionally run full test suite for the affected crate and confirm no regressions.", task.test_strategy)
    }
}

/// 327.10: Create high-quality, SKILL.md-aligned subtasks for a large task.
/// Produces 3-4 phased subtasks with proper testStrategy, sequential dependencies,
/// and actionable scope (following the project's standard subtask conventions).
fn create_skilled_auto_expansion(parent: &Task) -> Vec<Task> {
    let base = parent.id;
    let prio = parent.priority.clone();
    let title = &parent.title;

    let mut subs = Vec::new();

    // Phase 1: Analysis & Design
    subs.push(Task {
        id: base * 100 + 1,
        title: format!("1. Analysis & Design: {}", title),
        description: format!("Analyze requirements and produce a clear plan for: {}", title),
        details: format!(
            "Break down the large task into concrete requirements, identify interfaces/risks, and create an implementation plan.\n\nContext from parent:\n{}",
            parent.details.chars().take(500).collect::<String>()
        ),
        priority: prio.clone(),
        status: "pending".to_string(),
        test_strategy: "Review produces explicit acceptance criteria, dependency list, and high-level design notes.".to_string(),
        dependencies: vec![],
        ..Default::default()
    });

    // Phase 2: Core Implementation
    subs.push(Task {
        id: base * 100 + 2,
        title: format!("2. Core Implementation: {}", title),
        description: "Implement the main logic and changes for the task.".to_string(),
        details: format!("Execute the core work according to the plan from subtask {}.1. Focus on correctness and clarity.", base),
        priority: prio.clone(),
        status: "pending".to_string(),
        test_strategy: "Core functionality works. Relevant unit tests pass. Code is clippy-clean.".to_string(),
        dependencies: vec![base * 100 + 1],
        ..Default::default()
    });

    // Phase 3: Testing & Validation
    subs.push(Task {
        id: base * 100 + 3,
        title: format!("3. Testing & Validation: {}", title),
        description: "Add/enhance tests and validate the complete solution.".to_string(),
        details: "Implement or strengthen the test strategy. Verify behavior against the parent's acceptance criteria and edge cases.".to_string(),
        priority: prio.clone(),
        status: "pending".to_string(),
        test_strategy: "All tests defined in the parent task's testStrategy (and new tests) pass. No regressions.".to_string(),
        dependencies: vec![base * 100 + 2],
        ..Default::default()
    });

    // Phase 4: Documentation & Polish (for very large/complex tasks)
    if parent.details.len() > 2800 {
        subs.push(Task {
            id: base * 100 + 4,
            title: format!("4. Documentation & Polish: {}", title),
            description: "Update docs, examples, and clean up related artifacts.".to_string(),
            details: "Add or improve documentation, usage notes, SKILL.md references, or module docs so the change is maintainable.".to_string(),
            priority: "medium".to_string(),
            status: "pending".to_string(),
            test_strategy: "Documentation is accurate, up-to-date, and sufficient to understand the change without reading the code.".to_string(),
            dependencies: vec![base * 100 + 3],
            ..Default::default()
        });
    }

    subs
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_propose_and_apply() {
        let dir = tempdir().unwrap();
        let adapter = TaskListAdapter::new(dir.path().to_path_buf(), false); // must write so load() sees data

        // Seed a simple list
        let mut list = TaskList::default();
        list.tasks.push(Task {
            id: 100,
            title: "Huge task with lots of details".to_string(),
            details: "x".repeat(2100),
            priority: "medium".to_string(),
            status: "pending".to_string(),
            ..Default::default()
        });

        adapter.save(&list).await.unwrap();

        let engine = TaskEvolutionEngine::new(adapter);
        let (proposals, _) = engine.run_evolution_cycle(false, None).await.unwrap();

        assert!(!proposals.is_empty());
        // Should have proposed a split
        let split = proposals.iter().find(|p| matches!(p, TaskMutation::SplitTask { .. }));
        assert!(split.is_some());

        // 327.10: verify the new subtasks are SKILL.md quality (have test_strategy + phased structure)
        if let Some(TaskMutation::SplitTask { new_subtasks, .. }) = split {
            assert!(new_subtasks.len() >= 3, "auto-expansion should produce at least 3 phases");
            assert!(new_subtasks.iter().all(|s| !s.test_strategy.trim().is_empty()), "all subtasks must have testStrategy");
            assert!(new_subtasks.iter().any(|s| s.title.contains("Analysis")));
            assert!(new_subtasks.iter().any(|s| s.title.contains("Implementation")));
        }
    }

    #[test]
    fn test_32711_auto_refinement_proposals() {
        // Vague high-priority pending task
        let vague = Task {
            id: 42,
            title: "Fix the thing".to_string(),
            description: "Fix it.".to_string(),
            details: "Do the fix.".to_string(),
            test_strategy: "".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
            ..Default::default()
        };

        let mutations = propose_auto_refinements(&vague);

        // Should propose at least description + details + testStrategy improvements
        assert!(!mutations.is_empty(), "should propose refinements for vague task");

        let has_desc = mutations.iter().any(|m| matches!(m, TaskMutation::SetDescription { .. }));
        let has_details = mutations.iter().any(|m| matches!(m, TaskMutation::SetDetails { .. }));
        let has_ts = mutations.iter().any(|m| matches!(m, TaskMutation::SetTestStrategy { .. }));

        assert!(has_desc || has_details || has_ts, "should propose at least one concrete refinement");

        // Check that generated content is meaningfully longer / better
        for m in &mutations {
            match m {
                TaskMutation::SetDescription { new_description, .. } => {
                    assert!(new_description.len() > 30);
                }
                TaskMutation::SetDetails { new_details, .. } => {
                    assert!(new_details.contains("Refined Scope") || new_details.len() > 100);
                }
                TaskMutation::SetTestStrategy { strategy, .. } => {
                    assert!(strategy.len() > 20);
                    assert!(strategy.contains("test") || strategy.contains("cargo"));
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_32711_refinement_in_evolution_cycle() {
        let dir = tempdir().unwrap();
        let adapter = TaskListAdapter::new(dir.path().to_path_buf(), false); // real writes so load sees data

        let mut list = TaskList::default();
        list.tasks.push(Task {
            id: 77,
            title: "Add user export feature".to_string(),
            description: "".to_string(),
            details: "Export users.".to_string(),   // intentionally weak
            test_strategy: "test it".to_string(),   // weak
            priority: "high".to_string(),
            status: "pending".to_string(),
            ..Default::default()
        });

        adapter.save(&list).await.unwrap();

        let engine = TaskEvolutionEngine::new(adapter);
        let (proposals, _) = engine.run_evolution_cycle(false, None).await.unwrap();

        // Should have produced at least one refinement mutation (SetDescription / SetDetails / SetTestStrategy)
        let has_refinement = proposals.iter().any(|p| {
            matches!(p, TaskMutation::SetDescription { .. })
                || matches!(p, TaskMutation::SetDetails { .. })
                || matches!(p, TaskMutation::SetTestStrategy { .. })
        });
        assert!(has_refinement, "Auto-refinement (327.11) should propose improvements for vague tasks");
    }

    // === 327.12 Auto-Pruning Tests ===

    #[test]
    fn test_32712_auto_pruning_proposals() {
        let list = TaskList {
            tasks: vec![
                // Should be pruned: low-substance placeholder
                Task {
                    id: 10,
                    title: "TODO: old hack for v1".to_string(),
                    description: "".to_string(),
                    details: "fix later".to_string(),
                    status: "pending".to_string(),
                    priority: "low".to_string(),
                    subtasks: vec![],
                    ..Default::default()
                },
                // High priority real work — must NOT be pruned
                Task {
                    id: 11,
                    title: "Implement critical auth flow".to_string(),
                    description: "Add proper auth".to_string(),
                    details: "Detailed spec here with many paragraphs of requirements...".to_string(),
                    status: "pending".to_string(),
                    priority: "high".to_string(),
                    ..Default::default()
                },
                // Obsolete keyword
                Task {
                    id: 12,
                    title: "Legacy deprecated reporting".to_string(),
                    description: "Old reporting module".to_string(),
                    details: "This is superseded".to_string(),
                    status: "pending".to_string(),
                    priority: "medium".to_string(),
                    ..Default::default()
                },
                // Duplicate low-value tasks
                Task {
                    id: 13,
                    title: "Investigate performance".to_string(),
                    description: "Look at perf".to_string(),
                    details: "check it".to_string(),
                    status: "pending".to_string(),
                    priority: "low".to_string(),
                    ..Default::default()
                },
                Task {
                    id: 14,
                    title: "Investigate performance".to_string(),
                    description: "Look at perf".to_string(),
                    details: "check it".to_string(),
                    status: "pending".to_string(),
                    priority: "low".to_string(),
                    ..Default::default()
                },
                // Low priority + almost no content
                Task {
                    id: 15,
                    title: "Temp cleanup task".to_string(),
                    description: "".to_string(),
                    details: "".to_string(),
                    status: "pending".to_string(),
                    priority: "low".to_string(),
                    ..Default::default()
                },
            ],
        };

        let prunings = propose_auto_prunings(&list);
        let deferred: Vec<u64> = prunings
            .iter()
            .filter_map(|m| {
                if let TaskMutation::SetStatus { task_id, new_status } = m {
                    if new_status == "deferred" { Some(*task_id) } else { None }
                } else {
                    None
                }
            })
            .collect();

        assert!(deferred.contains(&10), "low-substance TODO should be proposed for pruning");
        assert!(deferred.contains(&12), "deprecated/legacy should be pruned");
        assert!(deferred.contains(&13) || deferred.contains(&14), "duplicates should trigger pruning");
        assert!(deferred.contains(&15), "very low content low-priority task");

        // Safety: high priority real work must survive
        assert!(!deferred.contains(&11), "high priority tasks must never be auto-pruned");
    }

    #[tokio::test]
    async fn test_32712_pruning_in_evolution_cycle() {
        let dir = tempdir().unwrap();
        let adapter = TaskListAdapter::new(dir.path().to_path_buf(), false);

        let mut list = TaskList::default();
        list.tasks.push(Task {
            id: 99,
            title: "TODO: remove this old thing".to_string(),
            details: "old".to_string(),
            description: "".to_string(),
            status: "pending".to_string(),
            priority: "low".to_string(),
            ..Default::default()
        });

        adapter.save(&list).await.unwrap();

        let engine = TaskEvolutionEngine::new(adapter);
        let (proposals, _) = engine.run_evolution_cycle(false, None).await.unwrap();

        let has_deferral = proposals.iter().any(|p| {
            if let TaskMutation::SetStatus { new_status, .. } = p {
                new_status == "deferred"
            } else {
                false
            }
        });

        assert!(has_deferral, "Auto-pruning (327.12) should propose deferral for obsolete low-value tasks");
    }
}
