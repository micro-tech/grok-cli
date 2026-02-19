# Hooks and Extensions Guide

## Overview

Grok CLI features a powerful **hooks and extensions system** that allows you to extend functionality without modifying core code. This system is similar to what you've seen in Gemini, allowing AI to run scripts, intercept tool calls, add custom logic, and more.

## What Are Hooks?

**Hooks** are points in the code where custom logic can be injected. They allow you to:

- **Intercept tool calls** before and after execution
- **Modify or validate** tool inputs and outputs
- **Execute custom scripts** or commands
- **Log and monitor** AI behavior
- **Implement security policies**
- **Add custom functionality** without touching core code

## What Are Extensions?

**Extensions** are modular packages that register hooks and provide additional capabilities. They consist of:

- **Manifest file** (`extension.json`) - Metadata and configuration
- **Hook implementations** - Code that runs at specific points
- **Optional scripts** - External programs that can be executed
- **Configuration** - Custom settings for the extension

---

## Quick Start

### 1. Enable Extensions

Edit your config file (`~/.config/grok-cli/config.toml` or `C:\Users\<username>\AppData\Roaming\grok-cli\config.toml`):

```toml
[experimental.extensions]
enabled = true
extension_dir = "~/.grok/extensions"  # Optional: custom directory
enabled_extensions = []  # Empty = all enabled, or list specific ones
allow_config_extensions = true
```

### 2. Create Your First Extension

```bash
# Create extension directory
mkdir -p ~/.grok/extensions/my-first-hook

# Create the manifest
cat > ~/.grok/extensions/my-first-hook/extension.json << 'EOF'
{
  "name": "my-first-hook",
  "version": "1.0.0",
  "description": "My first custom hook",
  "author": "Your Name",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "my-hook",
      "hook_type": "both",
      "script": null,
      "config": {
        "message": "Hello from my hook!"
      }
    }
  ],
  "dependencies": [],
  "enabled": true
}
EOF
```

### 3. Test It

```bash
grok interactive
> /help

# Your hook will execute before and after every tool call!
# Check the logs to see it in action
```

---

## Hook Types

### 1. **before_tool** Hook

Executes **before** a tool is invoked. Can:
- Validate inputs
- Block execution (return `false`)
- Log the attempt
- Modify context

**Use cases:**
- Security validation
- Input sanitization
- Rate limiting
- Access control
- Audit logging

### 2. **after_tool** Hook

Executes **after** a tool completes. Can:
- Log results
- Transform output
- Trigger follow-up actions
- Collect metrics

**Use cases:**
- Result logging
- Performance monitoring
- Output validation
- Cleanup operations
- Analytics

### 3. **both** Hook

Executes both before and after. Perfect for:
- Complete lifecycle tracking
- Timing measurements
- Transaction management

---

## Extension Structure

### Directory Layout

```
~/.grok/extensions/
├── my-extension/
│   ├── extension.json      # Required: manifest
│   ├── README.md           # Optional: documentation
│   ├── scripts/            # Optional: executable scripts
│   │   ├── before.sh
│   │   ├── after.py
│   │   └── validate.js
│   ├── config/             # Optional: additional config
│   └── lib/                # Optional: shared libraries
```

### Manifest Format (extension.json)

```json
{
  "name": "extension-name",
  "version": "1.0.0",
  "description": "What this extension does",
  "author": "Your Name <email@example.com>",
  "extension_type": "hook",
  
  "hooks": [
    {
      "name": "hook-identifier",
      "hook_type": "before_tool",
      "script": "scripts/before.sh",
      "config": {
        "key": "value",
        "enabled": true
      }
    }
  ],
  
  "dependencies": [],
  "enabled": true
}
```

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Unique extension name (lowercase, hyphens) |
| `version` | String | Yes | Semantic version (e.g., "1.0.0") |
| `description` | String | No | Brief description of functionality |
| `author` | String | No | Author name and contact |
| `extension_type` | String | Yes | "hook", "tool", or "combined" |
| `hooks` | Array | Yes | List of hook configurations |
| `dependencies` | Array | No | List of required extensions |
| `enabled` | Boolean | No | Whether enabled by default (default: true) |

