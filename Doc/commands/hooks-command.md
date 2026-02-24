# /hooks Command Guide

## Overview

The `/hooks` command displays information about the hooks system in Grok CLI. Hooks allow you to execute custom logic before and after tool calls, enabling logging, validation, security checks, and other extensions to tool behavior.

## Usage

In interactive mode, simply type:

```
/hooks
```

## What It Shows

The command displays:

1. **Hooks Status** - Whether the hooks system is enabled or disabled
2. **Extensions System Status** - Whether the extensions system is enabled
3. **Extension Directory** - Path to the directory where extensions are loaded from
4. **Enabled Extensions** - List of all enabled extensions
5. **Usage Tips** - Helpful information on enabling and using hooks

## Example Output

### When Hooks Are Enabled

```
Hooks System Information

  ✓ Hooks Status: Enabled
  ✓ Extensions System: Enabled
  ℹ Extension Directory: C:\Users\username\.grok\extensions

  📦 Enabled Extensions:
    • audit-logger
    • security-validator
    • performance-monitor

About Hooks:
  Hooks allow you to execute custom logic before and after tool calls.
  They can be used for logging, validation, security checks, and more.

Tip:
  Enable the extensions system to load custom hooks from extensions.
  Set 'experimental.extensions.enabled = true' in your config.
```

### When Hooks Are Disabled

```
Hooks System Information

  ✗ Hooks Status: Disabled
  ✗ Extensions System: Disabled

About Hooks:
  Hooks allow you to execute custom logic before and after tool calls.
  They can be used for logging, validation, security checks, and more.

To enable hooks:
  1. Edit your config file (use /settings)
  2. Set 'tools.enable_hooks = true'
  3. Optionally enable extensions system for custom hooks
```

## Configuration

### Enabling Hooks

To enable the hooks system, edit your configuration file:

**Option 1: Using the CLI**
```bash
grok config set tools.enable_hooks true
```

**Option 2: Manual Edit**
Edit `~/.grok/config.toml` (or your project's config):

```toml
[tools]
enable_hooks = true
```

### Enabling Extensions

To load custom hooks from extensions:

```toml
[experimental.extensions]
enabled = true
extension_dir = "~/.grok/extensions"  # Optional, uses default if not specified
enabled_extensions = ["my-custom-hook", "another-extension"]
```

## What Are Hooks?

Hooks are callback functions that execute at specific points during tool execution:

### Before Tool Hooks
- Execute before a tool is called
- Can inspect tool arguments
- Can block tool execution by returning `false`
- Useful for: validation, security checks, logging

### After Tool Hooks
- Execute after a tool completes
- Can inspect tool results
- Can log or process output
- Useful for: audit logging, result transformation, metrics

## Use Cases

### 1. Security Validation
```rust
// Before-hook: Validate file paths before read_file
fn validate_path(context: &ToolContext) -> bool {
    if context.tool_name == "read_file" {
        let path = context.args["path"].as_str().unwrap();
        return is_safe_path(path);
    }
    true
}
```

### 2. Audit Logging
```rust
// After-hook: Log all tool executions
fn audit_log(context: &ToolContext, result: &str) {
    log::info!(
        "Tool executed: {} with args: {:?} => {}",
        context.tool_name,
        context.args,
        result
    );
}
```

### 3. Performance Monitoring
```rust
// Track tool execution times
fn monitor_performance(context: &ToolContext) {
    start_timer(&context.tool_name);
}

fn record_metrics(context: &ToolContext, _result: &str) {
    let duration = stop_timer(&context.tool_name);
    metrics::record_duration(&context.tool_name, duration);
}
```

## Creating Custom Hooks

### 1. Create an Extension Directory

```bash
mkdir -p ~/.grok/extensions/my-hook
```

### 2. Create Extension Manifest

Create `~/.grok/extensions/my-hook/extension.toml`:

```toml
name = "my-hook"
version = "1.0.0"
description = "My custom hook"
author = "Your Name"
extension_type = "hook"
enabled = true

[[hooks]]
name = "my-before-hook"
hook_type = "before_tool"

[[hooks]]
name = "my-after-hook"
hook_type = "after_tool"
```

### 3. Implement Hook Logic

Currently, hooks are implemented in Rust. Future versions may support scripting languages.

See the [Extensions Guide](../extensions.md) for more details on creating custom hooks.

## Related Commands

- `/tools` - List available coding tools that can use hooks
- `/settings` - Open settings menu to configure hooks
- `/help` - Show all available commands

## Troubleshooting

### "Hooks Status: Disabled"

**Solution:** Enable hooks in your configuration:
```bash
grok config set tools.enable_hooks true
```

### "No extensions found"

**Possible causes:**
1. Extension directory doesn't exist
2. Extensions are not properly configured
3. Extension manifests have errors

**Solution:** Check your extension directory and manifest files.

### Hooks not executing

**Checklist:**
- [ ] Hooks are enabled (`tools.enable_hooks = true`)
- [ ] Extensions system is enabled (`experimental.extensions.enabled = true`)
- [ ] Extension is listed in `enabled_extensions`
- [ ] Extension manifest is valid TOML
- [ ] Running in ACP mode (hooks currently only work in ACP)

## Technical Details

### Where Hooks Execute

Hooks are primarily used in **ACP (Agent Client Protocol)** mode when Grok CLI acts as a language server for editors like Zed.

In interactive mode, the `/hooks` command shows configuration and status but doesn't execute hooks on tool calls (as interactive mode doesn't use the tool system in the same way).

### Hook Manager API

The `HookManager` provides:
- `register(hook)` - Register a new hook
- `execute_before_tool(tool_name, args)` - Run before-tool hooks
- `execute_after_tool(tool_name, args, result)` - Run after-tool hooks
- `list_hooks()` - List registered hook names
- `hook_count()` - Count registered hooks

## Examples

### Checking Hook Status

```
> /hooks
Hooks System Information
  ✓ Hooks Status: Enabled
  ✓ Extensions System: Enabled
```

### Viewing Enabled Extensions

```
> /hooks
...
  📦 Enabled Extensions:
    • security-validator
    • audit-logger
```

### Getting Help When Disabled

```
> /hooks
...
To enable hooks:
  1. Edit your config file (use /settings)
  2. Set 'tools.enable_hooks = true'
  3. Optionally enable extensions system for custom hooks
```

## See Also

- [Configuration Guide](../CONFIGURATION.md)
- [Extensions System](../extensions.md)
- [ACP Integration](../acp-integration.md)
- [Security Guide](../security.md)

## Notes

- Hooks are an advanced feature for customizing tool behavior
- They require some Rust knowledge to implement custom hooks
- Future versions may support scripting languages for easier hook creation
- Hooks are most useful in ACP mode for editor integration