# Configuration Quick Start Guide

## Overview

The Grok CLI allows you to customize its behavior through configuration settings. This guide covers how to view, set, and manage your configuration to tailor Grok CLI to your needs.

## Available Configuration Commands

| Command | Description |
|---------|-------------|
| `grok config show` | Display the current configuration settings. |
| `grok config set` | Set a specific configuration value. |
| `grok config get` | Get the value of a specific configuration key. |
| `grok config init` | Initialize a new configuration file with defaults. |
| `grok config validate` | Validate the current configuration for errors. |

## Quick Start

### 1. View Current Configuration

Use the `show` command to see all current configuration settings, organized by category.

```bash
grok config show
```

**Expected Output:**
- A detailed list of settings under categories like API, ACP, Network, UI, and Logging, showing current and default values.

### 2. Set a Configuration Value

Use the `set` command to modify a specific configuration setting. Replace `<key>` with the setting name (e.g., `api_key`, `default_model`) and `<value>` with the desired value.

```bash
# Set your API key
grok config set api_key YOUR_API_KEY

# Change the default model
grok config set default_model grok-2

# Enable ACP
grok config set acp.enabled true
```

**Expected Output:**
- A confirmation message indicating the setting has been updated, along with any relevant tips (e.g., testing the API key after setting it).

### 3. Get a Specific Configuration Value

Use the `get` command to retrieve the current value of a specific setting.

```bash
grok config get default_model
```

**Expected Output:**
- The current value of the specified setting (e.g., `default_model: grok-3`).

### 4. Initialize a New Configuration

Use the `init` command to create a new configuration file with default settings. Use the `--force` flag to overwrite an existing configuration if needed.

```bash
# Initialize a new configuration
grok config init

# Force overwrite of existing configuration
grok config init --force
```

**Expected Output:**
- A success message with the path to the new configuration file and next steps for setting up essential values like the API key.

### 5. Validate Configuration

Use the `validate` command to check if your current configuration is valid and identify potential issues.

```bash
grok config validate
```

**Expected Output:**
- A success message if the configuration is valid, or an error message with details on issues found. Warnings and suggestions may also be provided (e.g., missing API key).

## Best Practices

### ✅ DO

- **Set your API key first**: Use `grok config set api_key YOUR_API_KEY` to enable API access for Grok AI.
- **Validate after changes**: Run `grok config validate` after modifying settings to catch errors early.
- **Use `show` to explore settings**: Review all available settings with `grok config show` to understand customization options.

### ❌ DON'T

- **Don't overwrite without backup**: Avoid using `grok config init --force` without saving a copy of your current configuration if it contains important customizations.
- **Don't set invalid values**: Ensure values match the expected type (e.g., boolean for `acp.enabled`, number for `timeout_secs`) to avoid validation errors.

## Troubleshooting

### "Invalid configuration value" Error

```bash
grok config set default_temperature invalid
✗ Error: Invalid configuration value: expected a number between 0.0 and 2.0
```

**Solution:** Provide a valid value for the setting. Check `grok config show` for the expected format or range.
```bash
grok config set default_temperature 0.7
```

### "Configuration file already exists" Warning

```bash
grok config init
⚠ Warning: Configuration file already exists!
```

**Solution:** Use the `--force` flag to overwrite the existing file, or back up your current configuration before initializing a new one.
```bash
grok config init --force
```

### Validation Fails

```bash
grok config validate
✗ Error: Configuration validation failed: invalid timeout value
```

**Solution:** Review the error message for specific issues, check settings with `grok config show`, and correct values using `grok config set <key> <value>`.

## Next Steps

1. **View your configuration**: Start with `grok config show` to see current settings.
2. **Set essential values**: Configure your API key and other critical settings using `grok config set`.
3. **Validate changes**: Use `grok config validate` to ensure your configuration is correct.
4. **Initialize if needed**: If starting fresh, use `grok config init` to create a default configuration file.

## Resources

- [Full CLI Documentation](../README.md)
- [Skills Quick Start Guide](SKILLS_QUICK_START.md)

## Summary

```bash
# Show current configuration
grok config show

# Set an API key
grok config set api_key YOUR_API_KEY

# Get a specific setting value
grok config get default_model

# Initialize a new configuration
grok config init --force

# Validate the configuration
grok config validate
```

**Remember:** Proper configuration ensures Grok CLI works as expected. Regularly validate your settings after changes to avoid runtime issues!
