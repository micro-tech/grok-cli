# Fix Summary: "Max tool loop iterations reached" Error

## Problem Description

You were encountering the error:
```
Max tool loop iterations reached
```

This error occurs when Grok CLI's AI agent makes too many tool calls (reading files, writing files, searching, etc.) without completing the task. The previous implementation had a hardcoded limit of **10 iterations**, which was too restrictive for complex tasks.

## Root Cause

According to xAI's documentation, this error happens when:
1. The model enters an infinite or very long loop of tool calls
2. The system enforces a hard limit to prevent runaway usage, cost, or timeouts
3. The agent doesn't know when to stop calling tools

The issue was that the limit was **hardcoded to 10** with no way to configure it.

## Solution Implemented

### What Was Changed

1. **Made the limit configurable** - No longer hardcoded
2. **Increased default limit** - From 10 to 25 (2.5x increase)
3. **Added configuration options** - Three ways to set the limit
4. **Improved error message** - Now shows the limit and suggests solutions
5. **Added comprehensive documentation** - Multiple guides updated

### Code Changes

#### 1. Configuration Structure (`src/config/mod.rs`)
- Added `max_tool_loop_iterations` field to `AcpConfig` struct
- Default value: 25 iterations
- Supported via environment variable: `GROK_ACP_MAX_TOOL_LOOP_ITERATIONS`

#### 2. ACP Module (`src/acp/mod.rs`)
- Changed from hardcoded `let max_loops = 10;`
- Now uses `let max_loops = self.config.acp.max_tool_loop_iterations;`
- Enhanced error message with helpful guidance

#### 3. Documentation Updates
- `TOOLS.md` - Updated troubleshooting section
- `CONFIGURATION.md` - Added ACP configuration examples
- `settings.md` - Added setting to reference table
- `README.md` - Added troubleshooting section
- `CHANGELOG.md` - Documented the change
- Created `MAX_TOOL_LOOP_ITERATIONS.md` - Comprehensive guide

## How to Use

### Option 1: Configuration File (Recommended)

Edit `~/.config/grok-cli/config.toml`:

```toml
[acp]
max_tool_loop_iterations = 50  # Increase as needed
```

Or use the CLI:
```bash
grok config set acp.max_tool_loop_iterations 50
```

### Option 2: Environment Variable

**Linux/macOS:**
```bash
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
```

**Windows PowerShell:**
```powershell
$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS="50"
```

**Windows CMD:**
```cmd
set GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
```

### Option 3: Project .env File

Add to `.env` in your project root:
```env
GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
```

## Recommended Values

| Use Case | Value | Notes |
|----------|-------|-------|
| Simple tasks | 10-15 | Quick operations |
| **Default** | **25** | Handles most scenarios |
| Complex tasks | 50 | Multi-file operations |
| Very complex | 100+ | Large-scale refactoring |

## Build Status

✅ **Code compiles successfully**
```bash
cargo check    # ✅ Passed
cargo build --release  # ✅ Passed (1m 38s)
```

## Files Modified

1. `src/config/mod.rs` - Added configuration field and env var support
2. `src/acp/mod.rs` - Use configurable limit instead of hardcoded value
3. `Doc/docs/TOOLS.md` - Updated troubleshooting section
4. `Doc/docs/settings.md` - Added setting reference
5. `CONFIGURATION.md` - Added ACP configuration examples
6. `README.md` - Added troubleshooting guide
7. `CHANGELOG.md` - Documented the change

## Files Created

1. `config.example.toml` - Example configuration with all settings
2. `Doc/MAX_TOOL_LOOP_ITERATIONS.md` - Comprehensive 346-line guide
3. `FIX_SUMMARY.md` - This file

## Testing

The fix has been validated:
- ✅ Compiles without errors
- ✅ Configuration loads correctly
- ✅ Environment variables are applied
- ✅ Default value (25) is used when not configured
- ✅ Error message includes helpful guidance

## Next Steps

1. **Rebuild the project:**
   ```bash
   cd H:\GitHub\grok-cli
   cargo build --release
   ```

2. **Test with increased limit:**
   ```bash
   # Set a higher limit for testing
   export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
   
   # Try your previous command that failed
   grok acp stdio  # or whatever command you were using
   ```

3. **Monitor usage:**
   - If you still hit the limit, increase further
   - If tasks complete quickly, you may lower it
   - Default of 25 should work for most cases

## Best Practices to Avoid Hitting the Limit

1. **Break complex tasks into smaller steps**
   - Instead of: "Refactor everything"
   - Try: "First update main.rs, then update tests"

2. **Be more specific in your requests**
   - Vague: "Fix the bugs"
   - Specific: "Fix the null pointer on line 42"

3. **Provide clear context**
   - Tell the AI where relevant code is located
   - Explain what you're trying to accomplish

4. **Use progressive disclosure**
   - Start simple, add complexity gradually
   - Let the AI complete one step before asking for the next

## When to Increase the Limit

Increase if you're doing:
- ✅ Large refactoring operations across multiple files
- ✅ Complex code generation with dependencies
- ✅ Deep codebase analysis reading many files
- ✅ Multi-step workflows with tool chaining
- ✅ Iterative improvements over multiple passes

## When NOT to Increase

Keep it low if:
- ❌ The AI is calling the same tool repeatedly
- ❌ The task seems to be in an infinite loop
- ❌ You want to quickly identify stuck operations
- ❌ You're debugging tool behavior

## Additional Resources

- **Comprehensive Guide:** `Doc/MAX_TOOL_LOOP_ITERATIONS.md`
- **Tool Documentation:** `Doc/docs/TOOLS.md`
- **Configuration Guide:** `CONFIGURATION.md`
- **Settings Reference:** `Doc/docs/settings.md`
- **Example Config:** `config.example.toml`

## Support

If you continue to experience issues:

1. **Check configuration:**
   ```bash
   grok config get acp.max_tool_loop_iterations
   grok config validate
   ```

2. **Enable verbose logging:**
   ```bash
   grok --verbose chat "your command"
   ```

3. **Create an issue:**
   - GitHub: https://github.com/microtech/grok-cli/issues
   - Include: The error message, task description, and iteration count

## Summary

✅ **Fixed:** Max tool loop iterations is now configurable  
✅ **Increased:** Default from 10 → 25 iterations  
✅ **Added:** Three configuration methods (config file, env var, .env)  
✅ **Improved:** Error messages with actionable guidance  
✅ **Documented:** Comprehensive guides and examples  
✅ **Tested:** Compiles and runs successfully  

You should now be able to handle more complex tasks without hitting the iteration limit. Start with the default of 25, and increase to 50 or higher only if needed for particularly complex operations.

---

**Author:** john mcconnell (john.microtech@gmail.com)  
**Repository:** https://github.com/microtech/grok-cli  
**Support:** https://buymeacoffee.com/micro.tech