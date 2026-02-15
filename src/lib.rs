//! Grok CLI Library
//!
//! This library provides the core functionality for the Grok CLI,
//! including API integration, configuration management, and display utilities.
//!
//! # Architecture Notes
//!
//! ## Library vs Binary Separation
//!
//! This crate contains both library and binary code. According to Rust best practices,
//! libraries should NOT contain:
//! - Terminal I/O operations (println!, eprintln!, print!, etc.)
//! - Progress bars (indicatif)
//! - Terminal UI (ratatui, crossterm)
//! - Direct runtime dependencies (#[tokio::main])
//! - Code that exits the process
//!
//! ## Current State
//!
//! The following modules currently violate library/binary separation:
//! - `cli::mod` - Contains I/O helper functions (marked deprecated)
//! - `cli::commands::*` - Command handlers print directly to stdout/stderr
//! - `display` - Some functions perform direct I/O
//!
//! ## Migration Path
//!
//! 1. ✅ Created `src/terminal/` module for binary-only I/O (not exposed in lib.rs)
//! 2. ⏳ TODO: Refactor command handlers to return Result<DisplayData> instead of printing
//! 3. ⏳ TODO: Move `cli::app` and command dispatch to binary crate
//! 4. ⏳ TODO: Make all `display` functions pure (return String, no I/O)
//!
//! For now, I/O functions are marked with `#[deprecated]` to indicate they should
//! be moved to the binary crate in a future refactor.

use clap::Subcommand;

pub mod acp;
pub mod cli;
pub mod config;
pub mod display;
pub mod grok_client_ext;
pub mod hooks;
pub mod mcp;
pub mod skills;
pub mod utils;

// Re-export grok_api types for use throughout the crate
pub use grok_api::{
    ChatResponse, Choice, Error as GrokApiError, FunctionCall, Message, ToolCall, Usage,
};

// Re-export the extended GrokClient and types
pub use grok_client_ext::{GrokClient, MessageWithFinishReason};

/// Helper function to extract text content from String
/// Kept for backwards compatibility with refactored code
pub fn extract_text_content(content: &str) -> String {
    content.to_string()
}

/// Helper function to convert Option<String> to String
/// Kept for backwards compatibility with refactored code
pub fn content_to_string(content: Option<&String>) -> String {
    content.cloned().unwrap_or_default()
}

/// Helper function to create text content
/// Kept for backwards compatibility with refactored code
pub fn text_content(text: impl Into<String>) -> String {
    text.into()
}

#[derive(Subcommand, Clone, Debug)]
pub enum CodeAction {
    /// Explain code functionality
    Explain {
        /// File path or code snippet
        input: String,
        /// Input is a file path (default: auto-detect)
        #[arg(short, long)]
        file: bool,
    },
    /// Review code for improvements
    Review {
        /// File path or code snippet
        input: String,
        /// Input is a file path (default: auto-detect)
        #[arg(short, long)]
        file: bool,
        /// Focus on specific aspects (security, performance, style, etc.)
        #[arg(long)]
        focus: Option<String>,
    },
    /// Generate code from description
    Generate {
        /// Description of what to generate
        description: Vec<String>,
        /// Programming language
        #[arg(short, long)]
        language: Option<String>,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Fix code issues
    Fix {
        /// File path containing code to fix
        file: String,
        /// Description of the issue to fix
        issue: Vec<String>,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum AcpAction {
    /// Start ACP server for Zed integration
    Server {
        /// Port to bind to (default: auto-assign)
        #[arg(short, long)]
        port: Option<u16>,
        /// Host to bind to
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Start ACP in stdio mode (default for Zed)
    Stdio {
        /// Model to use (overrides default)
        #[arg(long)]
        model: Option<String>,
    },
    /// Test ACP connection
    Test {
        /// ACP server address
        #[arg(short, long)]
        address: String,
    },
    /// Show ACP capabilities
    Capabilities,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// Initialize configuration with defaults
    Init {
        /// Force overwrite existing config
        #[arg(long)]
        force: bool,
    },
    /// Validate configuration
    Validate,
}

#[derive(Subcommand, Clone, Debug)]
pub enum SettingsAction {
    /// Show interactive settings browser
    Show,
    /// Edit settings interactively
    Edit,
    /// Reset settings to defaults
    Reset {
        /// Category to reset (optional, resets all if not specified)
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Export settings to file
    Export {
        /// Export file path
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Import settings from file
    Import {
        /// Import file path
        #[arg(short, long)]
        path: String,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum HistoryAction {
    /// List all chat sessions
    List,
    /// View a specific chat session
    View {
        /// Session ID to view
        session_id: String,
    },
    /// Search through chat sessions
    Search {
        /// Search query
        query: String,
    },
    /// Clear chat history
    Clear {
        /// Confirm deletion
        #[arg(long)]
        confirm: bool,
    },
}

// Re-export commonly used types and functions
pub use config::{Config, ConfigSource, RateLimitConfig};
pub use display::{
    ascii_art::{get_logo_for_width, print_grok_logo},
    banner::{BannerConfig, BannerType, print_banner, print_welcome_banner},
    get_terminal_size,
};
