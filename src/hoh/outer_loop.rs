//! HOH Outer Loop Manager (Task 297.1 + core orchestration + 327 TaskList Intelligence)

use crate::hoh::planner::HOHPlanner;
use crate::hoh::state::{IterationState, IterationStatus, HOHPlan, PatchSet, EvaluationReport, HOHError};
use crate::hoh::tasklist_adapter::TaskListAdapter;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct HOHManager {
    pub config: crate::hoh::HOHConfig,
    pub current_iteration: Option<IterationState>,
    pub data_dir: PathBuf,
    planner: Option<HOHPlanner>,
}

impl HOHManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let simulation = false; // will be overridden by config
        Self {
            data_dir: data_dir.clone(),
            planner: Some(HOHPlanner::new(data_dir.clone(), simulation)),
            ..Default::default()
        }
    }

    pub fn with_config(mut self, config: crate::hoh::HOHConfig) -> Self {
        self.config = config;
        if let Some(p) = &mut self.planner {
            // Recreate planner with correct simulation flag
            *p = HOHPlanner::new(self.data_dir.clone(), self.config.simulation_mode);
        }
        self
    }

    /// Main entry point for running one full HOH iteration (297.1, 297.3, 327)
    pub async fn run_iteration(&mut self) -> Result<IterationState, HOHError> {
        let mut state = IterationState::new(
            self.current_iteration.as_ref().map_or(1, |s| s.iteration_id + 1)
        );

        // Phase 1: Planning (now uses real TaskListAdapter + prioritization)
        state.status = IterationStatus::Planning;
        state.plan = Some(self.plan_phase().await?);

        // Phase 2: Execute (delegate to inner harness - 297.4)
        state.status = IterationStatus::Executing;
        let patches = self.execute_phase(&state.plan.as_ref().unwrap()).await?;
        state.patches = patches;

        // Phase 3: Test (297.6)
        state.status = IterationStatus::Testing;
        // TODO: integrate real cargo test + HOH validation

        // Phase 4: Evaluate (297.7)
        state.status = IterationStatus::Evaluating;
        let eval = self.evaluate_phase().await?;
        state.evaluations.push(eval);

        // Finalize
        state.mark_completed("Iteration completed (tasklist-aware)".to_string());

        self.current_iteration = Some(state.clone());
        Ok(state)
    }

    async fn plan_phase(&mut self) -> Result<HOHPlan, HOHError> {
        let goals = vec![
            "Improve autonomous development".to_string(),
            "Evolve task list intelligently".to_string(),
        ];

        if let Some(planner) = &mut self.planner {
            planner.create_plan(goals).await
        } else {
            // Fallback
            Ok(HOHPlan {
                goals,
                selected_tasks: vec![327, 297, 361],
                experiments: vec!["tasklist_driven".to_string()],
                created_at: chrono::Utc::now().timestamp() as u64,
            })
        }
    }

    async fn execute_phase(&self, _plan: &HOHPlan) -> Result<Vec<PatchSet>, HOHError> {
        // Placeholder - would call into agent / task execution
        Ok(vec![])
    }

    async fn evaluate_phase(&self) -> Result<EvaluationReport, HOHError> {
        Ok(EvaluationReport {
            iteration_id: 1,
            helix_score: Some(0.75),
            internal_metrics: Default::default(),
            notes: "Skeleton evaluation (327 tasklist integration active)".to_string(),
        })
    }

    pub fn is_simulation(&self) -> bool {
        self.config.simulation_mode
    }

    /// Direct access to TaskListAdapter for 327 features (selection, mutation, consistency)
    pub fn tasklist_adapter(&self) -> TaskListAdapter {
        TaskListAdapter::new(self.data_dir.clone(), self.config.simulation_mode)
    }
}