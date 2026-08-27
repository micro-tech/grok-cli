//! Configuration management for grok-cli
//!
//! This module handles loading, saving, and validating configuration settings
//! for the Grok CLI application, with support for environment variables,
//! configuration files, and default values.

use anyhow::{Result, anyhow};

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// Alias std::env because we declare a private `mod env;` below.
// This prevents name clashes when using `env::` for std functions.
use std::env as std_env;

use tracing::{debug, info, warn};

use crate::mcp::config::McpConfig;

pub mod accessors;
pub mod acp;
pub mod bayesian;
pub mod context;
mod defaults;
mod env;
pub mod experimental;
pub mod general;
pub mod logging;
pub mod model;
pub mod network;
pub mod okf;
pub mod security_cfg;
pub mod thinking;
pub mod tools;
pub mod ui;

pub use acp::AcpConfig;
pub use bayesian::{BayesianConfig, BayesianPriorsConfig};
pub use context::{ContextConfig, FileFilteringConfig};
pub use experimental::{CodebaseInvestigatorConfig, ExperimentalConfig, ExtensionsConfig};
pub use general::{GeneralConfig, OutputConfig, SessionRetentionConfig};
pub use logging::{LoggingConfig, RateLimitConfig, TelemetryConfig};
pub use model::ModelConfig;
pub use network::NetworkConfig;
pub use okf::OkfConfig;
pub use security_cfg::{
    EnvVarRedactionConfig, ExternalAccessConfig, FolderTrustConfig, SecurityConfig,
};
pub use thinking::ThinkingMode;
pub use tools::{ShellConfig, ToolsConfig};
pub use ui::{
    AccessibilityConfig, CustomTheme, FooterConfig, InteractiveUIConfig, ThemeColors, UiConfig,
};

// Import default value functions from the dedicated module.
// This makes `default_xxx` names available for `#[serde(default = "...")]` attributes
// on the Config struct and for the `impl Default`.
use defaults::{
    default_max_retries, default_max_tokens, default_model, default_temperature,
    default_timeout_secs,
};

/// Main configuration structure for grok-cli
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Source of the configuration (for display purposes)
    #[serde(skip)]
    pub config_source: Option<ConfigSource>,

    /// Bayesian intent router configuration
    #[serde(default)]
    pub bayesian: BayesianConfig,

    /// X API key for Grok access
    #[serde(skip)]
    pub api_key: Option<String>,

    /// Default model to use
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Default temperature for completions
    #[serde(default = "default_temperature")]
    pub default_temperature: f32,

    /// Default max tokens for completions
    #[serde(default = "default_max_tokens")]
    pub default_max_tokens: u32,

    /// Request timeout in seconds
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,

    /// Maximum number of retries for failed requests
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// General settings
    #[serde(default)]
    pub general: GeneralConfig,

    /// Output format settings
    #[serde(default)]
    pub output: OutputConfig,

    /// UI and display preferences
    #[serde(default)]
    pub ui: UiConfig,

    /// Model configuration
    #[serde(default)]
    pub model: ModelConfig,

    /// Context and file handling settings
    #[serde(default)]
    pub context: ContextConfig,

    /// Tools configuration
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Security settings
    #[serde(default)]
    pub security: SecurityConfig,

    /// Experimental features
    #[serde(default)]
    pub experimental: ExperimentalConfig,

    /// ACP (Agent Client Protocol) configuration
    #[serde(default)]
    pub acp: AcpConfig,

    /// MCP (Model Context Protocol) configuration
    #[serde(default)]
    pub mcp: McpConfig,

    /// Network configuration for Starlink optimization
    #[serde(default)]
    pub network: NetworkConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Telemetry configuration
    #[serde(default)]
    pub telemetry: TelemetryConfig,

    /// OKF (Observability / Workflow Trace Forwarding) configuration.
    /// Used to forward WorkflowTrace records to a central OKF server
    /// (e.g. for Proxmox / cluster-wide analysis).
    #[serde(default)]
    pub okf: OkfConfig,

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limits: RateLimitConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_source: None,
            api_key: None,
            default_model: default_model(),
            default_temperature: default_temperature(),
            default_max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            general: GeneralConfig::default(),
            output: OutputConfig::default(),
            ui: UiConfig::default(),
            model: ModelConfig::default(),
            context: ContextConfig::default(),
            tools: ToolsConfig::default(),
            security: SecurityConfig::default(),
            experimental: ExperimentalConfig::default(),
            bayesian: BayesianConfig::default(),
            acp: AcpConfig::default(),
            mcp: McpConfig::default(),
            network: NetworkConfig::default(),
            logging: LoggingConfig::default(),
            telemetry: TelemetryConfig::default(),
            okf: OkfConfig::default(),
            rate_limits: RateLimitConfig::default(),
        }
    }
}

