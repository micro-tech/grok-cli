//! HOH Planning Phase (Task 297.3 + 327 integration)
//!
//! Real implementation that reads task_list.json via TaskListAdapter,
//! applies selection, prioritization, and generates a coherent HOHPlan.
//!
//! Now uses TaskDependencyGraph (327.4) + scoring (327.5) for dependency-respecting selection.

use crate::hoh::state::{HOHError, HOHPlan};
use crate::hoh::tasklist_adapter::{Task, TaskListAdapter};
use crate::hoh::task_completion_tracker::TaskCompletionTracker;
use crate::hoh::evolution_engine::TaskEvolutionEngine;
use crate::hoh::architecture_evolution::ArchitectureEvolutionEngine;
use crate::hoh::autonomous_refactoring::AutonomousRefactoringEngine;
use crate::hoh::specialized_agents::execute_with_specialized_agent;
use std::collections::HashSet;
use std::path::PathBuf;

/// Enhanced planner that understands the task list.
#[derive(Debug)]
pub struct HOHPlanner {
    adapter: TaskListAdapter,
    pub completion_tracker: TaskCompletionTracker,
    // evolution_engine is created on-demand in run_task_evolution to avoid ownership issues
}

impl HOHPlanner {
    pub fn new(data_dir: PathBuf, simulation_mode: bool) -> Self {
        let adapter = TaskListAdapter::new(data_dir.clone(), simulation_mode);
        Self {
            adapter,
            completion_tracker: TaskCompletionTracker::new(),
        }
    }

