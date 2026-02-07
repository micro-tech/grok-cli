---
name: cli-design
description: Expert guidance for designing intuitive, user-friendly command-line interfaces. Use when building CLI tools or improving command-line user experience.
license: MIT
metadata:
  author: grok-cli
  version: "1.0"
  category: design
---

# CLI Design Skill

## Overview

This skill provides expert guidance for creating excellent command-line interfaces that are both powerful and user-friendly. It covers argument parsing, output formatting, error messages, and user experience best practices.

## Core Principles

1. **Discoverability**: Users should be able to figure out how to use your CLI
2. **Consistency**: Follow established conventions and patterns
3. **Feedback**: Provide clear feedback for all operations
4. **Forgiveness**: Make it hard to accidentally do destructive things

## Command Structure Best Practices

### Command Naming

- Use lowercase, hyphen-separated names: `my-tool`, not `MyTool` or `my_tool`
- Keep commands short but descriptive
- Use verbs for actions: `create`, `delete`, `list`, `show`
- Group related commands with subcommands: `git commit`, `docker run`

### Argument Design

```
tool [OPTIONS] <COMMAND> [ARGS]
```

- Required arguments: `<arg>`
- Optional arguments: `[arg]`
- Options/flags: `--flag`, `-f`
- Use long form (`--verbose`) and short form (`-v`)
- Group boolean flags: `-vvv` for verbosity levels

### Exit Codes

- 0: Success
- 1: General error
- 2: Misuse of command (invalid arguments)
- 130: Terminated by Ctrl+C
- Custom codes for specific errors (document them!)

## Output Guidelines

### Standard Output vs Standard Error

- Normal output → stdout
- Error messages, warnings, diagnostics → stderr
- Progress indicators, prompts → stderr (so they don't pollute piped output)

### Progress Indicators

```
[=====>    ] 50% Processing files...
⠋ Loading...
✓ Done!
✗ Failed
```

- Use spinners for indeterminate operations
- Use progress bars for operations with known duration
- Show what's happening: "Downloading package..." not just "Loading..."

### Color Usage

- Success: Green
- Errors: Red
- Warnings: Yellow
- Info: Blue/Cyan
- Dimmed text: Less important details
- Always provide `--no-color` option for CI/scripts
- Respect `NO_COLOR` environment variable

### Table Output

```
NAME        STATUS   AGE
my-app      Running  2h
database    Running  5d
cache       Stopped  1w
```

- Align columns properly
- Include headers
- Consider JSON/YAML output for machine consumption (`--output=json`)

## Error Messages

### Good Error Messages

✓ Clear and specific:
```
Error: File 'config.toml' not found in /home/user/project

Expected location: /home/user/project/config.toml
Create one with: my-tool init
```

✗ Vague:
```
Error: File not found
```

### Error Message Structure

1. **What went wrong**: "Failed to connect to database"
2. **Why it failed**: "Connection timeout after 30s"
3. **How to fix it**: "Check your network connection or increase timeout with --timeout"
4. **Context**: Show relevant values, file paths, etc.

## Interactive Features

### Prompts

```rust
// Confirmation
Do you want to delete all files? (y/N): 

// Selection
Choose environment:
  1) Development
  2) Staging
> 3) Production
Enter number (1-3): 

// Input with default
Project name [my-project]: 
```

- Show defaults in brackets: `[default]`
- Use capital letter for default choice: `(Y/n)` or `(y/N)`
- Validate input and show helpful errors
- Support `--yes` flag to skip prompts in automation

### Editor Integration

- Support `$EDITOR` environment variable for editing
- Provide sensible fallback (vim, nano, notepad)
- Validate input after editing

## Documentation

### Help Text

```
my-tool - A brief description

USAGE:
    my-tool [OPTIONS] <COMMAND>

OPTIONS:
    -v, --verbose       Increase verbosity
    -q, --quiet         Suppress output
    -h, --help          Print help
    -V, --version       Print version

COMMANDS:
    init                Initialize a new project
    build               Build the project
    test                Run tests
    help                Print this message or the help of a given subcommand

Run 'my-tool help <command>' for more information on a specific command.
```

### Man Pages

- Create man pages for serious CLI tools
- Include in package distribution
- Generate from help text when possible

## Configuration

### Configuration Precedence

1. Command-line flags (highest priority)
2. Environment variables
3. Config file in current directory (`.my-tool.toml`)
4. Config file in home directory (`~/.my-tool/config.toml`)
5. System config (`/etc/my-tool/config.toml`)
6. Built-in defaults (lowest priority)

### Config File Format

- Use TOML, YAML, or JSON
- Document all options
- Provide example config file
- Support `my-tool config` to show current config

## Common Patterns

### Piping Support

```bash
# Input from pipe
cat file.txt | my-tool process

# Output to pipe
my-tool list | grep pattern

# Both
cat input.txt | my-tool transform | less
```

- Read from stdin when no file specified
- Detect TTY vs pipe: different output formatting
- Support `-` as filename to mean stdin/stdout

### Watch Mode

```bash
my-tool watch --on-change "npm test"
```

- Useful for development workflows
- Debounce file changes
- Clear screen between runs

### Dry Run Mode

```bash
my-tool delete --dry-run
```

- Show what would happen without doing it
- Use for destructive operations
- Also called `--preview` or `--simulate`

## Platform Considerations

### Windows Support

- Use `\` and `/` for paths (accept both)
- Handle `CRLF` line endings
- Test with Windows Terminal and PowerShell
- Consider installer (MSI, Chocolatey, Scoop)

### macOS/Linux

- Follow XDG Base Directory spec for config files
- Support Homebrew installation
- Respect Unix conventions

## Performance

### Startup Time

- Keep CLI startup under 100ms
- Lazy load heavy dependencies
- Profile with `time` command
- Consider daemon mode for repeated calls

### Large Output

- Stream results instead of buffering
- Implement pagination for large lists
- Provide filtering options

## Security

- Never log sensitive data (passwords, tokens)
- Mask secrets in output: `token: abc***xyz`
- Read secrets from environment variables or secret stores
- Warn when using insecure options

## Testing CLI Tools

```rust
#[test]
fn test_cli_help() {
    let output = Command::new("my-tool")
        .arg("--help")
        .output()
        .expect("Failed to execute");
    
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("USAGE"));
}
```

- Test help text renders
- Test error codes
- Test with various input combinations
- Test with pipes (stdin/stdout)
- Snapshot testing for output

## Rust-Specific Tools

### Clap (Command-line Argument Parser)

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "my-tool")]
#[command(about = "A brief description", long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}
```

### Color Libraries

- `colored`: Simple color support
- `termcolor`: Cross-platform with no-color support
- `owo-colors`: Modern alternative

### Progress Bars

- `indicatif`: Feature-rich progress bars and spinners

### Terminal Interaction

- `dialoguer`: Prompts and confirmations
- `crossterm`: Cross-platform terminal manipulation

## When to Use This Skill

Activate this skill when:
- Designing a new CLI tool
- Improving existing CLI user experience
- Implementing command parsing
- Creating help text or documentation
- Handling user input and output formatting
- Making CLI tools more user-friendly

## Quick Checklist

- [ ] Help text is clear and complete
- [ ] All commands have descriptions
- [ ] Error messages are actionable
- [ ] Colors used appropriately (with --no-color option)
- [ ] Exit codes are meaningful
- [ ] Works with pipes (stdin/stdout)
- [ ] Configuration precedence is clear
- [ ] Destructive operations require confirmation
- [ ] Progress shown for long operations
- [ ] Tested on target platforms