### Hook Configuration

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Hook identifier |
| `hook_type` | String | Yes | "before_tool", "after_tool", or "both" |
| `script` | String | No | Path to executable script (relative to extension dir) |
| `config` | Object | No | Custom configuration for this hook |

---

## Example Extensions

### Example 1: Logging Hook

Log all tool invocations for debugging:

```json
{
  "name": "logging-hook",
  "version": "1.0.0",
  "description": "Logs all tool invocations",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "tool-logger",
      "hook_type": "both",
      "config": {
        "log_level": "debug",
        "include_args": true,
        "include_results": true,
        "max_result_length": 500
      }
    }
  ],
  "enabled": true
}
```

**What it does:**
- Logs tool name and arguments before execution
- Logs results after execution
- Truncates long results to avoid huge logs
- Useful for debugging and monitoring

### Example 2: Security Validator

Block dangerous operations:

```json
{
  "name": "security-validator",
  "version": "1.0.0",
  "description": "Validates tool usage against security policy",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "security-check",
      "hook_type": "before_tool",
      "config": {
        "blocked_tools": ["run_shell_command"],
        "allowed_paths": ["/home/user/projects"],
        "block_message": "This operation is not allowed by security policy"
      }
    }
  ],
  "enabled": true
}
```

**What it does:**
- Blocks specific tools from executing
- Restricts file operations to allowed paths
- Returns custom error messages
- Prevents dangerous operations

### Example 3: Performance Monitor

Track tool execution times:

```json
{
  "name": "performance-monitor",
  "version": "1.0.0",
  "description": "Monitors tool performance",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "perf-tracker",
      "hook_type": "both",
      "config": {
        "log_slow_tools": true,
        "slow_threshold_ms": 1000,
        "save_metrics": true,
        "metrics_file": "~/.grok/metrics.json"
      }
    }
  ],
  "enabled": true
}
```

**What it does:**
- Measures execution time for each tool
- Logs slow operations (>1 second)
- Saves metrics to file
- Helps identify performance bottlenecks

### Example 4: Custom Script Execution

Execute external scripts before/after tools:

```json
{
  "name": "script-runner",
  "version": "1.0.0",
  "description": "Runs custom scripts on tool invocation",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "backup-hook",
      "hook_type": "before_tool",
      "script": "scripts/backup.sh",
      "config": {
        "tools_to_backup": ["write_file", "edit_file"],
        "backup_dir": "~/.grok/backups"
      }
    }
  ],
  "enabled": true
}
```

**Script example** (`scripts/backup.sh`):

```bash
#!/bin/bash
# Backup files before AI modifies them

TOOL_NAME="$1"
TOOL_ARGS="$2"

if [[ "$TOOL_NAME" == "write_file" || "$TOOL_NAME" == "edit_file" ]]; then
    # Extract file path from args (simplified)
    FILE_PATH=$(echo "$TOOL_ARGS" | jq -r '.path')
    
    if [ -f "$FILE_PATH" ]; then
        BACKUP_DIR="${HOME}/.grok/backups"
        mkdir -p "$BACKUP_DIR"
        cp "$FILE_PATH" "$BACKUP_DIR/$(basename $FILE_PATH).$(date +%s).bak"
        echo "Backed up: $FILE_PATH"
    fi
fi
```

---

## Advanced Features

### Script Execution

Extensions can execute external scripts in any language:

**Supported script types:**
- Shell scripts (`.sh`, `.bash`)
- Python scripts (`.py`)
- JavaScript/Node.js (`.js`)
- Ruby (`.rb`)
- Any executable program

**Script interface:**

Scripts receive information via:
1. **Command line arguments:**
   - `$1` - Tool name
   - `$2` - Tool arguments (JSON string)
   - `$3` - Hook type ("before" or "after")
   - `$4` - Result (if after hook, JSON string)

2. **Environment variables:**
   - `GROK_TOOL_NAME` - Name of the tool
   - `GROK_HOOK_TYPE` - Type of hook
   - `GROK_EXTENSION_NAME` - Extension name
   - `GROK_EXTENSION_DIR` - Extension directory path

**Script return codes:**
- `0` - Success, continue
- `1` - Error, fail the operation
- `2` - Block, don't execute tool (before hook only)

