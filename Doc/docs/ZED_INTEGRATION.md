# Zed Editor Integration Guide for Grok CLI

This guide will help you integrate Grok CLI with the Zed editor using the Agent Client Protocol (ACP).

## Prerequisites

- Zed editor installed (latest version recommended)
- Grok CLI built and installed
- X/Grok API key configured

## Quick Start

### 1. Build Grok CLI

```bash
cd grok-cli
cargo build --release
```

The binary will be located at `target/release/grok.exe` (Windows) or `target/release/grok` (Linux/Mac).

### 2. Set Your API Key

Create a `.env` file in `~/.grok/.env` (system-wide):

```bash
# Windows
mkdir %USERPROFILE%\.grok
echo GROK_API_KEY=xai-your-key-here > %USERPROFILE%\.grok\.env

# Linux/Mac
mkdir -p ~/.grok
echo "GROK_API_KEY=xai-your-key-here" > ~/.grok/.env
```

Or for a specific project, create `.grok/.env` in your project directory:

```bash
mkdir .grok
echo "GROK_API_KEY=xai-your-key-here" > .grok/.env
echo "GROK_MODEL=grok-code-fast-1" >> .grok/.env
```

### 3. Verify Configuration

```bash
grok config show
grok health --all
```

## Zed Editor Configuration

### Method 1: STDIO Mode (Recommended)

Zed can launch Grok CLI as a subprocess and communicate via stdin/stdout.

#### Add to Zed Settings

Open Zed settings (`Ctrl+,` or `Cmd+,`) and add:

```json
{
  "language_models": {
    "grok": {
      "version": "1",
      "provider": "agent",
      "default_model": "grok-2-latest",
      "agent": {
        "command": "grok",
        "args": ["acp", "stdio"],
        "env": {
          "GROK_API_KEY": "your-api-key-here"
        }
      }
    }
  }
}
```

**Note**: Replace `"grok"` in the command with the full path to your Grok CLI binary if it's not in your PATH:
- Windows: `"H:\\GitHub\\grok-cli\\target\\release\\grok.exe"`
- Linux/Mac: `"/path/to/grok-cli/target/release/grok"`

### Method 2: Server Mode

Run Grok CLI as a persistent server that Zed connects to.

#### Start the ACP Server

```bash
grok acp server --port 3000
```

Or let it auto-assign a port:
```bash
grok acp server
```

#### Add to Zed Settings

```json
{
  "language_models": {
    "grok": {
      "version": "1",
      "provider": "agent",
      "default_model": "grok-2-latest",
      "agent": {
        "endpoint": "http://127.0.0.1:3000"
      }
    }
  }
}
```

## Configuration Options

### Environment Variables

Configuration is managed via `.env` files. Create either:
- System-wide: `~/.grok/.env`
- Project-specific: `.grok/.env` (higher priority)

```env
# API Configuration
GROK_API_KEY=xai-your-key-here
GROK_MODEL=grok-code-fast-1

# Network settings (for Starlink)
GROK_STARLINK_OPTIMIZATIONS=true
GROK_TIMEOUT=60
GROK_MAX_RETRIES=5

# ACP Configuration
GROK_ACP_ENABLED=true
GROK_ACP_DEFAULT_PORT=3000
GROK_ACP_DEV_MODE=false
```

See `.grok/ENV_CONFIG_GUIDE.md` in the repository for all available options.

## Testing the Integration

### Test ACP Capabilities

```bash
grok acp capabilities
```

This shows all available tools and models that Zed can use.

### Test ACP Server Connection

If running in server mode:

```bash
grok acp test --address 127.0.0.1:3000
```

### Test STDIO Mode

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":"1.0","client_info":{"name":"test","version":"1.0"}}}' | grok acp stdio
```

## Available Features

### Chat Completion
Ask questions and get AI responses directly in Zed.

### Code Explanation
Select code and ask Grok to explain what it does.

### Code Review
Get detailed code reviews with security and performance suggestions.

### Code Generation
Describe what you want in natural language and generate code.

### File Operations
Read, write, and search files in your project.

## Troubleshooting

### Issue: "failed to deserialize response"

This was caused by incorrect Clap argument definitions. The issue has been fixed in the latest version. Make sure you:

1. Rebuild the project: `cargo build --release`
2. Verify your `.env` configuration is correct

### Issue: ACP Server Not Starting

Check if the port is already in use:

**Windows:**
```bash
netstat -ano | findstr :3000
```

**Linux/Mac:**
```bash
lsof -i :3000
```

Try a different port:
```bash
grok acp server --port 3001
```

### Issue: API Key Not Found

Verify your API key is set:
```bash
grok config show
```

If not set, create or edit `~/.grok/.env`:
```bash
# Windows
echo GROK_API_KEY=xai-your-key-here > %USERPROFILE%\.grok\.env

