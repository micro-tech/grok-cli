//! HOH Meta-Planning Engine (409)
//!
//! Plans improvements to HOH itself (planner, evaluator, birth, lifecycle, etc.).

use serde::{Deserialize, Serialize};
use crate::hoh::state::HOHError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetaPlan {
    pub target: String, // "planner", "evaluator", "birth_system", "lifecycle"
    pub change: String,
    pub expected_impact: f32,
    pub simulation_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaPlanningEngine {
    pub simulation_mode: bool,
}

impl MetaPlanningEngine {
    pub fn new(simulation_mode: bool) -> Self {
        Self { simulation_mode }
    }

    pub async fn generate_meta_plans(
        &self,
        recent_meta_score: Option<f32>,
        recent_births: usize,
    ) -> Result<Vec<MetaPlan>, HOHError> {
        let mut plans = vec![];

        if recent_meta_score.map_or(true, |s| s < 0.75) {
            plans.push(MetaPlan {
                target: "planner".to_string(),
                change: "Increase weight of 405 evolution signals in scoring".to_string(),
                expected_impact: 0.12,
                simulation_required: true,
            });
        }

        if recent_births > 2 {
            plans.push(MetaPlan {
                target: "birth_system".to_string(),
                change: "Add capacity budgeting before new births".to_string(),
                expected_impact: 0.09,
                simulation_required: true,
            });
        }

        plans.push(MetaPlan {
            target: "evaluator".to_string(),
            change: "Feed 410 meta-eval directly into next planning goals".to_string(),
            expected_impact: 0.15,
            simulation_required: true,
        });

        if !self.simulation_mode && !plans.is_empty() {
            tracing::info!("409: Generated {} meta-plans for HOH self-improvement", plans.len());
        }

        Ok(plans)
    }
}
