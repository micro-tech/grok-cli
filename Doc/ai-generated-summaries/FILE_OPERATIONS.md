# Automated File Operations Guide

## Overview

Grok CLI now features **automatic tool execution** that allows Grok AI to create, modify, and manage files directly during conversations. This powerful feature eliminates the need for manual copy-pasting and enables true conversational project creation.

## What Changed?

### Before (v0.1.1 and earlier)
When you asked Grok to create a project, it would output text descriptions:
```
Creating project structure:
- Create directory: src/
- Create file: src/main.rs with content...
- Create file: Cargo.toml with content...
```

**Problem**: You had to manually create each file and copy the content.

### After (v0.1.2+)
Grok automatically executes file operations:
```
You: Create a new Rust project structure

Grok is executing operations...
  ✓ Successfully wrote to Cargo.toml
  ✓ Successfully wrote to src/main.rs
  ✓ Successfully wrote to .gitignore
All operations completed!
```

**Result**: Files are created automatically in your current directory!

## Available Tools

### 1. write_file
Creates a new file or overwrites an existing one with specified content.

**Usage Example**:
```
You: Write a hello world program to main.rs
You: Create a README.md with project documentation
You: Make a .env.example file with the required environment variables
```

**What happens**:
- **Automatically creates parent directories** (e.g., `.grok/` for `.grok/context.md`)
- Overwrites existing files (use with caution!)
- Shows confirmation with file path
- Works with deeply nested paths like `src/utils/helpers/file.rs`

### 2. read_file
Reads the content of an existing file.

**Usage Example**:
```
You: What's in the main.rs file?
You: Show me the contents of config.toml
You: Read the README
```

**What happens**:
- Reads and displays file contents
- Shows file size in bytes
- Returns error if file doesn't exist

### 3. replace
Finds and replaces text within a file.

**Usage Example**:
```
You: Replace "localhost" with "0.0.0.0" in config.toml
You: Change the version from 0.1.0 to 0.2.0 in Cargo.toml
You: Update the author name in all source files
```

**What happens**:
- Performs find/replace operation
- Shows number of replacements made
- Validates expected replacement count if specified

### 4. list_directory
Lists all files and directories in a specified path.

**Usage Example**:
```
You: What files are in the src directory?
You: List all files in the current folder
You: Show me what's in the docs/ directory
```

**What happens**:
- Lists all entries with directories marked with `/`
- Sorted alphabetically
- Shows error if directory doesn't exist
- Works with nested directories like `src/utils/`

### 5. glob_search
Finds files matching a glob pattern.

**Usage Example**:
```
You: Find all Rust files in the project
You: List all markdown files
You: Show me all .toml configuration files
```

**What happens**:
- Searches recursively from current directory
- Returns full paths of matching files
- Respects security restrictions

### 6. save_memory
Saves important facts to long-term memory.

**Usage Example**:
```
You: Remember that this project uses PostgreSQL
You: Save the fact that we're following clean architecture
You: Note that deployment happens via GitHub Actions
```

**What happens**:
- Appends fact to `~/.grok/memory.md`
- Persists across sessions
- Available for future context

### 7. run_shell_command
Executes shell commands in the current directory.

**Usage Example**:
```
You: Run cargo init to initialize a new Rust project
You: Execute git init to start version control
You: Run cargo build to compile the project
You: Run cargo new my_project --lib and then cd into it and run git init
```

**What happens**:
- Executes the command via PowerShell (Windows) or sh (Linux/Mac)
- **Automatic syntax conversion on Windows**: `&&` is converted to `;` for PowerShell compatibility
- Shows command output
- Returns exit status
- Subject to security validation

**PowerShell Note**:
On Windows, bash-style command chaining (`command1 && command2`) is automatically converted to PowerShell syntax (`command1; command2`). This means you can use natural bash syntax and it will work correctly on Windows.

**Examples**:
```
# This works on all platforms:
You: Run cargo new project && cd project && git init

# Automatically converted to on Windows:
cargo new project; cd project; git init
```

## Security & Permissions

### Trusted Directories
Tool execution is **restricted to safe directories**:
- ✅ Current working directory
- ✅ Subdirectories of current directory
- ❌ Parent directories
- ❌ System directories
- ❌ User home directory (except `.grok/`)

### How Security Works
```rust
// Current directory is automatically trusted
let mut security = SecurityPolicy::new();
security.add_trusted_directory(&env::current_dir()?);

// All tool operations are validated
tools::write_file(path, content, &security)?;
```

### Best Practices
1. **Start in your project directory** before running grok CLI
2. **Review operations** - Grok shows what it's doing
3. **Use version control** - Commit before major changes
4. **Check file contents** - Use `read_file` to verify results

## Practical Examples

### Example 1: Create a New Rust CLI Project

```bash
cd ~/projects
mkdir my-cli && cd my-cli
grok

You: Create a new Rust CLI project structure with:
- Cargo.toml for a binary called my-cli
- src/main.rs with argument parsing using clap
- README.md with basic documentation
- .gitignore for Rust projects
- .grok/context.md with project rules
```

**Result**:
```
Grok is executing operations...
  ✓ Successfully wrote to Cargo.toml
  ✓ Successfully wrote to src/main.rs
  ✓ Successfully wrote to README.md
  ✓ Successfully wrote to .gitignore
  ✓ Successfully wrote to .grok/context.md
All operations completed!
```

**Note**: The `.grok/` directory is automatically created!

