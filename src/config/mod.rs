//! Configuration management for grok-cli
//!
//! This module handles loading, saving, and validating configuration settings
//! for the Grok CLI application, with support for environment variables,
//! configuration files, and default values.

use anyhow::{Result, anyhow};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::mcp::config::McpConfig;

/// Main configuration structure for grok-cli
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Source of the configuration (for display purposes)
    #[serde(skip)]
    pub config_source: Option<ConfigSource>,

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

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limits: RateLimitConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_tokens_per_minute: u32,
    pub max_requests_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_minute: 100000,
            max_requests_per_minute: 60,
        }
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryConfig {
    /// Enable telemetry
    pub enabled: bool,

    /// Path to telemetry log file
    pub log_file: Option<PathBuf>,
}

/// ACP-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpConfig {
    /// Enable ACP server functionality
    pub enabled: bool,

    /// Default port for ACP server
    pub default_port: Option<u16>,

    /// Host to bind ACP server to
    pub bind_host: String,

    /// ACP protocol version to use
    pub protocol_version: String,

    /// Enable development features
    pub dev_mode: bool,

    /// Maximum number of tool loop iterations before timeout
    /// This prevents infinite loops when the AI repeatedly calls tools
    /// Default: 25 (increase for complex multi-step tasks)
    #[serde(default = "default_max_tool_loop_iterations")]
    pub max_tool_loop_iterations: u32,
}

/// Network configuration optimized for satellite connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable Starlink-specific optimizations
    pub starlink_optimizations: bool,

    /// Base retry delay in seconds
    pub base_retry_delay: u64,

    /// Maximum retry delay in seconds
    pub max_retry_delay: u64,

    /// Enable network health monitoring
    pub health_monitoring: bool,

    /// Connection timeout in seconds
    pub connect_timeout: u64,

    /// Read timeout in seconds
    pub read_timeout: u64,
}

/// UI and display configuration
/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Enable colored output
    #[serde(default = "default_true")]
    pub colors: bool,

    /// Enable progress indicators
    #[serde(default = "default_true")]
    pub progress_bars: bool,

    /// Show detailed error information
    #[serde(default)]
    pub verbose_errors: bool,

    /// Terminal width override (0 = auto-detect)
    #[serde(default)]
    pub terminal_width: usize,

    /// Enable Unicode characters
    #[serde(default = "default_true")]
    pub unicode: bool,

    /// Color theme for the UI
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Custom theme definitions
    #[serde(default)]
    pub custom_themes: std::collections::HashMap<String, CustomTheme>,

    /// Hide window title bar
    #[serde(default)]
    pub hide_window_title: bool,

    /// Show status information in terminal title
    #[serde(default)]
    pub show_status_in_title: bool,

    /// Hide helpful tips in the UI
    #[serde(default)]
    pub hide_tips: bool,

    /// Hide startup banner (ASCII art logo)
    #[serde(default)]
    pub hide_banner: bool,

    /// Hide context summary above input
    #[serde(default)]
    pub hide_context_summary: bool,

    /// Footer configuration
    #[serde(default)]
    pub footer: FooterConfig,

    /// Hide the footer from the UI
    #[serde(default)]
    pub hide_footer: bool,

    /// Display memory usage information in the UI
    #[serde(default)]
    pub show_memory_usage: bool,

    /// Show line numbers in the chat
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,

    /// Show citations for generated text in the chat
    #[serde(default)]
    pub show_citations: bool,

    /// Show the model name in the chat for each model turn
    #[serde(default)]
    pub show_model_info_in_chat: bool,

    /// Use the entire width of the terminal for output
    #[serde(default = "default_true")]
    pub use_full_width: bool,

    /// Use an alternate screen buffer for the UI, preserving shell history
    #[serde(default)]
    pub use_alternate_buffer: bool,

    /// Enable incremental rendering for the UI
    #[serde(default)]
    pub incremental_rendering: bool,

    /// Custom witty phrases to display during loading
    #[serde(default)]
    pub custom_witty_phrases: Vec<String>,

    /// Accessibility settings
    #[serde(default)]
    pub accessibility: AccessibilityConfig,

    /// Interactive mode configuration
    #[serde(default)]
    pub interactive: InteractiveUIConfig,
}

/// Footer display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooterConfig {
    /// Hide current working directory in footer
    #[serde(default)]
    pub hide_cwd: bool,

    /// Hide sandbox status indicator in footer
    #[serde(default)]
    pub hide_sandbox_status: bool,

    /// Hide model information in footer
    #[serde(default)]
    pub hide_model_info: bool,

    /// Hide context window percentage in footer
    #[serde(default = "default_true")]
    pub hide_context_percentage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub background: ThemeColors,
    #[serde(default)]
    pub foreground: ThemeColors,
    #[serde(default)]
    pub accent: ThemeColors,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeColors {
    #[serde(default)]
    pub primary: String,
    #[serde(default)]
    pub secondary: String,
    #[serde(default)]
    pub success: String,
    #[serde(default)]
    pub warning: String,
    #[serde(default)]
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessibilityConfig {
    #[serde(default)]
    pub disable_loading_phrases: bool,
    #[serde(default)]
    pub screen_reader: bool,
}

/// Interactive mode UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveUIConfig {
    /// Prompt style (simple, rich, minimal)
    #[serde(default = "default_prompt_style")]
    pub prompt_style: String,

    /// Enable context usage display
    #[serde(default = "default_true")]
    pub show_context_usage: bool,

    /// Auto-save sessions
    #[serde(default)]
    pub auto_save_sessions: bool,

    /// Check for home directory usage
    #[serde(default = "default_true")]
    pub check_directory: bool,

    /// Enable startup animation
    #[serde(default = "default_true")]
    pub startup_animation: bool,

    /// Update check frequency in hours (0 = disabled)
    #[serde(default = "default_update_check_hours")]
    pub update_check_hours: u64,

    /// Custom key bindings
    #[serde(default)]
    pub key_bindings: std::collections::HashMap<String, String>,
}

