# Quick Reference Guide - New Features

## Environment-Based Configuration

### Configuration Priority

Grok CLI uses a hierarchical `.env` file configuration system:

1. **Process environment variables** (highest priority)
2. **CLI arguments**: `--model`, `--config`, etc.
3. **Project-local**: `.grok/.env` in project root
4. **System-level**: `~/.grok/.env` (or `%USERPROFILE%\.grok\.env` on Windows)
5. **Built-in defaults** (lowest priority)

### How It Works

```bash
# Create project-specific config
cd ~/my-project
mkdir -p .grok
cat > .grok/.env << EOF
GROK_MODEL=grok-code-fast-1
GROK_TEMPERATURE=0.5
EOF

# Now when you run grok in this directory, it uses project settings
grok interactive
# Uses temperature 0.5 from project config!

# In other directories, uses system config or defaults
cd ~/other-project
grok interactive
# Uses system config or defaults
```

### Benefits

- **Per-project settings**: Different models, temperatures per project
- **Simple format**: Standard environment variable syntax
- **Flexible overrides**: Override system settings without changing global config
- **Automatic detection**: Walks up directory tree to find project root
- **Secure**: `.env` files are in `.gitignore` by default

### Example: Project-Specific Model

```env
# .grok/.env
GROK_MODEL=grok-code-fast-1
GROK_TEMPERATURE=0.3

# Starlink optimizations
GROK_STARLINK_OPTIMIZATIONS=true
GROK_TIMEOUT=60
GROK_MAX_RETRIES=5
```

---

## Enhanced Context Discovery

### Supported Context Files

Grok CLI now discovers and loads context from multiple editor-specific files:

| Priority | File | Editor/Tool |
|----------|------|-------------|
| 1 | `GEMINI.md` | Gemini CLI |
| 2 | `.gemini.md` | Gemini CLI (hidden) |
| 3 | `.claude.md` | Claude AI |
| 4 | `.zed/rules` | Zed Editor |
| 5 | `.grok/context.md` | Grok CLI |
| 6 | `.ai/context.md` | Generic AI |
| 7 | `CONTEXT.md` | Generic |
| 8 | `.gemini/context.md` | Gemini CLI (alt) |
| 9 | `.cursor/rules` | Cursor Editor |
| 10 | `AI_RULES.md` | Generic |

### Multi-File Context Merging

All available context files are automatically merged:

```bash
# Create multiple context files
cd ~/my-project

# Gemini-specific rules
cat > GEMINI.md << EOF
# Gemini Rules
Use descriptive variable names
EOF

# Zed editor rules
mkdir -p .zed
cat > .zed/rules << EOF
# Zed Rules
Format code with rustfmt
EOF

# Claude-specific rules
cat > .claude.md << EOF
# Claude Rules
Write comprehensive tests
EOF

# Start grok - all files are merged!
grok-cli interactive
# ✓ Loaded and merged 3 context files
#   • GEMINI.md
#   • .claude.md
#   • .zed/rules
```

### Context Merging Format

When multiple files are loaded, they're merged with source annotations:

```markdown
## From: GEMINI.md

# Gemini Rules
Use descriptive variable names

---

## From: .zed/rules

# Zed Rules
Format code with rustfmt

---

## From: .claude.md

# Claude Rules
Write comprehensive tests
```

---

## Session Persistence

### Save Current Session
```bash
/save <session_name>
```
Example: `/save debugging-auth`

### Load Saved Session
```bash
/load <session_name>
```
Example: `/load debugging-auth`

### List All Sessions
```bash
/list
```

### Session Storage Location
- **Windows**: `C:\Users\<username>\.grok\sessions\`
- **Linux/macOS**: `~/.grok/sessions/`

---

### Project Context Integration

### Supported Files (Priority Order)
See "Enhanced Context Discovery" section above for complete list of supported files.

### How to Use
1. Create a context file in your project root:
   ```bash
   # Example GEMINI.md
   # Project: My Web App
   
   ## Tech Stack
   - Rust with Axum
   - PostgreSQL database
   - JWT authentication
   
   ## Conventions
   - Use async/await
   - Error handling with anyhow
   - Tests in separate modules
   ```

2. Start Grok CLI in the project directory:
   ```bash
   cd ~/projects/my-project
   grok-cli interactive
   ```

3. Context loads automatically and is injected into the system prompt!

### Context File Template
```markdown
# Project Context

## Project Overview
Brief description of what this project does

## Architecture
High-level architecture and key components

## Development Guidelines
Coding standards, conventions, best practices

## Key Technologies
Main frameworks, libraries, and tools used

## Common Tasks
Frequently performed development tasks

