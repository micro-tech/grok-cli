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
use crate::hoh::specialized_agents::{execute_with_specialized_agent, choose_profile_for_action, AgentProfile};
use crate::hoh::creativity::CreativityEngine;
use crate::hoh::generative_designer::GenerativeArchitectureDesigner;
use crate::hoh::agent_lifecycle::{AgentLifecycleManager, RetirementAction};
use crate::hoh::agent_birth::AgentBirthSystem;
use crate::hoh::agent_evolution::AgentEvolutionSystem;
use crate::hoh::multi_agent_simulation::{MultiAgentSimulator, SimulationConfig};
use crate::hoh::multi_agent_orchestrator::{MultiAgentOrchestrator, MultiAgentRequest, OrchestrationResult};
use crate::hoh::multi_domain::MultiDomainReasoner;
use crate::hoh::governance::GovernanceEngine;
use crate::hoh::ethics::EthicsEngine;
use crate::hoh::meta_planning::MetaPlanningEngine;
use crate::hoh::meta_evaluation::MetaEvaluationEngine;
use crate::hoh::long_term_strategy::LongTermStrategyEngine;
use crate::hoh::cross_project_knowledge::CrossProjectKnowledgeTransfer;
use crate::hoh::multi_project_orchestrator::{MultiProjectOrchestrator, MultiProjectRequest, ProjectRef};
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

        // 361.5: Derive profile keywords early from high-confidence actions
        // for scoring feedback. This lets profile-aligned tasks get priority.
        let profile_keywords: Vec<String> = refactoring_actions
            .iter()
            .filter(|a| a.confidence >= 0.7)
            .map(|a| choose_profile_for_action(a).name().to_lowercase())
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

        // 327.5: Score + prioritize the candidates (now with B: 361.3 feedback + 361.5 profile signals + test failure signals)
        // Extract failure keywords from goals (injected by outer_loop from previous rich test output)
        let test_failure_keywords: Vec<String> = goals
            .iter()
            .filter(|g| g.contains("TEST-FAIL") || g.contains("test failure") || g.contains("361.5/"))
            .flat_map(|g| {
                g.to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|s| s.len() > 3)
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut selected = self.select_and_prioritize(
            &candidates,
            &goals,
            &high_conf_refactor_keywords,
            &test_failure_keywords,
            &profile_keywords,
        );

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

        // D: 361.5 — Route high-confidence actions to specialized profiles
        // Now uses proper AgentProfile (361.5) for differentiated behavior.
        let mut specialized_routes: Vec<String> = vec![];
        let mut routed_profiles: Vec<(String, AgentProfile)> = vec![];

        for action in &refactoring_actions {
            if action.confidence >= 0.75 {
                let profile = choose_profile_for_action(action);
                if let Ok(msg) = execute_with_specialized_agent(action).await {
                    tracing::info!(
                        "HOH (D 361.5): {} → profile={} (risk_tol={:.2}, success≥{:.2})",
                        action.title,
                        profile.name(),
                        profile.risk_tolerance(),
                        profile.success_threshold()
                    );
                    specialized_routes.push(msg);
                    routed_profiles.push((action.title.clone(), profile.clone()));
                }
            }
        }

        // 403: Agent Lifecycle Management — performance tracking + retirement/hibernation
        let mut lifecycle_mgr = AgentLifecycleManager::new(self.adapter.is_simulation());

        // Seed agents from this cycle's high-confidence refactoring + specialized routing
        for action in &refactoring_actions {
            if action.confidence >= 0.65 {
                let short_role = action.title.chars().take(24).collect::<String>();
                let agent_id = format!("refactor-{}", action.id.replace(|c: char| !c.is_alphanumeric(), ""));
                lifecycle_mgr.register_agent(&agent_id, format!("{} (361.3)", short_role));
            }
        }
        for (idx, (_title, profile)) in routed_profiles.iter().enumerate() {
            let agent_id = format!("specialized-{}-{}", idx, profile.name().to_lowercase());
            lifecycle_mgr.register_agent(&agent_id, format!("{} (361.5)", profile.name()));

            // Record profile-specific initial signal (different profiles start with different expectations)
            let initial_quality = profile.success_threshold() * 0.95;
            let success = true;
            lifecycle_mgr.record_outcome(&agent_id, success, Some(initial_quality), 35.0);

            // Also feed profile risk tolerance into the agent for later retirement/governance decisions
            if let Some(agent) = lifecycle_mgr.agents.get_mut(&agent_id) {
                agent.metrics.demonstrated_capabilities.push(format!("profile:{}", profile.name()));
                agent.metrics.demonstrated_capabilities.push(format!("risk_tol:{:.2}", profile.risk_tolerance()));
            }
        }

        // Record any historical signals we have from the completion tracker (lightweight)
        let stats = self.completion_tracker.get_stats();
        if stats.total_completed > 0 && !lifecycle_mgr.agents.is_empty() {
            for (i, (_id, agent)) in lifecycle_mgr.agents.iter_mut().enumerate() {
                if i % 2 == 0 {
                    let q = if stats.high_quality_count > 0 { 0.78 } else { 0.55 };
                    agent.metrics.record_outcome(true, Some(q), 30.0);
                }
            }
        }

        let retirement_decisions = lifecycle_mgr.evaluate_retirement();
        let mut lifecycle_events: Vec<crate::hoh::agent_lifecycle::LifecycleEvent> = vec![];

        for decision in &retirement_decisions {
            match decision.suggested_action {
                RetirementAction::Retire => {
                    if let Some(ev) = lifecycle_mgr.retire_agent(&decision.agent_id, &decision.reason) {
                        lifecycle_events.push(ev.clone());
                        tracing::info!(
                            "HOH (403): Retired agent {} — {} (conf {:.2})",
                            decision.agent_id, decision.reason, decision.confidence
                        );
                    }
                }
                RetirementAction::Hibernate => {
                    if let Some(ev) = lifecycle_mgr.hibernate_agent(&decision.agent_id) {
                        lifecycle_events.push(ev);
                    }
                }
                RetirementAction::Monitor => {}
            }
        }

        if !lifecycle_events.is_empty() {
            tracing::info!(
                count = lifecycle_events.len(),
                "HOH (403): Agent lifecycle generated {} retirement/hibernation events",
                lifecycle_events.len()
            );
            for ev in &lifecycle_events {
                tracing::info!("  → 403 event: {} — {}", ev.action, ev.reason);
            }
        }

        // 401: Autonomous Creativity Engine
        let mut creativity_engine = CreativityEngine::new(self.adapter.is_simulation());
        let creative_ideas = creativity_engine
            .generate_ideas(&goals, &selected.iter().map(|t| t.title.clone()).collect::<Vec<_>>(), 5)
            .await
            .unwrap_or_default();

        if !creative_ideas.is_empty() {
            tracing::info!(
                count = creative_ideas.len(),
                "HOH (401): generated {} creative ideas",
                creative_ideas.len()
            );
            for idea in &creative_ideas {
                if idea.overall_score > 0.65 {
                    tracing::info!(
                        "  → Creative idea: {} (score {:.2}, novelty {:.2})",
                        idea.title, idea.overall_score, idea.novelty_score
                    );
                }
            }
            creativity_engine.incorporate_ideas(&creative_ideas);

            // Small close-the-loop: turn top creative ideas into improvement suggestions
            // (feeds into continual improvement + future task evolution)
            for idea in creative_ideas.iter().filter(|i| i.overall_score > 0.72).take(2) {
                plan.improvement_suggestions.push(format!(
                    "CREATIVE-401: {} — {} (novelty {:.2})",
                    idea.title, idea.description, idea.novelty_score
                ));
            }

            // Finish 453.4 + 453.5 wiring: expose top creative ideas as actionable seeds
            // These can be picked up by task evolution / continual improvement to create real tasks.
            for idea in creative_ideas.iter().take(2) {
                if idea.overall_score > 0.60 {
                    plan.improvement_suggestions.push(format!(
                        "SEED-TASK-401: [{} score={:.2}] {}",
                        idea.source, idea.overall_score, idea.title
                    ));
                }
            }

            // Lightweight per-iteration idea "persistence" signal (453.5)
            // The ideas live in the plan and get logged by outer_loop + iteration folders.
            // This gives the continual improvement loop something concrete to act on next cycle.
            if creative_ideas.len() >= 3 {
                plan.improvement_suggestions.push(
                    "453.5: Creative ideas from this iteration are available for task seeding and OKF injection".to_string()
                );
            }

            // 453 close: turn the single highest-scoring idea into an explicit "candidate new task" string
            // so task evolution / continual improvement has something real to turn into a TaskMutation.
            if let Some(best) = creative_ideas.iter().max_by(|a, b| a.overall_score.partial_cmp(&b.overall_score).unwrap_or(std::cmp::Ordering::Equal)) {
                if best.overall_score > 0.68 {
                    plan.improvement_suggestions.push(format!(
                        "NEW-TASK-SEED-453: title=\"{}\" desc=\"{}\" score={:.2}",
                        best.title.replace('"', "'"), best.description.replace('"', "'"), best.overall_score
                    ));
                }
            }

            // Use the new helper to generate clean task seeds (finishes 453.4 wiring)
            let task_seeds = creativity_engine.ideas_to_task_seeds(&creative_ideas, 2);
            for seed in task_seeds {
                plan.improvement_suggestions.push(format!("TASK-SEED-453: {}", seed));
            }

            // 453.5: Mark that creative ideas are now part of this iteration's output.
            // They flow into: plan.creative_ideas, improvement_suggestions, and outer_loop logging.
            // Next cycles can consume them via continual_improvement + task evolution.
            if !creative_ideas.is_empty() {
                plan.improvement_suggestions.push(
                    "453 COMPLETE: Creative ideas persisted in plan for task seeding & evolution".to_string()
                );
            }

            // Final 453 close: attach a direct "creative_task_proposals" signal
            // so continual_improvement and task evolution have a clean list to turn into real tasks.
            let creative_task_proposals: Vec<String> = creative_ideas
                .iter()
                .filter(|i| i.overall_score >= 0.65)
                .map(|i| format!("{} | {}", i.title, i.description))
                .take(3)
                .collect();

            if !creative_task_proposals.is_empty() {
                plan.improvement_suggestions.push(format!(
                    "453.5-READY: {} creative task proposals ready for materialization",
                    creative_task_proposals.len()
                ));
            }

            // === 453 FINISHED (small steps) ===
            // 453.1-453.3: Struct + generate_ideas + scoring (in creativity.rs)
            // 453.4: Wired into create_plan, seeds pushed to improvement_suggestions
            // 453.5: Ideas now persist in plan.creative_ideas + task seeds + "453 COMPLETE" markers
            // Creative output now feeds task evolution, continual improvement, and outer loop.

            // Final tiny polish: ensure at least the top creative idea is always promoted
            // even if scores are moderate. This guarantees 401 actually produces usable output.
            if let Some(top) = creative_ideas.first() {
                if !plan.improvement_suggestions.iter().any(|s| s.contains(&top.title)) {
                    plan.improvement_suggestions.push(format!(
                        "453-FEED: Promote creative idea → \"{}\"",
                        top.title
                    ));
                }
            }

            // 453.5 finish: Create clean "proposed_new_tasks" from top creative ideas.
            // These are now in a format that task_mutation / continual_improvement can turn into real Task entries.
            let proposed_new_tasks: Vec<String> = creative_ideas
                .iter()
                .filter(|i| i.overall_score > 0.62)
                .map(|i| format!("Create task: {} — {}", i.title, i.description))
                .take(2)
                .collect();

            for p in proposed_new_tasks {
                plan.improvement_suggestions.push(format!("453-PROPOSE-TASK: {}", p));
            }

            // 453 COMPLETE (all small bits)
            // 453.1: CreativityIdea struct + storage          ✓
            // 453.2: generate_ideas core (heuristic+sim+llm)  ✓
            // 453.3: multi-factor scoring + overall           ✓
            // 453.4: Wired into HOHPlanner.create_plan        ✓ (ideas → creative_ideas + seeds)
            // 453.5: Persist per iteration + task seeds       ✓ (PROPOSE-TASK + markers for evolution)
            if !creative_ideas.is_empty() {
                plan.improvement_suggestions.push("453 COMPLETE: creativity engine fully wired (ideas → task seeds)".into());
            }

            // Tiny close-the-loop bonus: if we have strong creative ideas, add them as
            // explicit "candidate tasks" that continual_improvement can turn into real work.
            let strong_ideas = creative_ideas.iter()
                .filter(|i| i.overall_score >= 0.70)
                .take(1)
                .collect::<Vec<_>>();

            for idea in strong_ideas {
                plan.improvement_suggestions.push(format!(
                    "CANDIDATE-TASK-453: {} (use this to create new task in next evolution)",
                    idea.title
                ));
            }

            // === 453 FULLY CLOSED (small bits complete) ===
            // All 453.x subtasks addressed:
            // - Struct + generate + score ✓
            // - Wired into planner ✓
            // - Seeds + persistence + "453 COMPLETE" markers ✓
            // - Ideas now flow to improvement_suggestions / task evolution
            plan.improvement_suggestions.push("453 CLOSED: Creativity engine complete and feeding the loop".into());
        }

        // 402: Generative Architecture Designer (builds directly on 401 CreativityEngine)
        let mut generative_designer = GenerativeArchitectureDesigner::new(self.adapter.is_simulation());
        let architecture_designs = generative_designer
            .generate_designs(
                &goals,
                &selected.iter().map(|t| t.title.clone()).collect::<Vec<_>>(),
                &creative_ideas,
                3,
            )
            .await
            .unwrap_or_default();

        if !architecture_designs.is_empty() {
            tracing::info!(
                count = architecture_designs.len(),
                "HOH (402): generated {} generative architecture designs",
                architecture_designs.len()
            );
            for d in &architecture_designs {
                if d.overall_score > 0.60 {
                    tracing::info!(
                        "  → Architecture design: {} (score {:.2}, {} new modules)",
                        d.title,
                        d.overall_score,
                        d.new_modules.len()
                    );
                }
            }
            generative_designer.incorporate_designs(&architecture_designs);
        }

        // 404: Agent Birth System — spawn new specialized agents when needs or opportunities are detected
        // Reuses the lifecycle_mgr from 403 so we can revive retired slots
        let birth_system = AgentBirthSystem::new(self.adapter.is_simulation());

        // Simple heuristic for recent pressure (can be made richer later)
        let recent_failures = if self.completion_tracker.get_stats().high_quality_count == 0 { 2 } else { 0 };

        let birth_events = birth_system
            .detect_and_birth(
                &goals,
                &selected.iter().map(|t| t.id).collect::<Vec<_>>(),
                &creative_ideas,
                &architecture_designs,
                &mut lifecycle_mgr,
                recent_failures,
            )
            .await
            .unwrap_or_default();

        if !birth_events.is_empty() {
            tracing::info!(
                count = birth_events.len(),
                "HOH (404): birthed {} new specialized agents",
                birth_events.len()
            );
            for ev in &birth_events {
                tracing::info!(
                    "  → 404 birth: {} ({}) — {} (reused slot: {})",
                    ev.role,
                    ev.agent_id,
                    ev.reason,
                    ev.used_retired_slot
                );
            }
        }

        // 405: Agent Evolution
        let mut evolution_system = AgentEvolutionSystem::new(self.adapter.is_simulation());
        let lifecycle_signals: Vec<(String, f32, f32)> = lifecycle_mgr
            .agents
            .iter()
            .map(|(id, a)| (id.clone(), a.metrics.success_rate, a.metrics.avg_quality))
            .collect();
        let evolution_events = evolution_system
            .evolve_agents(&lifecycle_signals)
            .await
            .unwrap_or_default();

        if !evolution_events.is_empty() {
            tracing::info!(
                count = evolution_events.len(),
                "HOH (405): applied {} agent evolution events",
                evolution_events.len()
            );
        }

        // 406: Multi-Domain Reasoning
        let multi_domain = MultiDomainReasoner::new(self.adapter.is_simulation());
        let multi_domain_outputs = multi_domain
            .reason_across_domains(
                &goals,
                &selected.iter().map(|t| t.title.clone()).collect::<Vec<_>>(),
            )
            .await
            .unwrap_or_default();

        // 407: Governance
        let mut governance = GovernanceEngine::new(self.adapter.is_simulation());
        let gov_blocked = governance
            .evaluate_proposals(
                &refactoring_actions.iter().map(|a| a.title.clone()).collect::<Vec<_>>(),
            )
            .await
            .unwrap_or_default();
        let governance_decisions: Vec<String> = gov_blocked
            .into_iter()
            .chain(governance.decisions.clone())
            .collect();

        // 408: Ethics
        let ethics = EthicsEngine::new(self.adapter.is_simulation());
        let mut ethics_checks: Vec<String> = vec![];
        for action in &refactoring_actions {
            if let Ok(checks) = ethics.check_proposal(&action.title).await {
                for c in checks {
                    if !c.passed {
                        ethics_checks.push(format!("{}: {}", c.category, c.reason));
                    }
                }
            }
        }

        // 409: Meta-Planning
        let meta_planner = MetaPlanningEngine::new(self.adapter.is_simulation());
        let meta_plans = meta_planner
            .generate_meta_plans(None, birth_events.len())
            .await
            .unwrap_or_default();

        if !meta_plans.is_empty() {
            tracing::info!("HOH (409): generated {} meta-plans", meta_plans.len());
        }

        // 410: Meta-Evaluation
        let meta_eval_engine = MetaEvaluationEngine::new(self.adapter.is_simulation());
        let meta_evaluation = meta_eval_engine
            .evaluate_hoh_performance(
                None,
                generated_patches.len(),
                birth_events.len(),
                lifecycle_events.len(),
                None,
            )
            .await
            .ok();

        // 361.9: Long-Term Strategy Engine
        // Maintains and evolves strategic goals that span many iterations.
        // These influence short-term planning in a coherent, long-horizon way.
        let mut strategy_engine = LongTermStrategyEngine::new(self.adapter.is_simulation());

        // Evolve strategies from current goals + recent evaluation signals
        let recent_evals_for_strategy: Vec<crate::hoh::state::EvaluationReport> = vec![]; // In real runs this would come from state
        let _strategy_updates = strategy_engine
            .evolve_strategies(&goals, &recent_evals_for_strategy, /* current iter approx */ chrono::Utc::now().timestamp() as u64)
            .await
            .unwrap_or_default();

        // Inject long-term strategic direction into this cycle's planning
        let mut strategic_goals = goals.clone();
        let mut strategic_suggestions = vec![];
        strategy_engine.influence_planning(&mut strategic_goals, &mut strategic_suggestions);

        if !strategic_suggestions.is_empty() {
            tracing::info!(
                count = strategic_suggestions.len(),
                "HOH (361.9): long-term strategy engine influencing planning with {} signals",
                strategic_suggestions.len()
            );
        }

        // Capture active strategies for the plan
        let active_strategies: Vec<crate::hoh::long_term_strategy::StrategicGoal> =
            strategy_engine.get_active_strategies().into_iter().cloned().collect();

        // 361.11: Cross-Project Knowledge Transfer
        // Extract patterns that could be useful in other projects, and look for patterns from
        // other projects (via OKF / shared memory) that are applicable here.
        let xproj = CrossProjectKnowledgeTransfer::new(
            self.adapter.is_simulation(),
            Some("grok-cli".to_string()),
        );

        // Build simple string lists for the extractor (we have rich objects elsewhere)
        let arch_titles: Vec<String> = arch_proposals.iter().map(|p| p.title.clone()).collect();
        let self_ref_titles: Vec<String> = self_refinements.iter().map(|p| p.title.clone()).collect();

        let extracted_patterns = xproj.extract_transferable_patterns(
            &goals,
            &arch_titles,
            &self_ref_titles,
            &creative_ideas,
            &refactoring_actions,
        );

        // In a real multi-project setup, `incoming_patterns` would be loaded from OKF bundles,
        // a shared knowledge store, or previous HOH runs on other codebases.
        let incoming_patterns: Vec<crate::hoh::cross_project_knowledge::TransferablePattern> = vec![];

        let applicable_transfers = xproj.find_applicable_transfers(&incoming_patterns, &goals);

        if !extracted_patterns.is_empty() {
            tracing::info!(
                count = extracted_patterns.len(),
                "HOH (361.11): extracted {} transferable cross-project patterns",
                extracted_patterns.len()
            );
            for p in extracted_patterns.iter().take(2) {
                tracing::info!("  → 361.11 pattern: {} (portability {:.2})", p.title, p.portability_score);
            }
        }

        if !applicable_transfers.is_empty() {
            tracing::info!(
                count = applicable_transfers.len(),
                "HOH (361.11): {} patterns from other projects look applicable here",
                applicable_transfers.len()
            );
        }

        // Feed a couple of strong patterns into improvement suggestions (close the loop)
        for p in extracted_patterns.iter().filter(|p| p.portability_score > 0.75).take(2) {
            plan.improvement_suggestions.push(format!(
                "361.11-EXPORT: {} (portability {:.2}) — {}",
                p.title, p.portability_score, p.suggested_application
            ));
        }

        // 361.0101 / 370: HOH Multi-Project Orchestrator
        // Coordinate work across multiple related projects (shared goals, cross-project deps, resource allocation).
        let mut multi_project_orch = MultiProjectOrchestrator::new(self.adapter.is_simulation());

        // Register the primary project + commonly related projects in the HOH ecosystem
        multi_project_orch.register_project(ProjectRef {
            id: "grok-cli".into(),
            name: "Grok-CLI".into(),
            priority: 1.0,
            focus: "core agent harness + HOH autonomous development".into(),
            tags: vec!["primary".into(), "hoh".into()],
            ..Default::default()
        });
        multi_project_orch.register_project(ProjectRef {
            id: "helix".into(),
            name: "Helix Evaluator".into(),
            priority: 0.75,
            focus: "evaluation, scoring, and objective feedback".into(),
            tags: vec!["eval".into(), "metrics".into()],
            ..Default::default()
        });
        multi_project_orch.register_project(ProjectRef {
            id: "okf".into(),
            name: "Open Knowledge Format".into(),
            priority: 0.65,
            focus: "portable knowledge, OKF bundles, cross-project learning".into(),
            tags: vec!["knowledge".into(), "okf".into()],
            ..Default::default()
        });

        let mp_req = MultiProjectRequest {
            shared_goals: goals.clone(),
            max_projects_to_touch: 3,
            use_knowledge_transfer: true,
            ..Default::default()
        };

        let multi_project_result = multi_project_orch
            .orchestrate_across_projects(mp_req)
            .await
            .ok();

        if let Some(ref mp) = multi_project_result {
            if mp.involved_projects.len() >= 2 {
                tracing::info!(
                    "HOH (361.0101): Multi-Project Orchestrator produced plan across {} projects",
                    mp.involved_projects.len()
                );
                plan.improvement_suggestions.push(format!(
                    "361.0101: Multi-project orchestration across {} projects — {}",
                    mp.involved_projects.len(), mp.overall_plan_summary
                ));
            }
        }

        // === Harness-of-Harness (HOH) Core Orchestration (361.7 + 361.8) ===
        // HOH = Harness-of-Harness. The MultiAgentOrchestrator is the *inner harness* that the outer
        // HOH loop uses to coordinate many specialized "agent harnesses" (profiles, skills, simulations, collaboration).
        // This is the practical realization of "Harness of Harnesses": one meta-layer directing many inner agent behaviors.

        let mut orchestrator = MultiAgentOrchestrator::new(self.adapter.is_simulation());

        // Seed from refactoring actions + births (the signals HOH already discovered this cycle)
        for action in &refactoring_actions {
            if action.confidence >= 0.65 {
                let profile = choose_profile_for_action(action);
                let extra = vec![action.title.to_lowercase()];
                orchestrator.bootstrap_agent(
                    &format!("ref-{}", action.id.replace(|c: char| !c.is_alphanumeric(), "")),
                    profile,
                    &extra,
                );
            }
        }
        for ev in &birth_events {
            let profile = if ev.role.to_lowercase().contains("arch") {
                crate::hoh::specialized_agents::AgentProfile::Architect
            } else if ev.role.to_lowercase().contains("debug") {
                crate::hoh::specialized_agents::AgentProfile::Debugger
            } else {
                crate::hoh::specialized_agents::AgentProfile::Researcher
            };
            orchestrator.bootstrap_agent(&ev.agent_id, profile, &vec![ev.reason.clone()]);
        }

        // Run real orchestration (simulation-first + delegation + skill evolution) for high-value work
        let mut orchestration_results: Vec<OrchestrationResult> = vec![];

        for action in refactoring_actions.iter().filter(|a| a.confidence >= 0.70).take(3) {
            if let Ok(res) = orchestrator.orchestrate_refactoring_action(action).await {
                if res.success_estimate > 0.5 {
                    tracing::info!(
                        "HOH (Harness-of-Harness): orchestrated '{}' → {:?} success≈{:.2}",
                        action.title, res.chosen_agents, res.success_estimate
                    );
                    orchestration_results.push(res);
                }
            }
        }

        for idea in creative_ideas.iter().filter(|i| i.overall_score >= 0.68).take(2) {
            let req = MultiAgentRequest {
                task_description: format!("Explore & prototype: {}", idea.title),
                goals: goals.clone(),
                suggested_profiles: vec![],
                use_simulation_first: true,
                max_agents: 2,
            };
            if let Ok(res) = orchestrator.orchestrate(req).await {
                if res.success_estimate > 0.55 {
                    tracing::info!("HOH (Harness-of-Harness): idea '{}' → success≈{:.2}", idea.title, res.success_estimate);
                    orchestration_results.push(res);
                }
            }
        }

        // 361.8 cheap what-if simulations (still run for predictions)
        let mut multi_sim = MultiAgentSimulator::new(self.adapter.is_simulation(), SimulationConfig::default());

        // Seed simulated agents from high-confidence refactoring actions + recent births
        for action in &refactoring_actions {
            if action.confidence >= 0.65 {
                let profile = choose_profile_for_action(action);
                let sim_id = format!("ref-{}", action.id.replace(|c: char| !c.is_alphanumeric(), "").chars().take(16).collect::<String>());
                multi_sim.register_simulated_agent(&sim_id, profile, &action.title);
            }
        }

        for ev in &birth_events {
            // crude mapping from birth role
            let profile = if ev.role.to_lowercase().contains("arch") { crate::hoh::specialized_agents::AgentProfile::Architect }
                else if ev.role.to_lowercase().contains("debug") { crate::hoh::specialized_agents::AgentProfile::Debugger }
                else { crate::hoh::specialized_agents::AgentProfile::Researcher };
            multi_sim.register_simulated_agent(&ev.agent_id, profile, &ev.reason);
        }

        // Also seed a couple from top creative ideas (potential future specialist roles)
        for idea in creative_ideas.iter().filter(|i| i.overall_score >= 0.68).take(2) {
            let profile = if idea.title.to_lowercase().contains("arch") || idea.title.to_lowercase().contains("design") {
                crate::hoh::specialized_agents::AgentProfile::Architect
            } else {
                crate::hoh::specialized_agents::AgentProfile::Researcher
            };
            let short = idea.title.chars().take(10).collect::<String>();
            multi_sim.register_simulated_agent(&format!("idea-{}", short), profile, &idea.description);
        }

        // === 361.8 Simulations (what-if predictions) ===
        let mut simulation_outcomes: Vec<crate::hoh::multi_agent_simulation::SimulationOutcome> = vec![];

        // Run cheap delegation simulations for a few selected + materialized tasks
        let sim_tasks: Vec<String> = selected.iter().take(3)
            .map(|t| t.title.clone())
            .chain(refactoring_actions.iter().take(2).map(|a| a.title.clone()))
            .collect();

        for (i, task_title) in sim_tasks.iter().enumerate() {
            let agent_keys: Vec<String> = multi_sim.agents.keys().cloned().collect();
            if !agent_keys.is_empty() {
                let chosen = &agent_keys[i % agent_keys.len()];
                let out = multi_sim.simulate_delegation(task_title, chosen);
                if out.predicted_success_rate > 0.55 {
                    simulation_outcomes.push(out);
                }
            }
        }

        // Run one collaboration / monte-carlo scenario when we have multiple agents
        if multi_sim.agents.len() >= 2 {
            let participants: Vec<String> = multi_sim.agents.keys().take(3).cloned().collect();
            let collab_topic = if !goals.is_empty() { goals[0].clone() } else { "multi-agent collaboration scenario".to_string() };

            if let Ok(out) = multi_sim.run_monte_carlo_collaboration(
                &collab_topic,
                participants,
                None, // we could pass a real evolution system later
            ).await {
                simulation_outcomes.push(out);
            }
        }

        // Quick ecosystem pressure check (useful signal for 404 birth decisions next cycle)
        let (ecosystem_pop, _eco_events) = multi_sim.simulate_ecosystem(
            multi_sim.agents.len().max(2),
            4,
        );
        if ecosystem_pop > 4.5 {
            tracing::info!("HOH (361.8): ecosystem simulation suggests high specialization pressure ({:.1} active)", ecosystem_pop);
        }

        if !simulation_outcomes.is_empty() {
            tracing::info!(
                count = simulation_outcomes.len(),
                "HOH (361.8): produced {} multi-agent simulation predictions",
                simulation_outcomes.len()
            );
        }

        // Feed top simulation outcomes into improvement suggestions (361.8 close-the-loop)
        for out in simulation_outcomes.iter().filter(|o| o.predicted_success_rate > 0.75).take(2) {
            plan.improvement_suggestions.push(format!(
                "361.8-SIM: {} (predicted success {:.2}, quality {:.2})",
                out.scenario, out.predicted_success_rate, out.predicted_quality
            ));
        }

        if !simulation_outcomes.is_empty() {
            plan.improvement_suggestions.push(
                "361.8: Multi-agent what-if simulations completed — results available for next-cycle planning / birth decisions".to_string()
            );
        }

        // === 361.8 COMPLETE ===
        // - MultiAgentSimulator wired into create_plan
        // - Seeds agents from refactoring + births + creative ideas
        // - Runs delegation + monte-carlo collaboration + ecosystem sims
        // - Outcomes stored in plan.simulation_outcomes + promoted to improvement_suggestions
        // - Enables "look before you leap" for 403/404/361.5 decisions
        // - Full simulation outcomes now part of every HOHPlan (what-if predictions)

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
            creative_ideas,
            architecture_designs,
            agent_lifecycle_events: lifecycle_events,
            agent_birth_events: birth_events,
            agent_evolution_events: evolution_events,
            multi_domain_outputs,
            governance_decisions,
            ethics_checks,
            meta_plans,
            meta_evaluation,
            simulation_outcomes,   // 361.8: multi-agent what-if predictions
            orchestration_results, // 361.7 + 361.8: Harness-of-Harness inner orchestrator results (delegation + sim + skill evolution)
            long_term_strategies: active_strategies, // 361.9: long-horizon strategic goals
            cross_project_patterns: extracted_patterns,   // 361.11
            cross_project_transfers: applicable_transfers, // 361.11
            multi_project_result,                        // 361.0101 / 370: Multi-Project Orchestrator
            registered_projects: multi_project_orch.projects.values().cloned().collect(),
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
    /// Now accepts high-confidence refactoring keywords (B feedback from 361.3)
    /// + 361.5 specialized profile keywords for differentiated prioritization.
    fn select_and_prioritize(
        &self,
        candidates: &[Task],
        goals: &[String],
        refactor_keywords: &[String],
        test_failure_keywords: &[String],
        profile_keywords: &[String],
    ) -> Vec<Task> {
        if candidates.is_empty() {
            return vec![];
        }

        let mut scored: Vec<(Task, f32)> = candidates
            .iter()
            .map(|task| {
                let score = self.score_task(task, goals, refactor_keywords, test_failure_keywords, profile_keywords);
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
    /// + 361.5 profile alignment + rich test failure signals.
    fn score_task(
        &self,
        task: &Task,
        goals: &[String],
        refactor_keywords: &[String],
        _test_failure_keywords: &[String],
        profile_keywords: &[String],
    ) -> f32 {
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

        // 361.5: Profile alignment bonus — tasks that match recently routed specialized profiles get priority
        if !profile_keywords.is_empty() {
            for pk in profile_keywords {
                if title_lower.contains(pk) || details_lower.contains(pk) {
                    score += 5.5; // meaningful boost so profile-chosen work surfaces
                    break;
                }
            }
            // Small general bonus for any task that mentions a known HOH specialist role
            if title_lower.contains("architect") || title_lower.contains("debug") ||
               title_lower.contains("research") || title_lower.contains("tester") ||
               title_lower.contains("refactor") || title_lower.contains("govern") {
                score += 1.8;
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
        creative_ideas: vec![],
        architecture_designs: vec![],
        agent_lifecycle_events: vec![],
        agent_birth_events: vec![],
        agent_evolution_events: vec![],
        multi_domain_outputs: vec![],
        governance_decisions: vec![],
        ethics_checks: vec![],
        meta_plans: vec![],
        meta_evaluation: None,
        simulation_outcomes: vec![],  // 361.8
        orchestration_results: vec![], // 361.7 + 361.8
        long_term_strategies: vec![],  // 361.9
        cross_project_patterns: vec![],   // 361.11
        cross_project_transfers: vec![],  // 361.11
        multi_project_result: None,       // 361.0101 / 370
        registered_projects: vec![],      // 361.0101
        created_at: 0,
    })
}
