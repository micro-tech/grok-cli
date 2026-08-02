//! Environment variable overrides for the configuration system.
//!
//! This module was extracted from the monolithic `apply_env_overrides`
//! implementation to reduce the size of `mod.rs` and improve maintainability.
//!
//! All environment variables are read here (highest priority after file loading).

use std::env;
use std::path::PathBuf;

use super::Config;

impl Config {
    /// Apply environment variable overrides.
    ///
    /// This function is invoked after loading configuration from files
    /// (system, project, or explicit) so that `GROK_*` and other environment
    /// variables always win.
    pub(super) fn apply_env_overrides(&mut self) {
        // API key from environment
        if let Ok(api_key) = env::var("GROK_API_KEY") {
            self.api_key = Some(api_key);
        } else if let Ok(api_key) = env::var("X_API_KEY") {
            self.api_key = Some(api_key);
        }

        // Model configuration
        if let Ok(model) = env::var("GROK_MODEL") {
            self.default_model = model;
        }

        if let Ok(temp) = env::var("GROK_TEMPERATURE") {
            if let Ok(temp_val) = temp.parse::<f32>() {
                self.default_temperature = temp_val;
            }
        }

        if let Ok(tokens) = env::var("GROK_MAX_TOKENS") {
            if let Ok(tokens_val) = tokens.parse::<u32>() {
                self.default_max_tokens = tokens_val;
            }
        }

        // Network configuration
        if let Ok(timeout) = env::var("GROK_TIMEOUT") {
            if let Ok(timeout_val) = timeout.parse::<u64>() {
                self.timeout_secs = timeout_val;
            }
        }

        if let Ok(retries) = env::var("GROK_MAX_RETRIES") {
            if let Ok(retries_val) = retries.parse::<u32>() {
                self.max_retries = retries_val;
            }
        }

        if let Ok(val) = env::var("GROK_STARLINK_OPTIMIZATIONS") {
            self.network.starlink_optimizations = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(delay) = env::var("GROK_BASE_RETRY_DELAY") {
            if let Ok(delay_val) = delay.parse::<u64>() {
                self.network.base_retry_delay = delay_val;
            }
        }

        if let Ok(delay) = env::var("GROK_MAX_RETRY_DELAY") {
            if let Ok(delay_val) = delay.parse::<u64>() {
                self.network.max_retry_delay = delay_val;
            }
        }

        if let Ok(val) = env::var("GROK_HEALTH_MONITORING") {
            self.network.health_monitoring = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(timeout) = env::var("GROK_CONNECT_TIMEOUT") {
            if let Ok(timeout_val) = timeout.parse::<u64>() {
                self.network.connect_timeout = timeout_val;
            }
        }

        if let Ok(timeout) = env::var("GROK_READ_TIMEOUT") {
            if let Ok(timeout_val) = timeout.parse::<u64>() {
                self.network.read_timeout = timeout_val;
            }
        }

        // UI configuration
        if let Ok(val) = env::var("GROK_COLORS") {
            self.ui.colors = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(val) = env::var("GROK_PROGRESS_BARS") {
            self.ui.progress_bars = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(val) = env::var("GROK_UNICODE") {
            self.ui.unicode = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(val) = env::var("GROK_VERBOSE_ERRORS") {
            self.ui.verbose_errors = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(width) = env::var("GROK_TERMINAL_WIDTH") {
            if let Ok(width_val) = width.parse::<usize>() {
                self.ui.terminal_width = width_val;
            }
        }

        // Disable colors if NO_COLOR is set
        if env::var("NO_COLOR").is_ok() {
            self.ui.colors = false;
        }

        // Logging configuration
        if let Ok(level) = env::var("GROK_LOG_LEVEL") {
            self.logging.level = level;
        }

        if let Ok(val) = env::var("GROK_FILE_LOGGING") {
            self.logging.file_logging = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(path) = env::var("GROK_LOG_FILE") {
            self.logging.log_file = Some(PathBuf::from(path));
        }

        if let Ok(size) = env::var("GROK_MAX_FILE_SIZE_MB") {
            if let Ok(size_val) = size.parse::<u64>() {
                self.logging.max_file_size_mb = size_val;
            }
        }

        if let Ok(count) = env::var("GROK_ROTATION_COUNT") {
            if let Ok(count_val) = count.parse::<u32>() {
                self.logging.rotation_count = count_val;
            }
        }

        // ACP configuration
        if let Ok(val) = env::var("GROK_ACP_ENABLED") {
            self.acp.enabled = val.parse::<bool>().unwrap_or(true);
        }

        if env::var("GROK_ACP_DISABLE").is_ok() {
            self.acp.enabled = false;
        }

        if let Ok(port) = env::var("GROK_ACP_PORT") {
            if let Ok(port_val) = port.parse::<u16>() {
                self.acp.default_port = Some(port_val);
            }
        }

        if let Ok(host) = env::var("GROK_ACP_BIND_HOST") {
            self.acp.bind_host = host;
        }

        if let Ok(version) = env::var("GROK_ACP_PROTOCOL_VERSION") {
            self.acp.protocol_version = version;
        }

        if let Ok(val) = env::var("GROK_ACP_DEV_MODE") {
            self.acp.dev_mode = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(iterations) = env::var("GROK_ACP_MAX_TOOL_LOOP_ITERATIONS") {
            if let Ok(iterations_val) = iterations.parse::<u32>() {
                self.acp.max_tool_loop_iterations = iterations_val;
            }
        }

        // Telemetry configuration
        if let Ok(val) = env::var("GROK_TELEMETRY_ENABLED") {
            self.telemetry.enabled = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(path) = env::var("GROK_TELEMETRY_LOG_FILE") {
            self.telemetry.log_file = Some(PathBuf::from(path));
        }

        // OKF configuration (Workflow Trace Forwarder)
        if let Ok(val) = env::var("GROK_OKF_ENABLED") {
            self.okf.enabled = val.parse::<bool>().unwrap_or(false);
        }
        if let Ok(s) = env::var("GROK_OKF_SERVER") {
            self.okf.server = s;
        }
        if let Ok(p) = env::var("GROK_OKF_PORT") {
            if let Ok(port_val) = p.parse::<u16>() {
                self.okf.port = port_val;
            }
        }
        if let Ok(key) = env::var("GROK_OKF_API_KEY") {
            self.okf.api_key = Some(key);
        }
        if let Ok(ep) = env::var("GROK_OKF_ENDPOINT") {
            self.okf.endpoint = ep;
        }
        if let Ok(val) = env::var("GROK_OKF_USE_HTTPS") {
            self.okf.use_https = val.parse::<bool>().unwrap_or(false);
        }
        if let Ok(t) = env::var("GROK_OKF_TIMEOUT") {
            if let Ok(t_val) = t.parse::<u64>() {
                self.okf.timeout_secs = t_val;
            }
        }
        if let Ok(val) = env::var("GROK_OKF_BUFFER_ON_FAILURE") {
            self.okf.buffer_on_failure = val.parse::<bool>().unwrap_or(true);
        }

        // OKF Knowledge Bundles (Open Knowledge Format - "Knowledge OS")
        if let Ok(bundles) = env::var("GROK_OKF_KNOWLEDGE_BUNDLES") {
            self.okf.knowledge_bundles = bundles
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if let Ok(url) = env::var("GROK_OKF_REMOTE_URL") {
            self.okf.remote_url = Some(url);
        }
        if let Ok(bundle) = env::var("GROK_OKF_DEFAULT_BUNDLE") {
            self.okf.default_bundle = bundle;
        }

        // Security configuration
        if let Ok(mode) = env::var("GROK_SHELL_APPROVAL_MODE") {
            self.security.shell_approval_mode = mode;
        }

        // External access configuration
        if let Ok(val) = env::var("GROK_EXTERNAL_ACCESS_ENABLED") {
            self.security.external_access.enabled = val.parse::<bool>().unwrap_or(false);
        }

        if let Ok(val) = env::var("GROK_EXTERNAL_ACCESS_REQUIRE_APPROVAL") {
            self.security.external_access.require_approval = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(val) = env::var("GROK_EXTERNAL_ACCESS_LOGGING") {
            self.security.external_access.logging = val.parse::<bool>().unwrap_or(true);
        }

        if let Ok(paths) = env::var("GROK_EXTERNAL_ACCESS_PATHS") {
            // Parse comma-separated list of paths
            let parsed_paths: Vec<PathBuf> = paths
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .collect();

            if !parsed_paths.is_empty() {
                self.security.external_access.allowed_paths = parsed_paths;
            }
        }
    }
}
