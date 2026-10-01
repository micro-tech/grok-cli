//! HOH Ethical Constraint Model (408)
//!
//! Explicit "do no harm" checks integrated with safety + governance.

use serde::{Deserialize, Serialize};
use crate::hoh::state::HOHError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EthicsCheck {
    pub category: String,
    pub passed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicsEngine {
    pub simulation_mode: bool,
}

impl EthicsEngine {
    pub fn new(simulation_mode: bool) -> Self {
        Self { simulation_mode }
    }

    pub async fn check_proposal(&self, proposal: &str) -> Result<Vec<EthicsCheck>, HOHError> {
        let mut checks = vec![];

        let lower = proposal.to_lowercase();

        checks.push(EthicsCheck {
            category: "user_data".to_string(),
            passed: !lower.contains("exfiltrate") && !lower.contains("read private"),
            reason: if lower.contains("exfiltrate") { "Potential data leak".into() } else { "OK".into() },
        });

        checks.push(EthicsCheck {
            category: "system_integrity".to_string(),
            passed: !lower.contains("rm -rf") && !lower.contains("format disk"),
            reason: "Basic destructive command guard".into(),
        });

        checks.push(EthicsCheck {
            category: "deception".to_string(),
            passed: !lower.contains("lie to user") && !lower.contains("hide changes"),
            reason: "Transparency default".into(),
        });

        if !self.simulation_mode {
            let blocked = checks.iter().filter(|c| !c.passed).count();
            if blocked > 0 {
                tracing::warn!("408 Ethics: {} checks failed for proposal", blocked);
            }
        }

        Ok(checks)
    }
}
