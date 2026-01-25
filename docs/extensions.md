# Extension System Documentation

## Overview

The Grok CLI extension system provides a powerful way to extend the functionality of the CLI through hooks and custom tools. Extensions can intercept tool executions, add custom behaviors, and integrate with external systems.

## Architecture

The extension system consists of several key components:

### 1. Extension Manager
- Manages the lifecycle of extensions
- Handles extension registration and initialization
- Coordinates between extensions and the core system

### 2. Hook Manager
- Manages execution hooks for tools
- Provides `before_tool` and `after_tool` hook points
- Ensures hooks execute in the correct order

### 3. Extension Loader
- Discovers extensions from configured directories
- Loads extension manifests
- Instantiates extensions and registers them with the manager

## Extension Types

Extensions can be one of three types:

### Hook Extensions
Provide lifecycle hooks that execute before/after tool invocations. Useful for:
- Logging and monitoring
- Security validation
- Performance profiling
- Result transformation

### Tool Extensions
Add new tools to the CLI that can be invoked by the agent. Useful for:
- Domain-specific functionality
- Integration with external services
- Custom workflows

### Combined Extensions
Provide both hooks and tools for complete feature sets.

## Creating an Extension

### Extension Structure

Each extension must be in its own directory with an `extension.json` manifest:

```
my-extension/
├── extension.json      # Extension manifest (required)
├── README.md          # Documentation (recommended)
└── scripts/           # Optional scripts or resources
```

### Extension Manifest

The `extension.json` file defines your extension:

```json
{
  "name": "my-extension",
  "version": "1.0.0",
  "description": "A sample extension",
  "author": "Your Name <your.email@example.com>",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "my-hook",
      "hook_type": "both",
      "script": null,
      "config": {
        "custom_setting": "value"
      }
    }
  ],
  "dependencies": [],
  "enabled": true
}
```

### Manifest Fields

- **name** (required): Unique identifier for the extension
- **version** (required): Semantic version string (e.g., "1.0.0")
- **description** (optional): Brief description of what the extension does
- **author** (optional): Author information
- **extension_type** (required): One of "hook", "tool", or "combined"
- **hooks** (optional): Array of hook configurations
- **dependencies** (optional): List of required extensions
- **enabled** (optional): Whether the extension is enabled by default (default: true)

### Hook Configuration

Each hook in the `hooks` array has:

- **name** (required): Unique name for this hook
- **hook_type** (required): One of:
  - `"before_tool"`: Executes before tool invocation
  - `"after_tool"`: Executes after tool completion
  - `"both"`: Executes both before and after
- **script** (optional): Path to external script to execute
- **config** (optional): Custom configuration object

## Configuration

### Enabling Extensions

In your `~/.config/grok-cli/config.toml`:

```toml
[experimental.extensions]
enabled = true
extension_dir = "~/.grok/extensions"
enabled_extensions = ["logging-hook", "my-extension"]
allow_config_extensions = true
```

### Configuration Options

- **enabled**: Master switch for the extension system (default: false)
- **extension_dir**: Directory to search for extensions (default: `~/.grok/extensions`)
- **enabled_extensions**: List of extension names to load. Empty list = load all
- **allow_config_extensions**: Allow loading extensions defined in config (default: false)

## Hook Lifecycle

### Before Tool Hook

Called before a tool is executed. Receives:
- **tool_name**: Name of the tool being invoked
- **args**: Tool arguments as JSON

Can return:
- `Ok(true)`: Continue with tool execution
- `Ok(false)`: Abort tool execution (silent)
- `Err(...)`: Abort with error message

```rust
fn before_tool(&self, context: &ToolContext) -> Result<bool> {
    // Validate, log, or modify behavior
    println!("About to execute: {}", context.tool_name);
    Ok(true) // Continue
}
```

### After Tool Hook

Called after a tool completes. Receives:
- **tool_name**: Name of the tool that executed
- **args**: Tool arguments as JSON
- **result**: Tool result as string

```rust
fn after_tool(&self, context: &ToolContext, result: &str) -> Result<()> {
    // Log, monitor, or post-process result
    println!("Tool {} completed with {} bytes", context.tool_name, result.len());
    Ok(())
}
```

## Use Cases

### 1. Logging and Monitoring

Track all tool invocations for debugging:

```json
{
  "name": "tool-logger",
  "extension_type": "hook",
  "hooks": [{
    "name": "logger",
    "hook_type": "both",
    "config": {
      "log_file": "/var/log/grok-tools.log"
    }
  }]
}
```

### 2. Security Policies

Enforce security constraints on tool usage:

```json
{
  "name": "security-policy",
  "extension_type": "hook",
  "hooks": [{
    "name": "validator",
    "hook_type": "before_tool",
    "config": {
      "blocked_tools": ["run_shell_command"],
      "allowed_paths": ["/home/user/projects"]
    }
  }]
}
```

### 3. Performance Profiling

Measure tool execution times:

```json
{
  "name": "profiler",
  "extension_type": "hook",
  "hooks": [{
    "name": "timer",
    "hook_type": "both",
    "config": {
      "output_format": "json",
      "min_duration_ms": 100
    }
  }]
}
```

### 4. Result Caching

Cache expensive tool results:

