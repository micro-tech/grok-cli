# Grok CLI - Enhanced AI Assistant with Gemini-like Experience

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/microtech/grok-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-support-yellow.svg)](https://buymeacoffee.com/micro.tech)



A powerful command-line interface for interacting with Grok AI via X API, featuring a beautiful interactive experience inspired by Gemini CLI.

> **🎉 New in v0.1.2**: 
> - Session Persistence - Save and resume conversations
> - Hierarchical Configuration - Project-specific settings
> - Enhanced Context Discovery - Multi-editor support (.zed/rules, .claude.md, etc.)
> - Extension System - Custom hooks and plugins
> 
> See [Quick Reference](docs/QUICK_REFERENCE.md) for details.
>
> **🔧 Latest Update**: 
> - **Chat Logging**: Automatic conversation logging with search and replay capabilities
> - Fixed "failed to deserialize response" error in Zed integration. See [FIXES.md](FIXES.md) for details.

## 🚀 Features

### 🆕 New Features (v0.1.2)
- **Session Persistence** - Save and resume conversations with `/save`, `/load`, and `/list` commands
- **Hierarchical Configuration** - Project-local settings override system defaults (`.grok/config.toml`)
- **Enhanced Context Discovery** - Multi-editor support: `.zed/rules`, `.claude.md`, `.cursor/rules`, and more
- **Context File Merging** - Automatically merges all available context files with source annotations
- **Extension System** - Extend functionality with custom hooks and plugins
- **Project-Aware AI** - Agent automatically understands your project conventions

### ✨ Beautiful Interactive Experience
- **Adaptive ASCII Art Logo** - Stunning terminal graphics that adapt to your screen size
- **Rich Interactive Mode** - Gemini CLI-inspired interface with context-aware prompts
- **Smart Banners & Tips** - Helpful startup guidance and contextual information
- **Colorful Output** - Professional color scheme with gradient effects
- **Progress Indicators** - Visual feedback for all operations

### 💬 Advanced Chat Capabilities
- **Interactive Sessions** - Persistent conversations with context tracking
- **Automatic Tool Execution** - Grok can now create files and directories automatically!
- **Chat Logging** - Automatic conversation logging with full history
- **Session Search** - Search through all past conversations
- **History Replay** - Review and analyze previous sessions
- **System Prompts** - Customize AI behavior for specialized tasks
- **Temperature Control** - Adjust creativity levels (0.0-2.0)
- **Token Management** - Real-time context usage monitoring

### 💻 Code Intelligence
- **Code Explanation** - Understand complex codebases instantly
- **Code Review** - Get detailed feedback with security focus
- **Code Generation** - Create code from natural language descriptions
- **Multi-language Support** - Works with any programming language

### 🔧 Developer Tools
- **Health Diagnostics** - Comprehensive system and API monitoring
- **Configuration Management** - Flexible TOML-based settings
- **Zed Editor Integration** - Agent Client Protocol (ACP) support
- **Network Resilience** - Starlink-optimized with retry logic

## 🎨 Visual Demo

```
  ░██████╗░██████╗░░█████╗░██╗░░██╗
  ██╔════╝░██╔══██╗██╔══██╗██║░██╔╝
  ██║░░██╗░██████╔╝██║░░██║█████═╝░
  ██║░░╚██╗██╔══██╗██║░░██║██╔═██╗░
  ╚██████╔╝██║░░██║╚█████╔╝██║░╚██╗
  ░╚═════╝░╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝

┌─────────────────────────────────────────────────────┐
│                 Welcome to Grok CLI                │
├─────────────────────────────────────────────────────┤
│ Tips for getting started:                          │
│ 1. Ask questions, edit files, or run commands.     │
│ 2. Be specific for the best results.               │
│ 3. /help for more information.                     │
│ 4. Try: "Create a new Rust project structure"      │
└─────────────────────────────────────────────────────┘

Grok (grok-4-1-fast-reasoning) [demo | 100% context left | 0 messages] >
```

## 🤖 Automatic File Operations

**NEW!** Grok CLI now supports automatic file and directory creation during chat! Simply ask naturally and Grok will execute the operations for you.

### Available Tools
- **write_file** - Create or overwrite files with content
- **read_file** - Read file contents
- **replace** - Find and replace text in files
- **list_directory** - List directory contents
- **glob_search** - Find files matching patterns
- **save_memory** - Save facts to long-term memory
- **run_shell_command** - Execute shell commands (cargo, git, etc.)

### Example Usage

```bash
# Start interactive mode
grok

# Then ask naturally:
You: Create a new Rust project with main.rs and a README
You: Write a hello world program to src/main.rs
You: Create a .gitignore file for Rust projects
You: List all .rs files in the project
```

### How It Works

1. You make a request that involves file operations
2. Grok responds with tool calls
3. The CLI automatically executes them in your current directory
4. You see confirmation for each operation:
   ```
   Grok is executing operations...
     ✓ Successfully wrote to src/main.rs
     ✓ Successfully wrote to README.md
   All operations completed!
   ```

### Security

- All operations are restricted to your current working directory and subdirectories
- No access to system files or parent directories
- Tool execution requires explicit requests (not triggered by accident)
```


## 📦 Installation

### Prerequisites
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- X/Grok API key from [x.ai](https://x.ai)

### Build from Source
```bash
git clone https://github.com/microtech/grok-cli
cd grok-cli
cargo build --release
```

### Windows Installation (Recommended)

```powershell
# Navigate to project directory
cd grok-cli

# Run the installer
cargo run --bin installer

# Follow prompts to install to %LOCALAPPDATA%\grok-cli\bin
# Installer will:
# - Build release binary
# - Install to AppData\Local\grok-cli\bin
# - Update PATH automatically
# - Detect and remove old Cargo installations
# - Copy documentation and examples
```

### Initialize Configuration

```bash
# Create default configuration
grok config init
```

#### Set your API key (Choose one method):

**Recommended: Use .env file**
```bash
# Create .env file in config directory
# Windows:
echo GROK_API_KEY=your-api-key-here > %APPDATA%\.grok\.env

# Linux/Mac:
echo "GROK_API_KEY=your-api-key-here" > ~/.config/grok-cli/.env
```

**Alternative: Environment variable**
```bash
# Add to your shell profile (.bashrc, .zshrc, etc.)
export GROK_API_KEY="your-api-key-here"
# or
export X_API_KEY="your-api-key-here"
```

> **⚠️ Security Note:** Never commit `.env` files with API keys to version control! They are automatically excluded via `.gitignore`.

### ⚠️ Troubleshooting Installation

If you experience issues after installation:

**Problem: Wrong version showing after install**
```powershell
# Check which version is running
(Get-Command grok).Path
grok --version

# If showing old version (0.1.3), remove old Cargo installation
.\scripts\cleanup_old_install.ps1
# OR
.\scripts\cleanup_old_install.bat

# Restart PowerShell
```

**Problem: Configuration not being used**
```powershell
# Verify you're in the project directory
cd H:\GitHub\grok-cli

# Check configuration loading
grok config show
# Should show: "Using project-local configuration from: H:\GitHub\grok-cli\.grok\.env"
```

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for comprehensive troubleshooting guide.

## 🎯 Quick Start

### Interactive Mode (Default)
```bash
# Start beautiful interactive session
grok

# Start without banner
grok --hide-banner

# Save and load sessions (NEW!)
> /save my-session
> /load my-session
> /list

# Project-specific config (NEW!)
mkdir .grok
echo 'default_temperature = 0.3' > .grok/config.toml

# Multi-editor context support (NEW!)
echo "# Gemini rules" > GEMINI.md
echo "# Claude rules" > .claude.md
mkdir .zed && echo "# Zed rules" > .zed/rules
# Grok merges ALL context files automatically!
```

### Single Commands
```bash
# Ask a question
grok chat "Explain Rust ownership"

# Interactive chat with system prompt
grok chat --interactive --system "You are a Rust expert"

# Code operations
grok code explain src/main.rs
grok code review --focus security *.rs
grok code generate --language rust "HTTP server with error handling"

# System diagnostics
grok health --all
grok config show
```

## 🎪 Interactive Commands

Once in interactive mode, use these special commands:

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/model [name]` | Change AI model (grok-4-1-fast-reasoning, grok-3, grok-2-latest, etc.) |
| `/system [prompt]` | Set system prompt for specialized behavior |
| `/history` | View conversation history |
| `/status` | Show session information |
| `/clear` | Clear screen and show logo |
| `/reset` | Clear conversation history |
| `/quit` | Exit interactive mode |

## 📜 Chat History Management

View and manage your conversation logs:

```bash
# List all saved chat sessions
grok history list

# View a specific session
grok history view <session-id>

# Search through all conversations
grok history search "authentication"

# Clear all chat history
grok history clear --confirm
```

Chat sessions are automatically logged to `~/.grok/logs/chat_sessions/` in both JSON and human-readable text formats. Configure logging behavior with environment variables:

```bash
GROK_CHAT_LOGGING_ENABLED=true              # Enable/disable logging
GROK_CHAT_LOG_DIR=/path/to/logs            # Custom log directory
GROK_CHAT_LOG_MAX_SIZE_MB=10               # Max size before rotation
GROK_CHAT_LOG_ROTATION_COUNT=5             # Number of logs to keep
```

See [docs/CHAT_LOGGING.md](docs/CHAT_LOGGING.md) for complete documentation.

## ⚙️ Configuration

Create `~/.config/grok-cli/config.toml`:

```toml
[api]
default_model = "grok-4-1-fast-reasoning"
default_temperature = 0.7
timeout_secs = 30

[ui]
hide_banner = false          # Show ASCII logo
hide_tips = false           # Show helpful tips
colors = true               # Enable colored output
unicode = true              # Enable emoji and Unicode

[ui.interactive]
prompt_style = "rich"       # "simple", "rich", or "minimal"
show_context_usage = true   # Show token/context info
check_directory = true      # Warn about home directory usage
startup_animation = true    # Animate logo display

[ui.footer]
hide_cwd = false           # Show current directory
hide_model_info = false    # Show model and context usage
hide_status = false        # Show session status

[network]
starlink_optimizations = true  # Enable satellite internet optimizations
max_retries = 3               # Network retry attempts
base_retry_delay = 2          # Base delay between retries
```

## 🌐 Starlink Optimization

Grok CLI includes specialized optimizations for satellite internet users:

- **Smart Retry Logic** - Exponential backoff with jitter
- **Connection Drop Detection** - Recognizes satellite handoff patterns  
- **Timeout Management** - Adaptive timeouts based on connection quality
- **Error Recovery** - Graceful handling of intermittent connectivity

## 🎭 Zed Editor Integration

Grok CLI supports the Agent Client Protocol (ACP) for seamless Zed editor integration.

**For complete setup instructions, see [ZED_INTEGRATION.md](docs/ZED_INTEGRATION.md)**

### Quick Setup

```bash
# Initialize configuration (required first)
grok config init --force

# Set your API key
grok config set api_key YOUR_API_KEY

# Test capabilities
grok acp capabilities
```

### Recommended: STDIO Mode

Add to your Zed `settings.json`:
```json
{
  "language_models": {
    "grok": {
      "version": "1",
      "provider": "agent",
      "default_model": "grok-4-1-fast-reasoning",
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

### Alternative: Server Mode

```bash
# Start ACP server
grok acp server --port 3000

# Test connection
grok acp test --address 127.0.0.1:3000
```

Add to Zed `settings.json`:
```json
{
  "language_models": {
    "grok": {
      "version": "1",
      "provider": "agent",
      "agent": {
        "endpoint": "http://127.0.0.1:3000"
      }
    }
  }
}
```

## 🎨 Customization

### Available Models
- `grok-4-1-fast-reasoning` - Latest fast reasoning model (cheaper & more up-to-date, default)
- `grok-3` - Previous flagship model
- `grok-2-latest` - Previous generation model
- `grok-code-fast-1` - Optimized for code tasks
- `grok-vision-1212` - Supports image analysis

### Prompt Styles
- **Rich** - Full context with model, directory, and usage info
- **Simple** - Clean prompt with basic info
- **Minimal** - Just a simple arrow prompt

### Color Themes
The CLI uses a professional color scheme:
- 🔵 **Blue** - Grok branding and headers
- 🟢 **Green** - Success messages and confirmations  
- 🟡 **Yellow** - Warnings and important notes
- 🔴 **Red** - Errors and critical issues
- 🟣 **Magenta** - Interactive prompts and accents
- 🔄 **Cyan** - Information and tips
- ⚫ **Dimmed** - Secondary text and details

## 📊 Health Monitoring

Comprehensive health checks for optimal performance:

```bash
# Check everything
grok health --all

# Specific checks
grok health --api      # API connectivity
grok health --config   # Configuration validation
```

Health metrics include:
- ✅ API key validation
- 🌐 Network connectivity  
- ⚡ Response latency
- 📊 Model availability
- 🔧 Configuration integrity

## 🚨 Troubleshooting

### Common Issues

**"failed to deserialize response" Error**

This error has been fixed in the latest version. If you're still experiencing it:
```bash
# Rebuild the project
cargo clean
cargo build --release

# Reinitialize configuration
grok config init --force
```

See [FIXES.md](FIXES.md) for complete details about this fix.

**"Max tool loop iterations reached" Error**

This error occurs when the AI repeatedly calls tools without completing the task. Solutions:

```bash
# Increase the limit in your config file (~/.config/grok-cli/config.toml)
grok config set acp.max_tool_loop_iterations 50

# Or set via environment variable
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=50

# Then retry your command
```

Tips to avoid this error:
- Break complex tasks into smaller, focused steps
- Provide clearer, more specific instructions
- Check if the task is too complex for a single request
- Default limit is 25 iterations (configurable)

See [MAX_TOOL_LOOP_ITERATIONS.md](Doc/MAX_TOOL_LOOP_ITERATIONS.md) for comprehensive configuration guide and [TOOLS.md](Doc/docs/TOOLS.md) for more details.

**API Key Problems**
```bash
# Verify key is set
grok config get api_key

# Set if missing
grok config set api_key YOUR_KEY

# Test connectivity
grok health --api
```

**Network Issues** 
```bash
# Enable verbose logging
grok --verbose chat "test"

# Check network health
grok health --all

# Enable Starlink optimizations if needed
grok config set network.starlink_optimizations true
```

**Configuration Problems**
```bash
# Validate config
grok config validate

# Reset to defaults
grok config init --force
```

**Zed Integration Issues**

See the comprehensive troubleshooting section in [ZED_INTEGRATION.md](docs/ZED_INTEGRATION.md)

### Debug Mode
```bash
# Enable debug output
RUST_LOG=debug grok --verbose chat "test"

# For ACP/Zed debugging
RUST_LOG=debug grok acp stdio
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup
```bash
git clone https://github.com/microtech/grok-cli
cd grok-cli
cargo test
cargo clippy
```

## 📚 Documentation

- [ZED_INTEGRATION.md](docs/ZED_INTEGRATION.md) - Complete Zed editor integration guide
- [FIXES.md](FIXES.md) - Recent bug fixes and solutions
- [SETUP.md](SETUP.md) - Detailed setup instructions
- [TESTING_TOOLS.md](TESTING_TOOLS.md) - Testing documentation

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by [Gemini CLI](https://github.com/google-gemini/gemini-cli) for the interactive experience
- Built with the [Agent Client Protocol](https://github.com/zed-industries/zed/tree/main/crates/agent_client_protocol) for Zed integration
- Powered by [X.ai Grok API](https://x.ai) for AI capabilities

## 📞 Support

- 🐛 **Issues**: [GitHub Issues](https://github.com/microtech/grok-cli/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/microtech/grok-cli/discussions)
- 📧 **Contact**: john.microtech@gmail.com

---

**Made with ❤️ for the Rust and AI community**
