use anyhow::{anyhow, Result};
use colored::*;
use std::process;
use tracing::error;

use crate::config::Config;

/// Resolves the API key from various sources (CLI arg, config, env vars).
pub fn resolve_api_key(cli_key: Option<String>, config: &Config) -> Option<String> {
    cli_key
        .or(config.api_key.clone())
        .or_else(|| std::env::var("GROK_API_KEY").ok())
        .or_else(|| std::env::var("X_API_KEY").ok())
}

/// Ensures an API key is present, or exits with a helpful error message.
///
/// Returns the API key if present.
pub fn require_api_key(
    api_key: Option<String>,
    hide_banner: bool,
    show_banner_fn: impl FnOnce(),
) -> String {
    if let Some(key) = api_key {
        return key;
    }

    // Show welcome banner even without API key if requested
    if !hide_banner {
        show_banner_fn();
    }

    error!("No API key provided. Set GROK_API_KEY environment variable or use --api-key option");
    eprintln!("{}", "Error: No API key provided".red());
    eprintln!(
        "Set the {} environment variable or use the {} option",
        "GROK_API_KEY".yellow(),
        "--api-key".yellow()
    );
    process::exit(1);
}
