// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

//! Config command handler for grok-cli
//!
//! Handles configuration management operations including showing, setting,
//! getting, initializing, and validating configuration settings.

use anyhow::{Result, anyhow};
use colored::*;

use crate::ConfigAction;
use crate::cli::{confirm, print_error, print_info, print_success, print_warning};
use crate::config::Config;

/// Handle configuration-related commands
pub async fn handle_config_action(action: ConfigAction, config: &Config) -> Result<()> {
    match action {
        ConfigAction::Show => show_config(config).await,
        ConfigAction::Set { key, value } => set_config_value(&key, &value).await,
        ConfigAction::Get { key } => get_config_value(&key).await,
        ConfigAction::Init { force } => init_config(force).await,
        ConfigAction::Validate => validate_config().await,
    }
}

/// Show current configuration
async fn show_config(config: &Config) -> Result<()> {
    println!("{}", "⚙️  Grok CLI Configuration".cyan().bold());
    println!();

    // API Configuration
    println!("{}", "API Configuration:".green().bold());
    let api_key_display = if config.api_key.is_some() {
        "✓ Set (hidden)".green()
    } else {
        "✗ Not set".red()
    };
    println!("  API Key: {}", api_key_display);
    println!("  Default Model: {}", config.default_model.cyan());
    println!("  Temperature: {}", config.default_temperature);
    println!("  Max Tokens: {}", config.default_max_tokens);
    println!("  Timeout: {}s", config.timeout_secs);
    println!("  Max Retries: {}", config.max_retries);
    println!();

    // ACP Configuration
    println!("{}", "ACP Configuration:".green().bold());
    let acp_status = if config.acp.enabled {
        "✓ Enabled".green()
    } else {
        "✗ Disabled".red()
    };
    println!("  Status: {}", acp_status);
    println!("  Bind Host: {}", config.acp.bind_host);
    let port_display = config
        .acp
        .default_port
        .map(|p| p.to_string())
        .unwrap_or_else(|| "Auto-assign".to_string());
    println!("  Default Port: {}", port_display);
    println!("  Protocol Version: {}", config.acp.protocol_version);
    let dev_mode = if config.acp.dev_mode {
        "✓ Enabled".yellow()
    } else {
        "✗ Disabled".dimmed()
    };
    println!("  Dev Mode: {}", dev_mode);
    println!(
        "  Max Tool Loop Iterations: {}",
        config.acp.max_tool_loop_iterations
    );
    println!();

    // Network Configuration
    println!("{}", "Network Configuration:".green().bold());
    let starlink_opt = if config.network.starlink_optimizations {
        "✓ Enabled".green()
    } else {
        "✗ Disabled".red()
    };
    println!("  Starlink Optimizations: {}", starlink_opt);
    println!("  Base Retry Delay: {}s", config.network.base_retry_delay);
    println!("  Max Retry Delay: {}s", config.network.max_retry_delay);
    let health_monitoring = if config.network.health_monitoring {
        "✓ Enabled".green()
    } else {
        "✗ Disabled".dimmed()
    };
    println!("  Health Monitoring: {}", health_monitoring);
    println!("  Connect Timeout: {}s", config.network.connect_timeout);
    println!("  Read Timeout: {}s", config.network.read_timeout);
    println!();

    // UI Configuration
    println!("{}", "UI Configuration:".green().bold());
    let colors = if config.ui.colors {
        "✓ Enabled".green()
    } else {
        "✗ Disabled".dimmed()
    };
    println!("  Colors: {}", colors);
    let progress = if config.ui.progress_bars {
        "✓ Enabled".green()
    } else {
        "✗ Disabled".dimmed()
    };
    println!("  Progress Bars: {}", progress);
    let verbose_errors = if config.ui.verbose_errors {
        "✓ Enabled".yellow()
    } else {
        "✗ Disabled".dimmed()
    };
    println!("  Verbose Errors: {}", verbose_errors);
    let terminal_width = if config.ui.terminal_width == 0 {
        "Auto-detect".to_string()
    } else {
        config.ui.terminal_width.to_string()
    };
    println!("  Terminal Width: {}", terminal_width);
    let unicode = if config.ui.unicode {
        "✓ Enabled".green()
    } else {
        "✗ Disabled".dimmed()
    };
    println!("  Unicode: {}", unicode);
    println!();

    // Logging Configuration
    println!("{}", "Logging Configuration:".green().bold());
    println!("  Level: {}", config.logging.level.cyan());
    let file_logging = if config.logging.file_logging {
        "✓ Enabled".green()
    } else {
        "✗ Disabled".dimmed()
    };
    println!("  File Logging: {}", file_logging);
    if let Some(ref log_file) = config.logging.log_file {
        println!("  Log File: {}", log_file.display());
    }
    println!("  Max File Size: {} MB", config.logging.max_file_size_mb);
    println!("  Rotation Count: {}", config.logging.rotation_count);
    println!();

    // Configuration source information
    println!("{}", "Configuration Source:".green().bold());
    if let Some(ref source) = config.config_source {
        println!("  {}", source.display());
    } else {
        println!("  Unknown");
    }

    Ok(())
}

