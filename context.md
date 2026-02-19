# Grok CLI - Project Context

## System Information

This system uses Starlink satellite internet which may drop from time to time. All network calls must include:
- Comprehensive error checking
- Network drop detection
- Configurable timeouts
- Automatic retry logic with exponential backoff

## Project Overview

Grok CLI is a powerful command-line interface for interacting with Grok AI via X API. The project focuses on providing an enhanced development experience with features like session persistence, skills system, extension hooks, and comprehensive tooling integration.

## Key Technologies

- **Language**: Rust (2021 edition)
- **Primary Editor**: Zed Editor
- **Platform**: Windows 11 (with cross-platform support)
- **Build System**: Cargo
- **APIs**: X API (Grok AI), Agent Client Protocol (ACP)
- **Key Crates**: tokio, serde, clap, anyhow, thiserror

## Project Structure

```
grok-cli/
├── src/                # Source code
├── .zed/               # Zed editor configuration and tasks
├── .grok/              # Grok CLI project-specific config
├── Doc/                # Documentation
├── examples/           # Example code, skills, and extensions
├── tests/              # Integration tests
└── target/             # Build artifacts (ignored)
```

### Important Directories

- **`.zed/`** - Zed editor settings, tasks.json for task tracking, and AI rules
- **`Doc/`** - Comprehensive documentation for features and capabilities
- **`.grok/`** - Project-specific Grok CLI configuration
- **`examples/skills/`** - Example skills (rust-expert, cli-design, task-list)
- **`examples/extensions/`** - Hook extensions (logging, file-backup, project-setup)

## Development Guidelines

### Rust Best Practices

- Follow Rust 2021 edition standards
- Prioritize safety, concurrency, and performance
- Use expressive variable names that convey intent
- Follow Rust naming conventions:
  - `snake_case` for variables, functions, and modules
  - `CamelCase` for types and traits
  - `SCREAMING_SNAKE_CASE` for constants
- Implement proper error handling with `Result<T, E>` and `Option<T>`
- Use the `?` operator for error propagation
- Write doc comments (`///`) for public API functions
- Always run `cargo clippy` to catch common mistakes

### Error Handling

- Use `anyhow` for application-level error handling
- Use `thiserror` for library-level custom error types
- Provide meaningful error messages with context
- Handle network errors with retry logic (Starlink consideration)
- Never use unwrap/expect in production code without justification

### Testing

- Write unit tests using `#[cfg(test)]` modules
- Write integration tests in the `tests/` directory
- Test error cases, not just happy paths
- Run `cargo test` before committing
- Include doc tests in public API documentation

### Documentation

- Maintain clear, comprehensive documentation in Markdown format
- Use proper heading hierarchies (H1, H2, H3) for organization
- Include code examples where appropriate
- Be consistent with terminology throughout documentation
- Update CHANGELOG.md for all significant changes
- Store documentation notes in `.zed/` directory

### Configuration Management

- Keep configurations well-commented and organized
- Follow idiomatic practices for each format (JSON, TOML, etc.)
- Store API keys and secrets in `.env` files (git-ignored)
- Use `.grok/config.toml` for project-specific Grok CLI settings
- Document any non-obvious configuration settings

### Windows Integration

- Consider Windows-specific paths and separators (use `std::path`)
- Handle CRLF line endings appropriately
- Test on Windows 11 when possible
- Use Git Bash or WSL for shell scripts on Windows
- Follow Windows 11 UI/UX guidelines when applicable

### Network Resilience (Starlink)

- Implement exponential backoff for retries
- Use configurable timeouts (default: 60 seconds)
- Detect network drops and handle gracefully
- Log retry attempts for debugging
- See `src/utils/network.rs` for network utilities

## AI Assistant Behavior

### Code Assistance

- Prioritize accurate, implementation-ready solutions
- Acknowledge Windows and Rust-specific considerations
- Provide explanations focusing on the "why" behind recommendations
- Consider broader implications when suggesting code changes
- Include references to relevant documentation when appropriate
- Run mental `cargo check` and `cargo clippy` before suggesting code

### File Operations

- Use appropriate tools (read_file, write_file, edit_file)
- Preserve existing code style and formatting
- Update tests when changing functionality
- Update documentation when changing APIs
- Check for breaking changes

### Project Awareness

- Understand the skills system (activate with `/activate skill-name`)
- Understand the hooks/extensions system
- Respect the task list in `.zed/tasks.json`
- Follow dependency chains when working on tasks
- Update CHANGELOG.md for completed work

## Security Considerations

- Never commit API keys, passwords, or secrets to git
- Use `.env` files for sensitive configuration (git-ignored)
- Validate all user input
- Avoid command injection, SQL injection, path traversal
- Use `.grok/.env` for project-specific secrets
- Be cautious with `unsafe` code blocks
- Run security audits with `cargo audit`

## Key Features

### Skills System

- Modular instruction sets for AI expertise
- Located in `~/.grok/skills/` and `examples/skills/`
- Activate on-demand with `/activate skill-name`
- Deactivate with `/deactivate skill-name`
- Example skills: rust-expert, cli-design, task-list

### Extensions/Hooks System

- Intercept tool calls before/after execution
- Execute custom scripts in any language
- Located in `~/.grok/extensions/` and `examples/extensions/`
- Configure in config.toml: `[experimental.extensions]`
- Example extensions: logging-hook, file-backup-hook, project-setup-hook

### Session Persistence

- Save conversations with `/save session-name`
- Load conversations with `/load session-name`
- List saved sessions with `/list`
- Sessions saved to `~/.grok/sessions/`

### Configuration Hierarchy

1. System config: `~/.config/grok-cli/config.toml` (or `%APPDATA%\grok-cli\config.toml`)
2. Project config: `.grok/config.toml` (this file)
3. Environment variables: `.grok/.env`

## Git Workflow

### What NOT to Commit

```gitignore
# Build artifacts
/target/
Cargo.lock  # Only for libraries; commit for binaries

# Environment and secrets
.env
.env.local
.env.*.local
.grok/.env

# Project-specific directories
.zed/
.grok/

# Editor files
.vscode/
.idea/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db

# Rust artifacts
**/*.rs.bk
*.pdb

# Logs
*.log
*.tmp
*.temp
```

### Commit Guidelines

- Write clear, descriptive commit messages
- Use present tense ("Add feature" not "Added feature")
- Keep commits focused on a single change
- Run tests before committing
- Update CHANGELOG.md for notable changes

## Common Commands

```bash
# Build the project
cargo build

# Run the binary
cargo run -- [args]

# Run tests
cargo test

# Run clippy linter
cargo clippy

# Format code
cargo fmt

# Check without building
cargo check

# Build for release
cargo build --release

# Run specific binary
cargo run --bin grok -- interactive
```

## Resources

- **Documentation**: See `Doc/` directory for comprehensive guides
- **Examples**: See `examples/` for skills and extensions
- **Tasks**: See `.zed/tasks.json` for project task list
- **Changelog**: See `CHANGELOG.md` for version history
- **Skills Guide**: `Doc/SKILLS_QUICK_START.md`
- **Hooks Guide**: `Doc/HOOKS_AND_EXTENSIONS.md`

## Notes

- This project is at version 0.1.4 with 75% completion (52/69 tasks done)
- 17 tasks remaining in `.zed/tasks.json`
- For task management methodology, activate the `task-list` skill
- For project scaffolding, use the `project-setup-hook` extension
- All network code includes Starlink-optimized retry logic