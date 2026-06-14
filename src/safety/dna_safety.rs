//! DNA-Aware Safety Controller
//!
//! Uses SessionDNA failure patterns to automatically raise safety thresholds.

use serde_json::Value;

pub struct DnaSafetyController;

impl DnaSafetyController {
    /// Returns true if the agent should force dry-run / confirmation mode
    pub fn should_enter_safe_mode(dna: &Value) -> bool {
        let failures = dna
            .get("repeated_file_write_failures")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let hallucinated_paths = dna
            .get("repeated_hallucinated_paths")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        failures > 2 || hallucinated_paths > 2
    }
}
