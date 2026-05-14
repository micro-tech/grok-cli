# Grok CLI Quick Start Guide

Welcome to Grok CLI! This guide will get you up and running in minutes.

## Installation

```bash
# Clone the repository
git clone https://github.com/microtech/grok-cli
cd grok-cli

# Build the project
cargo build --release

# Install (optional)
cargo install --path .
```

## Setup

### 1. Get Your API Key

Get your X API key from [https://developer.twitter.com/en/portal/dashboard](https://developer.twitter.com/en/portal/dashboard)

### 2. Configure

```bash
# Copy the example .env file
cp .env.example .env

# Edit .env and add your API key
GROK_API_KEY=xai-your-api-key-here
```

### 3. Test Connection

```bash
grok test-network
```

## Basic Usage

### Quick Query

```bash
# Ask a question
grok query "What is the meaning of life?"

# Get code help
grok query "How do I reverse a string in Rust?"
```

### Interactive Mode

```bash
# Start interactive chat
grok

# Or with specific model
grok --model grok-code-fast-1
```

## Interactive Mode Commands

Once in interactive mode, you have three types of commands:

### 1. Chat Messages (Default)
Just type naturally to talk to Grok:
```
What is Rust?
How do I handle errors in Rust?
Explain async/await
```

### 2. Built-in Commands (/)
Commands starting with `/` control the CLI:

| Command | Description |
|---------|-------------|
| `/help` | Show all commands |
| `/quit` or `/exit` | Exit interactive mode |
| `/clear` | Clear screen |
| `/model grok-code-fast-1` | Change model |
| `/save session-name` | Save conversation |
| `/load session-name` | Load saved session |
| `/status` | Show session info |
| `/history` | Show conversation history |

### 3. Shell Commands (!)
Commands starting with `!` run locally on your computer:

**Windows:**
```
!dir                    # List files
!cd                     # Show current directory  
!type file.txt          # Display file contents
!git status             # Check git status
!cargo build            # Build project
!echo Hello             # Print message
```

**Linux/macOS:**
```
!ls                     # List files
!ls -la                 # List files with details
!pwd                    # Show current directory
!cat file.txt           # Display file contents
!git status             # Check git status
!cargo build            # Build project
!echo "Hello"           # Print message
```

## Common Workflows

### Code Review
```
Grok> !git status
Grok> !git diff src/main.rs
Grok> Can you review these changes?
```

### Development
```
Grok> !cargo test
Grok> Tests are failing, can you help?
[AI provides suggestions]
Grok> !cargo test
```

### File Exploration
```
Grok> !ls src/
Grok> !cat src/config.rs
Grok> How can I improve this code?
```

### System Tasks
```
Grok> !df -h
Grok> My disk is 90% full, what should I check?
[AI suggests commands]
Grok> !du -sh ~/Downloads/* | sort -h | tail
```

## Available Models

| Model | Best For | Speed |
|-------|----------|-------|
| `grok-3` | General purpose, balanced | Medium |
| `grok-code-fast-1` | Coding tasks | Fast |
| `grok-2-latest` | Latest stable release | Medium |
| `grok-vision-beta` | Image analysis | Medium |

Change model anytime:
```bash
# In interactive mode
/model grok-code-fast-1

# Or at startup
grok --model grok-code-fast-1
```

## Configuration

### Project-Specific Settings

Create `.grok/.env` in your project:
```bash
mkdir -p .grok
cp .grok/.env.example .grok/.env
```

Edit `.grok/.env`:
```bash
GROK_MODEL=grok-code-fast-1
GROK_TEMPERATURE=0.7
GROK_TIMEOUT=60
```

### System-Wide Settings

Create `~/.grok/.env` (Linux/macOS) or `%USERPROFILE%\.grok\.env` (Windows):
```bash
mkdir -p ~/.grok
nano ~/.grok/.env
```

Add your preferences:
```bash
GROK_API_KEY=xai-your-key
GROK_MODEL=grok-3
GROK_COLORS=true
```

### Configuration Priority

Settings are applied in this order (highest first):
1. Environment variables (`export GROK_MODEL=grok-3`)
2. CLI arguments (`--model grok-code-fast-1`)
3. Project config (`.grok/.env`)
4. System config (`~/.grok/.env`)
5. Built-in defaults

## Project Context

Grok CLI automatically loads project context from these files:
- `.gemini.md`
- `.grok/context.md`
- `.zed/rules`
- `.claude.md`

Create `.grok/context.md` to provide project-specific instructions:
```markdown
# Project Context

This is a Rust CLI application for interacting with Grok AI.

## Coding Standards
- Use `anyhow::Result` for error handling
- Follow Rust 2025 edition guidelines
- Write tests for all public APIs

## Project Structure
- `src/api/` - API client code
- `src/cli/` - CLI commands
- `src/display/` - UI and formatting
```

## Tips & Tricks

### 1. Quick File Checks
```
!cat error.log
What does this error mean?
```

### 2. Build & Test
```
!cargo build
[Fix any errors suggested by AI]
!cargo test
```

### 3. Git Workflow
```
!git status
!git diff
Can you review my changes?
!git add .
!git commit -m "your message"
```

### 4. Combine AI with Local Tools
```
!npm test
These tests are failing: [paste output]
[Get AI suggestions]
!npm test
```

### 5. Save Important Sessions
```
/save feature-authentication
[Work on something else]
/load feature-authentication
```

## Common Issues

### API Key Not Found
```bash
# Check if key is set
echo $GROK_API_KEY

# Set it temporarily
export GROK_API_KEY=xai-your-key

# Or add to .env file
echo "GROK_API_KEY=xai-your-key" >> .env
```

### Wrong Model Being Used
```bash
# Check configuration
grok config show

# Override with environment variable
GROK_MODEL=grok-code-fast-1 grok
```

### Shell Commands Not Working
- Make sure command exists: `which <command>` (Unix) or `where <command>` (Windows)
- Use full paths if needed: `!/usr/local/bin/mycommand`
- Check permissions
- Try the command in your regular terminal first

### Network Timeout
```bash
# Increase timeout in .env
GROK_TIMEOUT=120
GROK_MAX_RETRIES=5
GROK_STARLINK_OPTIMIZATIONS=true
```

## Security Notes

### Shell Commands (!)
⚠️ **Important:** Shell commands execute with your user permissions!
- Commands are not sandboxed
- Dangerous commands (`rm`, `del`) will execute
- Always review before pressing Enter
- Use with caution in production environments

### API Keys
- Never commit `.env` files to git
- Use `.env.example` for templates
- Set proper file permissions: `chmod 600 .env`
- Use secret management in CI/CD

## Next Steps

1. **Read the Full Documentation**
   - [Configuration Guide](CONFIGURATION.md) - Detailed config options
   - [Interactive Mode Guide](docs/INTERACTIVE.md) - All interactive features
   - [API Documentation](docs/API.md) - API details

2. **Try Advanced Features**
   - Session management (`/save`, `/load`)
   - Custom system prompts (`/system`)
   - Project context files
   - Multiple models

3. **Integrate with Your Workflow**
   - Add Grok CLI to your build scripts
   - Create project-specific `.grok/.env` configs
   - Use in CI/CD pipelines

## Getting Help

- Show help: `grok --help`
- Interactive help: `/help` (in interactive mode)
- Check version: `grok --version`
- View config: `grok config show`
- Test connection: `grok test-network`
- Check status: `/status` (in interactive mode)

## Examples Repository

Check the `examples/` directory for:
- Extension examples
- Hook system demos
- MCP server configurations
- Integration examples

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

See [LICENSE](LICENSE) file for details.

---

**Ready to start?**

```bash
# Start chatting with Grok!
grok

# Or ask a quick question
grok query "How do I use async/await in Rust?"

# Try a shell command in interactive mode
!ls -la
```

Happy coding! 🚀