## Important Notes
Gotchas, quirks, or important considerations
```

### Cross-Editor Compatibility

Use the same context files across multiple AI assistants:

```bash
# Use .gemini.md for Gemini CLI
# Use .claude.md for Claude
# Use .zed/rules for Zed
# Grok CLI loads them ALL!
```

---

## Extension System

### Enable Extensions
```bash
grok-cli config set experimental.extensions.enabled true
```

### Extension Directory
- **Default**: `~/.grok/extensions/`
- **Custom**: Set in config file

### Create an Extension

1. **Create directory structure:**
   ```bash
   mkdir -p ~/.grok/extensions/my-extension
   ```

2. **Create manifest** (`extension.json`):
   ```json
   {
     "name": "my-extension",
     "version": "1.0.0",
     "description": "My custom extension",
     "author": "Your Name <email@example.com>",
     "extension_type": "hook",
     "hooks": [
       {
         "name": "my-hook",
         "hook_type": "both",
         "config": {
           "custom_setting": "value"
         }
       }
     ],
     "enabled": true
   }
   ```

3. **Enable in config** (optional - auto-loads if in directory):
   ```toml
   [experimental.extensions]
   enabled = true
   enabled_extensions = ["my-extension"]
   ```

### Hook Types
- `"before_tool"` - Executes before tool invocation
- `"after_tool"` - Executes after tool completion
- `"both"` - Executes both before and after

### Example: Logging Extension
See `examples/extensions/logging-hook/` for a complete working example.

---

## Configuration

### View Current Config
```bash
grok-cli config show
```

### Get Specific Value
```bash
grok-cli config get experimental.extensions.enabled
```

### Set Value
```bash
grok-cli config set experimental.extensions.enabled true
```

### Config File Location

- **System-wide**: `~/.grok/.env` (Linux/macOS) or `%USERPROFILE%\.grok\.env` (Windows)
- **Project-specific**: `.grok/.env` in your project root
- **Priority**: Project config overrides system config

---

## Interactive Mode Commands

### Session Management
| Command | Description |
|---------|-------------|
| `/save <name>` | Save current session |
| `/load <name>` | Load saved session |
| `/list` | List all saved sessions |
| `/reset` | Clear conversation history |

### General Commands
| Command | Description |
|---------|-------------|
| `/help` | Show help message |
| `/quit` or `/exit` | Exit interactive mode |
| `/clear` | Clear screen |
| `/history` | Show conversation history |
| `/version` | Show version info |

### Context Display
| Command | Description |
|---------|-------------|
| `/context` | Show current context usage |
| `/tokens` | Show token usage statistics |

---

## Common Workflows

### Workflow 1: Project with Custom Config and Context

```bash
# 1. Create project config
cd ~/projects/my-app
mkdir .grok

cat > .grok/config.toml << EOF
default_model = "grok-2-latest"
default_temperature = 0.3
EOF

# 2. Create context file
cat > GEMINI.md << EOF
# My App
Tech: Rust + Axum
Use anyhow for errors
EOF

# 3. Start session
grok-cli interactive
# Using project-local configuration
# ✓ Loaded project context from GEMINI.md

# 4. Work on project with context-aware agent
> Help me implement JWT authentication following our conventions

# 5. Save session for later
> /save jwt-implementation
```

### Workflow 2: Resume Previous Work

```bash
# 1. List available sessions
grok-cli interactive
> /list

# 2. Load previous session
> /load jwt-implementation

# 3. Continue where you left off
> Let's continue with the middleware implementation
```

### Workflow 3: Multi-Editor Context Support

```bash
# 1. Create context for multiple editors
cd ~/my-project

# For Gemini CLI users
echo "# Gemini: Use async/await" > GEMINI.md

# For Claude users  
echo "# Claude: Prefer functional style" > .claude.md

# For Zed users
mkdir .zed
echo "# Zed: Format on save" > .zed/rules

# 2. Grok merges all contexts!
grok-cli interactive
# ✓ Loaded and merged 3 context files
```

### Workflow 4: Team Collaboration with Shared Config

```bash
# 1. Create team config
cd ~/team-project
mkdir .grok

# Create shared config
cat > .grok/.env << EOF
GROK_MODEL=grok-2-latest
GROK_TEMPERATURE=0.4
GROK_FOLDER_TRUST_ENABLED=true
EOF

# 2. Add to version control
git add .grok/
git commit -m "Add team Grok configuration"

# 3. Team members get consistent settings
git pull
grok-cli interactive
# Uses team config automatically!
```

### Workflow 5: Custom Extension for Logging

```bash
# 1. Copy example extension
cp -r examples/extensions/logging-hook ~/.grok/extensions/

# 2. Enable extensions
grok-cli config set experimental.extensions.enabled true

# 3. Start and see logging
grok-cli interactive
# Extension logs all tool invocations
```

---

## Troubleshooting

### Session Won't Save
- Check disk space
- Verify `~/.grok/sessions/` directory exists and is writable
- Check for special characters in session name

### Context File Not Loading
- Ensure file is in project root
- Check file name matches supported names (see Enhanced Context Discovery section)
- Verify file size < 5 MB
- Check for UTF-8 encoding
- Enable debug logging: `RUST_LOG=debug grok-cli interactive`

### Config Not Loading

- Verify project has `.grok/.env`
- Check file exists: `cat .grok/.env` or `type .grok\.env` (Windows)
- Test config manually: `grok config show`
- Check directory tree walk: `RUST_LOG=grok_cli::config=debug grok-cli interactive`

### Extension Not Loading
- Verify extensions enabled: `grok-cli config get experimental.extensions.enabled`
- Check `extension.json` is valid JSON: `jq . extension.json`
- Verify extension directory: `ls -la ~/.grok/extensions/`
- Check logs: `RUST_LOG=grok_cli::hooks=debug grok-cli interactive`

### View Debug Logs
```bash
# All debug info
RUST_LOG=debug grok-cli interactive

