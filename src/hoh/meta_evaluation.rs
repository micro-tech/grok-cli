//! HOH Meta-Evaluation Engine (410)
//!
//! Evaluates HOH’s own performance (stability, improvement rate, agent effectiveness).

use serde::{Deserialize, Serialize};
use crate::hoh::state::HOHError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetaEvaluation {
    pub overall_health: f32,
    pub improvement_rate: f32,
    pub agent_effectiveness: f32,
    pub stability: f32,
    pub stagnation_risk: f32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEvaluationEngine {
    pub simulation_mode: bool,
}

impl MetaEvaluationEngine {
    pub fn new(simulation_mode: bool) -> Self {
        Self { simulation_mode }
    }

    pub async fn evaluate_hoh_performance(
        &self,
        prev_meta: Option<f32>,
        current_patches: usize,
        births: usize,
        retirements: usize,
        test_pass_rate: Option<bool>,
    ) -> Result<MetaEvaluation, HOHError> {
        let mut eval = MetaEvaluation::default();

        // Base health from activity + success
        let mut health: f32 = 0.65;
        if current_patches > 0 {
            health += 0.08;
        }
        if births > 0 {
            health += 0.05;
        }
        if retirements > 0 {
            health += 0.03; // healthy pruning
        }
        if test_pass_rate == Some(true) {
            health += 0.10;
        }

        eval.overall_health = health.min(0.96f32);

        // Improvement rate (compare to previous if available)
        eval.improvement_rate = if let Some(prev) = prev_meta {
            ((eval.overall_health - prev) + 0.5).clamp(0.0, 1.0)
        } else {
            0.72
        };

        eval.agent_effectiveness = (0.6 + (births as f32 * 0.04) - (retirements as f32 * 0.02)).clamp(0.4, 0.95);
        eval.stability = if retirements > births { 0.78 } else { 0.85 };

        eval.stagnation_risk = if current_patches == 0 && births == 0 { 0.65 } else { 0.25 };

        eval.notes.push(format!(
            "Health {:.2} | Improvement rate {:.2} | Agents eff {:.2}",
            eval.overall_health, eval.improvement_rate, eval.agent_effectiveness
        ));

        if !self.simulation_mode {
            tracing::info!("410: Meta-evaluation health={:.2} improvement={:.2}", eval.overall_health, eval.improvement_rate);
        }

        Ok(eval)
    }
}
