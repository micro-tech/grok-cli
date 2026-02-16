# Troubleshooting Tool Loop Issues

## Overview

This document helps you diagnose and fix issues where the Grok CLI AI assistant gets stuck in a tool loop, repeatedly calling tools until it hits the maximum iteration limit.

## Symptoms

- Error message: `Max tool loop iterations reached (50 iterations)`
- The AI makes the same tool call repeatedly
- Simple tasks (like reading one file) trigger many tool calls
- Sessions timeout or exhaust the iteration budget

## Understanding Tool Loops

The tool loop is the AI's reasoning cycle:

1. AI receives your request
2. AI decides which tools to use
3. AI calls tool(s) and receives results
4. AI analyzes results and decides next action
5. Repeat until task is complete

**Normal behavior**: Most tasks complete in 1-10 iterations.

**Abnormal behavior**: AI gets stuck calling the same tool(s) repeatedly without making progress.

## Common Causes

### 1. Configuration Issues

**Problem**: Missing or malformed configuration fields

**Check**:
```bash
# View your project config
cat .grok/config.toml

# View your system config (Windows)
cat "$env:APPDATA\grok-cli\config.toml"

# View your system config (Linux/Mac)
cat ~/.config/grok-cli/config.toml
```

**Solution**: Ensure MCP server configurations have required fields:
```toml
[mcp.servers.example]
type = "stdio"
command = "path/to/executable"
args = []
env = {}  # This field is REQUIRED even if empty
```

See `config.example.toml` for complete MCP server configuration examples.

### 2. AI Not Recognizing Task Completion

**Problem**: The AI doesn't understand when to stop

**Symptoms**:
- All finish reasons are `"tool_calls"` instead of `"stop"`
- AI keeps requesting more information even after getting what it needs
- Task seems complete but AI continues working

**Solutions**:

**Be explicit in your prompts**:
- ❌ Bad: "read the file"
- ✅ Good: "read the file config.toml and tell me what the api_key setting is. Then stop."

**Add termination instructions**:
- "After you find X, stop and tell me the result"
- "This is a single-step task - just read and report back"
- "Don't analyze or process the data, just show it to me"

**Use concrete questions**:
- ❌ Vague: "check the configuration"
- ✅ Specific: "what is the value of max_tool_loop_iterations in .grok/config.toml?"

### 3. Tool Response Confusion

**Problem**: Tool responses contain information that confuses the AI

**Symptoms**:
- AI reads the same file multiple times
- AI searches for something it already found
- AI tries to "verify" or "double-check" results

**Solutions**:
- Use more specific paths (avoid wildcards when possible)
- Request specific information rather than exploratory tasks
- Break complex tasks into explicit steps

### 4. Network Issues (Starlink/Satellite)

**Problem**: Network drops cause incomplete responses

**Symptoms**:
- Errors in debug log about timeouts or connection failures
- Retries that succeed but trigger additional tool calls
- Inconsistent behavior across sessions

**Solutions**:

Check network configuration in your config file:
```toml
[network]
starlink_optimizations = true
base_retry_delay = 2
max_retry_delay = 60
health_monitoring = true
connect_timeout = 30
read_timeout = 60
```

Increase timeouts if needed:
```toml
[network]
connect_timeout = 60  # Increase from 30
read_timeout = 120    # Increase from 60
```

## Diagnostic Tools

### 1. Analyze Debug Logs

Run the analyzer script:
```powershell
# Windows PowerShell
.\analyze_tool_loops.ps1

# With more log lines
.\analyze_tool_loops.ps1 -ShowLastN 200
```

Or manually inspect:
```bash
# Show tool iterations
grep "Tool loop iteration" acp_debug.log

# Show what tools were called
grep "Tool [0-9]" acp_debug.log

# Show finish reasons
grep "Finish reason" acp_debug.log

# Show errors
grep -i "error\|❌" acp_debug.log
```

### 2. Run Test Script

Test with a controlled scenario:
```bash
# Linux/Mac/Git Bash
./test_tool_loop_debug.sh

# Windows PowerShell (create your own based on the .sh version)
```

### 3. Set Lower Iteration Limit

Temporarily set a low limit to catch issues faster:

```powershell
# PowerShell
$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS = "10"

# Bash
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=10
```

Then run your problematic command. If it fails immediately, you know there's a loop issue.

## Solutions

### Quick Fixes

1. **Fix configuration syntax**:
   - Ensure all TOML is valid
   - Add missing required fields (`env = {}`)
   - Check for proper quoting and escaping

2. **Lower the limit temporarily**:
   ```toml
   [acp]
   max_tool_loop_iterations = 10
   ```
   This helps expose the issue faster during debugging.

3. **Rephrase your prompt**:
   - Add explicit stopping conditions
   - Use imperative language ("read", "show", "tell me")
   - Avoid open-ended exploration ("investigate", "check everything")

### Configuration Fix

Edit `.grok/config.toml`:

```toml
# Grok CLI Project Configuration

# ACP Configuration
[acp]
enabled = true
max_tool_loop_iterations = 25  # Default: 25, increase if needed

# MCP Servers Configuration (if using MCP)
[mcp.servers.example]
type = "stdio"
command = "path/to/executable"
args = []
env = {}  # Required field
```