/// Returns the **config-only** directory for grok-cli.
///
/// This is the location for `config.toml` and `.env`.
///
/// Platform-specific locations:
/// - Windows: `%APPDATA%\grok-cli` (e.g. `AppData\Roaming\grok-cli`)
/// - macOS:   `~/Library/Application Support/grok-cli`
/// - Linux:   `~/.config/grok-cli`
///
/// Falls back to `~/.grok-cli` if the platform config dir cannot be determined.
pub fn grok_config_dir() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("grok-cli"))
        .or_else(|| dirs::home_dir().map(|p| p.join(".grok-cli")))
        .unwrap_or_else(|| PathBuf::from(".").join(".grok-cli"))
}

/// Returns the **data** directory for grok-cli.
///
/// This is the location for `agents/`, `skills/`, `logs/`, `sessions/`, etc.
///
/// On all platforms this prefers the home-based location:
/// - `~/.grok-cli`
///
/// This allows users to keep rich data (agents, logs, skills) in their home
/// directory while still supporting the platform config dir for `config.toml`.
pub fn grok_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".grok-cli"))
        .unwrap_or_else(|| PathBuf::from(".").join(".grok-cli"))
}

/// Configuration scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    System,
    User,
    Project,
}

/// Configuration source tracking
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    /// Built-in defaults only
    Default,
    /// Loaded from system config (~/.grok-cli/config.toml)
    System(PathBuf),
    /// Loaded from project config (.grok/config.toml)
    Project(PathBuf),
    /// Explicitly specified via --config flag
    Explicit(PathBuf),
    /// Hierarchical load (combination of sources)
    Hierarchical {
        project: Option<PathBuf>,
        system: Option<PathBuf>,
    },
}

impl ConfigSource {
    /// Get a display string for the config source
    pub fn display(&self) -> String {
        match self {
            ConfigSource::Default => "built-in defaults".to_string(),
            ConfigSource::System(path) => format!("system config ({})", path.display()),
            ConfigSource::Project(path) => format!("project config ({})", path.display()),
            ConfigSource::Explicit(path) => format!("explicit config ({})", path.display()),
            ConfigSource::Hierarchical { project, system } => {
                let mut parts = Vec::new();
                if let Some(p) = project {
                    parts.push(format!("project ({})", p.display()));
                }
                if let Some(s) = system {
                    parts.push(format!("system ({})", s.display()));
                }
                if parts.is_empty() {
                    "defaults".to_string()
                } else {
                    parts.join(" + ")
                }
            }
        }
    }

    /// Get a short display string for the config source
    pub fn display_short(&self) -> String {
        match self {
            ConfigSource::Default => "defaults".to_string(),
            ConfigSource::System(_) => "system".to_string(),
            ConfigSource::Project(_) => "project".to_string(),
            ConfigSource::Explicit(_) => "explicit".to_string(),
            ConfigSource::Hierarchical { project, system } => {
                let mut parts = Vec::new();
                if project.is_some() {
                    parts.push("project");
                }
                if system.is_some() {
                    parts.push("system");
                }
                if parts.is_empty() {
                    "defaults".to_string()
                } else {
                    parts.join(" + ")
                }
            }
        }
    }
}

impl Config {
    /// Load configuration from file or create default
    pub async fn load(config_path: Option<&str>) -> Result<Self> {
        let config_file_path = match config_path {
            Some(path) => PathBuf::from(path),
            None => Self::default_config_path()?,
        };

        debug!("Loading configuration from: {:?}", config_file_path);

        if config_file_path.exists() {
            let contents = fs::read_to_string(&config_file_path)
                .map_err(|e| anyhow!("Failed to read config file: {}", e))?;

            let mut config: Config = toml::from_str(&contents).map_err(|e| {
                anyhow!(
                    "Failed to parse config file: {}\n\n\
                        This may be due to an outdated configuration format.\n\
                        Try running 'grok config init --force' to recreate the config file,\n\
                        or delete the existing config file at: {:?}",
                    e,
                    config_file_path
                )
            })?;

            // Set config source
            config.config_source = Some(if config_path.is_some() {
                ConfigSource::Explicit(config_file_path.clone())
            } else {
                ConfigSource::System(config_file_path.clone())
            });

            // Override with environment variables
            config.apply_env_overrides(); // implemented in env.rs

            info!("Configuration loaded from: {:?}", config_file_path);
            Ok(config)
        } else {
            warn!(
                "Config file not found, using defaults: {:?}",
                config_file_path
            );
            let mut config = Config {
                config_source: Some(ConfigSource::Default),
                ..Config::default()
            };
            config.apply_env_overrides(); // implemented in env.rs
            Ok(config)
        }
    }

