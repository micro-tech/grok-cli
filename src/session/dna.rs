//! Session DNA System
//!
//! Persistent personality and behavior configuration.

use serde::{Deserialize, Serialize};
use std::fs;

/// Session DNA configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDna {
    pub tone: String,
    pub verbosity: String,
    pub risk_tolerance: String,
    pub coding_style: String,
    pub tool_preferences: Vec<String>,
}

impl Default for SessionDna {
    fn default() -> Self {
        Self {
            tone: "neutral".to_string(),
            verbosity: "medium".to_string(),
            risk_tolerance: "medium".to_string(),
            coding_style: "standard".to_string(),
            tool_preferences: vec![],
        }
    }
}

impl SessionDna {
    /// Load Session DNA.
    ///
    /// Search order:
    /// 1. `./session_dna.json`  (project root – checked first so per-project DNA wins)
    /// 2. `~/.grok/session_dna.json`
    /// 3. Built-in defaults
    pub fn load() -> Self {
        // 1. Project-local file (highest priority)
        let local = std::path::Path::new("session_dna.json");
        if local.exists() {
            if let Ok(content) = fs::read_to_string(local) {
                if let Ok(dna) = serde_json::from_str(&content) {
                    return dna;
                }
            }
        }

        // 2. Global user file
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".grok").join("session_dna.json");
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(dna) = serde_json::from_str(&content) {
                        return dna;
                    }
                }
            }
        }

        tracing::warn!("No session_dna.json found – using defaults");
        Self::default()
    }

    /// Inject the full DNA fingerprint into a system prompt.
    pub fn inject_into_prompt(&self, prompt: &mut String) {
        prompt.push_str("\n\n## Session DNA (Behavioral Fingerprint)\n");
        prompt.push_str(&format!("Tone: {}\n", self.tone));
        prompt.push_str(&format!("Verbosity: {}\n", self.verbosity));
        prompt.push_str(&format!("Risk Tolerance: {}\n", self.risk_tolerance));
        prompt.push_str(&format!("Coding Style: {}\n", self.coding_style));
        if !self.tool_preferences.is_empty() {
            prompt.push_str(&format!(
                "Preferred Tools: {}\n",
                self.tool_preferences.join(", ")
            ));
        }
        prompt.push_str("Adopt this behavioral profile for the entire session.\n");
    }

    /// Apply DNA preferences to a Bayesian engine (router influence).
    /// High risk tolerance boosts shell/edit intents; tool_preferences give direct prior boosts.
    pub fn apply_to_bayes_engine(&self, engine: &mut crate::bayes::BayesianEngine) {
        // Risk tolerance influence
        match self.risk_tolerance.to_lowercase().as_str() {
            "high" => {
                if let Some(p) = engine.priors.get_mut("intent_shell") {
                    *p *= 1.3;
                }
                if let Some(p) = engine.priors.get_mut("intent_edit") {
                    *p *= 1.2;
                }
            }
            "low" => {
                if let Some(p) = engine.priors.get_mut("intent_shell") {
                    *p *= 0.6;
                }
            }
            _ => {}
        }

        // Tool preference influence
        for tool in &self.tool_preferences {
            let intent = match tool.as_str() {
                "run_shell_command" => "intent_shell",
                "write_file" | "replace" => "intent_edit",
                "web_search" | "web_fetch" => "intent_search",
                _ => continue,
            };
            if let Some(p) = engine.priors.get_mut(intent) {
                *p *= 1.25;
            }
        }

        // Re-normalise after DNA adjustments
        let total: f32 = engine.priors.values().sum();
        if total > f32::EPSILON {
            for v in engine.priors.values_mut() {
                *v /= total;
            }
        }
    }

    /// Feedback loop: update DNA after a tool call.
    /// Success slightly reinforces the tool; failure slightly penalises risk tolerance.
    pub fn update_from_tool_result(&mut self, success: bool, tool_name: &str) {
        if success {
            if !self.tool_preferences.contains(&tool_name.to_string()) {
                self.tool_preferences.push(tool_name.to_string());
                // Keep list small
                if self.tool_preferences.len() > 8 {
                    self.tool_preferences.remove(0);
                }
            }
        } else {
            // On failure, become slightly more conservative
            if self.risk_tolerance == "high" {
                self.risk_tolerance = "medium".to_string();
            } else if self.risk_tolerance == "medium" {
                self.risk_tolerance = "low".to_string();
            }
        }
    }
}