fn default_prompt_style() -> String {
    "rich".to_string()
}

fn default_true() -> bool {
    true
}

fn default_update_check_hours() -> u64 {
    24
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_model() -> String {
    "grok-4-1-fast-reasoning".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout_secs() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

fn default_hide_context_percentage() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    #[serde(default)]
    pub preview_features: bool,
    #[serde(default)]
    pub preferred_editor: String,
    #[serde(default)]
    pub vim_mode: bool,
    #[serde(default)]
    pub disable_auto_update: bool,
    #[serde(default)]
    pub disable_update_nag: bool,
    #[serde(default)]
    pub enable_prompt_completion: bool,
    #[serde(default)]
    pub retry_fetch_errors: bool,
    #[serde(default)]
    pub debug_keystroke_logging: bool,
    #[serde(default)]
    pub session_retention: SessionRetentionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRetentionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_age: u64, // in hours
    #[serde(default)]
    pub max_count: u32,
    #[serde(default)]
    pub min_retention: u64, // in hours
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_format")]
    pub format: String,
}

fn default_output_format() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub max_session_turns: i32,
    #[serde(default)]
    pub summarize_tool_output: bool,
    #[serde(default)]
    pub compression_threshold: f64,
    #[serde(default = "default_true")]
    pub skip_next_speaker_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub import_format: String,
    #[serde(default)]
    pub discovery_max_dirs: u32,
    #[serde(default)]
    pub include_directories: Vec<String>,
    #[serde(default)]
    pub load_memory_from_include_directories: bool,
    #[serde(default)]
    pub file_filtering: FileFilteringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilteringConfig {
    #[serde(default = "default_true")]
    pub respect_git_ignore: bool,
    #[serde(default = "default_true")]
    pub respect_grok_ignore: bool,
    #[serde(default = "default_true")]
    pub enable_recursive_file_search: bool,
    #[serde(default)]
    pub disable_fuzzy_search: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub auto_accept: bool,
    #[serde(default)]
    pub core: Vec<String>,
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub discovery_command: String,
    #[serde(default)]
    pub call_command: String,
    #[serde(default = "default_true")]
    pub use_ripgrep: bool,
    #[serde(default = "default_true")]
    pub enable_tool_output_truncation: bool,
    #[serde(default)]
    pub truncate_tool_output_threshold: u32,
    #[serde(default)]
    pub truncate_tool_output_lines: u32,
    #[serde(default = "default_true")]
    pub enable_message_bus_integration: bool,
    #[serde(default)]
    pub enable_hooks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    #[serde(default = "default_true")]
    pub enable_interactive_shell: bool,
    #[serde(default)]
    pub pager: String,
    #[serde(default)]
    pub show_color: bool,
    #[serde(default)]
    pub inactivity_timeout: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperimentalConfig {
    #[serde(default)]
    pub enable_agents: bool,
    #[serde(default)]
    pub extension_management: bool,
    #[serde(default)]
    pub extension_reloading: bool,
    #[serde(default)]
    pub jit_context: bool,
    #[serde(default)]
    pub codebase_investigator_settings: CodebaseInvestigatorConfig,
    #[serde(default)]
    pub extensions: ExtensionsConfig,
}

/// Extensions configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionsConfig {
    /// Enable extensions system
    #[serde(default)]
    pub enabled: bool,

    /// Directory to load extensions from
    #[serde(default)]
    pub extension_dir: Option<PathBuf>,

    /// List of enabled extensions
    #[serde(default)]
    pub enabled_extensions: Vec<String>,

    /// Allow loading extensions from config
    #[serde(default)]
    pub allow_config_extensions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseInvestigatorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub max_num_turns: u32,
    #[serde(default)]
    pub max_time_minutes: u32,
    #[serde(default)]
    pub thinking_budget: u32,
    #[serde(default)]
    pub model: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable file logging
    #[serde(default)]
    pub file_logging: bool,

    /// Log file path (None = default location)
    #[serde(default)]
    pub log_file: Option<PathBuf>,

    /// Maximum log file size in MB
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,

    /// Number of log files to rotate
    #[serde(default = "default_rotation_count")]
    pub rotation_count: u32,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_file_size_mb() -> u64 {
    10
}

fn default_rotation_count() -> u32 {
    5
}

fn default_max_tool_loop_iterations() -> u32 {
    25
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_source: None,
            api_key: None,
            default_model: default_model(),
            default_temperature: 0.7,
            default_max_tokens: 4096,
            timeout_secs: 30,
            max_retries: 3,
            general: GeneralConfig::default(),
            output: OutputConfig::default(),
            ui: UiConfig::default(),
            model: ModelConfig::default(),
            context: ContextConfig::default(),
            tools: ToolsConfig::default(),
            security: SecurityConfig::default(),
            experimental: ExperimentalConfig::default(),
            acp: AcpConfig::default(),
            mcp: McpConfig::default(),
            network: NetworkConfig::default(),
            logging: LoggingConfig::default(),
            telemetry: TelemetryConfig::default(),
            rate_limits: RateLimitConfig::default(),
        }
    }
}

