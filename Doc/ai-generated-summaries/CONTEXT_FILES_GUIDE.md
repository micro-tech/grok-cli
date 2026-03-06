# Context Files Guide - Which File Should You Use?

## Quick Answer

For **grok-cli** project, you can use any of these context files:

| File Name | Priority | Use Case | Recommended |
|-----------|----------|----------|-------------|
| `GEMINI.md` | 1 (Highest) | Universal, visible | ✅ **BEST CHOICE** |
| `.gemini.md` | 2 | Hidden version | ⭐ Good |
| `.claude.md` | 3 | Claude AI specific | ⭐ Good |
| `.zed/rules` | 4 | Zed Editor | ⭐ Good (already exists) |
| `.grok/context.md` | 5 | Grok CLI specific | ⭐ Good |
| `.ai/context.md` | 6 | Generic AI | ⭐ Good |
| `CONTEXT.md` | 7 | Generic | ⭐ Good |
| `.gemini/context.md` | 8 | Alt Gemini location | Ok |
| `.cursor/rules` | 9 | Cursor Editor | Ok |
| `AI_RULES.md` | 10 (Lowest) | Generic | Ok |

## What's Already in the Project?

Currently, the grok-cli project has:
- ✅ `.zed/rules` - Already exists (Zed Editor rules)

## Recommendation: Use `GEMINI.md` at Project Root

### Why GEMINI.md?

1. **Highest Priority**: Loaded first if multiple files exist
2. **Visible**: Not hidden, easy to find and edit
3. **Universal**: Works with multiple AI tools (Gemini, Grok, Claude, etc.)
4. **Standard**: Widely recognized in the AI coding community
5. **Well-documented**: Lots of examples available

### How to Create It

```bash
cd grok-cli
cat > GEMINI.md << 'EOF'
# Grok CLI Project Context

## Project Overview
Rust CLI application providing an interactive interface to Grok AI with automatic tool execution capabilities.

## Key Technologies
- Rust 2024 Edition
- Tokio for async runtime
- Clap for CLI parsing
- Colored for terminal output
- xAI/Grok API for AI functionality

## Project Structure
- `src/main.rs` - Application entry point
- `src/lib.rs` - Library exports
- `src/acp/` - Agent Client Protocol (Zed integration)
- `src/api/` - Grok API client
- `src/cli/` - CLI commands and handlers
- `src/display/` - Terminal UI and interactive mode
- `src/utils/` - Utility modules
- `src/config/` - Configuration management

## Development Guidelines

### Tool Execution
When users ask to create projects or files:
- ✅ USE TOOLS: `write_file`, `run_shell_command`, etc.
- ❌ DON'T give instructions - execute directly

Available tools:
1. `write_file(path, content)` - Create/overwrite files
2. `read_file(path)` - Read file contents
3. `replace(path, old, new)` - Find and replace
4. `list_directory(path)` - List contents
5. `glob_search(pattern)` - Find files by pattern
6. `save_memory(fact)` - Save facts to memory
7. `run_shell_command(command)` - Execute commands

### Shell Commands
- Windows PowerShell: `&&` automatically converts to `;`
- You can use bash syntax naturally
- Commands are security-validated

### Code Style
- Rust 2024 edition features
- Use `anyhow::Result` for error handling
- Use `tracing` for logging (not `println!` in library code)
- Follow idiomatic Rust patterns
- Comprehensive error messages

### Security
- File operations restricted to current directory and subdirectories
- No access to parent directories or system files
- Shell commands validated before execution

### Testing
- Run `cargo test` before committing
- Add tests for new features
- All 82 tests must pass

### Documentation
- Update CHANGELOG.md for user-facing changes
- Add doc comments for public APIs
- Update README.md for new features

## Common Tasks

### Creating a Project Structure
When asked to create a project:
1. Use `write_file` to create files (Cargo.toml, src/*, etc.)
2. Use `run_shell_command` to run initialization (cargo init, git init)
3. Execute as separate operations, not text instructions

Example:
```
User: "Create a Rust library with git"

Your actions:
1. write_file("Cargo.toml", content)
2. write_file("src/lib.rs", content)
3. write_file(".gitignore", content)
4. run_shell_command("cargo init --lib")
5. run_shell_command("git init")
```

### PowerShell Commands
Natural bash syntax works:
- `cargo new project && git init` ✅ Works (auto-converted)
- `cargo build && cargo test` ✅ Works (auto-converted)

### Response Pattern

❌ **DON'T DO THIS:**
```
To create a project, you need to:
1. Run: cargo new project_name
2. Create a file with...
```

✅ **DO THIS:**
Execute the tools directly. System shows:
```
Grok is executing operations...
  ✓ Successfully wrote to Cargo.toml
  ⚙ Executing: cargo init
  ✓ Command output: Created library package
```

## Project-Specific Notes

### Network Handling
- Starlink satellite internet with intermittent drops
- All network calls have retry logic
- Test connectivity before operations
- Timeout and retry settings configurable

### Configuration
- Hierarchical: project → system → defaults
- Project-local: `.grok/config.toml`
- System-level: `~/.grok/config.toml`
- Environment variables override all

### Logging
- Chat sessions automatically logged
- Location: `~/.grok/logs/chat_sessions/`
- JSON and text formats
- Searchable and replayable

## Error Handling
- Use `anyhow::Result` everywhere
- Provide context with `.context()` or `.with_context()`
- User-friendly error messages
- Network errors should retry automatically

## Dependencies
Key dependencies and their purposes:
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `serde` / `serde_json` - Serialization
- `clap` - CLI parsing
- `colored` - Terminal colors
- `anyhow` - Error handling
- `tracing` - Logging
- `dotenvy` - Environment variables

## Version Information
- Current Version: 0.1.2
- Rust Edition: 2024
- MSRV: 1.70+
- Target: Windows 11, macOS, Linux

## Contact
- Repository: https://github.com/microtech/grok-cli
- Author: john mcconnell <john.microtech@gmail.com>
- Issues: https://github.com/microtech/grok-cli/issues
EOF
```

