//! Agent definition loader.
//!
//! Reads TOML files from `.grok/agents/<name>.toml` (project-local) and
//! `~/.grok-cli/agents/<name>.toml` (user-global) and deserialises them into
//! [`SubAgentConfig`].  Project-local files take priority.

use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{debug, info, warn};

use super::config::SubAgentConfig;

/// TOML wrapper matching the `[agent.*]` table structure in definition files.
#[derive(serde::Deserialize)]
struct AgentFile {
    agent: SubAgentConfig,
}

/// Resolve the paths to check for `<name>.toml`, in priority order.
///
/// Two-tier design:
///
/// | Tier | Location | Purpose |
/// |---|---|---|
/// | 1 — project | `.grok/agents/<name>.toml` | Project-specific overrides (gitignored) |
/// | 2 — system  | `~/.grok-cli/agents/<name>.toml` | Shipped presets, installed globally |
///
/// Shipped preset TOMLs live in `config/agents/` in the repo and are copied
/// to `~/.grok-cli/agents/` by the installer.  `.grok/agents/` is empty by
/// default and exists only for project-specific customisation.
fn agent_search_paths(name: &str) -> Vec<PathBuf> {
    let filename = format!("{}.toml", name);
    let mut paths = Vec::new();

    // Tier 1 — project override (highest priority)
    paths.push(PathBuf::from(".grok").join("agents").join(&filename));

    // Tier 2 — system-wide preset (installed by installer)
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".grok-cli").join("agents").join(&filename));
    }

    paths
}

/// Load a named agent config from a TOML file, searching project-local
/// then user-global paths.  Returns `Err` if not found or parse fails.
pub fn load_agent_config(name: &str) -> Result<SubAgentConfig> {
    for path in agent_search_paths(name) {
        if !path.exists() {
            debug!("agent loader: not found at {}", path.display());
            continue;
        }
        info!("agent loader: loading '{}' from {}", name, path.display());

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read agent file: {}", path.display()))?;

        // Try flat format first (the file IS a SubAgentConfig directly)
        if let Ok(cfg) = toml::from_str::<SubAgentConfig>(&content) {
            return Ok(cfg);
        }

        // Try wrapped format: [agent] table
        let wrapped: AgentFile = toml::from_str(&content)
            .with_context(|| format!("Failed to parse agent file: {}", path.display()))?;

        return Ok(wrapped.agent);
    }

    Err(anyhow::anyhow!(
        "No agent definition found for '{}'. \
         Create .grok/agents/{}.toml or use SubAgentConfig::default().",
        name,
        name
    ))
}

/// List all named agent presets available in project and user dirs.
/// Returns `(name, path)` pairs sorted by name.
pub fn list_available_agents() -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();

    let dirs_to_scan = {
        let mut d = vec![PathBuf::from(".grok").join("agents")];
        if let Some(home) = dirs::home_dir() {
            d.push(home.join(".grok-cli").join("agents"));
        }
        d
    };

    for dir in dirs_to_scan {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "toml").unwrap_or(false) {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // Don't duplicate if already found in a higher-priority dir
                        if !found.iter().any(|(n, _)| n == stem) {
                            found.push((stem.to_string(), path));
                        }
                    }
                }
            }
        }
    }

    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Load all available agent configs as a map of `name → SubAgentConfig`.
pub fn load_all_agents() -> std::collections::HashMap<String, SubAgentConfig> {
    let mut map = std::collections::HashMap::new();
    for (name, _path) in list_available_agents() {
        match load_agent_config(&name) {
            Ok(cfg) => {
                map.insert(name, cfg);
            }
            Err(e) => {
                warn!("agent loader: failed to load '{}': {}", name, e);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Write a minimal agent TOML into a temp dir and load it.
    #[test]
    fn load_agent_config_from_file() {
        let dir = TempDir::new().unwrap();
        let agents_dir = dir.path().join(".grok").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let toml = r#"
model = "grok-3"
temperature = 0.5
max_tokens = 4096
max_tool_iterations = 5

[tool_permissions]
allow = ["read_file", "list_directory"]
deny  = ["write_file"]

[persona]
role           = "researcher"
tone           = "analytical"
verbosity      = "high"
reasoning_mode = "off"
system_prompt  = "You are a researcher."

[safety]
max_write_size    = 0
allow_destructive = false
require_dry_run   = true
intent_validation = "strict"

[context_budget]
max_tokens        = 120000
summary_threshold = 80000
compression_mode  = "semantic"

[sandbox]
enabled = false
path    = ""
keep    = false
"#;

        let path = agents_dir.join("test_agent.toml");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(toml.as_bytes())
            .unwrap();

        // Temporarily set CWD to the temp dir to pick up .grok/agents/
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = load_agent_config("test_agent");
        std::env::set_current_dir(original_cwd).unwrap();

        let cfg = result.expect("should load successfully");
        assert_eq!(cfg.model, "grok-3");
        assert_eq!(cfg.persona.role, "researcher");
        assert_eq!(
            cfg.tool_permissions.allow.as_ref().unwrap(),
            &["read_file", "list_directory"]
        );
        assert!(
            cfg.tool_permissions
                .deny
                .contains(&"write_file".to_string())
        );
        assert_eq!(cfg.context_budget.max_tokens, 120_000);
        assert!(cfg.safety.require_dry_run);
    }

    #[test]
    fn missing_agent_returns_err() {
        let result = load_agent_config("nonexistent_agent_xyz_abc_123");
        assert!(result.is_err());
    }

    #[test]
    fn list_available_agents_returns_vec() {
        // Should not panic even if dirs don't exist.
        let agents = list_available_agents();
        // We have the actual .grok/agents/ files so there should be some.
        // Just check the function runs without panicking.
        let _ = agents;
    }
}
