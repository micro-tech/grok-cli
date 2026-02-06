use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};

use crate::config::{Config, ConfigSource};
use crate::display::banner::{BannerConfig, print_welcome_banner};
use crate::display::interactive::{InteractiveConfig, PromptStyle, start_interactive_mode};
use crate::utils::auth::{require_api_key, resolve_api_key};
use crate::utils::network::test_connectivity;

/// Grok CLI - Command-line interface for Grok AI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// API key for authentication
    #[arg(short, long, env = "GROK_API_KEY")]
    pub api_key: Option<String>,

    /// Config file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Hide banner
    #[arg(long)]
    pub hide_banner: bool,

    /// Model to use
    #[arg(short, long)]
    pub model: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Chat with Grok AI
    Chat {
        /// The message to send
        #[arg(required = true)]
        message: Vec<String>,

        /// Start an interactive chat session
        #[arg(short, long)]
        interactive: bool,

        /// System prompt to use
        #[arg(short, long)]
        system: Option<String>,

        /// Temperature for response generation (0.0 to 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f32,

        /// Maximum tokens in response
        #[arg(long, default_value = "4096")]
        max_tokens: u32,
    },

    /// Code-related operations
    Code {
        #[command(subcommand)]
        action: crate::CodeAction,
    },

    /// ACP (Agent Client Protocol) operations for Zed integration
    Acp {
        #[command(subcommand)]
        action: crate::AcpAction,
    },

    /// Interactive chat mode
    Interactive,

    /// Send a single query
    Query {
        /// The question or prompt to send
        #[arg(required = true)]
        prompt: Vec<String>,
    },

    /// Test network connectivity
    TestNetwork {
        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: crate::ConfigAction,
    },

    /// Settings management and configuration
    Settings {
        #[command(subcommand)]
        action: crate::SettingsAction,
    },

    /// Chat history management
    History {
        #[command(subcommand)]
        action: crate::HistoryAction,
    },

    /// Health check and diagnostics
    Health {
        /// Check API connectivity
        #[arg(long)]
        api: bool,

        /// Check configuration
        #[arg(long)]
        config: bool,

        /// Check all systems
        #[arg(long)]
        all: bool,
    },

    /// Manage agent skills
    Skills {
        #[command(subcommand)]
        action: crate::cli::commands::skills::SkillsCommand,
    },
}

/// Main application entry point
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = if let Some(config_path) = &cli.config {
        // Use explicit config path if provided
        let path_str = config_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid config path: contains non-UTF8 characters"))?;
        Config::load(Some(path_str)).await?
    } else {
        // Use hierarchical loading: project → system → defaults
        match Config::load_hierarchical().await {
            Ok(cfg) => {
                info!("✓ Configuration loaded successfully");
                cfg
            }
            Err(e) => {
                error!("Failed to load hierarchical configuration: {}", e);
                error!("Falling back to default configuration");
                Config {
                    config_source: Some(ConfigSource::Default),
                    ..Config::default()
                }
            }
        }
    };

    // Initialize telemetry
    crate::utils::telemetry::init(config.telemetry.enabled, config.telemetry.log_file.clone());

    // Resolve API key once
    let api_key = resolve_api_key(cli.api_key.clone(), &config);

    // Resolve model: CLI argument overrides config
    let model = cli.model.as_deref().unwrap_or(&config.default_model);

    // Show banner function
    let show_banner_fn = || {
        let banner_config = BannerConfig {
            show_banner: true,
            show_tips: true,
            show_updates: true,
            width: None,
        };
        print_welcome_banner(&banner_config);
    };

    match &cli.command {
        Some(Commands::Chat {
            message,
            interactive,
            system,
            temperature,
            max_tokens,
        }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            crate::cli::commands::chat::handle_chat(crate::cli::commands::chat::ChatOptions {
                message: message.clone(),
                interactive: *interactive,
                system: system.clone(),
                temperature: *temperature,
                max_tokens: *max_tokens,
                api_key: &api_key,
                model,
                timeout_secs: config.timeout_secs,
                max_retries: config.max_retries,
                rate_limit_config: config.rate_limits,
            })
            .await?;
        }
        Some(Commands::Code { action }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            crate::cli::commands::code::handle_code_action(
                action.clone(),
                &api_key,
                model,
                config.timeout_secs,
                config.max_retries,
                config.rate_limits,
            )
            .await?;
        }
        Some(Commands::Acp { action }) => {
            crate::cli::commands::acp::handle_acp_action(action.clone(), &config).await?;
        }
        Some(Commands::Interactive) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let interactive_config = InteractiveConfig {
                show_banner: !cli.hide_banner,
                show_tips: true,
                show_status: true,
                auto_save_session: false,
                prompt_style: PromptStyle::Rich,
                check_directory: true,
            };
            start_interactive_mode(&api_key, model, &config, interactive_config).await?;
        }
        Some(Commands::Query { prompt }) => {
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let query = prompt.join(" ");

            if !cli.hide_banner {
                show_banner_fn();
            }

            info!("Sending query: {}", query);
            crate::cli::commands::chat::handle_chat(crate::cli::commands::chat::ChatOptions {
                message: vec![query],
                interactive: false,
                system: None,
                temperature: 0.7,
                max_tokens: 4096,
                api_key: &api_key,
                model,
                timeout_secs: config.timeout_secs,
                max_retries: config.max_retries,
                rate_limit_config: config.rate_limits,
            })
            .await?;
        }
        Some(Commands::TestNetwork { timeout }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            let timeout_duration = std::time::Duration::from_secs(*timeout);
            match test_connectivity(timeout_duration).await {
                Ok(duration) => {
                    println!("✓ Network connectivity test passed in {:?}", duration);
                }
                Err(e) => {
                    error!("Network connectivity test failed: {}", e);
                    return Err(e);
                }
            }
        }
        Some(Commands::Config { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::config::handle_config_action(action.clone(), &config).await?;
        }
        Some(Commands::Settings { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::settings::handle_settings_action(action.clone(), &config).await?;
        }
        Some(Commands::History { action }) => {
            if !cli.hide_banner {
                show_banner_fn();
            }
            crate::cli::commands::history::handle_history_action(action.clone()).await?;
        }
        Some(Commands::Health {
            api,
            config: check_config,
            all,
        }) => {
            let check_api = *api || *all;
            let check_cfg = *check_config || *all;
            crate::cli::commands::health::handle_health_check(
                check_api,
                check_cfg,
                api_key.as_deref(),
                &config,
                model,
                config.timeout_secs,
            )
            .await?;
        }
        Some(Commands::Skills { action }) => {
            crate::cli::commands::skills::handle_skills_command(action.clone()).await?;
        }
        None => {
            // Default to interactive mode
            let api_key = require_api_key(api_key, cli.hide_banner, show_banner_fn);
            let interactive_config = InteractiveConfig {
                show_banner: !cli.hide_banner,
                show_tips: true,
                show_status: true,
                auto_save_session: false,
                prompt_style: PromptStyle::Rich,
                check_directory: true,
            };
            start_interactive_mode(&api_key, model, &config, interactive_config).await?;
        }
    }

    Ok(())
}
