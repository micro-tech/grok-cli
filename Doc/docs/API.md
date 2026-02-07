# Grok CLI API Documentation

## Overview

Grok CLI interacts with the Grok AI through the X API, providing a seamless command-line interface for AI-powered tasks such as chat, code analysis, and more. This document outlines the API configuration and usage details for Grok CLI.

## API Configuration

To use Grok CLI, you must configure your API key for accessing the X API. This can be done through environment variables or configuration files.

### Setting Your API Key

- **Environment Variable**: Set `GROK_API_KEY` or `X_API_KEY` in your shell or `.env` file:
  ```bash
  GROK_API_KEY=xai-your-api-key-here
  ```
- **Configuration File**: Add your API key to a project-specific `.grok/.env` or system-wide `~/.grok/.env` file.

### Model Selection

Grok CLI supports multiple models. Configure the default model using:
- **Environment Variable**: `GROK_MODEL=grok-3`
- **CLI Flag**: `--model grok-2-latest`

Available models include:
- `grok-3` - Latest and most capable model.
- `grok-2-latest` - Previous generation model.
- `grok-code-fast-1` - Optimized for code tasks.
- `grok-vision-1212` - Supports image analysis.

### Other API Settings

- **Temperature**: Control creativity with `GROK_TEMPERATURE=0.7` (range 0.0-2.0).
- **Timeout**: Set request timeout with `GROK_TIMEOUT=30` (in seconds).
- **Retries**: Configure retry attempts with `GROK_MAX_RETRIES=3`.

## API Usage

### Chat API

Initiate a chat session with:
```bash
grok chat "Explain Rust ownership"
```

### Code Operations

Use the API for code-related tasks:
```bash
grok code explain src/main.rs
grok code review --focus security *.rs
grok code generate --language rust "HTTP server with error handling"
```

### Health Checks

Verify API connectivity:
```bash
grok health --api
```

## Network Optimizations

Grok CLI includes optimizations for various network conditions, especially for satellite internet like Starlink:
- **Starlink Optimizations**: Enable with `GROK_STARLINK_OPTIMIZATIONS=true`.
- **Retry Logic**: Automatically retries failed requests with exponential backoff.

## Troubleshooting API Issues

- **API Key Validation**: Ensure your key is set correctly with `grok config get api_key`.
- **Connectivity**: Test API health with `grok health --api`.
- **Verbose Logging**: Enable detailed logs with `GROK_LOG_LEVEL=debug`.

For more detailed configuration options, refer to the [CONFIGURATION.md](../CONFIGURATION.md) guide.