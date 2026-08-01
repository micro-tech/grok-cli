//! Security configuration types.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.
//! Contains SecurityConfig and its nested subtypes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub disable_yolo_mode: bool,
    #[serde(default)]
    pub enable_permanent_tool_approval: bool,
    #[serde(default)]
    pub block_git_extensions: bool,
    #[serde(default)]
    pub folder_trust: FolderTrustConfig,
    #[serde(default)]
    pub environment_variable_redaction: EnvVarRedactionConfig,
    #[serde(default = "default_shell_approval_mode")]
    pub shell_approval_mode: String,
    #[serde(default)]
    pub external_access: ExternalAccessConfig,
}

fn default_shell_approval_mode() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FolderTrustConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvVarRedactionConfig {
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
}

/// Configuration for external directory access
/// Allows read-only access to files outside project boundaries with security controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAccessConfig {
    /// Enable external directory access feature (disabled by default)
    #[serde(default)]
    pub enabled: bool,

    /// Require user approval for each external file access
    #[serde(default = "default_require_approval")]
    pub require_approval: bool,

    /// Log all external access attempts
    #[serde(default = "default_true_external")]
    pub logging: bool,

    /// List of allowed external paths (absolute paths only)
    #[serde(default)]
    pub allowed_paths: Vec<PathBuf>,

    /// Glob patterns to exclude even within allowed paths
    /// e.g., "**/.env", "**/.ssh/**", "**/*.key"
    #[serde(default = "default_excluded_patterns")]
    pub excluded_patterns: Vec<String>,

    /// Session-only trusted paths (not persisted to config)
    /// Used for "Trust Always" decisions during a session
    #[serde(skip)]
    pub session_trusted_paths: Arc<Mutex<Vec<PathBuf>>>,
}

fn default_require_approval() -> bool {
    true
}

fn default_true_external() -> bool {
    true
}

fn default_excluded_patterns() -> Vec<String> {
    vec![
        "**/.env".to_string(),
        "**/.env.*".to_string(),
        "**/.git/**".to_string(),
        "**/.ssh/**".to_string(),
        "**/*.key".to_string(),
        "**/*.pem".to_string(),
        "**/*.p12".to_string(),
        "**/*.pfx".to_string(),
        "**/id_rsa*".to_string(),
        "**/password*".to_string(),
        "**/secret*".to_string(),
        "**/.aws/**".to_string(),
        "**/.azure/**".to_string(),
    ]
}

impl Default for ExternalAccessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_approval: true,
            logging: true,
            allowed_paths: Vec::new(),
            excluded_patterns: default_excluded_patterns(),
            session_trusted_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