    /// Load configuration with hierarchical priority: project → system → defaults.
    ///
    /// Priority order:
    /// 1. Project-local: `.grok/config.toml` and `.grok/.env`  (only inside real projects, never ~/)
    /// 2. System-level config (~/.grok-cli/config.toml or platform equivalent)
    /// 3. Built-in defaults
    /// 4. Environment variables (highest priority)
    ///
    /// Legacy `~/.grok` is **never** treated as project or system config.
    /// All global user data lives under `~/.grok-cli`.
    pub async fn load_hierarchical() -> Result<Self> {
        debug!("Loading configuration with hierarchical priority");

        // Start with defaults
        let mut config = Config::default();
        debug!("✓ Loaded built-in defaults");

        let mut loaded_system_config: Option<PathBuf> = None;
        let mut loaded_system_env: Option<PathBuf> = None;
        let mut loaded_project_config: Option<PathBuf> = None;
        let mut loaded_project_env: Option<PathBuf> = None;

        // Try system-level config.toml
        let system_config_path = Self::default_config_path()?;
        if system_config_path.exists() {
            debug!("Loading system config.toml from: {:?}", system_config_path);
            match Self::load_config_from_path(&system_config_path).await {
                Ok(system_config) => {
                    config = Self::merge_configs(config, system_config);
                    loaded_system_config = Some(system_config_path.clone());
                    info!("✓ Loaded system config.toml from: {:?}", system_config_path);
                }
                Err(e) => {
                    warn!("Failed to load system config.toml: {}", e);
                }
            }
        } else {
            debug!("No system config.toml found at: {:?}", system_config_path);
        }

        // Try system-level .env
        let system_env_path = Self::get_system_env_path()?;
        if system_env_path.exists() {
            debug!("Loading system .env from: {:?}", system_env_path);
            if let Err(e) = Self::load_env_file(&system_env_path) {
                warn!("Failed to load system .env: {}", e);
            } else {
                loaded_system_env = Some(system_env_path.clone());
                debug!("✓ Loaded system .env from: {:?}", system_env_path);
            }
        } else {
            debug!("No system .env found at: {:?}", system_env_path);
        }

        // Try project-local config.toml
        match Self::find_project_config() {
            Ok(project_config_path) => {
                debug!(
                    "Loading project config.toml from: {:?}",
                    project_config_path
                );
                match Self::load_config_from_path(&project_config_path).await {
                    Ok(project_config) => {
                        config = Self::merge_configs(config, project_config);
                        loaded_project_config = Some(project_config_path.clone());
                        info!(
                            "✓ Loaded project config.toml from: {:?}",
                            project_config_path
                        );
                    }
                    Err(e) => {
                        warn!("Failed to load project config.toml: {}", e);
                    }
                }
            }
            Err(e) => {
                debug!("No project config.toml found in directory tree: {}", e);
            }
        }

        // Try project-local .env
        match Self::find_project_env() {
            Ok(project_env_path) => {
                debug!("Loading project .env from: {:?}", project_env_path);
                if let Err(e) = Self::load_env_file(&project_env_path) {
                    warn!("Failed to load project .env: {}", e);
                } else {
                    loaded_project_env = Some(project_env_path.clone());
                    info!(
                        "Using project-local configuration from: {:?}",
                        project_env_path
                    );
                    debug!("✓ Loaded project .env from: {:?}", project_env_path);
                }
            }
            Err(e) => {
                debug!("No project .env found in directory tree: {}", e);
            }
        }

        // Set config source based on what was loaded
        let system_path = loaded_system_config.or(loaded_system_env);
        let project_path = loaded_project_config.or(loaded_project_env);

        // Extra safety: aggressively reject anything under the legacy ~/.grok as "project".
        // Global data must live in ~/.grok-cli.
        let project_path = project_path.filter(|p| {
            if Self::is_legacy_home_grok(p) {
                debug!("Downgrading legacy ~/.grok path to non-project (use ~/.grok-cli): {:?}", p);
                return false;
            }
            true
        });

        config.config_source = Some(if project_path.is_some() || system_path.is_some() {
            ConfigSource::Hierarchical {
                project: project_path,
                system: system_path,
            }
        } else {
            ConfigSource::Default
        });

        // Apply environment variable overrides (highest priority)
        // This reads from already-loaded env vars (system env + project .env + process env)
        config.apply_env_overrides(); // implemented in env.rs

        Ok(config)
    }