/// Set a configuration value
async fn set_config_value(key: &str, value: &str) -> Result<()> {
    print_info(&format!(
        "Setting configuration: {} = {}",
        key.cyan(),
        value.yellow()
    ));

    // Load current config
    let mut config = Config::load(None).await?;

    // Set the value
    config
        .set_value(key, value)
        .map_err(|e| anyhow!("Failed to set configuration value: {}", e))?;

    // Validate the updated config
    if let Err(e) = config.validate() {
        print_error(&format!("Invalid configuration value: {}", e));
        return Err(e);
    }

    // Save the config
    config
        .save(None)
        .await
        .map_err(|e| anyhow!("Failed to save configuration: {}", e))?;

    print_success(&format!("Configuration updated: {} = {}", key, value));

    // Show a relevant tip based on the key that was set
    show_config_tip(key);

    Ok(())
}

/// Get a configuration value
async fn get_config_value(key: &str) -> Result<()> {
    print_info(&format!("Getting configuration value for: {}", key.cyan()));

    // Load current config
    let config = Config::load(None).await?;

    // Get the value
    match config.get_value(key) {
        Ok(value) => {
            if key == "api_key" && !value.is_empty() {
                println!("{}: {}", key.cyan(), "*** (hidden) ***".dimmed());
            } else {
                println!("{}: {}", key.cyan(), value.yellow());
            }
        }
        Err(e) => {
            print_error(&format!("Configuration key not found: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Initialize configuration with defaults
async fn init_config(force: bool) -> Result<()> {
    print_info("Initializing Grok CLI configuration...");

    if !force {
        let config_path = Config::default_config_path()?;
        if config_path.exists() {
            print_warning("Configuration file already exists!");
            println!("  Path: {}", config_path.display());

            if !confirm("Do you want to overwrite the existing configuration?")? {
                print_info("Configuration initialization cancelled.");
                return Ok(());
            }
        }
    }

    match Config::init(force).await {
        Ok(config_path) => {
            print_success("Configuration initialized successfully!");
            println!("  Path: {}", config_path.display());
            println!();
            print_info("Next steps:");
            println!(
                "  1. Set your X API key: {}",
                "grok config set api_key YOUR_API_KEY".yellow()
            );
            println!("  2. Verify configuration: {}", "grok config show".yellow());
            println!("  3. Test connection: {}", "grok health --api".yellow());
        }
        Err(e) => {
            print_error(&format!("Failed to initialize configuration: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Validate current configuration
async fn validate_config() -> Result<()> {
    print_info("Validating configuration...");

    let config = Config::load(None).await?;

    match config.validate() {
        Ok(()) => {
            print_success("Configuration is valid!");

            // Additional checks
            let mut warnings = Vec::new();
            let mut suggestions = Vec::new();

            // Check API key
            if config.api_key.is_none() {
                warnings.push("No API key configured".to_string());
                suggestions
                    .push("Set your X API key with: grok config set api_key YOUR_KEY".to_string());
            }

            // Check network settings for Starlink
            if config.network.starlink_optimizations {
                print_info("Starlink optimizations are enabled");
                if config.network.base_retry_delay < 2 {
                    suggestions.push("Consider increasing base_retry_delay to 2+ seconds for satellite connections".to_string());
                }
            }

            // Check ACP settings
            if config.acp.enabled {
                print_info("ACP (Zed integration) is enabled");
                if let Some(port) = config.acp.default_port
                    && port < 1024
                {
                    warnings.push(format!("ACP port {} may require elevated privileges", port));
                }
            }

            // Display warnings and suggestions
            if !warnings.is_empty() {
                println!();
                println!("{}", "⚠️  Warnings:".yellow().bold());
                for warning in warnings {
                    println!("  • {}", warning.yellow());
                }
            }

            if !suggestions.is_empty() {
                println!();
                println!("{}", "💡 Suggestions:".blue().bold());
                for suggestion in suggestions {
                    println!("  • {}", suggestion);
                }
            }
        }
        Err(e) => {
            print_error(&format!("Configuration validation failed: {}", e));

            println!();
            print_info("To fix configuration issues:");
            println!("  1. Check values with: {}", "grok config show".yellow());
            println!(
                "  2. Reset to defaults: {}",
                "grok config init --force".yellow()
            );
            println!(
                "  3. Set values manually: {}",
                "grok config set <key> <value>".yellow()
            );

            return Err(e);
        }
    }

    Ok(())
}

/// Show a helpful tip based on the configuration key that was set
fn show_config_tip(key: &str) {
    match key {
        "api_key" => {
            print_info("💡 Test your API key with: grok health --api");
        }
        "acp.enabled" => {
            print_info("💡 Start ACP server for Zed integration with: grok acp server");
        }
        "network.starlink_optimizations" => {
            print_info("💡 Starlink optimizations help with satellite network instability");
        }
        "default_model" => {
            print_info("💡 Available models: grok-2-latest, grok-2, grok-1");
        }
        "logging.level" => {
            print_info("💡 Valid log levels: trace, debug, info, warn, error");
        }
        _ => {}
    }
}

/// List all available configuration keys
pub fn list_config_keys() -> Vec<(&'static str, &'static str)> {
    vec![
        ("api_key", "X API key for Grok access"),
        ("default_model", "Default model to use"),
        ("default_temperature", "Default temperature (0.0-2.0)"),
        ("default_max_tokens", "Default maximum tokens"),
        ("timeout_secs", "Request timeout in seconds"),
        ("max_retries", "Maximum retry attempts"),
        ("acp.enabled", "Enable ACP functionality"),
        ("acp.bind_host", "ACP server bind host"),
        (
            "network.starlink_optimizations",
            "Enable Starlink optimizations",
        ),
        ("ui.colors", "Enable colored output"),
        ("ui.progress_bars", "Enable progress bars"),
        ("ui.verbose_errors", "Show detailed errors"),
        ("ui.unicode", "Enable Unicode characters"),
        ("logging.level", "Log level (trace/debug/info/warn/error)"),
        ("logging.file_logging", "Enable file logging"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_config_keys() {
        let keys = list_config_keys();
        assert!(!keys.is_empty());

        // Check that we have the essential keys
        assert!(keys.iter().any(|(key, _)| *key == "api_key"));
        assert!(keys.iter().any(|(key, _)| *key == "default_model"));
        assert!(keys.iter().any(|(key, _)| *key == "acp.enabled"));
    }

    #[test]
    fn test_config_tip_coverage() {
        // Test that show_config_tip doesn't panic for various keys
        show_config_tip("api_key");
        show_config_tip("unknown_key");
        show_config_tip("acp.enabled");
    }
}