### Python Script Example

```python
#!/usr/bin/env python3
import sys
import json
import os

def before_tool_hook(tool_name, args):
    """Hook executed before tool invocation"""
    print(f"[Hook] About to execute: {tool_name}")
    
    # Parse arguments
    try:
        tool_args = json.loads(args)
        print(f"[Hook] Arguments: {tool_args}")
    except json.JSONDecodeError:
        print(f"[Hook] Could not parse args")
    
    # Implement custom logic
    if tool_name == "run_shell_command":
        command = tool_args.get("command", "")
        if "rm -rf" in command:
            print("[Hook] BLOCKED: Dangerous command detected!")
            sys.exit(2)  # Block execution
    
    return 0  # Allow execution

def after_tool_hook(tool_name, args, result):
    """Hook executed after tool invocation"""
    print(f"[Hook] Completed: {tool_name}")
    print(f"[Hook] Result length: {len(result)}")
    return 0

if __name__ == "__main__":
    tool_name = sys.argv[1]
    tool_args = sys.argv[2]
    hook_type = sys.argv[3]
    
    if hook_type == "before":
        sys.exit(before_tool_hook(tool_name, tool_args))
    elif hook_type == "after":
        result = sys.argv[4] if len(sys.argv) > 4 else ""
        sys.exit(after_tool_hook(tool_name, tool_args, result))
    else:
        print(f"Unknown hook type: {hook_type}")
        sys.exit(1)
```

### Multiple Hooks in One Extension

```json
{
  "name": "multi-hook",
  "version": "1.0.0",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "validator",
      "hook_type": "before_tool",
      "script": "scripts/validate.py"
    },
    {
      "name": "logger",
      "hook_type": "both",
      "config": {"verbose": true}
    },
    {
      "name": "cleanup",
      "hook_type": "after_tool",
      "script": "scripts/cleanup.sh"
    }
  ]
}
```

### Extension Dependencies

```json
{
  "name": "advanced-extension",
  "version": "1.0.0",
  "dependencies": ["logging-hook", "security-validator"],
  "hooks": [...]
}
```

If dependencies aren't available, the extension won't load.

---

## Configuration

### System-Wide Configuration

In `config.toml`:

```toml
[experimental.extensions]
enabled = true
extension_dir = "~/.grok/extensions"
enabled_extensions = []  # Empty = all, or ["ext1", "ext2"]
allow_config_extensions = true
```

### Per-Extension Configuration

In the extension's `extension.json`, add custom config:

```json
{
  "name": "my-extension",
  "hooks": [
    {
      "name": "my-hook",
      "config": {
        "setting1": "value1",
        "setting2": 42,
        "setting3": true,
        "nested": {
          "key": "value"
        }
      }
    }
  ]
}
```

Access in scripts via `$GROK_HOOK_CONFIG` environment variable (JSON string).

---

## Use Cases

### 1. **Automatic Backups**

Create backups before AI modifies files:
- Hook: `before_tool` on `write_file`, `edit_file`
- Script: Copy file to backup directory with timestamp
- Restore: `~/.grok/backups/` contains all versions

### 2. **Code Review Integration**

Automatically review code changes:
- Hook: `after_tool` on `write_file`
- Script: Run linter/formatter, commit to git
- Post results back to conversation

### 3. **Security Audit Log**

Track all AI operations for compliance:
- Hook: `both` on all tools
- Script: Log to secure audit file with timestamps
- Include: user, tool, args, results, timestamp

### 4. **Custom Tool Validation**

Implement project-specific rules:
- Hook: `before_tool`
- Script: Check against project policies
- Block: Non-compliant operations

### 5. **Performance Profiling**

Identify slow operations:
- Hook: `both` on all tools
- Measure: Execution time
- Alert: When tools exceed threshold
- Store: Metrics in database

### 6. **Integration with External Systems**

Connect to other tools:
- Hook: `after_tool`
- Script: Send data to Slack, JIRA, monitoring
- Notify: Team about AI actions

### 7. **Resource Management**

Track resource usage:
- Hook: `both` on file operations
- Monitor: Disk space, file counts
- Alert: When approaching limits

### 8. **Testing & Validation**

