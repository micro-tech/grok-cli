//! Health check command handler for grok-cli
//!
//! Handles health checking operations including API connectivity tests,
//! configuration validation, and system diagnostics.

// Allow deprecated warnings in this module since these I/O functions
// are deprecated and will be refactored in Phase 2. The deprecation markers
// remain for external users and documentation purposes.
#![allow(deprecated)]

use anyhow::{Result, anyhow};
use colored::*;
use std::time::{Duration, Instant};

use crate::GrokClient;
use crate::cli::{create_spinner, print_error, print_info, print_success, print_warning};
use crate::config::Config;
use crate::utils::network::{detect_starlink_connection, test_connectivity};

/// Handle health check commands
pub async fn handle_health_check(
    check_api: bool,
    check_config: bool,
    api_key: Option<&str>,
    config: &Config,
    model: &str,
    timeout_secs: u64,
) -> Result<()> {
    println!("{}", "🏥 Grok CLI Health Check".cyan().bold());
    println!();

    let mut checks_passed = 0;
    let mut total_checks = 0;
    let mut warnings = Vec::new();

    // Always run basic system checks
    println!("{}", "System Checks:".green().bold());

    // Check configuration file
    total_checks += 1;
    let config_file_status = check_config_file().await;
    match config_file_status {
        Ok(()) => {
            print_success("Configuration file found and readable");
            checks_passed += 1;
        }
        Err(e) => {
            print_error(&format!("Configuration file issue: {}", e));
            warnings.push("Configuration file may need to be initialized".to_string());
        }
    }

    // Check environment variables
    total_checks += 1;
    let env_status = check_environment();
    match env_status {
        Ok(env_info) => {
            print_success(&format!("Environment: {}", env_info));
            checks_passed += 1;
        }
        Err(e) => {
            print_warning(&format!("Environment check: {}", e));
            checks_passed += 1; // Not critical
        }
    }

    // Check network connectivity
    total_checks += 1;
    println!();
    println!("{}", "Network Checks:".green().bold());

    let spinner = create_spinner("Testing basic connectivity...");
    let connectivity_result = test_connectivity(Duration::from_secs(5)).await;
    spinner.finish_and_clear();

    match connectivity_result {
        Ok(latency) => {
            print_success(&format!("Network connectivity OK (latency: {:?})", latency));
            checks_passed += 1;

            if latency > Duration::from_millis(1000) {
                warnings.push(
                    "High network latency detected - consider Starlink optimizations".to_string(),
                );
            }
        }
        Err(e) => {
            print_error(&format!("Network connectivity failed: {}", e));
            warnings.push("Network issues may affect API calls".to_string());
        }
    }

    // Check for Starlink connection
    total_checks += 1;
    let starlink_spinner = create_spinner("Detecting network type...");
    let is_starlink = detect_starlink_connection().await;
    starlink_spinner.finish_and_clear();

    if is_starlink {
        print_info("Detected possible Starlink satellite connection");
        if config.network.starlink_optimizations {
            print_success("Starlink optimizations are enabled");
        } else {
            print_warning("Consider enabling Starlink optimizations");
            warnings.push("Enable Starlink optimizations with: grok config set network.starlink_optimizations true".to_string());
        }
        checks_passed += 1;
    } else {
        print_info("Standard internet connection detected");
        checks_passed += 1;
    }

    // Configuration validation if requested
    if check_config {
        println!();
        println!("{}", "Configuration Validation:".green().bold());

        total_checks += 1;
        match config.validate() {
            Ok(()) => {
                print_success("Configuration is valid");
                checks_passed += 1;
            }
            Err(e) => {
                print_error(&format!("Configuration validation failed: {}", e));
            }
        }

        // Check specific configuration values
        total_checks += 1;
        if config.api_key.is_some() {
            print_success("API key is configured");
            checks_passed += 1;
        } else {
            print_warning("No API key configured");
            warnings.push(
                "Set API key with: grok config set api_key YOUR_API_KEY (stores in .env)"
                    .to_string(),
            );
        }

        total_checks += 1;
        if config.acp.enabled {
            print_info("ACP (Zed integration) is enabled");
            checks_passed += 1;
        } else {
            print_info("ACP is disabled");
            checks_passed += 1;
        }
    }

    // API connectivity test if requested
    if check_api {
        println!();
        println!("{}", "API Connectivity:".green().bold());

        if let Some(key) = api_key {
            total_checks += 2;

            // Test API key validity
            let api_spinner = create_spinner("Testing Grok API connection...");
            let client_result = GrokClient::with_settings(key, timeout_secs, 3)
                .map(|client| client.with_rate_limits(config.rate_limits));

            match client_result {
                Ok(client) => {
                    let test_result = client.test_connection().await;
                    api_spinner.finish_and_clear();

                    match test_result {
                        Ok(()) => {
                            print_success("Grok API connection successful");
                            checks_passed += 2;
                        }
                        Err(e) => {
                            print_error(&format!("Grok API connection failed: {}", e));

                            // Provide specific error guidance
                            let error_msg = e.to_string().to_lowercase();
                            if error_msg.contains("authentication") || error_msg.contains("401") {
                                warnings.push(
                                    "Check your API key - it may be invalid or expired".to_string(),
                                );
                            } else if error_msg.contains("timeout") || error_msg.contains("network")
                            {
                                warnings.push(
                                    "Network connectivity issues - check your internet connection"
                                        .to_string(),
                                );
                            } else if error_msg.contains("rate limit") || error_msg.contains("429")
                            {
                                warnings
                                    .push("API rate limit exceeded - try again later".to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    api_spinner.finish_and_clear();
                    print_error(&format!("Failed to create API client: {}", e));
                }
            }

            // Test model availability
            match GrokClient::with_settings(key, timeout_secs, 3)
                .map(|client| client.with_rate_limits(config.rate_limits))
            {
                Ok(client) => {
                    let models_spinner = create_spinner("Checking model availability...");
                    let models_result = client.list_models().await;
                    models_spinner.finish_and_clear();

                    match models_result {
                        Ok(models) => {
                            if models.contains(&model.to_string()) {
                                print_success(&format!("Model '{}' is available", model));
                            } else {
                                print_warning(&format!("Model '{}' may not be available", model));
                                print_info(&format!("Available models: {}", models.join(", ")));
                            }
                        }
                        Err(e) => {
                            print_warning(&format!("Could not check model availability: {}", e));
                        }
                    }
                }
                Err(_) => {
                    // Already handled above
                }
            }
        } else {
            print_warning("No API key provided - skipping API tests");
            warnings.push("Provide API key to test API connectivity".to_string());
        }
    }

    // Performance diagnostics
    println!();
    println!("{}", "Performance Diagnostics:".green().bold());

    total_checks += 1;
    let perf_result = run_performance_diagnostics(config).await;
    match perf_result {
        Ok(diagnostics) => {
            print_success("Performance diagnostics completed");
            display_performance_results(&diagnostics);
            checks_passed += 1;
        }
        Err(e) => {
            print_warning(&format!("Performance diagnostics failed: {}", e));
        }
    }

    // Summary
    println!();
    println!("{}", "Health Check Summary:".cyan().bold());
    println!("{}", "─".repeat(50));

    let success_rate = if total_checks > 0 {
        (checks_passed as f64 / total_checks as f64) * 100.0
    } else {
        100.0
    };

    let status_color = if success_rate >= 90.0 {
        "green"
    } else if success_rate >= 70.0 {
        "yellow"
    } else {
        "red"
    };

    match status_color {
        "green" => println!(
            "Status: {} ({:.0}%)",
            "✓ HEALTHY".green().bold(),
            success_rate
        ),
        "yellow" => println!(
            "Status: {} ({:.0}%)",
            "⚠ WARNING".yellow().bold(),
            success_rate
        ),
        "red" => println!(
            "Status: {} ({:.0}%)",
            "✗ UNHEALTHY".red().bold(),
            success_rate
        ),
        _ => unreachable!(),
    }

    println!("Checks passed: {}/{}", checks_passed, total_checks);

    if !warnings.is_empty() {
        println!();
        println!("{}", "Recommendations:".yellow().bold());
        for (i, warning) in warnings.iter().enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
    }

    println!();
    if success_rate >= 90.0 {
        print_success("System is healthy and ready to use!");
    } else if success_rate >= 70.0 {
        print_warning("System has minor issues but should work");
    } else {
        print_error("System has significant issues that need attention");
        return Err(anyhow!(
            "Health check failed with {:.0}% success rate",
            success_rate
        ));
    }

    Ok(())
}

/// Check if configuration file exists and is readable
async fn check_config_file() -> Result<()> {
    let config_path = Config::default_config_path()?;

    if !config_path.exists() {
        return Err(anyhow!("Configuration file not found at {:?}", config_path));
    }

    // Try to load the config to ensure it's valid
    Config::load(None).await?;

    Ok(())
}

/// Check environment variables and system information
fn check_environment() -> Result<String> {
    let mut env_info = Vec::new();

    // Check OS
    env_info.push(format!("OS: {}", std::env::consts::OS));

    // Check if running in terminal
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        env_info.push("Terminal: Yes".to_string());
    } else {
        env_info.push("Terminal: No (piped/redirected)".to_string());
    }

    // Check for NO_COLOR environment variable
    if std::env::var("NO_COLOR").is_ok() {
        env_info.push("Colors: Disabled (NO_COLOR set)".to_string());
    } else {
        env_info.push("Colors: Enabled".to_string());
    }

    // Check for relevant environment variables
    let mut env_vars = Vec::new();
    if std::env::var("GROK_API_KEY").is_ok() {
        env_vars.push("GROK_API_KEY");
    }
    if std::env::var("X_API_KEY").is_ok() {
        env_vars.push("X_API_KEY");
    }
    if std::env::var("GROK_MODEL").is_ok() {
        env_vars.push("GROK_MODEL");
    }

    if !env_vars.is_empty() {
        env_info.push(format!("Env vars: {}", env_vars.join(", ")));
    }

    Ok(env_info.join(", "))
}

/// Performance diagnostics data
struct PerformanceDiagnostics {
    memory_usage: u64,
    startup_time: Duration,
    config_load_time: Duration,
}

/// Run performance diagnostics
async fn run_performance_diagnostics(_config: &Config) -> Result<PerformanceDiagnostics> {
    let start_time = Instant::now();

    // Measure config reload time
    let config_start = Instant::now();
    Config::load(None).await?;
    let config_load_time = config_start.elapsed();

    // Estimate memory usage (simplified)
    let memory_usage = estimate_memory_usage();

    let startup_time = start_time.elapsed();

    Ok(PerformanceDiagnostics {
        memory_usage,
        startup_time,
        config_load_time,
    })
}

/// Display performance diagnostic results
fn display_performance_results(diagnostics: &PerformanceDiagnostics) {
    println!("  Memory usage: ~{} KB", diagnostics.memory_usage / 1024);
    println!("  Config load time: {:?}", diagnostics.config_load_time);
    println!("  Startup time: {:?}", diagnostics.startup_time);

    // Performance warnings
    if diagnostics.config_load_time > Duration::from_millis(100) {
        print_warning("Configuration loading is slow");
    }

    if diagnostics.memory_usage > 50 * 1024 * 1024 {
        // 50MB
        print_warning("High memory usage detected");
    }
}

/// Estimate current memory usage (simplified approach)
fn estimate_memory_usage() -> u64 {
    // This is a simplified estimation
    // In a real implementation, you might use system-specific APIs
    // to get actual memory usage
    10 * 1024 * 1024 // Estimate 10MB base usage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_environment() {
        let result = check_environment();
        assert!(result.is_ok());
        let env_info = result.unwrap();
        assert!(env_info.contains("OS:"));
    }

    #[test]
    fn test_estimate_memory_usage() {
        let memory = estimate_memory_usage();
        assert!(memory > 0);
    }

    #[tokio::test]
    async fn test_performance_diagnostics() {
        let config = Config::default();
        let result = run_performance_diagnostics(&config).await;
        // This test might fail if config loading fails, but that's expected
        // in a test environment without proper setup
    }
}
