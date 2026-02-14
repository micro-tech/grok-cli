# .env Configuration Guide for grok-cli

This file documents the recommended `.env` configuration for the grok-cli project.

## Location

Create this file at: `grok-cli/.grok/.env`

## Recommended Configuration

```env
# =============================================================================
# GROK-CLI PROJECT CONFIGURATION
# =============================================================================

# -----------------------------------------------------------------------------
# API Configuration
# -----------------------------------------------------------------------------
# Your Grok API key (REQUIRED - get from https://console.x.ai/)
# GROK_API_KEY=xai-your-key-here

# Default model to use for this project
GROK_MODEL=grok-code-fast-1

# Temperature setting (0.0 = deterministic, 1.0 = creative)
GROK_TEMPERATURE=0.7

# Maximum tokens in response
GROK_MAX_TOKENS=4096

# -----------------------------------------------------------------------------
# Network Configuration (Starlink Optimized)
# -----------------------------------------------------------------------------
# Enable Starlink-specific optimizations
GROK_STARLINK_OPTIMIZATIONS=true

# Request timeout in seconds
GROK_TIMEOUT=60

# Maximum retry attempts for failed requests
GROK_MAX_RETRIES=5

# Base retry delay in milliseconds
GROK_BASE_RETRY_DELAY=1000

# Maximum retry delay in milliseconds
GROK_MAX_RETRY_DELAY=30000

# Connection timeout in seconds
GROK_CONNECT_TIMEOUT=30

# Read timeout in seconds
GROK_READ_TIMEOUT=60

# Enable network health monitoring
GROK_HEALTH_MONITORING=true

# -----------------------------------------------------------------------------
# UI Configuration
# -----------------------------------------------------------------------------
# Enable colored output
GROK_COLORS=true

# Enable Unicode characters in output
GROK_UNICODE=true

# Enable progress bars
GROK_PROGRESS_BARS=true

# Enable animations
GROK_ANIMATIONS=true

# Show tips in interactive mode
GROK_SHOW_TIPS=true

# -----------------------------------------------------------------------------
# Security Configuration
# -----------------------------------------------------------------------------
# Shell command approval mode: strict, default, or permissive
GROK_SHELL_APPROVAL_MODE=default

# Enable sandbox for shell commands
GROK_SHELL_SANDBOX=true

# -----------------------------------------------------------------------------
# Logging Configuration
# -----------------------------------------------------------------------------
# Log level: trace, debug, info, warn, error
GROK_LOG_LEVEL=info

# Enable session logging
GROK_SESSION_LOGGING=true

# Log format: json or text
GROK_LOG_FORMAT=json

# Enable performance metrics
GROK_PERFORMANCE_METRICS=true

# -----------------------------------------------------------------------------
# Context Configuration
# -----------------------------------------------------------------------------
# Maximum file size to include in context (in bytes)
GROK_MAX_FILE_SIZE=1048576

# Maximum total context size (in bytes)
GROK_MAX_CONTEXT_SIZE=10485760

# Enable smart context filtering
GROK_SMART_CONTEXT_FILTERING=true

# Respect .grokignore files
GROK_RESPECT_GROKIGNORE=true

# -----------------------------------------------------------------------------
# Development Settings
# -----------------------------------------------------------------------------
# Enable preview features
GROK_PREVIEW_FEATURES=true

# Enable experimental features
GROK_EXPERIMENTAL_FEATURES=false

# Enable debug mode
GROK_DEBUG=false

# Enable verbose output
GROK_VERBOSE=false
```

## Notes

1. **API Key**: You must set `GROK_API_KEY` or `X_API_KEY`. For security, consider storing this in your system-wide `~/.grok/.env` instead of the project-level file.

2. **Model Selection**: The `GROK_MODEL=grok-code-fast-1` setting is optimized for coding tasks. Other options:
   - `grok-4-1-fast-reasoning` - Latest fast reasoning model (cheaper & more up-to-date, recommended default)
   - `grok-3` - Previous flagship model
   - `grok-code-fast-1` - Fast, optimized for code
   - `grok-2` - Balanced performance

3. **Starlink Settings**: The network configuration is tuned for Starlink satellite internet with higher timeouts and retry logic to handle connection drops.

4. **Priority Order**: Settings are loaded in this order (later overrides earlier):
   - Built-in defaults
   - System config (`~/.grok/.env`)
   - Project config (`.grok/.env`) ← This file
   - Environment variables
   - CLI arguments (highest priority)

## Quick Start

To use this configuration:

1. Copy the recommended settings above
2. Create `.grok/.env` in your project root
3. Paste the settings
4. Uncomment and set `GROK_API_KEY` (or set it system-wide)
5. Adjust other settings as needed

## Verification

After creating your `.env` file, verify it's being loaded:

```bash
grok config show
```

You should see:
- Configuration: Project (.grok/.env)
- Model: grok-code-fast-1

## Security Note

**Never commit `.env` files to git!** They're already in `.gitignore`, but be careful when sharing code.