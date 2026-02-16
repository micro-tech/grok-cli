# System Configuration Notes

## Location

Your system-wide configuration is located at:
- **Windows**: `C:\Users\johnm\AppData\Roaming\grok-cli\config.toml`
- **Linux/Mac**: `~/.config/grok-cli/config.toml`

This config applies to all Grok CLI usage on your system, unless overridden by project-specific configs.

## Current Settings

### Tool Loop Iterations

**Current value**: `max_tool_loop_iterations = 50`

This is **higher than the default (25)**, which means:

✅ **Good for**: Complex multi-step tasks that legitimately require many tool calls
❌ **Bad for**: Debugging tool loop issues - you'll wait longer before hitting the limit

### When This Setting Is Used

The system config is loaded with this priority:
1. Built-in defaults (`max_tool_loop_iterations = 25`)
2. **System config** (`C:\Users\johnm\AppData\Roaming\grok-cli\config.toml`) ← **Your setting here**
3. System `.env` file
4. Project config (`.grok/config.toml`)
5. Project `.env` file
6. Environment variables (`GROK_ACP_MAX_TOOL_LOOP_ITERATIONS`)

So your setting of 50 will apply to all projects UNLESS:
- The project has its own `.grok/config.toml` with a different value
- You set the environment variable to override it

## Recommendations

### For Normal Use

Keep your current setting of 50 if you frequently work on complex tasks that need many tool calls.

### For Debugging Tool Loops

Temporarily lower it to catch issues faster:

```powershell
# PowerShell - Set for this session only
$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS = "10"

# Then run your command
cargo run --bin grok -- acp
```

```bash
# Bash - Set for this session only
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=10

# Then run your command
cargo run --bin grok -- acp
```

### For Specific Projects

Create a `.grok/config.toml` in the project directory:

```toml
[acp]
enabled = true
max_tool_loop_iterations = 25  # Override system config for this project
```

## Understanding Your Current Issue

Even though you have `max_tool_loop_iterations = 50` set, you're hitting the limit. This tells us:

**❌ The problem is NOT that 50 is too low**
**✅ The problem IS that the AI is stuck in a loop**

Reading one file should take **1-3 iterations**, not 50. The issue is likely:

1. **Configuration error** - Now fixed in `.grok/config.toml` (missing `env` field)
2. **Vague prompts** - AI doesn't know when to stop
3. **Tool response confusion** - AI interprets responses incorrectly

## Testing After Fixes

To verify the fixes work, temporarily lower your limit to expose problems faster:

```powershell
# Set a low limit for testing
$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS = "10"

# Test a simple command - should complete in 2-3 iterations
echo "Read README.md and tell me the project name. Then stop." | cargo run --bin grok -- acp
```

If it completes in 2-3 iterations: ✅ **Problem is fixed!**
If it hits 10 iterations: ❌ **Still have a loop issue** - run the analyzer

## Updating System Config

### Adding MCP Server Section

Your system config doesn't have an MCP server section yet. To add it:

1. **Make sure Grok CLI is not running** (close all instances)

2. **Run the update script**:
   ```powershell
   .\update_system_config.ps1
   ```

3. **Or manually edit** `C:\Users\johnm\AppData\Roaming\grok-cli\config.toml`:
   - Open the file in a text editor
   - Scroll to the bottom
   - Copy the MCP section from `config.example.toml`
   - Uncomment and customize as needed

### Manual Backup First

Before editing:
```powershell
Copy-Item "$env:APPDATA\grok-cli\config.toml" "$env:APPDATA\grok-cli\config.toml.backup"
```

## Verification

After any config changes, verify it loads correctly:

```bash
# Show the loaded configuration
cargo run --bin grok -- config show

# Look for:
# - "Max Tool Loop Iterations: 50" (or your current value)
# - No TOML parsing errors
# - Correct config source path
```

## Summary

- ✅ Your system config is valid (no syntax errors)
- ⚠️ You have `max_tool_loop_iterations = 50` (higher than default)
- ✅ No MCP servers configured (so no missing `env` field issues)
- 📝 Consider adding MCP section as a template for future use
- 🔍 For debugging, temporarily lower the limit with environment variables

## Related Documentation

- [Troubleshooting Tool Loops](./TROUBLESHOOTING_TOOL_LOOPS.md) - Complete guide to diagnosing and fixing tool loop issues
- [Configuration Guide](../CONFIGURATION.md) - Full documentation of all config options
- `config.example.toml` - Example configuration with all options and comments