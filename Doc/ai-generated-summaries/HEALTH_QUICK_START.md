# Health Check Quick Start Guide

## Overview

The Grok CLI provides health check commands to diagnose issues with your setup, including network connectivity, API access, and configuration validation. This guide covers how to use these commands to ensure your Grok CLI environment is functioning correctly.

## Available Health Check Commands

| Command | Description |
|---------|-------------|
| `grok health` | Perform a basic health check on system and network. |
| `grok health --api` | Include API connectivity test in the health check. |
| `grok health --config` | Include configuration validation in the health check. |
| `grok health --all` | Perform all available health checks (API and config). |

## Quick Start

### 1. Perform a Basic Health Check

Use the `health` command without flags to check basic system and network status.

```bash
grok health
```

**Expected Output:**
- A summary of system checks (e.g., configuration file presence, environment variables) and network checks (e.g., connectivity, latency).
- A health status summary (Healthy, Warning, or Unhealthy) with a success rate percentage.

### 2. Check API Connectivity

Use the `--api` flag to test connectivity to the Grok API. This requires an API key to be configured or provided.

```bash
grok health --api
```

**Expected Output:**
- Additional results for API connection tests, including whether the API key is valid and if the default model is available.
- Warnings or errors if the API connection fails (e.g., invalid key, network issues).

### 3. Validate Configuration

Use the `--config` flag to validate your current configuration settings.

```bash
grok health --config
```

**Expected Output:**
- Results of configuration validation, indicating if settings are valid or if there are issues (e.g., missing API key).
- Suggestions for fixing configuration issues if any are found.

### 4. Perform All Health Checks

Use the `--all` flag to run all available health checks, including API and configuration tests.

```bash
grok health --all
```

**Expected Output:**
- A comprehensive report covering system, network, configuration, and API checks.
- A detailed summary with recommendations for any issues detected.

## Best Practices

### ✅ DO

- **Run health checks regularly**: Use `grok health --all` before starting a major task to ensure everything is set up correctly.
- **Include API checks after setting a key**: After configuring your API key, run `grok health --api` to verify connectivity.
- **Review recommendations**: Pay attention to warnings and suggestions in the health check summary to address potential issues.

### ❌ DON'T

- **Don't ignore warnings**: Even if the overall status is "Healthy," address warnings (e.g., high latency, missing API key) to prevent future problems.
- **Don't skip configuration validation**: Use `grok health --config` after making changes to settings to catch errors early.

## Troubleshooting

### "No API key provided" Warning

```bash
grok health --api
⚠ Warning: No API key provided - skipping API tests
```

**Solution:** Ensure your API key is set in the configuration or provide it via an environment variable or command-line argument.
```bash
grok config set api_key YOUR_API_KEY
```

### Network Connectivity Failure

```bash
grok health
✗ Error: Network connectivity failed: Connection timed out
```

**Solution:** Check your internet connection, firewall settings, or proxy configuration. Retry with Starlink optimizations if applicable:
```bash
grok config set network.starlink_optimizations true
```

### Configuration Validation Failure

```bash
grok health --config
✗ Error: Configuration validation failed: invalid timeout value
```

**Solution:** Review the error message, check settings with `grok config show`, and correct invalid values using `grok config set <key> <value>`.

## Next Steps

1. **Run a basic health check**: Start with `grok health` to verify system and network status.
2. **Test API connectivity**: Use `grok health --api` after setting your API key to ensure access to Grok AI.
3. **Validate configuration**: Run `grok health --config` to check for setting errors.
4. **Perform a full check**: Use `grok health --all` for a comprehensive diagnosis if issues persist.

## Resources

- [Full CLI Documentation](../README.md)
- [Configuration Quick Start Guide](CONFIG_QUICK_START.md)

## Summary

```bash
# Basic health check
grok health

# Check API connectivity
grok health --api

# Validate configuration
grok health --config

# Perform all health checks
grok health --all
```

**Remember:** Regular health checks help identify and resolve issues before they impact your workflow. Use them to keep Grok CLI running smoothly!
