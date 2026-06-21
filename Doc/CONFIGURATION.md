# Grok CLI Configuration Guide

> **Full detailed version** — This is the complete configuration reference.
> For a quick overview, see the [root README](../README.md).

This document explains how to configure the Grok CLI using `.env` files.

## Model Context Budgets

Different Grok models expose different context windows.  Grok-CLI automatically selects the correct token budget based on the active model name:

| Model prefix | Context window | Default soft budget (`config.toml`) |
|---|---|---|
| `grok-4.*` | 1,048,576 tokens (~1 M) | `grok4_max_context_tokens = 950000` |
| `grok-3.*` | 262,144 tokens (256 k) | `max_context_tokens = 220000` |
| `grok-2.*` | 131,072 tokens (128 k) | `max_context_tokens = 220000` |

The "soft budget" trims the oldest conversation messages before each API call so the request never exceeds the model's hard limit.  It leaves headroom for the model response and tool definitions.

To override either budget, edit `~/.grok-cli/config.toml` (system) or your project's `.grok/config.toml` under the `[acp]` section:

```toml
[acp]
# Budget for grok-3 and older models
max_context_tokens = 220000

# Budget for grok-4.x models (auto-selected when model starts with "grok-4")
grok4_max_context_tokens = 950000
```

### Output token budget (`default_max_tokens`)

This is separate from the *context* window.  It caps the number of tokens the model may generate in a single response:

```toml
# Top-level (not under [acp])
default_max_tokens = 16384  # grok-4.3 default; max supported is 32768
```

## Thinking Modes (grok-4.3 / grok-3-mini)

grok-4.3 supports extended chain-of-thought reasoning via the `reasoning_effort` API parameter.  Grok-CLI exposes this as a first-class feature:

| Mode | API value | Effect |
|---|---|---|
| `off` (default) | omitted | Standard response, no reasoning trace |
| `low` | `"low"` | Light reasoning — faster, slightly more thorough |
| `high` | `"high"` | Deep reasoning — most thorough, slower, higher token cost |

### Configure the default mode globally

```toml
[acp]
thinking_mode = "off"   # off | low | high
```

### Change per-session at runtime

Use the `/think` slash command in any ACP session (e.g. Zed):

```
/think high      # enable deep reasoning
/think low       # enable light reasoning
/think off       # disable reasoning
/think           # show the current mode
```

### Set via CLI flag

```bash
grok chat --thinking high "Explain Rust lifetimes in detail"
grok chat --thinking low  "Write a hello-world in Python"
```

### What happens with thinking content

When the model produces a reasoning trace it is displayed as a collapsible `<details>` block before the main answer:

```
<details><summary>🧠 Thinking…</summary>
  (chain-of-thought reasoning)
</details>

(main answer here)
```

> **Note:** Only send `reasoning_effort` to models that support it.  Sending it to unsupported models (grok-3, grok-2, etc.) returns an API error.  Grok-CLI never sends the field when `thinking_mode = "off"`.

## Overview

Grok CLI uses a hierarchical configuration system based on `.env` files. This approach provides:

- **Single configuration format** - All settings use environment variables
- **Project-specific overrides** - Different settings per project
- **System-wide defaults** - Shared settings across all projects
- **Clear precedence rules** - Predictable configuration behavior

## Configuration Hierarchy

Settings are loaded in this order (highest priority first):

1. **Shell/Process Environment Variables** - Set in your terminal or CI/CD
2. **CLI Arguments** - Flags like `--model`, `--config`
3. **Project Configuration** - `.grok/.env` in project root
4. **System Configuration** - `~/.grok/.env` (or `%USERPROFILE%\.grok\.env` on Windows)
5. **Built-in Defaults** - Fallback values

Later sources override earlier ones. For example, a project `.env` overrides system config, but environment variables override everything.

## Quick Start

### 1. Copy the Example File

```bash
# For root project configuration
cp .env.example .env

# For project-specific configuration
mkdir -p .grok
cp .grok/.env.example .grok/.env
```

### 2. Set Your API Key

Edit `.env` and add your X API key:

```bash
GROK_API_KEY=xai-your-api-key-here
```

### 3. Customize Settings (Optional)