    /// Load configuration from a specific path without merging
    async fn load_config_from_path(path: &PathBuf) -> Result<Self> {
        let contents =
            fs::read_to_string(path).map_err(|e| anyhow!("Failed to read config file: {}", e))?;

        let config: Config =
            toml::from_str(&contents).map_err(|e| anyhow!("Failed to parse config file: {}", e))?;

        Ok(config)
    }

    /// Find project-local config by walking up directory tree.
    ///
    /// CRITICAL: We must **never** treat `~/.grok/` (or anything under it) as a
    /// project configuration. `~/.grok` is the legacy global location.
    /// All global data now lives under `~/.grok-cli` (via `grok_config_dir()` / `grok_data_dir()`).
    ///
    /// Real project configs are only accepted when the `.grok/` directory is
    /// inside an actual project (git repo, etc.) that is **not** the user's home.
    fn find_project_config() -> Result<PathBuf> {
        let mut current_dir = std_env::current_dir()?;
        let home_dir = dirs::home_dir();

        loop {
            let grok_dir = current_dir.join(".grok");
            let config_path = grok_dir.join("config.toml");

            if config_path.exists() {
                let is_home = Self::is_user_home_directory(&current_dir, &home_dir);
                let is_legacy = is_home || Self::is_legacy_home_grok(&config_path);

                if is_legacy {
                    debug!(
                        "Ignoring legacy ~/.grok config (global data belongs in ~/.grok-cli): {:?}",
                        config_path
                    );
                    // Continue walking up — do NOT treat home .grok as project
                } else {
                    return Ok(config_path);
                }
            }

            // Only accept real project markers if we are NOT in/at the home directory.
            let is_home = Self::is_user_home_directory(&current_dir, &home_dir);
            let has_real_project_marker = !is_home
                && (current_dir.join(".git").exists()
                    || current_dir.join("Cargo.toml").exists()
                    || current_dir.join("package.json").exists());

            // If we're at a genuine project root but it has no .grok/config, stop.
            if has_real_project_marker && !grok_dir.join("config.toml").exists() {
                return Err(anyhow!("No project config found"));
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                return Err(anyhow!("No project config found"));
            }
        }
    }

    /// Get system-level .env path
    fn get_system_env_path() -> Result<PathBuf> {
        Ok(grok_config_dir().join(".env"))
    }

    /// Returns true if the given directory is the user's home directory.
    /// Used to prevent treating ~/.grok/ as a project config location.
    fn is_user_home_directory(dir: &std::path::Path, home: &Option<PathBuf>) -> bool {
        if let Some(home) = home {
            // Normalize both paths for comparison (important on Windows)
            // Fall back to non-canonicalized comparison if canonicalize fails
            // (e.g. path doesn't exist yet or permission issues).
            let dir_norm = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
            let home_norm = home.canonicalize().unwrap_or_else(|_| home.clone());

            if dir_norm == home_norm {
                return true;
            }

            // Extra safety on Windows: compare as strings case-insensitively
            // if canonicalize didn't resolve them.
            #[cfg(windows)]
            {
                let d = dir_norm.to_string_lossy().to_lowercase();
                let h = home_norm.to_string_lossy().to_lowercase();
                if d == h {
                    return true;
                }
            }
        }
        false
    }

    /// Returns true if this path lives inside the legacy `~/.grok` directory.
    ///
    /// This is used aggressively to stop treating old global data as "project"
    /// configuration. All global data should live under `~/.grok-cli` now.
    fn is_legacy_home_grok(path: &std::path::Path) -> bool {
        if let Some(home) = dirs::home_dir() {
            let legacy_root = home.join(".grok");
            let p = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let legacy = legacy_root.canonicalize().unwrap_or(legacy_root);

            p.starts_with(&legacy)
        } else {
            false
        }
    }