impl Default for AcpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_port: None, // Auto-assign
            bind_host: "127.0.0.1".to_string(),
            protocol_version: "1.0".to_string(),
            dev_mode: false,
            max_tool_loop_iterations: default_max_tool_loop_iterations(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            starlink_optimizations: true,
            base_retry_delay: 1,
            max_retry_delay: 60,
            health_monitoring: true,
            connect_timeout: 10,
            read_timeout: 30,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colors: true,
            progress_bars: true,
            verbose_errors: false,
            terminal_width: 0, // Auto-detect
            unicode: true,
            theme: "default".to_string(),
            custom_themes: std::collections::HashMap::new(),
            hide_window_title: false,
            show_status_in_title: false,
            hide_tips: false,
            hide_banner: false,
            hide_context_summary: false,
            footer: FooterConfig::default(),
            hide_footer: false,
            show_memory_usage: false,
            show_line_numbers: true,
            show_citations: false,
            show_model_info_in_chat: false,
            use_full_width: true,
            use_alternate_buffer: false,
            incremental_rendering: false,
            custom_witty_phrases: Vec::new(),
            accessibility: AccessibilityConfig::default(),
            interactive: InteractiveUIConfig::default(),
        }
    }
}

impl Default for FooterConfig {
    fn default() -> Self {
        Self {
            hide_cwd: false,
            hide_sandbox_status: false,
            hide_model_info: false,
            hide_context_percentage: true,
        }
    }
}

impl Default for SessionRetentionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_age: 168, // 7 days
            max_count: 50,
            min_retention: 24, // 1 day
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_output_format(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            max_session_turns: -1, // unlimited
            summarize_tool_output: false,
            compression_threshold: 0.2,
            skip_next_speaker_check: true,
        }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            file_name: String::new(),
            import_format: String::new(),
            discovery_max_dirs: 200,
            include_directories: Vec::new(),
            load_memory_from_include_directories: false,
            file_filtering: FileFilteringConfig::default(),
        }
    }
}

impl Default for FileFilteringConfig {
    fn default() -> Self {
        Self {
            respect_git_ignore: true,
            respect_grok_ignore: true,
            enable_recursive_file_search: true,
            disable_fuzzy_search: false,
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            shell: ShellConfig::default(),
            auto_accept: false,
            core: Vec::new(),
            allowed: Vec::new(),
            exclude: Vec::new(),
            discovery_command: String::new(),
            call_command: String::new(),
            use_ripgrep: true,
            enable_tool_output_truncation: true,
            truncate_tool_output_threshold: 10000,
            truncate_tool_output_lines: 100,
            enable_message_bus_integration: true,
            enable_hooks: false,
        }
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            enable_interactive_shell: true,
            pager: String::new(),
            show_color: false,
            inactivity_timeout: 0,
        }
    }
}

impl Default for CodebaseInvestigatorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_num_turns: 10,
            max_time_minutes: 15,
            thinking_budget: 1000,
            model: "auto".to_string(),
        }
    }
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            background: ThemeColors::default(),
            foreground: ThemeColors::default(),
            accent: ThemeColors::default(),
        }
    }
}

