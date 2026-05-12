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
    /// Load from `~/.grok/session_dna.json`.
    pub fn load() -> Self {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".grok").join("session_dna.json");
            if path.exists()
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(dna) = serde_json::from_str(&content)
            {
                return dna;
            }
        }
        tracing::warn!("Failed to load ~/.grok/session_dna.json, using defaults");
        Self::default()
    }

    /// Inject into system prompt.
    pub fn inject_into_prompt(&self, prompt: &mut String) {
        prompt.push_str(&format!(
            "\nTone: {}\nVerbosity: {}\n",
            self.tone, self.verbosity
        ));
    }
}