### Example 2: Create a Web API Project

```bash
mkdir api-server && cd api-server
grok

You: Set up a Rust web API project using Axum with:
- Cargo.toml with axum, tokio, and serde dependencies
- src/main.rs with a basic server setup
- src/routes/ directory structure for API endpoints
- .env.example with configuration variables
- README with API documentation
```

### Example 3: Modify Existing Project

```bash
cd existing-project
grok

You: Update all files to use version 2.0.0 instead of 1.0.0
You: Add error handling to src/main.rs
You: Create a new module src/utils/logger.rs
```

### Example 4: Generate Documentation

```bash
grok

You: Create comprehensive documentation:
- docs/ARCHITECTURE.md explaining the project structure
- docs/API.md with endpoint documentation
- docs/SETUP.md with installation instructions
- Update README.md with links to all docs
```

## Enabling/Disabling Tool Execution

### Interactive Mode (Default: Enabled)
Tool execution is **always enabled** in interactive mode:
```bash
grok interactive
# or just
grok
```

### Single Query Mode
Tool execution is **enabled** for single queries:
```bash
grok query "Create a hello world program"
```

### Chat Mode
Tool execution is **enabled** in chat mode:
```bash
grok chat "Set up a new project" --interactive
```

## Troubleshooting

### Issue: Files Not Being Created

**Problem**: Grok describes what to do but doesn't create files.

**Solution**: 
1. Make sure you're using **v0.1.2 or later**:
   ```bash
   grok --version
   ```
2. Check that you're in interactive mode or using the query command
3. Verify your API key is set correctly

### Issue: Permission Denied

**Problem**: "Access denied: Path is not in a trusted directory"

**Solution**:
1. Make sure you're in your project directory
2. Don't try to access parent directories with `../`
3. System directories are not accessible

### Issue: Directory Not Found (FIXED)

**Problem**: "Failed to resolve path '.grok/context.md': The system cannot find the file specified"

**Solution**: This is now automatically fixed! Parent directories are created automatically.

If you see this error, rebuild:
```bash
cargo build --release
```

### Issue: File Already Exists

**Problem**: Concerned about overwriting files

**Solution**:
1. Use version control (git) to track changes
2. Ask Grok to "read the file first" before modifying
3. Use the `replace` tool for targeted edits instead of full rewrites

### Issue: Tool Not Executed

**Problem**: Grok responds with text instead of executing tools

**Solution**:
1. Be more explicit: "Create the file..." instead of "You could create..."
2. Use imperative language: "Write to main.rs" not "Maybe write to main.rs"
3. Update to latest version - tool calling was added in v0.1.2

## Tips for Best Results

### 1. Be Specific and Direct
❌ "It would be nice to have a config file"
✅ "Create a config.toml file with database settings"

### 2. Provide Context
❌ "Make a web server"
✅ "Create a Rust web server using Axum on port 3000 with health check endpoint"

### 3. Break Down Complex Tasks
❌ "Build a complete authentication system"
✅ "First, create the user model in src/models/user.rs"
✅ "Next, add authentication middleware in src/middleware/auth.rs"

### 4. Verify Results
After file operations:
```
You: List all files we just created
You: Show me the contents of main.rs
```

### 5. Use Natural Language
The system understands natural requests:
- "Make a new file called..."
- "Write the following code to..."
- "Update the configuration in..."
- "Create a directory structure for..."

## Technical Details

### How It Works

1. **Tool Definitions**: The CLI sends tool schemas to Grok AI
2. **AI Decision**: Grok decides if tools are needed for your request
3. **Tool Calls**: Response includes structured function calls
4. **Execution**: CLI executes each tool call sequentially
5. **Feedback**: Results are displayed to you

### Response Format
```json
{
  "role": "assistant",
  "content": "I'll create those files for you.",
  "tool_calls": [
    {
      "id": "call_123",
      "type": "function",
      "function": {
        "name": "write_file",
        "arguments": "{\"path\":\"main.rs\",\"content\":\"fn main() {}\"}"
      }
    }
  ]
}
```

### Execution Flow
```
User Request
    ↓
Grok API (with tool definitions)
    ↓
Response with tool_calls
    ↓
CLI executes each tool
    ↓
Display results to user
```

## Integration with Other Features

### Works with Session Persistence
```bash
grok

You: Create a new project structure
# ... files created ...

You: /save my-project-setup

# Later...
You: /load my-project-setup
# Continue where you left off
```

### Works with Context Discovery
Grok understands your project context when creating files:
```bash
# In a project with .zed/rules
grok

You: Create a new module following our project conventions
# Grok uses context from .zed/rules to match your style
```

### Works with Chat Logging
All tool executions are logged:
```bash
grok history view <session-id>
# See exactly what files were created and when
```

## Future Enhancements

Planned features for future versions:
- [ ] Undo/redo for file operations
- [ ] Dry-run mode to preview changes
- [ ] Batch operation confirmation prompts
- [ ] Git integration for automatic commits
- [ ] File templates and scaffolding
- [ ] Multi-file refactoring operations

## Feedback

This is a new feature! If you encounter issues or have suggestions:
- Open an issue: https://github.com/microtech/grok-cli/issues
- Contribute: See CONTRIBUTING.md
- Discuss: Start a discussion in GitHub Discussions

---

**Last Updated**: Version 0.1.2  
**Feature Status**: ✅ Stable  
**Platform Support**: Windows 11, macOS, Linux