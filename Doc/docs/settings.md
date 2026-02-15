# Grok CLI Settings (`settings` command)

Grok CLI provides a powerful settings system using `.env` files with hierarchical configuration.

## Configuration Files

- **System settings**: `~/.grok/.env` (Linux/macOS) or `%USERPROFILE%\.grok\.env` (Windows)
- **Project settings**: `.grok/.env` (in your project directory)

Note: Project settings override system settings. Environment variables override all files.

## Using the Settings Command

### Interactive Settings Browser
```bash
# Launch interactive settings browser
grok settings show

# Edit settings interactively
grok settings edit
```

### Reset Settings
```bash
# Reset all settings to defaults
grok settings reset

# Reset specific category to defaults
grok settings reset --category ui
grok settings reset --category general
```

### Import/Export Settings
```bash
# Export current settings to a file
grok settings export --path my-settings.env

# Import settings from a file
grok settings import --path my-settings.env
```

## Settings Categories

Settings are organized into logical categories for easier management:

### General
Core application settings and preferences.

### UI
User interface appearance, themes, and display options.

### Model
AI model configuration and behavior settings.

### Context
File handling, context discovery, and memory management.

### Tools
Tool execution, shell integration, and external command settings.

### Security
Security policies, trust settings, and access controls.

### Experimental
Preview features and experimental functionality.

### ACP
Agent Client Protocol settings for Zed editor integration.

### Network
Network configuration, timeouts, and Starlink optimizations.

### Logging
Logging levels, file output, and debugging options.

## Settings Reference

Here is a comprehensive list of all available settings, grouped by category:

### General Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `general.preview_features` | Enable preview features and experimental models | `false` | Boolean |
| `general.vim_mode` | Enable Vim keybindings throughout the interface | `false` | Boolean |
| `general.disable_auto_update` | Disable automatic updates | `false` | Boolean |
| `general.disable_update_nag` | Hide update notification messages | `false` | Boolean |
| `general.enable_prompt_completion` | Enable AI-powered prompt completion while typing | `false` | Boolean |
| `general.retry_fetch_errors` | Automatically retry failed network requests | `false` | Boolean |
| `general.debug_keystroke_logging` | Log keystrokes for debugging purposes | `false` | Boolean |

### UI Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `ui.theme` | Color theme for the interface | `"default"` | String |
| `ui.colors` | Enable colored output in terminal | `true` | Boolean |
| `ui.progress_bars` | Show progress indicators during operations | `true` | Boolean |
| `ui.verbose_errors` | Display detailed error information | `false` | Boolean |
| `ui.terminal_width` | Terminal width override (0 = auto-detect) | `0` | Number |
| `ui.unicode` | Enable Unicode characters and emojis | `true` | Boolean |
| `ui.hide_window_title` | Hide the window title bar | `false` | Boolean |
| `ui.show_status_in_title` | Show Grok CLI status in terminal title | `false` | Boolean |
| `ui.hide_tips` | Hide helpful tips and suggestions | `false` | Boolean |
| `ui.hide_banner` | Hide the ASCII art startup banner | `false` | Boolean |
| `ui.hide_context_summary` | Hide context summary above input | `false` | Boolean |
| `ui.hide_footer` | Hide the status footer | `false` | Boolean |
| `ui.show_memory_usage` | Display memory usage information | `false` | Boolean |
| `ui.show_line_numbers` | Show line numbers in chat output | `true` | Boolean |
| `ui.show_citations` | Show citations for generated content | `false` | Boolean |
| `ui.show_model_info_in_chat` | Display model name in chat responses | `false` | Boolean |
| `ui.use_full_width` | Use entire terminal width for output | `true` | Boolean |
| `ui.use_alternate_buffer` | Use alternate screen buffer (preserves history) | `false` | Boolean |
| `ui.incremental_rendering` | Enable incremental text rendering | `false` | Boolean |

### Footer Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `ui.footer.hide_cwd` | Hide current working directory in footer | `false` | Boolean |
| `ui.footer.hide_sandbox_status` | Hide sandbox status indicator | `false` | Boolean |
| `ui.footer.hide_model_info` | Hide model information in footer | `false` | Boolean |
| `ui.footer.hide_context_percentage` | Hide context usage percentage | `true` | Boolean |

### Accessibility Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `ui.accessibility.disable_loading_phrases` | Disable witty loading phrases | `false` | Boolean |
| `ui.accessibility.screen_reader` | Optimize output for screen readers | `false` | Boolean |

### Interactive UI Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `ui.interactive.prompt_style` | Prompt style (simple, rich, minimal) | `"rich"` | String |
| `ui.interactive.show_context_usage` | Enable context usage display | `true` | Boolean |
| `ui.interactive.auto_save_sessions` | Auto-save sessions | `false` | Boolean |
| `ui.interactive.check_directory` | Check for home directory usage | `true` | Boolean |
| `ui.interactive.startup_animation` | Enable startup animation | `true` | Boolean |
| `ui.interactive.update_check_hours` | Update check frequency in hours (0 = disabled) | `24` | Number |

### Model Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `default_model` | Default Grok model to use | `"grok-3"` | String |
| `default_temperature` | Default temperature for responses (0.0-2.0) | `0.7` | Number |
| `default_max_tokens` | Default maximum tokens per response | `256000` | Number |
| `model.max_session_turns` | Maximum conversation turns (-1 = unlimited) | `-1` | Number |
| `model.compression_threshold` | Context compression threshold (0.1-1.0) | `0.2` | Number |
| `model.skip_next_speaker_check` | Skip next speaker validation | `true` | Boolean |