# Specific module
RUST_LOG=grok_cli::hooks=debug grok-cli interactive
RUST_LOG=grok_cli::utils::context=debug grok-cli interactive
```

---

## Best Practices

### Configuration Management
- Use project configs for team-shared settings
- Commit `.grok/config.toml` to version control
- Use system config for personal preferences
- Document config overrides in README

### Session Management
- Use descriptive session names: `feature-auth`, `debug-api`, `refactor-db`
- Save sessions regularly during long conversations
- Clean up old sessions periodically
- Don't store sensitive data in sessions (no encryption yet)

### Context Files
- Keep context files concise (< 1000 lines)
- Update context when conventions change
- Include examples in context
- Document gotchas and pitfalls
- Version control your context files


### Context Files
- Create separate files for different AI tools
- Keep each file focused and concise
- Use `.zed/rules` for Zed-specific conventions
- Use `.claude.md` for Claude-specific guidance
- Let Grok merge them all automatically

### Extensions
- Only install trusted extensions
- Test extensions in isolated environment first
- Keep extensions simple and focused
- Document extension configuration
- Report extension issues to authors

---

## Tips & Tricks

### Tip 1: Per-Project Configuration

```bash
# Development project - high creativity
cd ~/dev-project
mkdir -p .grok && echo 'GROK_TEMPERATURE=1.0' > .grok/.env

# Production project - conservative
cd ~/prod-project  
mkdir -p .grok && echo 'GROK_TEMPERATURE=0.2' > .grok/.env
```

### Tip 2: Multi-Project Context
Create project-specific context in each directory:
```bash
~/project-a/GEMINI.md  # Django project guidelines
~/project-b/GEMINI.md  # Rust project guidelines
```

### Tip 3: Session Templates
Save template sessions for common tasks:
```bash
/save template-debugging
/save template-code-review
/save template-refactoring
```

### Tip 4: Extension Chains
Multiple extensions execute in sequence:
```bash
~/.grok/extensions/
  ├── logger/          # Logs all calls
  ├── validator/       # Validates security
  └── profiler/        # Measures performance
```

### Tip 5: Portable Context and Config
Share context files in Git:
```bash
git add GEMINI.md .grok/
git commit -m "Add AI assistant context and config"
git push
# Team members get consistent AI guidance and settings
```

### Tip 6: Editor-Agnostic Rules
```bash
# Works with all editors/tools
cat > AI_RULES.md << EOF
# Universal AI Rules
- Follow project coding standards
- Write tests for all new features
- Document public APIs
EOF
```

---

## Examples

### Example 1: Web Development Context
```markdown
# My Web App

## Tech Stack
- Backend: Rust + Axum
- Database: PostgreSQL
- Frontend: React + TypeScript

## API Conventions
- RESTful endpoints
- JWT in Authorization header
- JSON request/response
- Error format: `{"error": "message"}`

## Testing
- Unit tests with `#[cfg(test)]`
- Integration tests in `tests/`
- Run: `cargo test`
```

### Example 2: Data Science Context
```markdown
# ML Pipeline Project

## Environment
- Python 3.11
- PyTorch 2.0
- Pandas, NumPy, Scikit-learn

## Code Style
- PEP 8
- Type hints required
- Docstrings for all functions

## Data Locations
- Raw: `data/raw/`
- Processed: `data/processed/`
- Models: `models/`
```

---

## Performance Tips

- **Sessions**: Save only when needed, sessions are small (~10 KB)
- **Context**: Keep under 50 KB for fast loading
- **Extensions**: Use only needed extensions, disable unused ones

---

## Security Notes

### Session Files
- Stored locally, not encrypted
- Contains full conversation history
- Protect sensitive session files manually
- Don't commit to public repos

### Context Files
- Public if in version control
- Don't include secrets or credentials
- Safe to share with team

### Extensions
- Run with full CLI permissions
- Only install from trusted sources
- Review code before installing
- No sandboxing yet

---

## Getting Help

### Documentation
- Full docs: `docs/extensions.md`
- Progress report: `docs/PROGRESS_REPORT.md`
- Changelog: `CHANGELOG.md`

### Commands
```bash
grok-cli --help
grok-cli interactive
> /help
```

### Support
- GitHub Issues: https://github.com/microtech/grok-cli/issues
- Repository: https://github.com/microtech/grok-cli

---

## Version Information

These features available in:
- **Version**: 0.1.2+
- **Release Date**: 2025-01-XX

Check your version:
```bash
grok-cli --version
```

---

**Happy coding with Grok CLI! 🚀**