Automatically test AI-generated code:
- Hook: `after_tool` on code generation
- Script: Run tests, check coverage
- Report: Pass/fail status

---

## Comparison with Gemini

| Feature | Grok CLI | Gemini |
|---------|----------|--------|
| **Hook System** | ✅ Yes | ✅ Yes |
| **Before/After Hooks** | ✅ Both | ✅ Both |
| **Script Execution** | ✅ Any language | ✅ Any language |
| **Configuration** | ✅ JSON + TOML | ✅ JSON |
| **Extension Discovery** | ✅ Auto-discovery | ✅ Auto-discovery |
| **Security Validation** | ✅ Built-in | ❓ Unknown |
| **Multiple Hooks** | ✅ Unlimited | ❓ Unknown |
| **Dependencies** | ✅ Supported | ❓ Unknown |
| **Tool Blocking** | ✅ Yes | ❓ Unknown |

---

## Best Practices

### ✅ DO:

1. **Keep hooks lightweight** - They run on every tool call
2. **Use specific hook types** - Don't use "both" if you only need one
3. **Validate inputs** - Check for required fields and types
4. **Handle errors gracefully** - Don't crash the entire system
5. **Log appropriately** - Use debug level for verbose output
6. **Document your extensions** - Include README.md
7. **Version your extensions** - Use semantic versioning
8. **Test thoroughly** - Test with various tool calls
9. **Use descriptive names** - Make intent clear
10. **Clean up resources** - Close files, connections, etc.

### ❌ DON'T:

1. **Don't block indefinitely** - Scripts should complete quickly
2. **Don't modify core files** - Use extensions, not patches
3. **Don't store secrets** - Use environment variables
4. **Don't trust input** - Always validate
5. **Don't log sensitive data** - API keys, passwords, etc.
6. **Don't create circular dependencies** - Extension A → B → A
7. **Don't use hardcoded paths** - Use relative paths or config
8. **Don't ignore errors** - Handle them appropriately

---

## Troubleshooting

### Extension Not Loading

**Check:**
1. Is `experimental.extensions.enabled = true` in config?
2. Does `extension.json` exist in the extension directory?
3. Is the JSON valid? (Use `jq` or JSON validator)
4. Is the extension listed in `enabled_extensions` (if specified)?
5. Are dependencies available?

**Debug:**
```bash
# Check logs
tail -f ~/.grok/logs/grok.log

# Validate JSON
jq . ~/.grok/extensions/my-extension/extension.json

# Test manually
grok --log-level debug interactive
```

### Script Not Executing

**Check:**
1. Is the script path correct (relative to extension dir)?
2. Is the script executable? (`chmod +x script.sh`)
3. Does the script have a shebang? (`#!/bin/bash`)
4. Is the interpreter available? (`which python3`)
5. Are there syntax errors in the script?

**Debug:**
```bash
# Test script manually
cd ~/.grok/extensions/my-extension
./scripts/test.sh "tool_name" '{"arg":"value"}' "before"

# Check script permissions
ls -l scripts/

# Test with bash -x for debugging
bash -x scripts/test.sh ...
```

### Hook Not Triggering

**Check:**
1. Is the hook type correct?
2. Is the extension enabled?
3. Are you using the right tool name?
4. Check logs for errors

**Debug:**
```bash
# Enable debug logging
export RUST_LOG=debug
grok interactive

# Or in config.toml
[logging]
level = "debug"
```

### Performance Issues

**Check:**
1. Are hooks taking too long?
2. Are scripts inefficient?
3. Too many extensions enabled?

**Optimize:**
- Profile scripts to find bottlenecks
- Cache results when possible
- Disable unused extensions
- Use async operations where appropriate

---

## Advanced: Creating Custom Hook Types

Currently, Grok CLI supports:
- `before_tool` - Before tool execution
- `after_tool` - After tool execution

**Future hook types** (roadmap):
- `on_message` - When user sends message
- `on_response` - When AI responds
- `on_session_start` - When session begins
- `on_session_end` - When session ends
- `on_error` - When errors occur
- `on_skill_activate` - When skill is activated
- `on_mcp_connect` - When MCP server connects

---

## Real-World Examples

### Example: Git Auto-Commit

Automatically commit changes when AI edits files:

