//! HOH LLM Backend Selection (Task 297.21)
//!
//! Allows HOH to use different LLM models for different phases:
//! planning (strategic), execution (coding), and evaluation (analytical).

use serde::{Deserialize, Serialize};

/// A specific LLM backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LLMBackend {
    /// Use whatever the global default is.
    Default,
    Planning { model: String },
    Execution { model: String },
    Evaluation { model: String },
    Custom { model: String, endpoint: Option<String> },
}

impl LLMBackend {
    /// Extract the model name string.
    pub fn model_name(&self) -> &str {
        match self {
            LLMBackend::Default => "grok-3",
            LLMBackend::Planning { model }
            | LLMBackend::Execution { model }
            | LLMBackend::Evaluation { model }
            | LLMBackend::Custom { model, .. } => model.as_str(),
        }
    }
}

/// Per-phase backend configuration for one HOH run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub planning: LLMBackend,
    pub execution: LLMBackend,
    pub evaluation: LLMBackend,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

impl BackendConfig {
    /// All phases use the global default model.
    pub fn default_config() -> Self {
        Self {
            planning: LLMBackend::Default,
            execution: LLMBackend::Default,
            evaluation: LLMBackend::Default,
        }
    }

    /// Load backend config from environment variables.
    ///
    /// | Env var                  | Phase      |
    /// |--------------------------|------------|
    /// | `HOH_PLANNING_MODEL`     | planning   |
    /// | `HOH_EXECUTION_MODEL`    | execution  |
    /// | `HOH_EVALUATION_MODEL`   | evaluation |
    pub fn load_from_env() -> Self {
        let planning = std::env::var("HOH_PLANNING_MODEL")
            .map(|m| LLMBackend::Planning { model: m })
            .unwrap_or(LLMBackend::Default);
        let execution = std::env::var("HOH_EXECUTION_MODEL")
            .map(|m| LLMBackend::Execution { model: m })
            .unwrap_or(LLMBackend::Default);
        let evaluation = std::env::var("HOH_EVALUATION_MODEL")
            .map(|m| LLMBackend::Evaluation { model: m })
            .unwrap_or(LLMBackend::Default);
        Self { planning, execution, evaluation }
    }

    /// Save config to a JSON file (e.g. `.grok/hoh/backend_config.json`).
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
    }

    /// Load config from a JSON file (falls back to default if missing/corrupt).
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Self::default_config)
    }
}

/// Get the model string for a given phase name.
///
/// Phase strings: `"planning"`, `"execution"`, `"evaluation"`.
pub fn get_model_for_phase(config: &BackendConfig, phase: &str) -> String {
    match phase {
        "planning" => match &config.planning {
            LLMBackend::Default => "grok-3".to_string(),
            b => b.model_name().to_string(),
        },
        "execution" => match &config.execution {
            LLMBackend::Default => "grok-3".to_string(),
            b => b.model_name().to_string(),
        },
        "evaluation" => match &config.evaluation {
            LLMBackend::Default => "grok-3-mini".to_string(),
            b => b.model_name().to_string(),
        },
        _ => "grok-3".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_returns_default_variants() {
        let cfg = BackendConfig::default_config();
        assert_eq!(cfg.planning, LLMBackend::Default);
        assert_eq!(cfg.execution, LLMBackend::Default);
        assert_eq!(cfg.evaluation, LLMBackend::Default);
    }

    #[test]
    fn test_get_model_returns_expected_defaults() {
        let cfg = BackendConfig::default_config();
        assert_eq!(get_model_for_phase(&cfg, "planning"), "grok-3");
        assert_eq!(get_model_for_phase(&cfg, "execution"), "grok-3");
        assert_eq!(get_model_for_phase(&cfg, "evaluation"), "grok-3-mini");
        assert_eq!(get_model_for_phase(&cfg, "other"), "grok-3");
    }

    #[test]
    fn test_custom_model_overrides_default() {
        let cfg = BackendConfig {
            planning: LLMBackend::Planning { model: "my-model".to_string() },
            ..BackendConfig::default_config()
        };
        assert_eq!(get_model_for_phase(&cfg, "planning"), "my-model");
    }
}
