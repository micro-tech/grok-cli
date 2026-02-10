# Max Tool Loop Iterations - Configuration Guide

## Overview

The "Max tool loop iterations reached" error occurs when Grok CLI's AI agent repeatedly calls tools without completing a task. This document explains the fix and how to configure it.

## What Was Changed

### Previous Behavior
- Max tool loop iterations was **hardcoded to 10**
- No way to adjust this limit
- Complex tasks would frequently hit this limit
- Error message didn't provide helpful guidance

### New Behavior (v0.1.3+)
- Max tool loop iterations is now **configurable**
- Default increased to **25 iterations** (2.5x more than before)
- Can be adjusted via config file or environment variable
- Improved error message with actionable suggestions

## Why This Matters

The AI agent uses tools to perform operations like:
- Reading files (`read_file`)
- Writing files (`write_file`)
- Searching code (`glob_search`)
- Executing shell commands
- And more...

Complex tasks may require many tool calls in sequence. The iteration limit prevents infinite loops while allowing legitimate multi-step operations to complete.

## Configuration Options

### Option 1: Configuration File (Recommended)

Edit your config file at `~/.config/grok-cli/config.toml`:

```toml
[acp]
max_tool_loop_iterations = 50  # Increase as needed
```

Or use the CLI:

```bash
grok config set acp.max_tool_loop_iterations 50
```

### Option 2: Environment Variable

Set the environment variable:

**Linux/macOS:**
```bash
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
```

**Windows (PowerShell):**
```powershell
$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS="50"
```

**Windows (CMD):**
```cmd
set GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
```

### Option 3: .env File

Add to your `.env` file in the project root:

```env
GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
```

## Recommended Values

| Use Case | Recommended Value | Notes |
|----------|-------------------|-------|
| **Simple tasks** | 10-15 | Quick file operations, single-step tasks |
| **Default** | 25 | Handles most common scenarios |
| **Complex tasks** | 50 | Multi-file refactoring, complex analysis |
| **Very complex** | 100+ | Large-scale codebase operations |
| **Debug mode** | 10 | Quickly identify infinite loops |

## When to Increase the Limit

Increase the limit if you encounter tasks like:

- **Large refactoring operations** - Modifying many files
- **Complex code generation** - Creating multiple related files
- **Deep code analysis** - Reading and analyzing many files
- **Multi-step workflows** - Chained operations with dependencies
- **Iterative improvements** - AI making multiple refinement passes

## When NOT to Increase the Limit

Keep the limit low if:

- The AI is calling the same tool repeatedly with the same arguments
- The task seems to be in an infinite loop
- You want to identify if the AI is stuck
- You're debugging tool behavior

## Best Practices to Avoid Hitting the Limit

### 1. Break Tasks Into Steps

**Instead of:**
```
"Refactor the entire codebase to use async/await, add error handling,
update tests, and generate documentation"
```

**Try:**
```
"First, update src/main.rs to use async/await"
"Now update the tests to match the async changes"
"Generate documentation for the new async API"
```

### 2. Be More Specific

**Vague:**
```
"Fix the bugs in this code"
```

**Specific:**
```
"Fix the null pointer error on line 42 by adding a null check"
```

### 3. Provide Context

Help the AI understand what you need:

```
"I need to add authentication to the API. 
The user model is in src/models/user.rs.
Add JWT token generation in src/auth.rs.
Update the API routes to require authentication."
```

### 4. Use Progressive Disclosure

Start simple and add complexity:

```
1. "Create a basic HTTP server"
2. "Add a /users endpoint"
3. "Add authentication to the endpoint"
```

## Troubleshooting

### Error Still Occurs After Increasing Limit

**Check if the AI is stuck in a loop:**
1. Look at the tool calls - are they repetitive?
2. Is the AI calling the same tool with the same arguments?
3. Try rephrasing your request more clearly

**Verify configuration is loaded:**
```bash
# Check current setting
grok config get acp.max_tool_loop_iterations

# Validate entire config
grok config validate
```

### Task Genuinely Needs Many Iterations

Some legitimate scenarios:

- **Bulk operations** - Processing many files
- **Iterative refinement** - AI improving code quality over multiple passes
- **Complex analysis** - Reading and cross-referencing many files
- **Code migration** - Updating patterns across a large codebase

For these cases, increase the limit to 100 or higher.

### Finding the Right Balance