    /// Find project-local .env file by walking up directory tree.
    ///
    /// Same home directory protection as `find_project_config`.
    fn find_project_env() -> Result<PathBuf> {
        let mut current_dir = std_env::current_dir()?;
        let home_dir = dirs::home_dir();

        loop {
            let env_path = current_dir.join(".grok").join(".env");
            if env_path.exists() {
                let is_home = Self::is_user_home_directory(&current_dir, &home_dir);
                let is_legacy = is_home || Self::is_legacy_home_grok(&env_path);

                if is_legacy {
                    debug!(
                        "Ignoring legacy ~/.grok/.env (global data belongs in ~/.grok-cli): {:?}",
                        env_path
                    );
                } else {
                    return Ok(env_path);
                }
            }

            let is_home = Self::is_user_home_directory(&current_dir, &home_dir);
            let has_real_project_marker = !is_home
                && (current_dir.join(".git").exists()
                    || current_dir.join("Cargo.toml").exists()
                    || current_dir.join("package.json").exists());

            if has_real_project_marker && !current_dir.join(".grok").join(".env").exists() {
                return Err(anyhow!("No project .env found"));
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                return Err(anyhow!("No project .env found"));
            }
        }
    }

    /// Load environment variables from a .env file
    fn load_env_file(path: &PathBuf) -> Result<()> {
        dotenvy::from_path(path)
            .map_err(|e| anyhow!("Failed to load .env file from {:?}: {}", path, e))?;
        Ok(())
    }

    /// Merge two configs, with override taking precedence over base
    fn merge_configs(base: Config, override_config: Config) -> Config {
        // Merge configs by taking non-default values from override_config
        // and keeping base values where override_config has defaults

        // For simplicity, we'll use a field-by-field approach
        // In a production system, you might use a macro or trait for this

        let mut merged = base;

        // Override API key if present
        if override_config.api_key.is_some() {
            merged.api_key = override_config.api_key;
        }

        // Always override these fields (they come from config file with defaults already applied)
        merged.default_model = override_config.default_model;
        merged.default_temperature = override_config.default_temperature;
        merged.default_max_tokens = override_config.default_max_tokens;
        merged.timeout_secs = override_config.timeout_secs;
        merged.max_retries = override_config.max_retries;

        // Override all nested configs
        merged.general = override_config.general;
        merged.output = override_config.output;
        merged.ui = override_config.ui;
        merged.model = override_config.model;
        merged.context = override_config.context;
        merged.tools = override_config.tools;
        merged.security = override_config.security;
        merged.experimental = override_config.experimental;
        merged.acp = override_config.acp;
        merged.mcp = override_config.mcp;
        merged.network = override_config.network;
        merged.logging = override_config.logging;
        merged.telemetry = override_config.telemetry;
        merged.okf = override_config.okf;

        merged
    }

    /// Save configuration to file
    pub async fn save(&self, config_path: Option<&str>) -> Result<()> {
        let config_file_path = match config_path {
            Some(path) => PathBuf::from(path),
            None => match &self.config_source {
                Some(ConfigSource::Explicit(path)) => path.clone(),
                Some(ConfigSource::Project(path)) => path.clone(),
                Some(ConfigSource::System(path)) => path.clone(),
                Some(ConfigSource::Hierarchical { project, system }) => {
                    if let Some(path) = project {
                        path.clone()
                    } else if let Some(path) = system {
                        path.clone()
                    } else {
                        Self::default_config_path()?
                    }
                }
                _ => Self::default_config_path()?,
            },
        };

        // Ensure config directory exists
        if let Some(parent) = config_file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create config directory: {}", e))?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;

        fs::write(&config_file_path, contents)
            .map_err(|e| anyhow!("Failed to write config file: {}", e))?;

        info!("Configuration saved to: {}", config_file_path.display());
        Ok(())
    }

    /// Save configuration to specific scope
    pub async fn save_to_scope(&self, scope: Scope) -> Result<()> {
        let path = self.get_path_for_scope(scope)?;
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid config path: contains non-UTF8 characters"))?;
        self.save(Some(path_str)).await
    }

