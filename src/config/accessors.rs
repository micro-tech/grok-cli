//! Dynamic accessors for getting/setting `Config` values by string key path.
//!
//! These were extracted from the monolithic `impl Config` in `mod.rs`
//! to keep the core module manageable. They power the `grok config get` /
//! `grok config set` commands and similar dynamic access.
//!
//! The match arms are intentionally exhaustive over all known config keys.

use anyhow::{Result, anyhow};

use super::Config;

impl Config {
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
            "tools.enable_hooks" => Ok(self.tools.enable_hooks.to_string()),

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
            "experimental.extensions.enabled" => {
                Ok(self.experimental.extensions.enabled.to_string())
            }
            "experimental.extensions.extension_dir" => Ok(self
                .experimental
                .extensions
                .extension_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()),
            "experimental.extensions.enabled_extensions" => {
                Ok(self.experimental.extensions.enabled_extensions.join(", "))
            }

            // Bayesian router settings
            "bayesian.enabled" => Ok(self.bayesian.enabled.to_string()),
            "bayesian.show_belief_graph" => Ok(self.bayesian.show_belief_graph.to_string()),
            "bayesian.clarification_threshold" => {
                Ok(self.bayesian.clarification_threshold.to_string())
            }
            "bayesian.uncertainty_threshold" => Ok(self.bayesian.uncertainty_threshold.to_string()),
            "bayesian.vagueness_threshold" => Ok(self.bayesian.vagueness_threshold.to_string()),
            "bayesian.intent_likelihood_weight" => {
                Ok(self.bayesian.intent_likelihood_weight.to_string())
            }
            "bayesian.profile_learning_rate" => Ok(self.bayesian.profile_learning_rate.to_string()),
            "bayesian.priors.intent_edit" => Ok(self.bayesian.priors.intent_edit.to_string()),
            "bayesian.priors.intent_shell" => Ok(self.bayesian.priors.intent_shell.to_string()),
            "bayesian.priors.intent_search" => Ok(self.bayesian.priors.intent_search.to_string()),
            "bayesian.priors.intent_question" => {
                Ok(self.bayesian.priors.intent_question.to_string())
            }
            "bayesian.priors.need_clarification" => {
                Ok(self.bayesian.priors.need_clarification.to_string())
            }
            "bayesian.priors.low_confidence" => Ok(self.bayesian.priors.low_confidence.to_string()),
            "bayesian.priors.is_vague" => Ok(self.bayesian.priors.is_vague.to_string()),

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
            "acp.require_permission" => Ok(self.acp.require_permission.to_string()),
            "acp.permission_timeout_secs" => Ok(self.acp.permission_timeout_secs.to_string()),
            "acp.max_tool_loop_iterations" => Ok(self.acp.max_tool_loop_iterations.to_string()),
            "acp.max_history_messages" => Ok(self.acp.max_history_messages.to_string()),
            "acp.thinking_mode" => Ok(format!("{:?}", self.acp.thinking_mode).to_lowercase()),
            "acp.stream_thinking" => Ok(self.acp.stream_thinking.to_string()),

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

            // OKF (Workflow Trace Forwarder) settings
            "okf.enabled" => Ok(self.okf.enabled.to_string()),
            "okf.server" => Ok(self.okf.server.clone()),
            "okf.port" => Ok(self.okf.port.to_string()),
            "okf.endpoint" => Ok(self.okf.endpoint.clone()),
            "okf.use_https" => Ok(self.okf.use_https.to_string()),
            "okf.timeout_secs" => Ok(self.okf.timeout_secs.to_string()),
            "okf.buffer_on_failure" => Ok(self.okf.buffer_on_failure.to_string()),
            "okf.max_buffer_size" => Ok(self.okf.max_buffer_size.to_string()),
            "okf.knowledge_bundles" => Ok(self.okf.knowledge_bundles.join(", ")),
            "okf.remote_url" => Ok(self.okf.remote_url.clone().unwrap_or_default()),
            "okf.default_bundle" => Ok(self.okf.default_bundle.clone()),

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
            "tools.enable_hooks" => {
                self.tools.enable_hooks = value
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
            "experimental.extensions.enabled" => {
                self.experimental.extensions.enabled = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "experimental.extensions.extension_dir" => {
                self.experimental.extensions.extension_dir = if value.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(value))
                };
            }
            "experimental.extensions.enabled_extensions" => {
                self.experimental.extensions.enabled_extensions = if value.is_empty() {
                    vec![]
                } else {
                    value.split(',').map(|s| s.trim().to_string()).collect()
                };
            }

            // Bayesian router settings
            "bayesian.enabled" => {
                self.bayesian.enabled = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "bayesian.show_belief_graph" => {
                self.bayesian.show_belief_graph = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean: {}", value))?;
            }
            "bayesian.clarification_threshold" => {
                let v: f32 = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(anyhow!("clarification_threshold must be in [0.0, 1.0]"));
                }
                self.bayesian.clarification_threshold = v;
            }
            "bayesian.uncertainty_threshold" => {
                let v: f32 = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(anyhow!("uncertainty_threshold must be in [0.0, 1.0]"));
                }
                self.bayesian.uncertainty_threshold = v;
            }
            "bayesian.vagueness_threshold" => {
                let v: f32 = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(anyhow!("vagueness_threshold must be in [0.0, 1.0]"));
                }
                self.bayesian.vagueness_threshold = v;
            }
            "bayesian.intent_likelihood_weight" => {
                self.bayesian.intent_likelihood_weight = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
            }
            "bayesian.profile_learning_rate" => {
                let v: f32 = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(anyhow!("profile_learning_rate must be in [0.0, 1.0]"));
                }
                self.bayesian.profile_learning_rate = v;
            }
            "bayesian.priors.intent_edit" => {
                self.bayesian.priors.intent_edit = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
            }
            "bayesian.priors.intent_shell" => {
                self.bayesian.priors.intent_shell = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
            }
            "bayesian.priors.intent_search" => {
                self.bayesian.priors.intent_search = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
            }
            "bayesian.priors.intent_question" => {
                self.bayesian.priors.intent_question = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
            }
            "bayesian.priors.need_clarification" => {
                self.bayesian.priors.need_clarification = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
            }
            "bayesian.priors.low_confidence" => {
                self.bayesian.priors.low_confidence = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
            }
            "bayesian.priors.is_vague" => {
                self.bayesian.priors.is_vague = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid float: {}", value))?;
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
            "acp.require_permission" => {
                self.acp.require_permission = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid boolean value: {}", value))?;
            }
            "acp.permission_timeout_secs" => {
                let secs: u64 = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
                if secs < 10 {
                    return Err(anyhow!(
                        "permission_timeout_secs must be at least 10 seconds"
                    ));
                }
                self.acp.permission_timeout_secs = secs;
            }
            "acp.max_tool_loop_iterations" => {
                let n: u32 = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
                if n == 0 {
                    return Err(anyhow!("max_tool_loop_iterations must be at least 1"));
                }
                self.acp.max_tool_loop_iterations = n;
            }
            "acp.max_history_messages" => {
                let n: usize = value
                    .parse()
                    .map_err(|_| anyhow!("Invalid number: {}", value))?;
                if n < 2 {
                    return Err(anyhow!("max_history_messages must be at least 2"));
                }
                self.acp.max_history_messages = n;
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
}