Too low:
- Legitimate tasks fail
- Frustrating user experience
- Need to retry frequently

Too high:
- Stuck tasks take longer to fail
- May consume API tokens unnecessarily
- Harder to identify infinite loops

**Recommendation:** Start at 25, increase to 50 if needed, go higher only for specific complex tasks.

## Error Message Details

**Old Error Message:**
```
Max tool loop iterations reached
```

**New Error Message:**
```
Max tool loop iterations reached (25 iterations). 
Consider increasing 'acp.max_tool_loop_iterations' in config 
or breaking task into smaller steps.
```

The new message:
- Shows the current limit (25 in this example)
- Suggests the configuration option to change
- Recommends breaking tasks into smaller steps

## Impact on Performance

### API Token Usage

Each tool call consumes tokens:
- Tool definition in system prompt
- Tool call request
- Tool result in context
- AI response processing tool result

More iterations = more token usage.

**Recommendation:** Use the lowest limit that works for your tasks to optimize token usage and cost.

### Response Time

More iterations = longer wait times.

If a task takes 3 seconds per iteration:
- 10 iterations = 30 seconds
- 25 iterations = 75 seconds (1.25 minutes)
- 50 iterations = 150 seconds (2.5 minutes)
- 100 iterations = 300 seconds (5 minutes)

**Recommendation:** Balance functionality with acceptable wait times.

## Examples

### Example 1: Default Works Fine

```bash
# Task: Simple file read and explain
grok chat "Explain what src/main.rs does"

# This typically uses 1-3 tool calls:
# 1. read_file(src/main.rs)
# 2. Maybe glob_search for dependencies
# 3. Response with explanation

# Result: ✅ Completes within default 25 iterations
```

### Example 2: Needs Increased Limit

```bash
# Task: Refactor multiple files
grok chat "Refactor all the database code to use connection pooling"

# This might use 30-40 tool calls:
# 1-10: glob_search and read_file to find all DB code
# 11-30: write_file to update each file
# 31-35: read_file to verify changes
# 36-40: Maybe additional fixes

# Solution:
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50
grok chat "Refactor all the database code to use connection pooling"

# Result: ✅ Completes with increased limit
```

### Example 3: Task Should Be Split

```bash
# Task: Too complex for single request
grok chat "Rewrite the entire application in a different framework"

# This would require hundreds of tool calls
# Even with high limits, quality may suffer

# Better approach:
grok chat "Create a plan to migrate to the new framework"
grok chat "Migrate the user authentication module first"
grok chat "Now migrate the API routes"
# ... continue step by step

# Result: ✅ Better results with incremental approach
```

## Related Documentation

- [TOOLS.md](TOOLS.md) - Complete tool documentation
- [CONFIGURATION.md](../CONFIGURATION.md) - Full configuration guide
- [settings.md](settings.md) - Settings reference
- [ZED_INTEGRATION.md](ZED_INTEGRATION.md) - Zed editor integration

## FAQ

**Q: Will increasing the limit slow down my tasks?**
A: Only if they actually use more iterations. The limit is a maximum, not a target.

**Q: Can I set different limits for different projects?**
A: Yes! Use project-specific `.env` files or config files.

**Q: What's the maximum value I can set?**
A: Technically unlimited (u32::MAX), but practically 100-200 is reasonable for even the most complex tasks.

**Q: Does this affect non-ACP usage?**
A: No, this setting only applies to ACP (Agent Client Protocol) mode used with Zed editor integration.

**Q: Will this be fixed to not need configuration?**
A: The AI will continue to improve, but complex tasks will always require many steps. Having configurability gives you control.

## Contributing

Found an issue or have suggestions? Please:

1. Check existing issues: https://github.com/microtech/grok-cli/issues
2. Create a new issue with details about your use case
3. Include the number of iterations your task required

## Changelog

- **v0.1.3** - Made max_tool_loop_iterations configurable, increased default from 10 to 25
- **v0.1.0** - Initial implementation with hardcoded limit of 10

## Credits

This fix was implemented based on:
- User feedback about hitting iteration limits
- xAI's guidance on tool loop management
- Best practices from other agentic frameworks (crewAI, LangChain)

---

**Questions or Issues?**

- GitHub: https://github.com/microtech/grok-cli/issues
- Documentation: [README.md](../../README.md)
- Support the project: https://buymeacoffee.com/micro.tech