# Linux/Mac
echo "GROK_API_KEY=xai-your-key-here" > ~/.grok/.env
```

### Issue: Network Connectivity

Test network connectivity:
```bash
grok test-network --timeout 10
grok health --api
```

Enable Starlink optimizations if using satellite internet by adding to `.env`:
```env
GROK_STARLINK_OPTIMIZATIONS=true
```

### Issue: Zed Can't Find Grok Binary

Use the full path to the binary in Zed settings:

**Windows:**
```json
"command": "H:\\GitHub\\grok-cli\\target\\release\\grok.exe"
```

**Linux/Mac:**
```json
"command": "/home/user/grok-cli/target/release/grok"
```

### Debug Logging

Enable verbose logging to troubleshoot issues:

```bash
RUST_LOG=debug grok acp stdio
```

Or for server mode:
```bash
RUST_LOG=debug grok acp server --port 3000
```

## Advanced Configuration

### Custom Model Selection

In Zed settings, specify which Grok model to use:

```json
{
  "language_models": {
    "grok": {
      "default_model": "grok-2-latest"
    }
  }
}
```

Available models:
- `grok-2-latest` - Latest Grok 2 model (recommended)
- `grok-2` - Stable Grok 2 model
- `grok-1` - Original Grok model

### Multiple Configurations

You can configure multiple Grok instances:

```json
{
  "language_models": {
    "grok-fast": {
      "provider": "agent",
      "agent": {
        "command": "grok",
        "args": ["acp", "stdio", "--model", "grok-2-latest"]
      }
    },
    "grok-creative": {
      "provider": "agent",
      "agent": {
        "command": "grok",
        "args": ["acp", "stdio", "--model", "grok-1"]
      }
    }
  }
}
```

### Custom Timeout and Retries

Adjust network settings for your connection in `.env`:

```env
GROK_TIMEOUT=45
GROK_MAX_RETRIES=5
GROK_BASE_RETRY_DELAY=3000
```

## Performance Tips

1. **Use STDIO Mode**: Faster startup than server mode
2. **Enable Starlink Optimizations**: If using satellite internet
3. **Adjust Timeouts**: Increase for slow connections in `.env`
4. **Use Release Build**: Much faster than debug builds
5. **Use `.env` Files**: Project `.env` overrides system `.env`

## Security Considerations

1. **API Key Storage**: Never commit `.env` files with API keys to version control (they're in `.gitignore`)
2. **Use System `.env`**: Store API keys in `~/.grok/.env` (not in project)
3. **Restrict Network Access**: Bind to localhost only (127.0.0.1)
4. **Enable Security Features**: Use the built-in policy engine for shell commands

## Getting Help

If you encounter issues:

- Check the logs: `RUST_LOG=debug grok acp stdio`
2. Verify configuration: `grok config show`
3. Test connectivity: `grok health --all`
4. Review this guide's troubleshooting section
5. Open an issue: https://github.com/microtech/grok-cli/issues

## Example Workflow

1. Open a project in Zed
2. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on Mac)
3. Type "assistant" and select "Toggle Assistant"
4. The Grok AI assistant panel opens
5. Ask questions about your code or request changes
6. Grok analyzes your project context and responds
7. Accept suggested code changes with one click

## Next Steps

- Explore all available commands: `grok --help`
- Check ACP capabilities: `grok acp capabilities`
- View configuration: `grok config show`
- Configure settings: Edit `.grok/.env` or `~/.grok/.env`
- Read the main README: `README.md`

---

**Made with ❤️ for the Zed and Rust community**

Repository: https://github.com/microtech/grok-cli
Author: John McConnell (john.microtech@gmail.com)