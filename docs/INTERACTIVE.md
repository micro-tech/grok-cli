# Grok CLI Interactive Mode Guide

This document explains how to use the Grok CLI interactive mode, including all available commands and features.

## Overview

Interactive mode provides a conversational interface to Grok AI with built-in commands for session management, local shell execution, and more.

## Starting Interactive Mode

```bash
# Start with default settings
grok

# Or explicitly
grok interactive

# With custom model
grok --model grok-code-fast-1

# Hide banner
grok --hide-banner
```

## Interactive Commands

All commands in interactive mode start with a special character:
- `/` - Built-in commands (help, quit, settings, etc.)
- `!` - Local shell commands (execute on your computer)
- Regular text - Send to Grok AI

### Built-in Commands (/)

#### Session Management

- `/quit`, `/exit`, `/q` - Exit interactive mode
- `/reset` - Reset conversation history and start fresh
- `/save [name]` - Save current session to disk
- `/load [name]` - Load a previously saved session
- `/list` - List all saved sessions

#### Display & UI

- `/clear`, `/cls` - Clear screen and show logo
- `/help`, `/h` - Show help message with all commands
- `/status` - Show current session status and statistics
- `/history` - Display conversation history
- `/version` - Show version information

#### Configuration

- `/model [name]` - Show current model or change it
  ```
  /model                    # Show current model
  /model grok-code-fast-1   # Switch to code model
  /model grok-3             # Switch to grok-3
  ```

- `/system [prompt]` - Show or set system prompt
  ```
  /system                           # Show current prompt
  /system You are a helpful assistant  # Set new prompt
  ```

- `/settings` - Open settings menu
- `/config` - Show current configuration
- `/tools` - List available coding tools

### Shell Commands (!)

Execute commands locally on your computer without sending them to the AI. Perfect for checking files, running builds, or viewing system information.

#### Syntax

```
!<command>
```

The `!` prefix tells Grok CLI to execute the command on your local system instead of sending it to the AI.

#### Examples

**Windows:**
```
!dir                        # List files in current directory
!dir /b                     # List files (bare format)
!cd                         # Show current directory
!type README.md             # Display file contents
!echo Hello World           # Print message
!git status                 # Check git status
!cargo build                # Build Rust project
!npm test                   # Run npm tests
!systeminfo                 # Show system information
!tasklist                   # List running processes
```

**Linux/macOS:**
```
!ls                         # List files
!ls -la                     # List files with details
!pwd                        # Print working directory
!cat README.md              # Display file contents
!echo "Hello World"         # Print message
!git status                 # Check git status
!cargo build                # Build Rust project
!npm test                   # Run npm tests
!uname -a                   # Show system information
!ps aux                     # List running processes
!df -h                      # Show disk usage
!top -n 1                   # Show processes (one iteration)
```

#### How It Works

1. **Local Execution** - Commands run on your computer, not in the cloud
2. **Output Display** - stdout and stderr are shown immediately
3. **Exit Codes** - Non-zero exit codes are highlighted
4. **No AI Processing** - Commands never sent to Grok API
5. **Full Shell Access** - Use any command available in your shell

#### Benefits

- **Quick File Checks** - `!cat config.json` to see a file
- **Build Status** - `!cargo build` or `!npm run build`
- **Git Operations** - `!git status`, `!git diff`
- **System Info** - Check disk space, memory, processes
- **Testing** - Run tests without leaving chat
- **Debugging** - Execute diagnostic commands inline

#### Security Notes

⚠️ **Important Security Considerations:**

1. **Full Shell Access** - Shell commands have the same permissions as your user account
2. **No Sandboxing** - Commands execute directly in your shell
3. **Be Careful** - Dangerous commands (rm, del, format) will execute if you type them
4. **Review Before Enter** - Always double-check commands before pressing Enter
5. **No Confirmation** - Commands execute immediately without prompts

**Safe practices:**
- Use `!ls` or `!dir` to explore before acting
- Test destructive commands in a safe directory first
- Use version control (git) to protect against mistakes
- Keep backups of important files

#### Common Use Cases

**Workflow Integration:**
```
User: !git status
Output: On branch main, nothing to commit

User: Can you review my changes in src/main.rs?
Grok: [Reviews the file]

User: !cargo test
Output: All tests passed

User: Great! !git add . && git commit -m "feat: add new feature"
```

**File Exploration:**
```
User: !ls src/
Output: main.rs  lib.rs  config.rs

User: !cat src/config.rs
Output: [file contents]

User: Can you help me refactor this config module?
Grok: [Provides suggestions]
```

**Build and Deploy:**
```
User: !cargo build --release
Output: Compiling project...

User: !./target/release/app --version
Output: app 1.0.0

User: The build worked! !git push
```

#### Troubleshooting

**Command not found:**
- Ensure the command is in your PATH
- Use full path: `!/usr/local/bin/mycommand`
- On Windows, use proper extensions: `!node.exe` not just `!node`

**No output:**
- Some commands may be silent on success
- Check exit code (shown if non-zero)
- Try adding verbose flags: `!cargo build -v`