**extension.json:**
```json
{
  "name": "git-auto-commit",
  "version": "1.0.0",
  "description": "Auto-commit changes made by AI",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "git-commit",
      "hook_type": "after_tool",
      "script": "scripts/git-commit.sh",
      "config": {
        "tools": ["write_file", "edit_file", "create_file"],
        "commit_message_prefix": "[AI] ",
        "auto_push": false
      }
    }
  ]
}
```

**scripts/git-commit.sh:**
```bash
#!/bin/bash
set -e

TOOL_NAME="$1"
TOOL_ARGS="$2"
HOOK_TYPE="$3"
RESULT="$4"

# Only process file modification tools
if [[ "$TOOL_NAME" != "write_file" && "$TOOL_NAME" != "edit_file" ]]; then
    exit 0
fi

# Extract file path
FILE_PATH=$(echo "$TOOL_ARGS" | jq -r '.path // .file_path')

if [ -z "$FILE_PATH" ] || [ ! -f "$FILE_PATH" ]; then
    exit 0
fi

# Check if in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    exit 0
fi

# Commit the change
git add "$FILE_PATH"
git commit -m "[AI] Modified $FILE_PATH via $TOOL_NAME" \
    -m "Tool: $TOOL_NAME" \
    -m "Timestamp: $(date -Iseconds)"

echo "Committed changes to $FILE_PATH"
exit 0
```

### Example: Slack Notifications

Notify team when AI makes significant changes:

**extension.json:**
```json
{
  "name": "slack-notifier",
  "version": "1.0.0",
  "description": "Send Slack notifications for AI actions",
  "extension_type": "hook",
  "hooks": [
    {
      "name": "slack-notify",
      "hook_type": "after_tool",
      "script": "scripts/notify.py",
      "config": {
        "webhook_url_env": "SLACK_WEBHOOK_URL",
        "notify_tools": ["write_file", "run_shell_command"],
        "min_file_size": 1000
      }
    }
  ]
}
```

**scripts/notify.py:**
```python
#!/usr/bin/env python3
import sys
import json
import os
import requests

def notify_slack(message):
    webhook_url = os.environ.get('SLACK_WEBHOOK_URL')
    if not webhook_url:
        print("No webhook URL configured")
        return
    
    payload = {
        "text": message,
        "username": "Grok CLI Bot",
        "icon_emoji": ":robot_face:"
    }
    
    try:
        response = requests.post(webhook_url, json=payload)
        response.raise_for_status()
        print("Notification sent to Slack")
    except Exception as e:
        print(f"Failed to send notification: {e}")

if __name__ == "__main__":
    tool_name = sys.argv[1]
    tool_args = json.loads(sys.argv[2])
    
    if tool_name == "write_file":
        file_path = tool_args.get("path", "unknown")
        message = f"🤖 AI modified file: `{file_path}`"
        notify_slack(message)
    elif tool_name == "run_shell_command":
        command = tool_args.get("command", "unknown")
        message = f"🤖 AI executed command: `{command}`"
        notify_slack(message)
    
    sys.exit(0)
```

---

## Resources

### Documentation
- [Extension Loader Source](../src/hooks/loader.rs)
- [Hook Manager Source](../src/hooks/mod.rs)
- [Example Extensions](../examples/extensions/)
- [Skills System](SKILLS_QUICK_START.md)

### Community
- GitHub Issues: Report bugs or request features
- Discussions: Share your extensions
- Examples: Browse community extensions

### Tools
- JSON Validator: https://jsonlint.com/
- jq: Command-line JSON processor
- ShellCheck: Shell script linter

---

## Summary

🎉 **Grok CLI has a powerful hooks and extensions system** that lets you:

✅ Intercept and modify tool calls  
✅ Execute custom scripts in any language  
✅ Implement security policies  
✅ Monitor and log AI behavior  
✅ Integrate with external systems  
✅ Create reusable, shareable extensions  

**Get started today:**
```bash
# 1. Enable extensions
# Edit config.toml: experimental.extensions.enabled = true

# 2. Create extension directory
mkdir -p ~/.grok/extensions/my-hook

# 3. Create extension.json (see examples above)

# 4. Test it
grok interactive
```

**Happy hacking!** 🚀