```bash
# Set preferred model
GROK_MODEL=grok-code-fast-1

# Adjust temperature (0.0-2.0)
GROK_TEMPERATURE=0.7

# Set timeout for slow connections
GROK_TIMEOUT=60
```

## Configuration Locations

### Project Configuration: `.grok/.env`

Place this in your project root for project-specific settings:

```
your-project/
├── .grok/
│   └── .env          ← Project-specific config
├── .git/
├── src/
└── README.md
```

**When to use:**
- Project requires a specific model (e.g., code projects use `grok-code-fast-1`)
- Different timeout settings for specific projects
- Project-specific logging or debugging

### System Configuration: `~/.grok-cli/.env`

Place this in your home directory for user-wide defaults:

**Linux/macOS:** `~/.grok-cli/.env`
**Windows:** `%APPDATA%\grok-cli\.env` (or `%USERPROFILE%\.grok-cli\.env`)

**When to use:**
- Settings shared across all your projects
- Your API key (so you don't repeat it in every project)
- Personal preferences (colors, UI settings, etc.)

### Root Project: `.env`

The repository root `.env` file for development/testing of Grok CLI itself.

## Available Settings

### API Configuration

```bash
# X API Key for Grok access (REQUIRED)
GROK_API_KEY=xai-your-api-key-here

# Alternative variable name (same effect)
X_API_KEY=xai-your-api-key-here
```

### Model Settings

```bash
# Default model to use
# Options: grok-4-1-fast-reasoning, grok-3, grok-2-latest, grok-2, grok-code-fast-1, grok-vision-beta
GROK_MODEL=grok-4-1-fast-reasoning

# Temperature (0.0 = deterministic, 2.0 = very creative)
GROK_TEMPERATURE=0.7

# Maximum tokens in response
GROK_MAX_TOKENS=4096
```

### Network Configuration

```bash
# Request timeout in seconds
GROK_TIMEOUT=30

# Maximum number of retries for failed requests
GROK_MAX_RETRIES=3

# Enable Starlink-specific optimizations
GROK_STARLINK_OPTIMIZATIONS=true

# Retry delay settings (in seconds)
GROK_BASE_RETRY_DELAY=1
GROK_MAX_RETRY_DELAY=60

# Enable network health monitoring
GROK_HEALTH_MONITORING=true

# Connection and read timeouts
GROK_CONNECT_TIMEOUT=10
GROK_READ_TIMEOUT=30
```

### UI Configuration

```bash
# Enable colored output
GROK_COLORS=true

# Enable progress bars
GROK_PROGRESS_BARS=true

# Enable Unicode characters in output
GROK_UNICODE=true

# Show verbose error information
GROK_VERBOSE_ERRORS=false

# Terminal width (0 = auto-detect)
GROK_TERMINAL_WIDTH=0

# Disable colors (useful for CI/CD)
NO_COLOR=1
```

### Logging Configuration

```bash
# Log level: trace, debug, info, warn, error
GROK_LOG_LEVEL=info

# Enable file logging
GROK_FILE_LOGGING=false

# Log file path (optional)
GROK_LOG_FILE=/path/to/grok.log

# Maximum log file size in MB
GROK_MAX_FILE_SIZE_MB=10

# Number of log files to rotate
GROK_ROTATION_COUNT=5

# Rust internal logging (overrides GROK_LOG_LEVEL)
RUST_LOG=grok_cli=debug
```

### Chat Logging Configuration

```bash
# Enable chat session logging (saves conversations to disk)
GROK_CHAT_LOGGING_ENABLED=true

# Directory for chat logs
# Project-scoped (preferred): <project>/.grok/logs/chat_sessions/
# System fallback: ~/.grok/logs/chat_sessions/ (only if no project .grok/ exists)
GROK_CHAT_LOG_DIR=/path/to/chat/logs

# Maximum log file size in MB before rotation
GROK_CHAT_LOG_MAX_SIZE_MB=10

# Number of rotated chat log files to keep
GROK_CHAT_LOG_ROTATION_COUNT=5

# Include system messages in chat logs
GROK_CHAT_LOG_INCLUDE_SYSTEM=true
```

Chat logs are saved in both JSON and human-readable text formats:
- **JSON format**: `<project>/.grok/logs/chat_sessions/<id>.json` (preferred) or `~/.grok/logs/chat_sessions/<id>.json`
- **Text format**: same locations with `.txt` extension

View chat history with:
```bash
grok history list              # List all sessions
grok history view <session-id> # View specific session
grok history search "query"    # Search through sessions
grok history clear --confirm   # Clear all history
```

### ACP (Agent Client Protocol) Configuration

```bash
# Enable ACP functionality
GROK_ACP_ENABLED=true

# Disable ACP (alternative way)
GROK_ACP_DISABLE=true

# Port for ACP server (0 = auto-assign)
GROK_ACP_PORT=0

# Host to bind ACP server to
GROK_ACP_BIND_HOST=127.0.0.1

# ACP protocol version
GROK_ACP_PROTOCOL_VERSION=1.0

# Enable development mode
GROK_ACP_DEV_MODE=false

# Maximum tool loop iterations (prevents infinite loops)
# Increase this for complex multi-step tasks
# Default: 25
GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=100
```

**Note about Max Tool Loop Iterations:**
- This setting prevents infinite loops when the AI repeatedly calls tools
- If you encounter "Max tool loop iterations reached" errors:
  - Increase this value for complex tasks (e.g., 50 or higher)
  - Break your task into smaller, more focused steps
  - Check if the AI is stuck calling the same tool repeatedly
- The default of 25 iterations should handle most tasks
- You can also set this in your `config.toml` file:
  ```toml
  [acp]
  max_tool_loop_iterations = 50
  ```

### Commit Message Generator (Task 161)

Grok CLI can generate high-quality Conventional Commits messages from your staged (or unstaged) git changes.

**Slash command (recommended):**
```
/commit                    # Uses default Conventional Commits style
/commit fix auth edge case  # Add extra instructions
```

**Tool for the agent:**
- `generate_commit_message` — the AI can call this tool when it needs to propose a commit message during a workflow.

**Configuration:**
```toml
[acp]
commit_message_instructions = "Use Conventional Commits with scope and breaking-change footer when appropriate."
```

This string is appended to every commit prompt (both from `/commit` and the tool).

**How it works:**
- Runs `git diff --cached` first; falls back to `git diff` if nothing is staged.
- Builds a prompt containing the diff, current session goal (if any), Session DNA, and any custom instructions.
- The model returns a properly formatted commit message ready to copy into your Git UI or commit directly.

**Example output style:**
```
feat(auth): add JWT refresh token rotation

- Implement secure refresh token rotation with HttpOnly cookies
- Add rate limiting on refresh endpoint
- Update tests for new rotation logic

Closes #123
```

This feature is especially useful inside Zed or other ACP clients when you want the agent to help write commit messages.

### Telemetry Configuration

```bash
# Enable telemetry (opt-in)
GROK_TELEMETRY_ENABLED=false

# Path to telemetry log file (optional)
GROK_TELEMETRY_LOG_FILE=/path/to/telemetry.log
```

## Common Scenarios

### Scenario 1: Different Models per Project

**System config** (`~/.grok/.env`):
```bash
GROK_API_KEY=xai-your-key
GROK_MODEL=grok-4-1-fast-reasoning  # Default for most projects
```

**Code project** (`.grok/.env`):
```bash
GROK_MODEL=grok-code-fast-1  # Optimized for coding
```

**Creative project** (`.grok/.env`):
```bash
GROK_MODEL=grok-4-1-fast-reasoning
GROK_TEMPERATURE=1.5  # More creative
```

### Scenario 2: Satellite/Slow Connection

**Project config** (`.grok/.env`):
```bash
GROK_TIMEOUT=120
GROK_MAX_RETRIES=5
GROK_STARLINK_OPTIMIZATIONS=true
GROK_BASE_RETRY_DELAY=2
GROK_MAX_RETRY_DELAY=120
```

### Scenario 3: CI/CD Pipeline

**GitHub Actions** (`.github/workflows/test.yml`):
```yaml
env:
  GROK_API_KEY: ${{ secrets.GROK_API_KEY }}
  GROK_MODEL: grok-code-fast-1
  GROK_TIMEOUT: 60
  NO_COLOR: 1  # Disable colors in CI
  GROK_PROGRESS_BARS: false
```

### Scenario 4: Debugging

**Temporary override** (in terminal):
```bash
# Enable debug logging for one run
GROK_LOG_LEVEL=debug RUST_LOG=grok_cli=debug grok

# Or export for session
export GROK_LOG_LEVEL=debug
export GROK_VERBOSE_ERRORS=true
grok
```

## Verifying Configuration

Check which settings are active:

```bash
grok config show
```

Output shows:
- Current values for all settings
- Configuration source (project/system/defaults)

Example:
```
Configuration Source:
  project (H:\GitHub\grok-cli\.grok\.env)

API Configuration:
  API Key: ✓ Set (hidden)
  Default Model: grok-code-fast-1
  Temperature: 0.7
  ...
```

## Troubleshooting

### Issue: Wrong Model Being Used

**Check the hierarchy:**
1. Is `GROK_MODEL` set in your shell? (`echo $GROK_MODEL` or `echo %GROK_MODEL%`)
2. Did you pass `--model` flag?
3. Check project `.env`: `cat .grok/.env | grep GROK_MODEL`
4. Check system `.env`: `cat ~/.grok/.env | grep GROK_MODEL`

**Solution:** Unset environment variable if unwanted:
```bash
unset GROK_MODEL  # Linux/macOS
set GROK_MODEL=   # Windows CMD
```

### Issue: Configuration Not Loading

**Verify file locations:**
```bash
# Check if .env exists
ls -la .grok/.env
ls -la ~/.grok/.env

# Check file contents
cat .grok/.env
```

**Common mistakes:**
- File named `.env.example` instead of `.env`
- `.env` file in wrong directory
- Syntax errors in `.env` file (check for quotes, spaces)

### Issue: API Key Not Found

**Check all possible locations:**
```bash
# 1. Check environment variables
echo $GROK_API_KEY
echo $X_API_KEY

# 2. Check project .env
grep GROK_API_KEY .grok/.env

# 3. Check system .env
grep GROK_API_KEY ~/.grok/.env

# 4. Check root .env
grep GROK_API_KEY .env
```

## Security Best Practices

### 1. Never Commit `.env` Files

Add to `.gitignore`:
```gitignore
.env
.grok/.env
**/.env
```

### 2. Use `.env.example` Templates

Commit example files without secrets:
```bash
# .env.example
GROK_API_KEY=xai-your-key-here
GROK_MODEL=grok-4-1-fast-reasoning
```

### 3. Use Secret Management in Production

For CI/CD and production:
- GitHub Actions: Use repository secrets
- GitLab CI: Use CI/CD variables
- Kubernetes: Use Secrets
- Docker: Use secrets or environment injection

### 4. Restrict File Permissions

```bash
# Make .env readable only by you
chmod 600 .grok/.env
chmod 600 ~/.grok/.env
```

## Migration from TOML

If you have existing `config.toml` files, convert them to `.env`:

**Old** (`config.toml`):
```toml
default_model = "grok-4-1-fast-reasoning"
default_temperature = 0.7
timeout_secs = 30

[ui]
colors = true
progress_bars = true
```

**New** (`.env`):
```bash
GROK_MODEL=grok-4-1-fast-reasoning
GROK_TEMPERATURE=0.7
GROK_TIMEOUT=30
GROK_COLORS=true
GROK_PROGRESS_BARS=true
```

## Advanced Usage

### Using Multiple Configuration Files

```bash
# Load specific .env file
grok --config /path/to/custom/.env

# Override with temporary settings
GROK_MODEL=grok-2 grok query "test"
```

### Environment Variable Naming Convention

All Grok CLI variables follow this pattern:
```
GROK_<SECTION>_<SETTING>
```

Examples:
- `GROK_MODEL` - Top-level model setting
- `GROK_ACP_ENABLED` - ACP section, enabled setting
- `GROK_LOG_LEVEL` - Logging section, level setting

### Programmatic Configuration

For scripts and automation:

```bash
#!/bin/bash
export GROK_API_KEY="${MY_SECRET_KEY}"
export GROK_MODEL="grok-code-fast-1"
export GROK_TIMEOUT=60

grok query "What is the meaning of life?"
```

## Getting Help

- Show current configuration: `grok config show`
- Validate configuration: `grok config validate`
- View help: `grok --help`
- Check version: `grok --version`

For more information, see the [root README](../README.md).
