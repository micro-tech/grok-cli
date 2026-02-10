# Grok CLI Configuration Guide

This document explains how to configure the Grok CLI using `.env` files.

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

### System Configuration: `~/.grok/.env`

Place this in your home directory for user-wide defaults:

**Linux/macOS:** `~/.grok/.env`
**Windows:** `%USERPROFILE%\.grok\.env` (e.g., `C:\Users\YourName\.grok\.env`)

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
# Options: grok-3, grok-2-latest, grok-2, grok-code-fast-1, grok-vision-beta
GROK_MODEL=grok-3

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

# Directory for chat logs (default: ~/.grok/logs/chat_sessions)
GROK_CHAT_LOG_DIR=/path/to/chat/logs

# Maximum log file size in MB before rotation
GROK_CHAT_LOG_MAX_SIZE_MB=10

# Number of rotated chat log files to keep
GROK_CHAT_LOG_ROTATION_COUNT=5

# Include system messages in chat logs
GROK_CHAT_LOG_INCLUDE_SYSTEM=true
```

Chat logs are saved in both JSON and human-readable text formats:
- **JSON format**: `~/.grok/logs/chat_sessions/<session-id>.json` - Machine-readable, full metadata
- **Text format**: `~/.grok/logs/chat_sessions/<session-id>.txt` - Human-readable conversation transcript

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
GROK_MODEL=grok-3  # Default for most projects
```

**Code project** (`.grok/.env`):
```bash
GROK_MODEL=grok-code-fast-1  # Optimized for coding
```

**Creative project** (`.grok/.env`):
```bash
GROK_MODEL=grok-3
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
GROK_MODEL=grok-3
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
default_model = "grok-3"
default_temperature = 0.7
timeout_secs = 30

[ui]
colors = true
progress_bars = true
```

**New** (`.env`):
```bash
GROK_MODEL=grok-3
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

For more information, see the [README](README.md) or [API documentation](docs/API.md).