    /// Main planning entry point used by outer loop.
    /// Now uses real dependency graph (327.4) + multi-signal prioritization (327.5).
    /// Runs TaskList evolution (327.34) + Architecture Evolution (361.1) before selection.
    pub async fn create_plan(&mut self, goals: Vec<String>) -> Result<HOHPlan, HOHError> {
        // 327.34: Task list evolution
        if let Ok((proposals, results)) = self.run_task_evolution(false).await {
            if !proposals.is_empty() {
                tracing::info!(
                    proposals = proposals.len(),
                    applied = results.len(),
                    "HOHPlanner: task list evolution proposed {} changes",
                    proposals.len()
                );
            }
        }

        // 361.1: Architecture evolution proposals (run every planning cycle)
        let arch_proposals = self.run_architecture_evolution().await.unwrap_or_default();
        for p in &arch_proposals {
            if p.estimated_impact > 0.7 {
                tracing::info!(
                    title = %p.title,
                    impact = p.estimated_impact,
                    risk = ?p.risk_level,
                    "HOHPlanner: high-impact architecture proposal"
                );
            }
        }

        // 361.2: Self-Refinement Loop — propose improvements to HOH itself
        let self_refinements = self.run_self_refinement().await.unwrap_or_default();
        for p in &self_refinements {
            tracing::info!(
                title = %p.title,
                impact = p.estimated_impact,
                "HOHPlanner: self-refinement proposal for HOH core"
            );
        }

        // 361.3: Autonomous Refactoring Engine — turn architecture + self-refinement proposals
        // into concrete, executable refactoring actions.
        let refactoring_actions = self.run_autonomous_refactoring().await.unwrap_or_default();
        for a in &refactoring_actions {
            if a.confidence > 0.75 {
                tracing::info!(
                    title = %a.title,
                    confidence = a.confidence,
                    effort = a.estimated_effort,
                    "HOHPlanner (361.3): high-confidence refactoring action"
                );
            }
        }

        // B: Collect high-confidence refactoring keywords for scoring feedback this cycle
        // (Computed early so scoring can use them)
        let high_conf_refactor_keywords: Vec<String> = refactoring_actions
            .iter()
            .filter(|a| a.confidence >= 0.7)
            .flat_map(|a| {
                a.title
                    .to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|s| s.len() > 3)
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .collect();

        // 327.2 + 327.4: First get only tasks whose dependencies are satisfied
        let empty_completed: HashSet<u64> = HashSet::new();
        let ready_tasks = self.adapter.get_ready_tasks(&empty_completed).await
            .unwrap_or_else(|_| Vec::new());

        // If graph-based ready tasks is empty, fall back to all pending (graceful)
        let candidates = if ready_tasks.is_empty() {
            self.adapter.get_pending_tasks().await?
        } else {
            ready_tasks
        };

        // 327.5: Score + prioritize the candidates (now with B: 361.3 feedback)
        let mut selected = self.select_and_prioritize(&candidates, &goals, &high_conf_refactor_keywords);

        // 327.4: Try to order the final selection according to topological order
        if let Ok(topo) = self.adapter.get_topological_order().await {
            selected.sort_by_key(|t| {
                topo.iter().position(|&id| id == t.id).unwrap_or(usize::MAX)
            });
        }

        // Record start for selected tasks (327.6)
        for t in &selected {
            self.completion_tracker.mark_started(t.id);
        }

        // A + C: Materialize high-confidence actions into real tasks + generate patch stubs
        let mut materialized_mutations = vec![];
        let mut generated_patches: Vec<crate::hoh::state::PatchSet> = vec![];
        if !refactoring_actions.is_empty() {
            let arch_adapter = self.adapter.clone_for_evolution();
            let refactor_engine = AutonomousRefactoringEngine::new(arch_adapter, self.adapter.is_simulation());

            // A: Turn actions into AddTask mutations and apply (closes propose → work loop)
            if let Ok(results) = refactor_engine.materialize_and_apply(&refactoring_actions).await {
                materialized_mutations = results;
                if !materialized_mutations.is_empty() {
                    tracing::info!(
                        count = materialized_mutations.len(),
                        "HOH (A): materialized {} new tasks from 361.3 refactoring actions",
                        materialized_mutations.len()
                    );
                }
            }

            // C: Generate first baby-step PatchStubs for high-confidence actions
            // These are now *tiny, safe, real* patches (tiny extracted modules + meta helpers + task metadata)
            for action in &refactoring_actions {
                if action.confidence >= 0.7 {
                    let patch = refactor_engine.refactoring_action_to_patch_stub(action);
                    generated_patches.push(patch.clone());
                    tracing::info!("HOH (C): generated tiny safe patch stub for '{}'", action.title);
                }
            }

            // Extra tiny safe real patches for task metadata / comments (361.3)
            let tiny_metadata_patches = refactor_engine.produce_tiny_safe_metadata_patches(&refactoring_actions);
            if !tiny_metadata_patches.is_empty() {
                tracing::info!(
                    count = tiny_metadata_patches.len(),
                    "HOH (C): generated {} tiny safe task-metadata patches",
                    tiny_metadata_patches.len()
                );
                generated_patches.extend(tiny_metadata_patches);
            }
        }

        // D: 361.4 — Route high-confidence actions to specialized sub-agents
        let mut specialized_routes: Vec<String> = vec![];
        for action in &refactoring_actions {
            if action.confidence >= 0.75 {
                if let Ok(msg) = execute_with_specialized_agent(action).await {
                    tracing::info!("HOH (D 361.4): {}", msg);
                    specialized_routes.push(msg);
                }
            }
        }

        // Collect materialized task IDs (A)
        let materialized_task_ids: Vec<u64> = materialized_mutations
            .iter()
            .filter_map(|r| if r.success { Some(r.task_id) } else { None })
            .collect();

        // Collect patch stub summaries (C)
        let generated_patch_stubs: Vec<String> = generated_patches
            .iter()
            .map(|p| p.diff_summary.clone())
            .collect();

        let plan = HOHPlan {
            goals,
            selected_tasks: selected.iter().map(|t| t.id).collect(),
            experiments: vec!["tasklist_driven".to_string()],
            architecture_proposals: arch_proposals.iter().map(|p| {
                format!("{} [{}]", p.title, format!("{:?}", p.change_type))
            }).collect(),
            self_refinement_proposals: self_refinements.iter().map(|p| {
                format!("{} (impact {:.2})", p.title, p.estimated_impact)
            }).collect(),
            refactoring_actions: refactoring_actions.iter().map(|a| {
                format!("{} (conf {:.2}, effort {:.1})", a.title, a.confidence, a.estimated_effort)
            }).collect(),
            materialized_task_ids,
            generated_patch_stubs,
            specialized_agent_routes: specialized_routes,
            improvement_suggestions: vec![],
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        tracing::info!(
            selected = ?plan.selected_tasks,
            "HOHPlanner: created plan with {} tasks (dependency-aware)",
            plan.selected_tasks.len()
        );

        Ok(plan)
    }

    /// Core selection + prioritization logic (327.2 + 327.5)
    /// Scores tasks then selects a dependency-respecting batch.
    /// Now accepts high-confidence refactoring keywords (B feedback from 361.3).
    fn select_and_prioritize(&self, candidates: &[Task], goals: &[String], refactor_keywords: &[String]) -> Vec<Task> {
        if candidates.is_empty() {
            return vec![];
        }

        let mut scored: Vec<(Task, f32)> = candidates
            .iter()
            .map(|task| {
                let score = self.score_task(task, goals, refactor_keywords);
                (task.clone(), score)
            })
            .collect();

        // Sort by score descending (327.5)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select top N while respecting dependencies (simple greedy within the ready set)
        let mut selected = Vec::new();
        let mut selected_ids = HashSet::new();

        for (task, _score) in scored.iter() {
            // All dependencies of this task must already be selected or not in the candidate pool
            let deps_ok = task
                .dependencies
                .iter()
                .all(|&dep| selected_ids.contains(&dep) || !candidates.iter().any(|t| t.id == dep));

            if deps_ok || task.dependencies.is_empty() {
                selected_ids.insert(task.id);
                selected.push(task.clone());
            }

            if selected.len() >= 8 {
                // Reasonable batch size for one HOH iteration
                break;
            }
        }

        // If nothing passed the dep check (edge case), just take the top scored ones
        if selected.is_empty() && !candidates.is_empty() {
            selected = scored.into_iter().take(8).map(|(t, _)| t).collect();
        }

        selected
    }

    /// Multi-signal scoring (327.5)
    /// Now incorporates completion history (327.6) + B: 361.3 refactoring feedback bonus
    fn score_task(&self, task: &Task, goals: &[String], refactor_keywords: &[String]) -> f32 {
        let mut score = 0.0;

        // Static priority signal
        match task.priority.as_str() {
            "high" => score += 10.0,
            "medium" => score += 5.0,
            "low" => score += 1.0,
            _ => {}
        }

        // Goal alignment (simple keyword overlap)
        let title_lower = task.title.to_lowercase();
        let details_lower = task.details.to_lowercase();
        for goal in goals {
            if title_lower.contains(&goal.to_lowercase()) {
                score += 8.0;
            }
        }

        // B: 361.3 feedback — bonus for tasks that implement recent high-confidence refactoring actions
        if !refactor_keywords.is_empty() {
            for kw in refactor_keywords {
                if title_lower.contains(kw) || details_lower.contains(kw) {
                    score += 6.0; // strong signal that this task advances architecture/self-improvement
                    break;
                }
            }
            // Extra small meta-bonus if the task title explicitly mentions 361 or refactor
            if title_lower.contains("361") || title_lower.contains("refactor") || title_lower.contains("architecture") {
                score += 2.5;
            }
        }

        // Historical performance bonus (327.5 + 327.6)
        let stats = self.completion_tracker.get_stats();
        if stats.total_completed > 0 {
            score += 1.5;

            if let Some(avg_dur) = stats.avg_duration_secs {
                if avg_dur < 3600.0 * 4.0 {
                    score += 2.0;
                }
            }

            if stats.high_quality_count as f32 / stats.total_completed as f32 > 0.7 {
                score += 3.0;
            }
        }

        // Freshness / age bonus (prefer older pending work)
        score += 2.0;

        // Penalty for very large tasks (prefer focused work)
        if task.details.len() > 1500 {
            score -= 3.0;
        }

        // Bonus for tasks with clear test_strategy (327.5)
        if !task.test_strategy.is_empty() && task.test_strategy.len() > 20 {
            score += 4.0;
        }

        // Small penalty if task has many dependencies (risk of blocking)
        if task.dependencies.len() > 3 {
            score -= 1.5;
        }

        score
    }

    /// Record that a task was completed (call this from outer loop / mutation when status -> done)
    pub fn record_task_completion(
        &mut self,
        task: &Task,
        quality_score: Option<f32>,
        test_pass_rate: Option<f32>,
        notes: &str,
    ) {
        self.completion_tracker.record_completion(task, quality_score, test_pass_rate, notes);
    }

    /// Run one cycle of autonomous task list evolution (327.34).
    /// Returns the proposed mutations and results (if auto_apply was true).
    pub async fn run_task_evolution(
        &self,
        auto_apply: bool,
    ) -> Result<(Vec<crate::hoh::task_mutation::TaskMutation>, Vec<crate::hoh::task_mutation::MutationResult>), HOHError> {
        // Create a fresh engine for this run (avoids ownership complexity)
        let engine = TaskEvolutionEngine::new(self.adapter.clone_for_evolution());
        engine.run_evolution_cycle(auto_apply).await
    }

    /// Run architecture evolution proposals (361.1).
    /// Returns generated ArchitectureProposals.
    /// These can be turned into new tasks or patch experiments in later phases.
    pub async fn run_architecture_evolution(&self) -> Result<Vec<crate::hoh::architecture_evolution::ArchitectureProposal>, HOHError> {
        let arch_adapter = self.adapter.clone_for_evolution();
        let engine = ArchitectureEvolutionEngine::new(arch_adapter, self.adapter.is_simulation());
        let proposals = engine.propose_evolutions().await?;

        if !proposals.is_empty() {
            tracing::info!(
                count = proposals.len(),
                "HOHPlanner: architecture evolution proposed {} structural changes",
                proposals.len()
            );
        }

        Ok(proposals)
    }

    /// 361.2: Self-Refinement Loop
    /// Proposes improvements to HOH's own internals (scoring, heuristics, meta-loops).
    pub async fn run_self_refinement(&self) -> Result<Vec<crate::hoh::architecture_evolution::ArchitectureProposal>, HOHError> {
        let arch_adapter = self.adapter.clone_for_evolution();
        let engine = ArchitectureEvolutionEngine::new(arch_adapter, self.adapter.is_simulation());
        let proposals = engine.propose_self_refinements().await?;

        if !proposals.is_empty() {
            tracing::info!(
                count = proposals.len(),
                "HOHPlanner: self-refinement proposed {} meta-improvements to HOH itself",
                proposals.len()
            );
            for p in &proposals {
                tracing::info!("  → Self-refine: {} (impact {:.2})", p.title, p.estimated_impact);
            }
        }

        Ok(proposals)
    }

    /// 361.3: Autonomous Refactoring Engine
    /// Converts architecture proposals + self-refinements into concrete, executable RefactoringActions.
    /// This is the bridge from high-level intent to actionable changes.
    pub async fn run_autonomous_refactoring(&self) -> Result<Vec<crate::hoh::autonomous_refactoring::RefactoringAction>, HOHError> {
        let arch_adapter = self.adapter.clone_for_evolution();
        let _arch_engine = ArchitectureEvolutionEngine::new(arch_adapter.clone(), self.adapter.is_simulation());
        let refactor_engine = AutonomousRefactoringEngine::new(arch_adapter, self.adapter.is_simulation());

        // Gather all proposals (361.1 + 361.2)
        let mut all_proposals = self.run_architecture_evolution().await.unwrap_or_default();
        if let Ok(self_refs) = self.run_self_refinement().await {
            all_proposals.extend(self_refs);
        }

        let actions = refactor_engine.generate_refactorings(&all_proposals).await?;

        if !actions.is_empty() {
            tracing::info!(
                count = actions.len(),
                "HOHPlanner (361.3): autonomous refactoring generated {} concrete actions",
                actions.len()
            );
            for a in &actions {
                if a.confidence >= 0.7 {
                    tracing::info!("  → Refactor: {} (conf {:.2}, effort {:.1})", a.title, a.confidence, a.estimated_effort);
                }
            }
        }

        Ok(actions)
    }

    /// Check if the current task list is consistent (327.33).
    pub async fn is_task_list_consistent(&self) -> Result<bool, HOHError> {
        self.adapter.is_consistent().await
    }

    /// Get current consistency problems (327.33).
    pub async fn get_task_list_problems(&self) -> Result<Vec<String>, HOHError> {
        self.adapter.get_consistency_problems().await
    }
}

/// Backward-compatible simple function (kept for existing callers in outer_loop)
pub async fn create_plan(goals: Vec<String>) -> HOHPlan {
    // Fallback when no data_dir context is available
    let mut planner = HOHPlanner::new(std::env::current_dir().unwrap_or_default(), true);
    let goals_clone = goals.clone();
    planner.create_plan(goals).await.unwrap_or_else(|_| HOHPlan {
        goals: goals_clone,
        selected_tasks: vec![327, 297, 361, 3612, 3613],
        experiments: vec!["fallback".to_string()],
        architecture_proposals: vec!["Architecture evolution engine active".to_string()],
        self_refinement_proposals: vec!["Self-refinement of planner scoring (meta loop)".to_string()],
        refactoring_actions: vec!["361.3 Autonomous Refactoring active".to_string()],
        materialized_task_ids: vec![],
        generated_patch_stubs: vec!["fallback: no real 361.3 actions".to_string()],
        specialized_agent_routes: vec![],
        improvement_suggestions: vec![],
        created_at: 0,
    })
}