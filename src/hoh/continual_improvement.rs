//! Continual Improvement Engine (Task 297.8 + 361.2 / 361.5 self-refinement + meta loop)

use crate::hoh::state::{IterationState, HOHPlan};

/// 361.5: Rich meta-level continual improvement.
/// Now consumes the full 361.x closed-loop data (A/B/C/D/E) and produces
/// concrete, prioritized suggestions for HOH's own systems.
pub async fn generate_improvements(state: &IterationState) -> Vec<String> {
    let mut improvements = Vec::new();

    // Base signals
    improvements.push("Continue strengthening dependency-aware selection".to_string());

    if let Some(plan) = &state.plan {
        // === A: Materialization feedback ===
        if !plan.materialized_task_ids.is_empty() {
            improvements.push(format!(
                "361.5/A: Review and prioritize the {} newly materialized refactoring tasks ({}). Consider auto-promoting high-confidence ones.",
                plan.materialized_task_ids.len(),
                plan.materialized_task_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
            ));
            if plan.materialized_task_ids.len() > 3 {
                improvements.push("361.5: Materialization volume is high — add throttling / batching to AutonomousRefactoringEngine".to_string());
            }
        }

        // === B: Scoring feedback loop ===
        if plan.refactoring_actions.iter().any(|a| a.contains("score") || a.contains("heuristic") || a.contains("361")) {
            improvements.push("361.5/B: The B feedback path (refactor keywords → planner scoring) is active. Add decay or weighting for older signals.".to_string());
        }

        // === C: Patch stub quality ===
        if !plan.generated_patch_stubs.is_empty() {
            improvements.push(format!(
                "361.5/C: {} patch stubs were generated. Next: turn high-quality stubs into real PatchSets via patch_capture + diff engine.",
                plan.generated_patch_stubs.len()
            ));
        }

        // === D: Specialized agent routing ===
        if !plan.specialized_agent_routes.is_empty() {
            improvements.push(format!(
                "361.5/D: {} actions were routed to specialized sub-agents. Track success rate per role (HeuristicTuner vs ModuleExtractor etc.) in future evaluations.",
                plan.specialized_agent_routes.len()
            ));
            improvements.push("361.5: Promote successful specialized routes into permanent agent personas or skill profiles.".to_string());
        }

        // === Self-refinement (361.2) ===
        if !plan.self_refinement_proposals.is_empty() {
            improvements.push(format!(
                "Apply or convert {} self-refinement proposals from this cycle into concrete HOH changes",
                plan.self_refinement_proposals.len()
            ));
        }

        // High-value architecture proposals
        let high_impact_count = plan.architecture_proposals.iter()
            .filter(|p| p.contains("0.75") || p.contains("0.8") || p.contains("0.82") || p.contains("high-impact"))
            .count();

        if high_impact_count > 0 {
            improvements.push(format!(
                "Convert {} high-impact architecture proposals into tasks/patches or specialized agent work",
                high_impact_count
            ));
        }

        // Experiments that ran
        if plan.experiments.iter().any(|e| e.contains("autonomous_refactoring") || e.contains("self_refinement")) {
            improvements.push("361.5: The autonomous_refactoring + self_refinement experiments are producing data — wire their outcomes back into the next planning goals.".to_string());
        }
    }

    // === Evaluation signals (E loop) ===
    if let Some(last_eval) = state.evaluations.last() {
        if let Some(meta) = last_eval.meta_improvement_score {
            if meta >= 0.82 {
                improvements.push("Increase autonomy_level or planner batch size — recent meta-improvement score is strong".to_string());
            } else if meta < 0.6 {
                improvements.push("Slow down autonomous refactoring confidence threshold; meta score is low".to_string());
            }
        }

        if !last_eval.architecture_proposals_evaluated.is_empty() {
            improvements.push(format!(
                "Evaluate the impact of the {} architecture proposals that were carried into this iteration",
                last_eval.architecture_proposals_evaluated.len()
            ));
        }

        // === New 361.5 signals from patch quality + tests ===
        if last_eval.patch_count > 0 {
            improvements.push(format!(
                "361.5: {} patches produced ({} files changed, avg diff {:.0} chars). {}",
                last_eval.patch_count,
                last_eval.files_changed_count,
                last_eval.avg_diff_length,
                if last_eval.avg_diff_length < 300.0 { "Good focus — prefer this pattern." } else { "Consider breaking large patches." }
            ));
        }

        if let Some(passed) = last_eval.test_passed {
            if !passed {
                improvements.push("361.5: Tests failed last cycle — raise priority on test_strategy tasks and add more validation before materialization.".to_string());

                // NEW: Use the rich test_output if available (the key "no more blank" win)
                if let Some(output) = &last_eval.test_output {
                    let lower = output.to_lowercase();
                    // Extract crude module / test hints so the planner can act on real failures
                    let mut hints = vec![];
                    for line in output.lines().take(30) {
                        let l = line.to_lowercase();
                        if l.contains("error") || l.contains("failed") || l.contains("assertion") {
                            if let Some(hint) = line.split_whitespace().take(6).collect::<Vec<_>>().get(0..3) {
                                hints.push(hint.join(" "));
                            }
                        }
                        if hints.len() >= 3 { break; }
                    }
                    if !hints.is_empty() {
                        improvements.push(format!(
                            "361.5: Recent test failures included hints like: {}. Prioritize adding regression coverage for these areas.",
                            hints.join(" | ")
                        ));
                    }

                    // If the output mentions specific crates or modules, suggest targeted test_strategy work
                    if lower.contains("src/hoh") || lower.contains("planner") || lower.contains("refactor") {
                        improvements.push("361.5: Failures touched HOH core (planner/refactoring). Strongly boost tasks that add test_strategy to high-priority HOH modules.".to_string());
                    }
                }
            } else {
                improvements.push("361.5: Tests passed — safe to increase confidence threshold for refactoring actions slightly.".to_string());
            }
        }

        // Also surface the test_summary when we have rich data
        if !last_eval.test_summary.is_empty() && last_eval.test_summary.len() > 20 {
            improvements.push(format!("361.5: Test summary from last run: {}", last_eval.test_summary.chars().take(180).collect::<String>()));
        }
    }

    // === Cross-cutting meta suggestions ===
    if state.patches.len() > 5 {
        improvements.push("Add a lightweight patch quality / diff-size metric to EvaluationReport".to_string());
    }

    // === 401: Creative ideas feedback ===
    if let Some(plan) = &state.plan {
        if !plan.creative_ideas.is_empty() {
            let high_novelty = plan.creative_ideas.iter().filter(|i| i.novelty_score > 0.7).count();
            improvements.push(format!(
                "401: {} creative ideas generated this cycle ({} high-novelty). Consider promoting top ideas into tasks or experiments.",
                plan.creative_ideas.len(), high_novelty
            ));
            // Surface the best one as a direct meta-suggestion
            if let Some(best) = plan.creative_ideas.iter().max_by(|a, b| a.overall_score.partial_cmp(&b.overall_score).unwrap()) {
                if best.overall_score > 0.68 {
                    improvements.push(format!(
                        "401: Top creative idea candidate — \"{}\". Score {:.2}. Consider turning into a 40x task.",
                        best.title, best.overall_score
                    ));
                }
            }
        }
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    improvements.retain(|i| seen.insert(i.clone()));

    improvements
}

/// 361.5 helper: Turn a plan's materialized work + improvements into suggestions that can be fed back
/// into the next planning cycle (as extra goals or high-priority experiments).
pub fn improvements_to_planning_hints(plan: &HOHPlan, recent_meta: Option<f32>) -> Vec<String> {
    let mut hints = Vec::new();

    if !plan.materialized_task_ids.is_empty() {
        hints.push("Prioritize newly materialized 361.3 tasks in the next selection round".to_string());
    }
    if !plan.specialized_agent_routes.is_empty() {
        hints.push("Collect success/failure signals from specialized sub-agents (361.4) for scoring".to_string());
    }
    if let Some(m) = recent_meta {
        if m > 0.8 {
            hints.push("Raise confidence threshold slightly for autonomous refactoring".to_string());
        }
    }
    hints
}

/// Legacy helper kept for compatibility (361.2 style)
pub fn proposals_to_task_suggestions(
    proposals: &[crate::hoh::architecture_evolution::ArchitectureProposal]
) -> Vec<(u64, String, String)> {
    proposals
        .iter()
        .filter(|p| p.estimated_impact >= 0.65)
        .map(|p| {
            let suggested_id = 3610 + (p.title.len() % 900) as u64;
            let title = format!("[361] {}", p.title);
            let details = format!(
                "Architecture proposal from HOH self-refinement.\n\nRationale: {}\n\nImpact: {:.2}  Risk: {:?}\n\nProposed new tasks: {:?}",
                p.rationale, p.estimated_impact, p.risk_level, p.new_tasks
            );
            (suggested_id, title, details)
        })
        .collect()
}