### Context Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `context.discovery_max_dirs` | Maximum directories to search for context | `200` | Number |
| `context.load_memory_from_include_directories` | Load memory from included directories | `false` | Boolean |
| `context.file_filtering.respect_git_ignore` | Respect .gitignore files | `true` | Boolean |
| `context.file_filtering.respect_grok_ignore` | Respect .grokignore files | `true` | Boolean |
| `context.file_filtering.enable_recursive_file_search` | Enable recursive file search | `true` | Boolean |
| `context.file_filtering.disable_fuzzy_search` | Disable fuzzy file matching | `false` | Boolean |

### Tools Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `tools.shell.enable_interactive_shell` | Enable interactive shell mode | `true` | Boolean |
| `tools.shell.show_color` | Show colors in shell output | `false` | Boolean |
| `tools.auto_accept` | Automatically accept safe tool executions | `false` | Boolean |
| `tools.use_ripgrep` | Use ripgrep for faster file searches | `true` | Boolean |
| `tools.enable_tool_output_truncation` | Truncate large tool outputs | `true` | Boolean |
| `tools.truncate_tool_output_threshold` | Truncation threshold in characters | `10000` | Number |
| `tools.truncate_tool_output_lines` | Lines to keep when truncating | `100` | Number |
| `tools.enable_message_bus_integration` | Enable message bus integration | `true` | Boolean |

### Security Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `security.disable_yolo_mode` | Disable YOLO mode even if flagged | `false` | Boolean |
| `security.enable_permanent_tool_approval` | Allow permanent tool approvals | `false` | Boolean |
| `security.block_git_extensions` | Block Git-based extensions | `false` | Boolean |
| `security.folder_trust.enabled` | Enable folder trust system | `false` | Boolean |
| `security.environment_variable_redaction.enabled` | Enable env var redaction | `false` | Boolean |

### Experimental Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `experimental.enable_agents` | Enable experimental agent features | `false` | Boolean |
| `experimental.extension_management` | Enable extension management | `false` | Boolean |
| `experimental.jit_context` | Enable just-in-time context loading | `false` | Boolean |
| `experimental.codebase_investigator_settings.enabled` | Enable codebase investigator | `true` | Boolean |
| `experimental.codebase_investigator_settings.max_num_turns` | Max investigator turns | `10` | Number |

### ACP Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `acp.enabled` | Enable Agent Client Protocol | `true` | Boolean |
| `acp.bind_host` | ACP server bind address | `"127.0.0.1"` | String |
| `acp.default_port` | Default ACP server port | `None` | Number |
| `acp.protocol_version` | ACP protocol version | `"1.0"` | String |
| `acp.dev_mode` | Enable development mode | `false` | Boolean |
| `acp.max_tool_loop_iterations` | Maximum tool calling iterations (prevents infinite loops) | `25` | Number |

### Network Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `network.starlink_optimizations` | Enable Starlink satellite optimizations | `false` | Boolean |
| `network.base_retry_delay` | Base retry delay in seconds | `1` | Number |
| `network.max_retry_delay` | Maximum retry delay in seconds | `60` | Number |
| `network.health_monitoring` | Enable network health monitoring | `true` | Boolean |
| `network.connect_timeout` | Connection timeout in seconds | `10` | Number |
| `network.read_timeout` | Read timeout in seconds | `30` | Number |

### Logging Settings

| Setting | Description | Default | Type |
|---------|-------------|---------|------|
| `logging.level` | Log level (trace/debug/info/warn/error) | `"info"` | String |
| `logging.file_logging` | Enable logging to file | `false` | Boolean |
| `logging.max_file_size_mb` | Maximum log file size in MB | `10` | Number |
| `logging.rotation_count` | Number of rotated log files to keep | `5` | Number |

## Configuration File Example

Here's an example of what your `.env` file might look like:

```toml
api_key = "your-x-api-key-here"
default_model = "grok-3"
default_temperature = 0.7
default_max_tokens = 256000
timeout_secs = 30
max_retries = 3

[general]
preview_features = true
vim_mode = false
disable_auto_update = false

[ui]
theme = "default"
colors = true
hide_banner = false
hide_tips = false
show_line_numbers = true
use_full_width = true

[ui.footer]
hide_cwd = false
hide_model_info = false
hide_context_percentage = true

[model]
max_session_turns = -1
compression_threshold = 0.2

[acp]
enabled = true
bind_host = "127.0.0.1"
protocol_version = "1.0"
max_tool_loop_iterations = 25  # Maximum tool calling iterations (default: 25)

[network]
starlink_optimizations = true
base_retry_delay = 2
max_retry_delay = 60

[logging]
level = "info"
file_logging = true
```

## Tips

1. **Start with defaults**: The interactive settings browser shows current values and defaults for easy comparison.

2. **Category-specific resets**: Use `grok settings reset --category <name>` to reset only specific sections.

3. **Backup your settings**: Use `grok settings export` to create backups before making major changes.

4. **Project-specific configs**: Create a `.grok/.env` file in your project directory for project-specific settings.

5. **Restart requirements**: Some settings require restarting Grok CLI to take effect. The settings browser will indicate these.

6. **Use .env format**: All configuration now uses environment variable format (KEY=value) instead of TOML.

6. **Environment variables**: Many settings can also be overridden with environment variables (e.g., `GROK_API_KEY`).

For more information about specific features, see the main [Grok CLI documentation](README.md).