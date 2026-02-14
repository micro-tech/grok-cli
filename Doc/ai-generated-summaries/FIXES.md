# Grok CLI Fixes and Solutions

## Overview

This document outlines recent bug fixes and solutions for common issues encountered in Grok CLI. It serves as a reference for troubleshooting and understanding updates made to improve the user experience.

## Recent Fixes

### Version 0.1.2 Fixes

- **Fixed 'failed to deserialize response' Error**: 
  - **Issue**: Users encountered this error during Zed integration due to mismatched response formats from the API.
  - **Solution**: Updated the API response parsing logic to handle variations in response structure. Ensure you are using the latest version of Grok CLI (v0.1.2 or higher).
  - **Action**: If you still experience this issue, rebuild the project with:
    ```bash
    cargo clean
    cargo build --release
    ```
    And reinitialize configuration:
    ```bash
    grok config init --force
    ```

## Known Issues and Workarounds

- **API Key Validation Errors**:
  - **Issue**: Some users report intermittent API key validation failures.
  - **Workaround**: Verify your key with `grok config get api_key` and reset if necessary using `grok config set api_key YOUR_KEY` (stores in .env file). Alternatively, manually edit `~/.config/grok-cli/.env` (or `%APPDATA%\.grok\.env` on Windows) to add `GROK_API_KEY=your-key-here`. Test connectivity with `grok health --api`.
- **Network Connectivity Drops**:
  - **Issue**: Users on satellite internet (e.g., Starlink) may experience connection drops.
  - **Workaround**: Enable Starlink optimizations with `grok config set network.starlink_optimizations true`.

## Reporting Issues

If you encounter a problem not covered here, please report it on our [GitHub Issues page](https://github.com/microtech/grok-cli/issues). Include details such as your Grok CLI version, operating system, and steps to reproduce the issue.

For more troubleshooting tips, refer to the [README.md](README.md) or [ZED_INTEGRATION.md](docs/ZED_INTEGRATION.md) for specific integration issues.