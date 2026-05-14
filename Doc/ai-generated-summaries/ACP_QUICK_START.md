# ACP (Agent Client Protocol) Quick Start Guide

## Overview

The Grok CLI supports the Agent Client Protocol (ACP) for integration with editors like Zed. This allows Grok AI to assist with coding tasks directly within your editor environment. This guide covers how to set up and use ACP with Grok CLI.

## Available ACP Commands

| Command | Description |
|---------|-------------|
| `grok acp server` | Start the ACP server for editor integration. |
| `grok acp stdio` | Start ACP session over stdio (for advanced use). |
| `grok acp test` | Test connection to a running ACP server. |
| `grok acp capabilities` | Show ACP capabilities and supported features. |

## Quick Start

### 1. Start ACP Server for Zed Integration

Use the `server` command to start an ACP server that Zed can connect to. By default, it binds to `127.0.0.1` with an auto-assigned port, but you can specify a port if needed.

```bash
# Start ACP server with default settings
grok acp server

# Start ACP server on a specific port
grok acp server --port 4242
```

**Expected Output:**
- The server will display the address it's listening on (e.g., `127.0.0.1:4242`).
- Instructions for configuring Zed to connect to this server will be shown.

### 2. Configure Zed Editor

Once the server is running, configure Zed to connect to it:
1. Open Zed editor.
2. Go to `Settings → Extensions → Agent Client Protocol`.
3. Add a new agent configuration:
   - **Name**: Grok AI
   - **Command**: `grok acp server --port <port>` (replace `<port>` with the port shown by the server)
   - **Address**: `127.0.0.1:<port>` (replace `<port>` with the actual port)
4. Enable the agent and start coding!

### 3. Test ACP Connection

If you want to verify that an ACP server is running and accessible, use the `test` command:

```bash
# Test connection to a server at a specific address
grok acp test 127.0.0.1:4242
```

**Expected Output:**
- A success message if the connection is established, or an error if the server is not reachable.

### 4. View ACP Capabilities

To see what features and tools are supported by the ACP integration, use the `capabilities` command:

```bash
grok acp capabilities
```

**Expected Output:**
- A detailed list of protocol information, available tools, supported models, and configuration settings.

## Best Practices

### ✅ DO

- **Run the server in a dedicated terminal**: Keep the ACP server running in a separate terminal window while using Zed to avoid interruptions.
- **Specify a port if needed**: If you have multiple ACP servers or other services running, use the `--port` flag to avoid conflicts.
- **Enable ACP in config**: Ensure ACP is enabled in your Grok CLI configuration (`acp.enabled = true`).

### ❌ DON'T

- **Don't stop the server unexpectedly**: Closing the terminal or stopping the server will disconnect Zed from Grok AI.
- **Don't use privileged ports without permission**: Ports below 1024 may require elevated privileges; use higher ports if you encounter issues.

## Troubleshooting

### "ACP is disabled in configuration" Warning

```bash
grok acp server
⚠ Warning: ACP is disabled in configuration. Enable it with 'grok config set acp.enabled true'
```

**Solution:** Enable ACP in your configuration:
```bash
grok config set acp.enabled true
```

### "Failed to bind ACP server" Error

```bash
grok acp server --port 80
✗ Error: Failed to bind ACP server to 127.0.0.1:80: Permission denied
```

**Solution:** Use a non-privileged port (above 1024) or run with elevated privileges if necessary:
```bash
grok acp server --port 4242
```

### Connection Test Fails

```bash
grok acp test 127.0.0.1:4242
✗ Error: ACP connection test failed: Connection refused
```

**Solution:** Ensure the ACP server is running on the specified address and port, and check for firewall or network restrictions.

## Next Steps

1. **Start the ACP server**: Use `grok acp server` to begin integration with Zed.
2. **Configure Zed**: Follow the instructions provided by the server output to connect Zed.
3. **Test the connection**: Use `grok acp test <address>` to verify connectivity if issues arise.
4. **Explore capabilities**: Run `grok acp capabilities` to see what Grok AI can do within Zed.

## Resources

- [Full CLI Documentation](../README.md)
- [Skills Quick Start Guide](SKILLS_QUICK_START.md)

## Summary

```bash
# Start ACP server with default settings
grok acp server

# Start ACP server on a specific port
grok acp server --port 4242

# Test connection to a running server
grok acp test 127.0.0.1:4242

# View ACP capabilities
grok acp capabilities
```

**Remember:** ACP integration allows Grok AI to assist directly in your editor, enhancing your coding workflow. Keep the server running while using Zed for continuous support!