    /// Get path for a specific configuration scope
    pub fn get_path_for_scope(&self, scope: Scope) -> Result<PathBuf> {
        match scope {
            Scope::User => Self::default_config_path(),
            Scope::Project => {
                let current_dir = std_env::current_dir()?;
                Ok(current_dir.join(".grok").join("config.toml"))
            }
            Scope::System => {
                #[cfg(target_os = "windows")]
                {
                    let program_data = std_env::var("ProgramData")
                        .unwrap_or_else(|_| "C:\\ProgramData".to_string());
                    Ok(PathBuf::from(program_data)
                        .join("grok-cli")
                        .join("config.toml"))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Ok(PathBuf::from("/etc/grok-cli/config.toml"))
                }
            }
        }
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> Result<PathBuf> {
        let dir = grok_config_dir();
        Ok(dir.join("config.toml"))
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate temperature range
        if self.default_temperature < 0.0 || self.default_temperature > 2.0 {
            return Err(anyhow!(
                "Temperature must be between 0.0 and 2.0, got {}",
                self.default_temperature
            ));
        }

        // Validate max tokens
        if self.default_max_tokens == 0 {
            return Err(anyhow!("Max tokens must be greater than 0"));
        }

        // Validate timeout
        if self.timeout_secs == 0 {
            return Err(anyhow!("Timeout must be greater than 0"));
        }

        // Validate retry count
        if self.max_retries == 0 {
            return Err(anyhow!("Max retries must be greater than 0"));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(anyhow!(
                "Invalid log level '{}'. Must be one of: {}",
                self.logging.level,
                valid_levels.join(", ")
            ));
        }

        // Validate network timeouts
        if self.network.connect_timeout == 0 {
            return Err(anyhow!("Connect timeout must be greater than 0"));
        }

        if self.network.read_timeout == 0 {
            return Err(anyhow!("Read timeout must be greater than 0"));
        }

        // Validate ACP port range
        if let Some(port) = self.acp.default_port
            && port < 1024
        {
            warn!(
                "ACP port {} is below 1024, may require elevated privileges",
                port
            );
        }

        Ok(())
    }

    /// Initialize a new configuration file with defaults
    pub async fn init(force: bool) -> Result<PathBuf> {
        let config_path = Self::default_config_path()?;

        if config_path.exists() && !force {
            return Err(anyhow!(
                "Configuration file already exists at {:?}. Use --force to overwrite.",
                config_path
            ));
        }

        let config = Config::default();
        config.save(None).await?;

        Ok(config_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_default() {
        let config = Config::default();
        // After the change to lead with grok-4 as the default
        assert!(config.default_model.starts_with("grok-4"));
        assert_eq!(config.default_temperature, 0.7);
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_config_validation() {
        // Invalid temperature
        let mut config = Config {
            default_temperature: -1.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Invalid log level
        config.default_temperature = 0.7;
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_config_get_set_value() {
        let mut config = Config::default();

        // Test getting values
        assert_eq!(config.get_value("default_model").unwrap(), "grok-4");
        assert_eq!(config.get_value("ui.colors").unwrap(), "true");

        // Test setting values
        config.set_value("default_model", "grok-1").unwrap();
        assert_eq!(config.default_model, "grok-1");

        config.set_value("ui.colors", "false").unwrap();
        assert!(!config.ui.colors);

        // Test invalid key
        assert!(config.get_value("invalid.key").is_err());
        assert!(config.set_value("invalid.key", "value").is_err());
    }

    #[tokio::test]
    async fn test_config_save_load() {
        // Ensure env var doesn't interfere
        unsafe {
            std::env::remove_var("GROK_MODEL");
        }

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create and save config
        let original_config = Config {
            default_model: "test-model".to_string(),
            ..Default::default()
        };
        original_config
            .save(Some(config_path.to_str().unwrap()))
            .await
            .unwrap();

        // Load config and verify
        let loaded_config = Config::load(Some(config_path.to_str().unwrap()))
            .await
            .unwrap();
        assert_eq!(loaded_config.default_model, "test-model");
    }

    #[test]
    fn test_is_user_home_directory() {
        // This test is best-effort; it mainly verifies the function doesn't panic
        // and correctly identifies obvious cases.
        let home = dirs::home_dir();
        if let Some(h) = &home {
            assert!(Config::is_user_home_directory(h, &home));
            let other = h.join("some_subdir");
            assert!(!Config::is_user_home_directory(&other, &home));
        }
    }
}
