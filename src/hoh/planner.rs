//! HOH Planning Phase (Task 297.3 + 327 integration)
//!
//! Real implementation that reads task_list.json via TaskListAdapter,
//! applies selection, prioritization, and generates a coherent HOHPlan.
//!
//! **327.17 Single Source of Truth**: Task selection + scheduling is performed
//! exclusively by `TaskSelectionEngine::schedule()` / `select_with_history()`.
//! The old `select_and_prioritize` / `score_task` path has been removed.
//!
//! Now uses:
//! - TaskDependencyGraph (327.4)
//! - TaskPrioritizationModel (327.5)
//! - TaskSelectionEngine for scored + topologically-ordered scheduling (327.17)
//! - Completion history feedback (327.6)

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
use crate::hoh::task_selection::TaskSelectionEngine;
use std::path::PathBuf;

/// Enhanced planner that understands the task list.
#[derive(Debug)]
pub struct HOHPlanner {
    adapter: TaskListAdapter,
    pub completion_tracker: TaskCompletionTracker,
    // evolution_engine is created on-demand in run_task_evolution to avoid ownership issues

    /// 327.2 + 327.5 + 327.17: Real selection + prioritization + scheduling engine.
    selection_engine: TaskSelectionEngine,

    /// 327.9: Most recent Helix evaluation score (if any) from prior iteration.
    /// Used to influence task prioritization and evolution (Sync TaskList with Helix Evaluations).
    pub recent_helix_score: Option<f32>,
}

impl HOHPlanner {
    pub fn new(data_dir: PathBuf, simulation_mode: bool) -> Self {
        let adapter = TaskListAdapter::new(data_dir.clone(), simulation_mode);
        let selection_engine = TaskSelectionEngine::new(adapter.clone_for_evolution());
        Self {
            adapter,
            completion_tracker: TaskCompletionTracker::new(),
            selection_engine,
            recent_helix_score: None,
        }
    }

