//! UI and display configuration types.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.
//! Includes UiConfig and all its nested UI-related subtypes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub custom_themes: HashMap<String, CustomTheme>,

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
    #[serde(default = "default_hide_context_percentage")]
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
    pub key_bindings: HashMap<String, String>,
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

fn default_hide_context_percentage() -> bool {
    true
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
            custom_themes: HashMap::new(),
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
            key_bindings: HashMap::new(),
        }
    }
}