**Special characters:**
- Quote arguments with spaces: `!echo "hello world"`
- Escape special characters if needed
- Use your shell's syntax rules

**Long-running commands:**
- Commands block until completion
- Use Ctrl+C to interrupt Grok CLI (not the command)
- For background tasks, use your shell's background operator (`&` on Unix)

## Session Information

Interactive mode displays session information in the prompt:

```
Grok (grok-code-fast-1) [project-name | 2 context files] 
```

This shows:
- Current model: `grok-code-fast-1`
- Current directory: `project-name`
- Context files loaded: `2 context files`

## Context Loading

Grok CLI automatically loads project context from these files:
- `.gemini.md`
- `.grok/context.md`
- `.zed/rules`
- `.claude.md`

Context is shown in the session info and used in all conversations.

## Keyboard Shortcuts

- **Ctrl+C** - Cancel current input (in interactive mode)
- **Ctrl+D** - Exit interactive mode (Unix-like systems)
- **Tab** - Autocomplete commands (if supported by your terminal)
- **Up/Down Arrows** - Navigate command history (if supported)

## Tips & Best Practices

### Efficient Workflows

1. **Use Shell Commands for Quick Checks**
   ```
   !cat error.log
   What does this error mean?
   ```

2. **Combine with AI Analysis**
   ```
   !npm test
   These tests are failing, can you help?
   ```

3. **Iterate Quickly**
   ```
   Suggest a fix for this function
   [AI responds]
   !nano src/main.rs
   [Make changes]
   !cargo test
   ```

### Session Management

- **Save Important Sessions**: `/save project-name-feature`
- **Reset When Switching Topics**: `/reset` clears history
- **Load Previous Work**: `/load` to continue where you left off

### Model Selection

- **grok-3** - General purpose, balanced
- **grok-code-fast-1** - Optimized for coding (faster, focused)
- **grok-2-latest** - Latest stable release
- **grok-vision-beta** - For image analysis (when available)

Change models mid-session: `/model grok-code-fast-1`

### Context Management

- Keep context files updated
- Use `.grok/context.md` for project-specific instructions
- Check loaded context with `/status`

## Examples

### Code Review Session

```
Grok (grok-code-fast-1) [my-project] !git status
Executing: git status
On branch feature/new-api
Changes not staged for commit:
  modified:   src/api.rs

Grok (grok-code-fast-1) [my-project] !git diff src/api.rs
[Shows diff]

Grok (grok-code-fast-1) [my-project] Can you review these API changes?
[Grok provides detailed review]

Grok (grok-code-fast-1) [my-project] !cargo clippy
[Shows clippy warnings]

Grok (grok-code-fast-1) [my-project] How do I fix these warnings?
[Grok explains fixes]
```

### Debugging Session

```
Grok (grok-3) [webapp] !npm run dev
[Server starts with error]

Grok (grok-3) [webapp] The server won't start, see the error above
[Grok analyzes error]

Grok (grok-3) [webapp] !cat package.json
[Shows dependencies]

Grok (grok-3) [webapp] Should I update these packages?
[Grok recommends updates]

Grok (grok-3) [webapp] !npm update
[Updates packages]

Grok (grok-3) [webapp] !npm run dev
[Server starts successfully]
```

### System Administration

```
Grok (grok-3) [~] !df -h
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1       100G   85G   10G  90% /

Grok (grok-3) [~] My disk is almost full, what's using space?
[Grok suggests commands to check]

Grok (grok-3) [~] !du -sh ~/Downloads/* | sort -h | tail
[Shows largest directories]

Grok (grok-3) [~] Can I safely delete these old backups?
[Grok helps identify safe-to-delete files]
```

## Configuration

Interactive mode respects all configuration from `.env` files:

```bash
# In .grok/.env or ~/.grok/.env
GROK_MODEL=grok-code-fast-1
GROK_TEMPERATURE=0.7
GROK_COLORS=true
GROK_UNICODE=true
```

See [CONFIGURATION.md](CONFIGURATION.md) for full details.

## Exit Codes

- `0` - Normal exit
- `1` - Error occurred
- `130` - Interrupted by Ctrl+C

## Troubleshooting

### Interactive Mode Won't Start

- Check API key: `echo $GROK_API_KEY`
- Verify connectivity: `grok test-network`
- Check config: `grok config show`

### Commands Not Working

- Ensure you're using the correct prefix (/ or !)
- Check spelling: `/help` shows all commands
- Try `/version` to verify Grok CLI is working

### Shell Commands Failing

- Verify command exists: `which command` (Unix) or `where command` (Windows)
- Check permissions
- Try running the command in your regular terminal first
- Use full paths if needed

### Output Issues

- Enable colors: Set `GROK_COLORS=true` in `.env`
- Disable Unicode: Set `GROK_UNICODE=false` if characters display incorrectly
- Increase terminal width for better formatting

## See Also

- [Configuration Guide](CONFIGURATION.md) - Full configuration options
- [README](../README.md) - Getting started
- [API Documentation](API.md) - API details

## Feedback

If you have suggestions for improving interactive mode or encounter issues, please open an issue on GitHub.