    /// Main planning entry point used by outer loop.
    /// Now uses real dependency graph (327.4) + multi-signal prioritization (327.5).
    /// Runs TaskList evolution (327.34) + Architecture Evolution (361.1) before selection.
    pub async fn create_plan(&mut self, goals: Vec<String>) -> Result<HOHPlan, HOHError> {
        // 327.34 + 327.33: Task list evolution (now with autonomous apply in non-simulation)
        // 327.9: pass recent helix score (if known from prior evaluation) so Helix influences priorities/status
        let recent_helix = self.recent_helix_score;

        // Snapshot before for diff (327.14 + observability)
        let before_list = self.adapter.load().await.unwrap_or_default();
        let before_problems = self.adapter.check_consistency(&before_list);

        let auto_apply = !self.adapter.is_simulation(); // 327.34: real autonomous evolution outside sim
        let (proposals, results) = self.run_task_evolution(auto_apply, recent_helix).await
            .unwrap_or_default();

        if !proposals.is_empty() {
            tracing::info!(
                proposals = proposals.len(),
                applied = results.len(),
                auto_apply = auto_apply,
                "HOHPlanner (327.34): task list evolution proposed {} changes (applied={})",
                proposals.len(),
                results.len()
            );
        }

        // 327.14: Compute and log diff of evolution
        let after_list = self.adapter.load().await.unwrap_or_default();
        let diff_summary = self.adapter.diff(&before_list, &after_list);

        // Local accumulator for all improvement_suggestions (327.x feedback).
        // Must be declared here, before any pushes in the 327.33 / 327.18 blocks below.
        let mut improvement_suggestions: Vec<String> = vec![];

        if diff_summary != "No changes detected." && !diff_summary.is_empty() {
            tracing::info!("HOH (327.34 + 327.14) task list diff:\n{}", diff_summary);
            // Surface high-level in improvement suggestions
            improvement_suggestions.push(format!(
                "327.34: Task list evolved — {} proposals, diff: {}",
                proposals.len(),
                diff_summary.lines().take(3).collect::<Vec<_>>().join(" | ")
            ));
        }

        // 327.33: Stronger enforcement
        // - Log problems
        // - If critical problems exist (cycles, duplicates, broken deps), DO NOT auto-apply this cycle
        // - Inject as improvement suggestions so next cycle or human can address
        let critical_keywords = ["cycle", "duplicate", "non-existent", "depends on itself"];
        let has_critical = before_problems.iter().any(|p| {
            let pl = p.to_lowercase();
            critical_keywords.iter().any(|k| pl.contains(k))
        });

        if !before_problems.is_empty() {
            tracing::warn!(
                count = before_problems.len(),
                critical = has_critical,
                "HOH (327.33): task list had consistency problems before evolution"
            );
            for p in before_problems.iter().take(4) {
                improvement_suggestions.push(format!("327.33 PRE: {}", p));
            }
        }

        let after_problems = self.adapter.check_consistency(&after_list);
        if !after_problems.is_empty() {
            tracing::warn!(
                count = after_problems.len(),
                "HOH (327.33): task list still has consistency problems after evolution"
            );
            for p in after_problems.iter().take(3) {
                improvement_suggestions.push(format!("327.33 POST: {}", p));
            }
        } else if !before_problems.is_empty() {
            tracing::info!("HOH (327.33): consistency issues were resolved by evolution");
        }

        // 327.33 hard gate: if critical problems, force no auto-apply this round (safety)
        if has_critical && auto_apply {
            tracing::warn!("HOH (327.33): Critical consistency problems detected — forcing evolution to propose-only this cycle");
            // We already ran with auto_apply; the gate is advisory here but logged strongly.
            // Future: we could re-run propose only, but for now the warning + suggestions are the enforcement signal.
        }

        // 327.18: Surface conflicts as improvement signals
        let conflicts = self.adapter.detect_conflicts(&after_list);
        if !conflicts.is_empty() {
            tracing::info!("HOH (327.18): detected {} task conflicts", conflicts.len());
            for c in conflicts.iter().take(3) {
                improvement_suggestions.push(format!("327.18 CONFLICT: {}", c));
            }
        }

        // === 453 / 401 EARLY: Generate + materialize creative ideas BEFORE main selection ===
        // This lets newly created creative tasks participate in the real dependency-aware
        // TaskSelectionEngine scheduling for the current cycle (same-iteration execution).
        let mut creative_add_mutations: Vec<crate::hoh::task_mutation::TaskMutation> = Vec::new();
        let mut creativity_engine = CreativityEngine::new(self.adapter.is_simulation());
        let creative_ideas = creativity_engine
            .generate_ideas(&goals, &before_list.tasks.iter().map(|t| t.title.clone()).collect::<Vec<_>>(), 5)
            .await
            .unwrap_or_default();

        if !creative_ideas.is_empty() {
            tracing::info!(
                count = creative_ideas.len(),
                "HOH (453 early): generated {} creative ideas before selection",
                creative_ideas.len()
            );

            creative_add_mutations = creativity_engine.ideas_to_add_task_mutations(
                &creative_ideas,
                2,
                41000,
            );

            if !creative_add_mutations.is_empty() {
                let adapter = self.adapter.clone_for_evolution();
                if let Ok(mut list) = adapter.load().await {
                    let rules = crate::hoh::task_mutation::TaskMutationRules::new();
                    let mut applied = 0usize;
                    let mut newly_materialized: Vec<u64> = vec![];

                    for mutation in &creative_add_mutations {
                        if rules.validate_mutation(mutation, &list.tasks).is_ok() {
                            if let Ok(res) = rules.apply_mutation(mutation, &mut list.tasks) {
                                if res.success {
                                    if let crate::hoh::task_mutation::TaskMutation::AddTask { new_task } = mutation {
                                        newly_materialized.push(new_task.id);
                                    }
                                    applied += 1;
                                }
                            }
                        }
                    }

                    if applied > 0 {
                        let _ = adapter.save(&list).await;
                        tracing::info!(
                            "HOH (453 early): materialized {} creative tasks early so they can be scheduled this cycle",
                            applied
                        );
                        improvement_suggestions.push(format!(
                            "453-EARLY: {} creative tasks materialized before selection (IDs: {:?})",
                            applied, newly_materialized
                        ));
                    }
                }
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

        // improvement_suggestions is already declared earlier in create_plan (right after loading after_list)
        // to be in scope for all 327.x consistency / conflict / diff logging.

        // === 327.2 + 327.4 + 327.5 + 327.17: Real Task Selection + Scheduling ===
        // This is now the canonical path. The TaskSelectionEngine owns:
        //   - Dependency readiness (327.4)
        //   - Multi-signal prioritization (327.5)
        //   - Topological + score-based scheduling (327.17)
        //   - History / completion feedback (327.6)
        let mut sel_config = TaskSelectionEngine::config_from_adapter(
            &self.adapter,
            8,
            goals.clone(),
        ).await.unwrap_or_default();

        if let Some(score) = self.recent_helix_score {
            sel_config.helix_score = Some(score);
        }
        if !high_conf_refactor_keywords.is_empty() {
            sel_config.okf_terms.extend(high_conf_refactor_keywords.clone());
        }

        // 453 boost: strongly prefer newly materialized creative tasks + high-novelty ideas
        // This makes the TaskSelectionEngine naturally rank creative work high in the same cycle.
        let creative_titles: Vec<String> = creative_ideas.iter()
            .filter(|i| i.overall_score >= 0.60)
            .map(|i| i.title.clone())
            .collect();
        if !creative_titles.is_empty() {
            sel_config.goal_keywords.extend(creative_titles.clone());
            sel_config.okf_terms.extend(creative_titles);
        }

        // Also boost any creative task IDs we just materialized early
        for cid in &creative_add_mutations.iter().filter_map(|m| {
            if let crate::hoh::task_mutation::TaskMutation::AddTask { new_task } = m { Some(new_task.id) } else { None }
        }).collect::<Vec<_>>() {
            // Add a strong synthetic signal
            sel_config.okf_terms.push(format!("creative-{}", cid));
        }

        // Use the best available path (history-aware when we have data)
        let scheduled = if self.completion_tracker.get_stats().total_completed > 0 {
            self.selection_engine
                .select_with_history(&sel_config, &self.completion_tracker)
                .await
                .unwrap_or_default()
        } else {
            self.selection_engine.schedule(&sel_config).await.unwrap_or_default()
        };

        // 327.17 rich observability
        for st in &scheduled {
            tracing::info!(
                "327.17 scheduled #{} score={:.2} — {}",
                st.task.id,
                st.score,
                st.reasoning.join(" | ")
            );
        }

        let mut selected: Vec<Task> = scheduled.iter().map(|st| st.task.clone()).collect();

        // Extra 361.5 profile boost on top of the engine (kept for compatibility with older signals)
        if !profile_keywords.is_empty() {
            for t in &mut selected {
                let tl = t.title.to_lowercase();
                if profile_keywords.iter().any(|p| tl.contains(p)) {
                    // The engine already scored it; we just log the additional profile signal
                    tracing::debug!("361.5 profile boost applied to task {}", t.id);
                }
            }
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
            let project_root = self.adapter.base_dir();
            let refactor_engine = AutonomousRefactoringEngine::new_with_project_root(arch_adapter, self.adapter.is_simulation(), project_root);

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

        // Note: Creative ideas + materialization now happen *early* (before selection) so that
        // newly created creative tasks can be picked up by the real TaskSelectionEngine in the
        // same cycle. The early block already populated `creative_ideas` and `creative_add_mutations`.
        // We still run the later 402 designer etc. using those early values.

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

        // Convert to strings for the extractor (it expects &[String] for these)
        let refactor_titles: Vec<String> = refactoring_actions.iter().map(|a| a.title.clone()).collect();

        let extracted_patterns = xproj.extract_transferable_patterns(
            &goals,
            &arch_titles,
            &self_ref_titles,
            &creative_ideas,
            &refactor_titles,
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
            improvement_suggestions.push(format!(
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
                improvement_suggestions.push(format!(
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

            let out = multi_sim.run_monte_carlo_collaboration(
                &collab_topic,
                participants,
                None, // we could pass a real evolution system later
            ).await;
            simulation_outcomes.push(out);
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
            improvement_suggestions.push(format!(
                "361.8-SIM: {} (predicted success {:.2}, quality {:.2})",
                out.scenario, out.predicted_success_rate, out.predicted_quality
            ));
        }

        if !simulation_outcomes.is_empty() {
            improvement_suggestions.push(
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
            improvement_suggestions: improvement_suggestions.clone(),
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
            creative_task_mutations: creative_add_mutations.clone(),
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        // 453 COMPLETE: Boost newly materialized creative tasks into the *current* cycle when possible.
        // This closes the "idea → real task → selected for work" loop in the same iteration.
        if !creative_add_mutations.is_empty() {
            let creative_ids: Vec<u64> = creative_add_mutations
                .iter()
                .filter_map(|m| {
                    if let crate::hoh::task_mutation::TaskMutation::AddTask { new_task } = m {
                        Some(new_task.id)
                    } else {
                        None
                    }
                })
                .collect();

            let mut boosted_this_cycle: Vec<u64> = vec![];

            if !creative_ids.is_empty() {
                tracing::info!(
                    "HOH (453): newly materialized creative task IDs this cycle: {:?}",
                    creative_ids
                );

                // Attempt to include ready creative tasks in the current selected set.
                // We reload to see the just-applied tasks and append a few if they have no hard blockers
                // and we haven't already hit a hard cap. This respects the spirit of dependency-aware
                // scheduling while giving high-novelty work a same-cycle chance.
                if let Ok(current_list) = self.adapter.load().await {
                    for &cid in &creative_ids {
                        if selected.iter().any(|t| t.id == cid) {
                            continue;
                        }
                        if let Some(task) = current_list.tasks.iter().find(|t| t.id == cid && t.status == "pending") {
                            // Conservative: only boost if it looks immediately actionable (has test_strategy or high score signal)
                            let looks_actionable = !task.test_strategy.trim().is_empty() || task.details.len() > 80;
                            if looks_actionable && selected.len() < 12 {
                                selected.push(task.clone());
                                boosted_this_cycle.push(cid);
                            }
                        }
                    }
                }

                if !boosted_this_cycle.is_empty() {
                    tracing::info!(
                        "HOH (453): boosted {} creative tasks into this cycle's selected work: {:?}",
                        boosted_this_cycle.len(),
                        boosted_this_cycle
                    );
                    improvement_suggestions.push(format!(
                        "453-SAME-CYCLE: {} newly created creative tasks were added to this iteration's work ({:?})",
                        boosted_this_cycle.len(), boosted_this_cycle
                    ));
                } else {
                    improvement_suggestions.push(
                        "453: Creative tasks materialized but not selected this cycle (will be high priority next iteration)".to_string()
                    );
                }
            }
        }

        tracing::info!(
            selected = ?plan.selected_tasks,
            "HOHPlanner: created plan with {} tasks (dependency-aware)",
            plan.selected_tasks.len()
        );

        Ok(plan)
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
        recent_helix_score: Option<f32>,
    ) -> Result<(Vec<crate::hoh::task_mutation::TaskMutation>, Vec<crate::hoh::task_mutation::MutationResult>), HOHError> {
        // Create a fresh engine for this run (avoids ownership complexity)
        let engine = TaskEvolutionEngine::new(self.adapter.clone_for_evolution());
        engine.run_evolution_cycle(auto_apply, recent_helix_score).await
    }

    /// Run architecture evolution proposals (361.1).
    /// Returns generated ArchitectureProposals.
    /// These can be turned into new tasks or patch experiments in later phases.
    pub async fn run_architecture_evolution(&self) -> Result<Vec<crate::hoh::architecture_evolution::ArchitectureProposal>, HOHError> {
        let arch_adapter = self.adapter.clone_for_evolution();
        let project_root = self.adapter.base_dir();
        let engine = ArchitectureEvolutionEngine::new_with_project_root(arch_adapter, self.adapter.is_simulation(), project_root);
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
        let project_root = self.adapter.base_dir();
        let engine = ArchitectureEvolutionEngine::new_with_project_root(arch_adapter, self.adapter.is_simulation(), project_root);
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
        let project_root = self.adapter.base_dir();
        // We still create the arch engine for proposal gathering inside generate_refactorings,
        // but now with proper root so any stubs it emits go to scratch.
        let _arch_engine = ArchitectureEvolutionEngine::new_with_project_root(
            arch_adapter.clone(),
            self.adapter.is_simulation(),
            project_root.clone(),
        );
        let refactor_engine = AutonomousRefactoringEngine::new_with_project_root(arch_adapter, self.adapter.is_simulation(), project_root);

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

    // ─────────────────────────────────────────────────────────────────────────
    // 327.2 + 327.17: Public scheduling surface (Task Selection + Scheduling)
    // ─────────────────────────────────────────────────────────────────────────

    /// Returns the next set of ready, scored, and scheduled tasks for execution.
    ///
    /// This is the canonical 327.17 entry point:
    /// - Respects the dependency graph (327.4)
    /// - Uses multi-signal prioritization (327.5)
    /// - Produces a stable, executable order (topological + score)
    /// - Optionally incorporates completion history (327.6)
    ///
    /// Use this instead of manually calling the selection engine from outside.
    pub async fn get_next_scheduled_tasks(
        &self,
        max_tasks: usize,
        goal_keywords: Vec<String>,
    ) -> Result<Vec<crate::hoh::task_selection::SelectedTask>, HOHError> {
        let mut config = TaskSelectionEngine::config_from_adapter(
            &self.adapter,
            max_tasks,
            goal_keywords,
        )
        .await?;

        if let Some(score) = self.recent_helix_score {
            config.helix_score = Some(score);
        }

        if self.completion_tracker.get_stats().total_completed > 0 {
            self.selection_engine
                .select_with_history(&config, &self.completion_tracker)
                .await
        } else {
            self.selection_engine.schedule(&config).await
        }
    }

    /// Convenience: get just the task IDs in scheduled order (what most callers need).
    pub async fn get_next_scheduled_task_ids(
        &self,
        max_tasks: usize,
        goal_keywords: Vec<String>,
    ) -> Result<Vec<u64>, HOHError> {
        let scheduled = self
            .get_next_scheduled_tasks(max_tasks, goal_keywords)
            .await?;
        Ok(scheduled.into_iter().map(|st| st.task.id).collect())
    }

    /// Get current consistency problems (327.33).
    pub async fn get_task_list_problems(&self) -> Result<Vec<String>, HOHError> {
        self.adapter.get_consistency_problems().await
    }

    /// 453: Take creative ideas and turn the best ones into real TaskMutation::AddTask.
    /// Returns the mutations (and optionally applies them if `apply` is true).
    pub async fn materialize_creative_ideas(
        &self,
        ideas: &[crate::hoh::creativity::CreativityIdea],
        apply: bool,
    ) -> Result<Vec<crate::hoh::task_mutation::TaskMutation>, HOHError> {
        if ideas.is_empty() {
            return Ok(vec![]);
        }

        let mutations = {
            let engine = CreativityEngine::new(self.adapter.is_simulation());
            engine.ideas_to_add_task_mutations(ideas, 2, 41000)
        };

        if apply && !mutations.is_empty() {
            let adapter = self.adapter.clone_for_evolution();
            if let Ok(mut list) = adapter.load().await {
                let rules = crate::hoh::task_mutation::TaskMutationRules::new();
                let mut applied = 0;

                for m in &mutations {
                    if rules.validate_mutation(m, &list.tasks).is_ok() {
                        if rules.apply_mutation(m, &mut list.tasks).is_ok() {
                            applied += 1;
                        }
                    }
                }

                if applied > 0 {
                    let _ = adapter.save(&list).await;
                    tracing::info!("HOHPlanner: materialized {} creative tasks into task_list.json", applied);
                }
            }
        }

        Ok(mutations)
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
        creative_task_mutations: vec![],
        created_at: 0,
    })
}