impl Default for InteractiveUIConfig {
    fn default() -> Self {
        Self {
            prompt_style: "rich".to_string(),
            show_context_usage: true,
            auto_save_sessions: false,
            check_directory: true,
            startup_animation: true,
            update_check_hours: 24,
            key_bindings: std::collections::HashMap::new(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_logging: false,
            log_file: None,
            max_file_size_mb: 10,
            rotation_count: 5,
        }
    }
}

use std::env;

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
    /// Loaded from system config (~/.grok/config.toml)
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
            config.apply_env_overrides();

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
            config.apply_env_overrides();
            Ok(config)
        }
    }

    /// Load configuration with hierarchical priority: project → system → defaults
    ///
    /// Priority order:
    /// 1. Project-local: .grok/.env in current directory or parent
    /// 2. System-level: ~/.grok/.env (or %APPDATA%\.grok\.env on Windows)
    /// 3. Built-in defaults
    /// 4. Environment variables (highest priority, applied last)
    /// Load configuration with hierarchical priority: project → system → defaults
    ///
    /// Settings from higher priority sources override lower priority sources.
    pub async fn load_hierarchical() -> Result<Self> {
        debug!("Loading configuration with hierarchical priority");

        // Start with defaults
        let mut config = Config::default();
        debug!("✓ Loaded built-in defaults");

        let mut loaded_system: Option<PathBuf> = None;
        let mut loaded_project: Option<PathBuf> = None;

        // Try loading system-level config.toml first
        let system_config_path = Self::default_config_path()?;
        if system_config_path.exists() {
            debug!("Loading system config.toml from: {:?}", system_config_path);
            match Self::load_config_from_path(&system_config_path).await {
                Ok(system_config) => {
                    config = Self::merge_configs(config, system_config);
                    loaded_system = Some(system_config_path.clone());
                    debug!("✓ Loaded system config.toml from: {:?}", system_config_path);
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
                if loaded_system.is_none() {
                    loaded_system = Some(system_env_path.clone());
                }
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
                        loaded_project = Some(project_config_path.clone());
                        info!(
                            "Using project-local config.toml from: {:?}",
                            project_config_path
                        );
                        debug!(
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
                    if loaded_project.is_none() {
                        loaded_project = Some(project_env_path.clone());
                    }
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
        config.config_source = Some(if loaded_project.is_some() || loaded_system.is_some() {
            ConfigSource::Hierarchical {
                project: loaded_project,
                system: loaded_system,
            }
        } else {
            ConfigSource::Default
        });

        // Apply environment variable overrides (highest priority)
        // This reads from already-loaded env vars (system env + project .env + process env)
        config.apply_env_overrides();

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

    /// Find project-local config by walking up directory tree
    fn find_project_config() -> Result<PathBuf> {
        let mut current_dir = env::current_dir()?;

        loop {
            let config_path = current_dir.join(".grok").join("config.toml");
            if config_path.exists() {
                return Ok(config_path);
            }

            // Also check for project root markers (.git, Cargo.toml, etc.)
            let has_project_marker = current_dir.join(".git").exists()
                || current_dir.join("Cargo.toml").exists()
                || current_dir.join("package.json").exists()
                || current_dir.join(".grok").exists();

            // If we found a project root but no config, stop searching
            if has_project_marker && !current_dir.join(".grok").join("config.toml").exists() {
                return Err(anyhow!("No project config found"));
            }

            // Move to parent directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                // Reached filesystem root
                return Err(anyhow!("No project config found"));
            }
        }
    }

    /// Get system-level config path (legacy TOML)
    fn get_system_config_path() -> Result<PathBuf> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home_dir.join(".grok").join("config.toml"))
    }

    /// Get system-level .env path
    fn get_system_env_path() -> Result<PathBuf> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home_dir.join(".grok").join(".env"))
    }

    /// Find project-local .env file by walking up directory tree
    fn find_project_env() -> Result<PathBuf> {
        let mut current_dir = env::current_dir()?;

        loop {
            let env_path = current_dir.join(".grok").join(".env");
            if env_path.exists() {
                return Ok(env_path);
            }

            // Also check for project root markers (.git, Cargo.toml, etc.)
            let has_project_marker = current_dir.join(".git").exists()
                || current_dir.join("Cargo.toml").exists()
                || current_dir.join("package.json").exists()
                || current_dir.join(".grok").exists();

            // If we found a project root but no .env, stop searching
            if has_project_marker && !current_dir.join(".grok").join(".env").exists() {
                return Err(anyhow!("No project .env found"));
            }

            // Move to parent directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                // Reached filesystem root
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
        // Simple override: all values from override_config replace base values
        // This is the correct behavior for hierarchical configs where
        // project config should fully override system config

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

        merged
    }

    /// Save configuration to file
    pub async fn save(&self, config_path: Option<&str>) -> Result<()> {
        let config_file_path = match config_path {
            Some(path) => PathBuf::from(path),
            None => Self::default_config_path()?,
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

        info!("Configuration saved to: {:?}", config_file_path);
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
                let current_dir = env::current_dir()?;
                Ok(current_dir.join(".grok").join("config.toml"))
            }
            Scope::System => {
                #[cfg(target_os = "windows")]
                {
                    let program_data =
                        env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
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
        let config_dir =
            config_dir().ok_or_else(|| anyhow!("Could not determine config directory"))?;

        Ok(config_dir.join("grok-cli").join("config.toml"))
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // API key from environment
        if let Ok(api_key) = std::env::var("GROK_API_KEY") {
            self.api_key = Some(api_key);
        } else if let Ok(api_key) = std::env::var("X_API_KEY") {
            self.api_key = Some(api_key);
        }

        // Model configuration
        if let Ok(model) = std::env::var("GROK_MODEL") {
            self.default_model = model;
        }

        if let Ok(temp) = std::env::var("GROK_TEMPERATURE") {
            if let Ok(temp_val) = temp.parse::<f32>() {
                self.default_temperature = temp_val;
            }
        }

        if let Ok(tokens) = std::env::var("GROK_MAX_TOKENS") {
            if let Ok(tokens_val) = tokens.parse::<u32>() {
                self.default_max_tokens = tokens_val;
            }
        }

        // Network configuration
        if let Ok(timeout) = std::env::var("GROK_TIMEOUT") {
            if let Ok(timeout_val) = timeout.parse::<u64>() {
                self.timeout_secs = timeout_val;
            }
        }

        if let Ok(retries) = std::env::var("GROK_MAX_RETRIES") {
            if let Ok(retries_val) = retries.parse::<u32>() {
                self.max_retries = retries_val;
            }
        }

        if let Ok(val) = std::env::var("GROK_STARLINK_OPTIMIZATIONS") {
            self.network.starlink_optimizations = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(delay) = std::env::var("GROK_BASE_RETRY_DELAY") {
            if let Ok(delay_val) = delay.parse::<u64>() {
                self.network.base_retry_delay = delay_val;
            }
        }

        if let Ok(delay) = std::env::var("GROK_MAX_RETRY_DELAY") {
            if let Ok(delay_val) = delay.parse::<u64>() {
                self.network.max_retry_delay = delay_val;
            }
        }

        if let Ok(val) = std::env::var("GROK_HEALTH_MONITORING") {
            self.network.health_monitoring = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(timeout) = std::env::var("GROK_CONNECT_TIMEOUT") {
            if let Ok(timeout_val) = timeout.parse::<u64>() {
                self.network.connect_timeout = timeout_val;
            }
        }

        if let Ok(timeout) = std::env::var("GROK_READ_TIMEOUT") {
            if let Ok(timeout_val) = timeout.parse::<u64>() {
                self.network.read_timeout = timeout_val;
            }
        }

        // UI configuration
        if let Ok(val) = std::env::var("GROK_COLORS") {
            self.ui.colors = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(val) = std::env::var("GROK_PROGRESS_BARS") {
            self.ui.progress_bars = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(val) = std::env::var("GROK_UNICODE") {
            self.ui.unicode = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(val) = std::env::var("GROK_VERBOSE_ERRORS") {
            self.ui.verbose_errors = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(width) = std::env::var("GROK_TERMINAL_WIDTH") {
            if let Ok(width_val) = width.parse::<usize>() {
                self.ui.terminal_width = width_val;
            }
        }

        // Disable colors if NO_COLOR is set
        if std::env::var("NO_COLOR").is_ok() {
            self.ui.colors = false;
        }

        // Logging configuration
        if let Ok(level) = std::env::var("GROK_LOG_LEVEL") {
            self.logging.level = level;
        }

        if let Ok(val) = std::env::var("GROK_FILE_LOGGING") {
            self.logging.file_logging = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(path) = std::env::var("GROK_LOG_FILE") {
            self.logging.log_file = Some(PathBuf::from(path));
        }

        if let Ok(size) = std::env::var("GROK_MAX_FILE_SIZE_MB") {
            if let Ok(size_val) = size.parse::<u64>() {
                self.logging.max_file_size_mb = size_val;
            }
        }

        if let Ok(count) = std::env::var("GROK_ROTATION_COUNT") {
            if let Ok(count_val) = count.parse::<u32>() {
                self.logging.rotation_count = count_val;
            }
        }

        // ACP configuration
        if let Ok(val) = std::env::var("GROK_ACP_ENABLED") {
            self.acp.enabled = val.parse::<bool>().unwrap_or(true);
        }

        if std::env::var("GROK_ACP_DISABLE").is_ok() {
            self.acp.enabled = false;
        }

        if let Ok(port) = std::env::var("GROK_ACP_PORT") {
            if let Ok(port_val) = port.parse::<u16>() {
                self.acp.default_port = Some(port_val);
            }
        }

        if let Ok(host) = std::env::var("GROK_ACP_BIND_HOST") {
            self.acp.bind_host = host;
        }

        if let Ok(version) = std::env::var("GROK_ACP_PROTOCOL_VERSION") {
            self.acp.protocol_version = version;
        }

        if let Ok(val) = std::env::var("GROK_ACP_DEV_MODE") {
            self.acp.dev_mode = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(iterations) = std::env::var("GROK_ACP_MAX_TOOL_LOOP_ITERATIONS") {
            if let Ok(iterations_val) = iterations.parse::<u32>() {
                self.acp.max_tool_loop_iterations = iterations_val;
            }
        }

        // Telemetry configuration
        if let Ok(val) = std::env::var("GROK_TELEMETRY_ENABLED") {
            self.telemetry.enabled = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(path) = std::env::var("GROK_TELEMETRY_LOG_FILE") {
            self.telemetry.log_file = Some(PathBuf::from(path));
        }

        // Security configuration
        if let Ok(mode) = std::env::var("GROK_SHELL_APPROVAL_MODE") {
            self.security.shell_approval_mode = mode;
        }
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
        if let Some(port) = self.acp.default_port {
            if port < 1024 {
                warn!(
                    "ACP port {} is below 1024, may require elevated privileges",
                    port
                );
            }
        }

        Ok(())
    }

    /// Get configuration value by key path (e.g., "network.timeout")
    pub fn get_value(&self, key: &str) -> Result<String> {
        match key {
            // Root settings
            "api_key" => Ok(self.api_key.clone().unwrap_or_default()),
            "default_model" => Ok(self.default_model.clone()),
            "default_temperature" => Ok(self.default_temperature.to_string()),
            "default_max_tokens" => Ok(self.default_max_tokens.to_string()),
            "timeout_secs" => Ok(self.timeout_secs.to_string()),
            "max_retries" => Ok(self.max_retries.to_string()),

            // General settings
            "general.preview_features" => Ok(self.general.preview_features.to_string()),
            "general.preferred_editor" => Ok(self.general.preferred_editor.clone()),
            "general.vim_mode" => Ok(self.general.vim_mode.to_string()),
            "general.disable_auto_update" => Ok(self.general.disable_auto_update.to_string()),
            "general.disable_update_nag" => Ok(self.general.disable_update_nag.to_string()),
            "general.enable_prompt_completion" => {
                Ok(self.general.enable_prompt_completion.to_string())
            }
            "general.retry_fetch_errors" => Ok(self.general.retry_fetch_errors.to_string()),
            "general.debug_keystroke_logging" => {
                Ok(self.general.debug_keystroke_logging.to_string())
            }

            // UI settings
            "ui.colors" => Ok(self.ui.colors.to_string()),
            "ui.progress_bars" => Ok(self.ui.progress_bars.to_string()),
            "ui.verbose_errors" => Ok(self.ui.verbose_errors.to_string()),
            "ui.terminal_width" => Ok(self.ui.terminal_width.to_string()),
            "ui.unicode" => Ok(self.ui.unicode.to_string()),
            "ui.theme" => Ok(self.ui.theme.clone()),
            "ui.hide_window_title" => Ok(self.ui.hide_window_title.to_string()),
            "ui.show_status_in_title" => Ok(self.ui.show_status_in_title.to_string()),
            "ui.hide_tips" => Ok(self.ui.hide_tips.to_string()),
            "ui.hide_banner" => Ok(self.ui.hide_banner.to_string()),
            "ui.hide_context_summary" => Ok(self.ui.hide_context_summary.to_string()),
            "ui.hide_footer" => Ok(self.ui.hide_footer.to_string()),
            "ui.show_memory_usage" => Ok(self.ui.show_memory_usage.to_string()),
            "ui.show_line_numbers" => Ok(self.ui.show_line_numbers.to_string()),
            "ui.show_citations" => Ok(self.ui.show_citations.to_string()),
            "ui.show_model_info_in_chat" => Ok(self.ui.show_model_info_in_chat.to_string()),
            "ui.use_full_width" => Ok(self.ui.use_full_width.to_string()),
            "ui.use_alternate_buffer" => Ok(self.ui.use_alternate_buffer.to_string()),
            "ui.incremental_rendering" => Ok(self.ui.incremental_rendering.to_string()),
            "ui.accessibility.disable_loading_phrases" => {
                Ok(self.ui.accessibility.disable_loading_phrases.to_string())
            }
            "ui.accessibility.screen_reader" => Ok(self.ui.accessibility.screen_reader.to_string()),
            "ui.footer.hide_cwd" => Ok(self.ui.footer.hide_cwd.to_string()),
            "ui.footer.hide_sandbox_status" => Ok(self.ui.footer.hide_sandbox_status.to_string()),
            "ui.footer.hide_model_info" => Ok(self.ui.footer.hide_model_info.to_string()),
            "ui.footer.hide_context_percentage" => {
                Ok(self.ui.footer.hide_context_percentage.to_string())
            }

            // Model settings
            "model.name" => Ok(self.model.name.clone()),
            "model.max_session_turns" => Ok(self.model.max_session_turns.to_string()),
            "model.summarize_tool_output" => Ok(self.model.summarize_tool_output.to_string()),
            "model.compression_threshold" => Ok(self.model.compression_threshold.to_string()),
            "model.skip_next_speaker_check" => Ok(self.model.skip_next_speaker_check.to_string()),

            // Context settings
            "context.discovery_max_dirs" => Ok(self.context.discovery_max_dirs.to_string()),
            "context.load_memory_from_include_directories" => Ok(self
                .context
                .load_memory_from_include_directories
                .to_string()),
            "context.file_filtering.respect_git_ignore" => {
                Ok(self.context.file_filtering.respect_git_ignore.to_string())
            }
            "context.file_filtering.respect_grok_ignore" => {
                Ok(self.context.file_filtering.respect_grok_ignore.to_string())
            }
            "context.file_filtering.enable_recursive_file_search" => Ok(self
                .context
                .file_filtering
                .enable_recursive_file_search
                .to_string()),
            "context.file_filtering.disable_fuzzy_search" => {
                Ok(self.context.file_filtering.disable_fuzzy_search.to_string())
            }

            // Tools settings
            "tools.shell.enable_interactive_shell" => {
                Ok(self.tools.shell.enable_interactive_shell.to_string())
            }
            "tools.shell.show_color" => Ok(self.tools.shell.show_color.to_string()),
            "tools.auto_accept" => Ok(self.tools.auto_accept.to_string()),
            "tools.use_ripgrep" => Ok(self.tools.use_ripgrep.to_string()),
            "tools.enable_tool_output_truncation" => {
                Ok(self.tools.enable_tool_output_truncation.to_string())
            }
            "tools.truncate_tool_output_threshold" => {
                Ok(self.tools.truncate_tool_output_threshold.to_string())
            }
            "tools.truncate_tool_output_lines" => {
                Ok(self.tools.truncate_tool_output_lines.to_string())
            }
            "tools.enable_message_bus_integration" => {
                Ok(self.tools.enable_message_bus_integration.to_string())
            }

            // Security settings
            "security.disable_yolo_mode" => Ok(self.security.disable_yolo_mode.to_string()),
            "security.enable_permanent_tool_approval" => {
                Ok(self.security.enable_permanent_tool_approval.to_string())
            }
            "security.block_git_extensions" => Ok(self.security.block_git_extensions.to_string()),
            "security.folder_trust.enabled" => Ok(self.security.folder_trust.enabled.to_string()),
            "security.environment_variable_redaction.enabled" => Ok(self
                .security
                .environment_variable_redaction
                .enabled
                .to_string()),

            // Experimental settings
            "experimental.enable_agents" => Ok(self.experimental.enable_agents.to_string()),
            "experimental.extension_management" => {
                Ok(self.experimental.extension_management.to_string())
            }
            "experimental.jit_context" => Ok(self.experimental.jit_context.to_string()),
            "experimental.codebase_investigator_settings.enabled" => Ok(self
                .experimental
                .codebase_investigator_settings
                .enabled
                .to_string()),
            "experimental.codebase_investigator_settings.max_num_turns" => Ok(self
                .experimental
                .codebase_investigator_settings
                .max_num_turns
                .to_string()),

            // ACP settings
            "acp.enabled" => Ok(self.acp.enabled.to_string()),
            "acp.bind_host" => Ok(self.acp.bind_host.clone()),
            "acp.protocol_version" => Ok(self.acp.protocol_version.clone()),
            "acp.dev_mode" => Ok(self.acp.dev_mode.to_string()),
            "acp.default_port" => Ok(self
                .acp
                .default_port
                .map(|p| p.to_string())
                .unwrap_or_default()),

            // Network settings
            "network.starlink_optimizations" => Ok(self.network.starlink_optimizations.to_string()),
            "network.base_retry_delay" => Ok(self.network.base_retry_delay.to_string()),
            "network.max_retry_delay" => Ok(self.network.max_retry_delay.to_string()),
            "network.health_monitoring" => Ok(self.network.health_monitoring.to_string()),
            "network.connect_timeout" => Ok(self.network.connect_timeout.to_string()),
            "network.read_timeout" => Ok(self.network.read_timeout.to_string()),

            // Logging settings
            "logging.level" => Ok(self.logging.level.clone()),
            "logging.file_logging" => Ok(self.logging.file_logging.to_string()),
            "logging.max_file_size_mb" => Ok(self.logging.max_file_size_mb.to_string()),
            "logging.rotation_count" => Ok(self.logging.rotation_count.to_string()),

            // Telemetry settings
            "telemetry.enabled" => Ok(self.telemetry.enabled.to_string()),

            _ => Err(anyhow!("Unknown configuration key: {}", key)),
        }
    }

    /// Set configuration value by key path
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            // Root settings
            "api_key" => {
                self.api_key = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "default_model" => {
                self.default_model = value.to_string();
            }
            "default_temperature" => {
                self.default_temperature = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid temperature value: {}", value))?;
            }
            "default_max_tokens" => {
                self.default_max_tokens = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid max tokens value: {}", value))?;
            }
            "timeout_secs" => {
                self.timeout_secs = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid timeout value: {}", value))?;
            }
            "max_retries" => {
                self.max_retries = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid max retries value: {}", value))?;
            }

            // General settings
            "general.preview_features" => {
                self.general.preview_features = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "general.preferred_editor" => {
                self.general.preferred_editor = value.to_string();
            }
            "general.vim_mode" => {
                self.general.vim_mode = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "general.disable_auto_update" => {
                self.general.disable_auto_update = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "general.disable_update_nag" => {
                self.general.disable_update_nag = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "general.enable_prompt_completion" => {
                self.general.enable_prompt_completion = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "general.retry_fetch_errors" => {
                self.general.retry_fetch_errors = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "general.debug_keystroke_logging" => {
                self.general.debug_keystroke_logging = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }

            // UI settings
            "ui.colors" => {
                self.ui.colors = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.progress_bars" => {
                self.ui.progress_bars = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.verbose_errors" => {
                self.ui.verbose_errors = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.terminal_width" => {
                self.ui.terminal_width = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "ui.unicode" => {
                self.ui.unicode = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.theme" => {
                self.ui.theme = value.to_string();
            }
            "ui.hide_window_title" => {
                self.ui.hide_window_title = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.show_status_in_title" => {
                self.ui.show_status_in_title = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.hide_tips" => {
                self.ui.hide_tips = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.hide_banner" => {
                self.ui.hide_banner = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.hide_context_summary" => {
                self.ui.hide_context_summary = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.hide_footer" => {
                self.ui.hide_footer = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.show_memory_usage" => {
                self.ui.show_memory_usage = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.show_line_numbers" => {
                self.ui.show_line_numbers = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.show_citations" => {
                self.ui.show_citations = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.show_model_info_in_chat" => {
                self.ui.show_model_info_in_chat = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.use_full_width" => {
                self.ui.use_full_width = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.use_alternate_buffer" => {
                self.ui.use_alternate_buffer = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.incremental_rendering" => {
                self.ui.incremental_rendering = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.accessibility.disable_loading_phrases" => {
                self.ui.accessibility.disable_loading_phrases = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.accessibility.screen_reader" => {
                self.ui.accessibility.screen_reader = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.footer.hide_cwd" => {
                self.ui.footer.hide_cwd = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.footer.hide_sandbox_status" => {
                self.ui.footer.hide_sandbox_status = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.footer.hide_model_info" => {
                self.ui.footer.hide_model_info = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "ui.footer.hide_context_percentage" => {
                self.ui.footer.hide_context_percentage = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }

            // Model settings
            "model.name" => {
                self.model.name = value.to_string();
            }
            "model.max_session_turns" => {
                self.model.max_session_turns = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "model.summarize_tool_output" => {
                self.model.summarize_tool_output = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "model.compression_threshold" => {
                self.model.compression_threshold = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "model.skip_next_speaker_check" => {
                self.model.skip_next_speaker_check = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }

            // Context settings
            "context.discovery_max_dirs" => {
                self.context.discovery_max_dirs = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "context.load_memory_from_include_directories" => {
                self.context.load_memory_from_include_directories = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "context.file_filtering.respect_git_ignore" => {
                self.context.file_filtering.respect_git_ignore = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "context.file_filtering.respect_grok_ignore" => {
                self.context.file_filtering.respect_grok_ignore = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "context.file_filtering.enable_recursive_file_search" => {
                self.context.file_filtering.enable_recursive_file_search = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "context.file_filtering.disable_fuzzy_search" => {
                self.context.file_filtering.disable_fuzzy_search = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }

            // Tools settings
            "tools.shell.enable_interactive_shell" => {
                self.tools.shell.enable_interactive_shell = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "tools.shell.show_color" => {
                self.tools.shell.show_color = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "tools.auto_accept" => {
                self.tools.auto_accept = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "tools.use_ripgrep" => {
                self.tools.use_ripgrep = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "tools.enable_tool_output_truncation" => {
                self.tools.enable_tool_output_truncation = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "tools.truncate_tool_output_threshold" => {
                self.tools.truncate_tool_output_threshold = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "tools.truncate_tool_output_lines" => {
                self.tools.truncate_tool_output_lines = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "tools.enable_message_bus_integration" => {
                self.tools.enable_message_bus_integration = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }

            // Security settings
            "security.disable_yolo_mode" => {
                self.security.disable_yolo_mode = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "security.enable_permanent_tool_approval" => {
                self.security.enable_permanent_tool_approval = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "security.block_git_extensions" => {
                self.security.block_git_extensions = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "security.folder_trust.enabled" => {
                self.security.folder_trust.enabled = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "security.environment_variable_redaction.enabled" => {
                self.security.environment_variable_redaction.enabled = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }

            // Experimental settings
            "experimental.enable_agents" => {
                self.experimental.enable_agents = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "experimental.extension_management" => {
                self.experimental.extension_management = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "experimental.jit_context" => {
                self.experimental.jit_context = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "experimental.codebase_investigator_settings.enabled" => {
                self.experimental.codebase_investigator_settings.enabled = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "experimental.codebase_investigator_settings.max_num_turns" => {
                self.experimental
                    .codebase_investigator_settings
                    .max_num_turns = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }

            // ACP settings
            "acp.enabled" => {
                self.acp.enabled = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean value: {}", value))?;
            }
            "acp.bind_host" => {
                self.acp.bind_host = value.to_string();
            }
            "acp.protocol_version" => {
                self.acp.protocol_version = value.to_string();
            }
            "acp.dev_mode" => {
                self.acp.dev_mode = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean value: {}", value))?;
            }
            "acp.default_port" => {
                self.acp.default_port = if value.is_empty() {
                    None
                } else {
                    Some(
                        value
                            .parse()
                            .map_err(|_| anyhow!("Invalid port value: {}", value))?,
                    )
                };
            }

            // Network settings
            "network.starlink_optimizations" => {
                self.network.starlink_optimizations = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean value: {}", value))?;
            }
            "network.base_retry_delay" => {
                self.network.base_retry_delay = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "network.max_retry_delay" => {
                self.network.max_retry_delay = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "network.health_monitoring" => {
                self.network.health_monitoring = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "network.connect_timeout" => {
                self.network.connect_timeout = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "network.read_timeout" => {
                self.network.read_timeout = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }

            // Logging settings
            "logging.level" => {
                let valid_levels = ["trace", "debug", "info", "warn", "error"];
                if valid_levels.contains(&value) {
                    self.logging.level = value.to_string();
                } else {
                    return Err(anyhow!(
                        "Invalid log level. Must be one of: {}",
                        valid_levels.join(", ")
                    ));
                }
            }
            "logging.file_logging" => {
                self.logging.file_logging = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "logging.max_file_size_mb" => {
                self.logging.max_file_size_mb = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }
            "logging.rotation_count" => {
                self.logging.rotation_count = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
            }

            // Telemetry settings
            "telemetry.enabled" => {
                self.telemetry.enabled = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }

            _ => return Err(anyhow!("Unknown configuration key: {}", key)),
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
        assert_eq!(config.default_model, "grok-4-1-fast-reasoning");
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
        assert_eq!(
            config.get_value("default_model").unwrap(),
            "grok-4-1-fast-reasoning"
        );
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
}
