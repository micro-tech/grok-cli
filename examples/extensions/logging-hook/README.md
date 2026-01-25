# Logging Hook Extension

A sample extension that demonstrates the Grok CLI extension system by logging tool invocations.

## Overview

This extension provides a simple hook that logs all tool invocations before and after execution, which can be useful for:
- Debugging tool interactions
- Monitoring agent behavior
- Auditing tool usage
- Performance profiling

## Installation

1. Copy this directory to your Grok extensions folder:
   ```bash
   cp -r logging-hook ~/.grok/extensions/
   ```

2. Enable the extension in your `~/.config/grok-cli/config.toml`:
   ```toml
   [experimental.extensions]
   enabled = true
   enabled_extensions = ["logging-hook"]
   ```

3. Restart Grok CLI to load the extension.

## Configuration

The extension can be configured in the `extension.json` file:

- **log_level**: Set to "debug", "info", "warn", or "error" (default: "debug")
- **include_args**: Whether to log tool arguments (default: true)
- **include_results**: Whether to log tool results (default: true)
- **max_result_length**: Maximum length of result to log, to avoid huge logs (default: 500)

## How It Works

The logging hook implements both `before_tool` and `after_tool` hooks:

1. **before_tool**: Logs when a tool is about to be invoked, including:
   - Tool name
   - Arguments (if enabled)
   - Timestamp

2. **after_tool**: Logs after tool execution completes, including:
   - Tool name
   - Result preview (if enabled)
   - Execution duration
   - Timestamp

## Example Output

When enabled, you'll see log entries like:

```
[DEBUG] Extension 'logging-hook' hook 'tool-logger' executing before tool 'read_file'
[DEBUG] Tool args: {"path": "src/main.rs"}
[DEBUG] Extension 'logging-hook' hook 'tool-logger' executing after tool 'read_file'
[DEBUG] Result length: 1234 bytes
```

## Extending This Example

You can use this as a template to create your own extensions:

1. Modify `extension.json` to change the extension name and configuration
2. Add additional hooks as needed
3. In a full implementation, you could:
   - Execute external scripts or commands
   - Send metrics to monitoring systems
   - Implement security policies
   - Transform tool inputs/outputs
   - Add custom validation logic

## Limitations

This is a basic config-based extension. For more advanced functionality, you would need to:
- Implement dynamic library loading (`.so`, `.dll`, `.dylib`)
- Add scripting language support (Lua, Python, etc.)
- Create a proper extension SDK with more hook types
- Add tool registration capabilities for custom tools

## License

MIT License - feel free to modify and distribute.

## Author

john mcconnell <john.microtech@gmail.com>