**Tip**: Copy from `config.example.toml` to get a complete configuration template with all available options and MCP server examples.

### Prompt Engineering

**Structure your requests clearly**:

```
Good prompt template:
"[ACTION] [TARGET] [SPECIFIC_REQUIREMENT]. [TERMINATION_INSTRUCTION]."

Examples:
- "Read the file config.toml and show me the api_key value. That's all I need."
- "List files in the src directory. Just show the list, don't read the files."
- "Search for 'TODO' in README.md. Report the line numbers and stop."
```

### Advanced Solutions

1. **Enable verbose logging**:
   ```toml
   [logging]
   level = "debug"
   file_logging = true
   ```

2. **Check tool execution**:
   Review which tools are available and working:
   ```bash
   cargo run --bin grok -- config show
   ```

3. **Update dependencies**:
   ```bash
   cargo update
   cargo build --release
   ```

4. **Clear cache and state**:
   ```bash
   # Clear any cached sessions
   rm -rf ~/.grok/cache/
   
   # Windows
   Remove-Item "$env:USERPROFILE\.grok\cache\" -Recurse -Force
   ```

## Prevention

### Best Practices

1. **Write Clear Prompts**:
   - Be specific about what you want
   - Add stopping conditions
   - Avoid ambiguous language

2. **Monitor Iterations**:
   - Check `acp_debug.log` occasionally
   - Look for patterns of repeated tool calls
   - Adjust prompts based on patterns

3. **Use Appropriate Limits**:
   - Default (25): Most tasks
   - Medium (50): Multi-step complex tasks
   - High (100+): Very complex multi-file operations

4. **Test Configuration Changes**:
   - After changing config, run a simple test
   - Verify MCP servers are configured correctly
   - Check that tool execution works

## When to Increase the Limit

Only increase `max_tool_loop_iterations` if:

- ✅ You have a genuinely complex multi-step task
- ✅ Debug logs show steady progress (different tools, different files)
- ✅ The AI is not repeating the same actions
- ✅ You've verified there's no configuration issue

Do NOT increase the limit if:

- ❌ The same tool is called repeatedly
- ❌ The AI seems confused or stuck
- ❌ You're getting the error on simple tasks
- ❌ Debug logs show circular behavior

**Remember**: The limit exists to prevent infinite loops. Increasing it won't fix a loop problem - it will just let the AI run longer before failing.

## Reporting Issues

If you've tried all solutions and still experience loops:

1. **Collect information**:
   - Your prompt/request
   - Config files (`.grok/config.toml`, system config)
   - Debug log (`acp_debug.log`)
   - Output from `analyze_tool_loops.ps1`

2. **Create a minimal reproduction**:
   - Simplify the task to the smallest failing example
   - Document exact steps to reproduce

3. **File an issue**:
   - Repository: https://github.com/microtech/grok-cli
   - Include all collected information
   - Tag with "tool-loop" or "acp-bug"

## Examples

### Example 1: Reading a File

**Problem**:
```
User: "read config.toml"
Result: 50 iterations, max limit reached
```

**Analysis**:
- Prompt is too vague
- No stopping condition
- AI might try to analyze, explain, or explore

**Solution**:
```
User: "read the file .grok/config.toml and show me its contents. That's all."
Result: 2 iterations (read, respond)
```

### Example 2: Finding Configuration

**Problem**:
```
User: "find my api key"
Result: AI searches multiple files, checks environment, reads docs
```

**Analysis**:
- Too exploratory
- No specific target
- AI keeps searching "just in case"

**Solution**:
```
User: "check the api_key field in config.toml. If it's there, show it. If not, tell me it's missing."
Result: 3-4 iterations (find file, read, respond)
```

### Example 3: Complex Multi-Step Task

**Problem**:
```
User: "refactor all the error handling in the project"
Result: 25 iterations, incomplete
```

**Analysis**:
- Task is legitimately complex
- Requires many files and steps
- 25 iterations might not be enough

**Solution**:
```toml
# Increase limit for this session
[acp]
max_tool_loop_iterations = 100
```

```
User: "refactor error handling in the following files: [list specific files]. Update each file to use Result<T, AppError>. After each file, confirm the change. When all files are done, summarize the changes."
Result: 60-80 iterations (multiple files, systematic approach)
```

## Summary

Tool loops are usually caused by:
1. Configuration issues (missing fields, syntax errors)
2. Vague prompts that don't clearly define success
3. AI confusion about task completion

**The fix is usually NOT to increase the limit**, but to:
1. Fix configuration
2. Write clearer prompts
3. Add explicit stopping conditions

Use the diagnostic tools provided to identify the root cause, then apply the appropriate solution.

---

**Need Help?**
- Check the debug log: `acp_debug.log`
- Run the analyzer: `.\analyze_tool_loops.ps1`
- Join discussions: https://github.com/microtech/grok-cli/discussions
- Report bugs: https://github.com/microtech/grok-cli/issues