## Can You Use Multiple Context Files?

**YES!** Grok CLI supports **multi-file context merging**.

You can have:
- `GEMINI.md` - Main rules
- `.claude.md` - Additional rules
- `.zed/rules` - Editor-specific rules
- `.grok/context.md` - Grok-specific context

**All files will be loaded and merged!**

Example:
```bash
# Create multiple context files
echo "# Use descriptive variable names" > GEMINI.md
echo "# Write comprehensive tests" > .claude.md
mkdir -p .zed && echo "# Follow Zed conventions" > .zed/rules

# When grok starts:
# ✓ Loaded and merged 3 context files
#   • GEMINI.md
#   • .claude.md  
#   • .zed/rules
```

## Alternative: Grok-Specific Context

If you want a Grok-only context file:

```bash
mkdir -p .grok
cat > .grok/context.md << 'EOF'
# Grok CLI Context

This is specific to Grok CLI only.
Add your project-specific rules here.
EOF
```

## What About `.grok.md`?

**`.grok.md` is NOT automatically loaded** by grok-cli.

The system looks for these exact filenames:
- ✅ `GEMINI.md`
- ✅ `.gemini.md`
- ✅ `.claude.md`
- ✅ `.zed/rules`
- ✅ `.grok/context.md` (directory + file)
- ❌ `.grok.md` (not recognized)

If you want a hidden Grok file, use `.grok/context.md` instead.

## What About `agent.md`?

**`agent.md` is NOT automatically loaded** by grok-cli.

Use one of the recognized filenames listed above instead.

## Best Practices

### 1. Start with GEMINI.md
Most universal and visible:
```bash
echo "# Project Rules" > GEMINI.md
```

### 2. Add Tool-Specific Files as Needed
```bash
echo "# Claude-specific rules" > .claude.md
echo "# Zed-specific rules" > .zed/rules
```

### 3. Keep Context Focused
- Project structure and conventions
- Technology stack
- Development guidelines
- Common commands
- Error handling patterns

### 4. Avoid Large Files
- Max size: 5 MB (automatically enforced)
- Keep it concise and relevant
- Split into multiple files if needed

### 5. Version Control
```bash
# Add to git
git add GEMINI.md .claude.md .zed/rules
git commit -m "Add project context files"

# Or keep private
echo "GEMINI.md" >> .gitignore
```

## How Context Files Are Used

1. **Startup**: Grok loads all available context files
2. **Merging**: Files are combined with source annotations
3. **System Prompt**: Context is added to every AI request
4. **Guidance**: AI uses context to understand project conventions

Example merged context:
```markdown
## From: GEMINI.md

# Use descriptive variable names

## From: .claude.md

# Write comprehensive tests

## From: .zed/rules

# Follow Zed conventions
```

## Checking Loaded Context

In interactive mode:
```bash
grok

# Check status
/status

# Shows:
# Context loaded: 3 files
#   • GEMINI.md
#   • .claude.md
#   • .zed/rules
```

## Migration Guide

### From `.grok.md` (not supported)
```bash
# Rename to GEMINI.md
mv .grok.md GEMINI.md

# Or move to .grok directory
mkdir -p .grok
mv .grok.md .grok/context.md
```

### From `agent.md` (not supported)
```bash
# Rename to GEMINI.md
mv agent.md GEMINI.md

# Or create in .ai directory
mkdir -p .ai
mv agent.md .ai/context.md
```

## Context File Priority

When multiple files exist, they're ALL loaded and merged.

Priority only matters if you use the old single-file loading (not recommended):
1. `GEMINI.md` - Loaded first
2. `.gemini.md` - If GEMINI.md missing
3. `.claude.md` - If above missing
4. ... and so on

**With multi-file merging (default), all files are combined!**

## Examples

### Minimal Context
```markdown
# My Project

Tech: Rust + Tokio
Use anyhow for errors
```

### Comprehensive Context
```markdown
# Grok CLI Project

## Stack
- Rust 2024
- Tokio async runtime
- Clap CLI framework

## Conventions
- Use `anyhow::Result`
- Use `tracing` not `println!`
- Run `cargo test` before commit

## Tool Usage
When creating projects:
1. Use `write_file` to create files
2. Use `run_shell_command` for init
3. Don't give instructions - execute directly

## Commands
- `cargo build --release` - Build
- `cargo test` - Run tests
- `cargo clippy` - Lint
```

## Summary

**Recommended**: Use `GEMINI.md` at project root
- ✅ Highest priority
- ✅ Visible and easy to find
- ✅ Universal compatibility
- ✅ Well-documented standard

**Alternative**: Use `.grok/context.md` for Grok-specific rules

**Avoid**: `.grok.md` and `agent.md` (not auto-loaded)

**Best Practice**: Use multiple files for different concerns
- `GEMINI.md` - Main project rules
- `.claude.md` - Claude-specific rules
- `.zed/rules` - Editor-specific rules

All files will be automatically loaded and merged! 🎉