```json
{
  "name": "cache",
  "extension_type": "hook",
  "hooks": [{
    "name": "cache-handler",
    "hook_type": "both",
    "config": {
      "cache_dir": "~/.grok/cache",
      "ttl_seconds": 3600
    }
  }]
}
```

## Example Extensions

Several example extensions are provided in the `examples/extensions/` directory:

- **logging-hook**: Basic tool invocation logging
- More examples coming soon!

## Development Guide

### Testing Your Extension

1. Create your extension directory:
   ```bash
   mkdir -p ~/.grok/extensions/my-extension
   ```

2. Create the manifest:
   ```bash
   cat > ~/.grok/extensions/my-extension/extension.json << EOF
   {
     "name": "my-extension",
     "version": "1.0.0",
     "extension_type": "hook",
     "hooks": [...]
   }
   EOF
   ```

3. Enable extensions in config:
   ```toml
   [experimental.extensions]
   enabled = true
   enabled_extensions = ["my-extension"]
   ```

4. Run Grok CLI with debug logging:
   ```bash
   RUST_LOG=debug grok-cli interactive
   ```

### Debugging

Enable debug logging to see extension loading and execution:

```bash
export RUST_LOG=grok_cli::hooks=debug
grok-cli interactive
```

Look for log messages like:
- "Discovered extension: my-extension"
- "Loading extension: my-extension"
- "Extension 'my-extension' hook 'my-hook' executing before tool 'read_file'"

## Advanced Topics

### Extension Dependencies

Extensions can declare dependencies on other extensions:

```json
{
  "dependencies": ["base-extension", "util-extension"]
}
```

The loader will ensure dependencies are loaded first (future enhancement).

### Custom Configuration

Extensions can include custom configuration that's available at runtime:

```json
{
  "hooks": [{
    "config": {
      "api_key": "${ENV:MY_API_KEY}",
      "endpoint": "https://api.example.com",
      "timeout_seconds": 30
    }
  }]
}
```

### Script Execution

Extensions can execute external scripts (future enhancement):

```json
{
  "hooks": [{
    "script": "./scripts/validate.sh",
    "hook_type": "before_tool"
  }]
}
```

## Security Considerations

1. **Trust**: Only install extensions from trusted sources
2. **Isolation**: Extensions run in the same process as the CLI
3. **Permissions**: Extensions have the same permissions as the CLI
4. **Validation**: The extension manifest is validated on load
5. **Sandboxing**: Future versions may add sandboxing support

## Limitations

Current limitations of the extension system:

1. **Static Loading**: Extensions are loaded once at startup
2. **No Hot Reload**: Requires restart to load new extensions
3. **Limited Tool API**: Cannot yet register custom tools dynamically
4. **Config-Based Only**: No dynamic library (`.so`/`.dll`) loading yet
5. **No Scripting**: Cannot execute Python/Lua scripts yet

## Future Enhancements

Planned improvements:

- [ ] Hot reload support
- [ ] Dynamic library loading
- [ ] Custom tool registration API
- [ ] Scripting language support (Python, Lua)
- [ ] Extension marketplace
- [ ] Sandboxing and isolation
- [ ] Extension signing and verification
- [ ] Inter-extension communication
- [ ] Extension UI components

## Troubleshooting

### Extension Not Loading

1. Check that the extension system is enabled:
   ```bash
   grok-cli config get experimental.extension_management
   ```

2. Verify the extension directory exists:
   ```bash
   ls -la ~/.grok/extensions/
   ```

3. Validate the manifest JSON:
   ```bash
   jq . ~/.grok/extensions/my-extension/extension.json
   ```

4. Check logs for errors:
   ```bash
   RUST_LOG=debug grok-cli interactive 2>&1 | grep -i extension
   ```

### Extension Not Executing

1. Ensure the extension is in the enabled list (or list is empty)
2. Check that `enabled: true` in the manifest
3. Verify hook types match what you expect
4. Enable debug logging to see hook execution

### Performance Issues

If extensions cause performance issues:

1. Profile extension execution time
2. Reduce hook complexity
3. Cache expensive operations
4. Disable unused extensions

## API Reference

### Rust Types

For implementing extensions in Rust:

```rust
pub trait Extension: Send + Sync {
    fn name(&self) -> &str;
    fn register_hooks(&self, hook_manager: &mut HookManager) -> Result<()>;
}

pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    fn before_tool(&self, context: &ToolContext) -> Result<bool>;
    fn after_tool(&self, context: &ToolContext, result: &str) -> Result<()>;
}

pub struct ToolContext {
    pub tool_name: String,
    pub args: Value,
}
```

## Contributing

To contribute extensions or improvements to the extension system:

1. Fork the repository
2. Create a feature branch
3. Add your extension or changes
4. Include tests and documentation
5. Submit a pull request

See `CONTRIBUTING.md` for more details.

## Resources

- [Example Extensions](../examples/extensions/)
- [Hook API Reference](hooks-api.md)
- [Configuration Guide](configuration.md)
- [GitHub Repository](https://github.com/microtech/grok-cli)

## Support

For help with extensions:

- Open an issue on GitHub
- Check existing documentation
- Join the community discussions

## License

The extension system is part of Grok CLI and is licensed under the MIT License.