# Changelog

All notable changes to the Grok CLI project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Installer Updates**: Updated all installers to v0.1.42 with enhanced features and network reliability
  - Updated `package.json` version from 0.1.4 to 0.1.42
  - Enhanced package description to highlight new features (external access, audit logging, tool loop debugging)
  - Updated Windows installer (`src/bin/installer.rs`) version display to show v0.1.42
  - Updated default model in installer config template to `grok-2-latest`
  - Enhanced installer success messages with feature announcements

- **Tool Message Handling**: Upgraded to use native `ChatMessage::tool()` method from `grok_api` v0.1.2
  - Replaced workaround that converted tool results to user messages
  - Tool messages now properly use `role: "tool"` with `tool_call_id` field
  - Improves compatibility with Grok API's expected message format
  - Eliminates potential confusion from tool results appearing as user messages
  - Requires `grok_api` v0.1.2 or higher with native tool support

### Added

- **Dynamic Skill Builder v2.0** - Create and activate custom skills on-the-fly without session restart:
  - Complete rewrite of skill-builder system with dynamic skill creation capabilities
  - Create skills from natural language descriptions or structured specifications (YAML/JSON)
  - Interactive step-by-step skill building with guided prompts
  - Clone and extend existing skills with automatic adaptation
  - Immediate activation - skills available instantly in current session without restart
  - Security validation with automatic tool permission checking
  - Comprehensive skill specification format (SKILL_SPEC.md) with validation rules
  - Support for skill dependencies, conflicts, and auto-activation triggers
  - Four creation modes: Natural Language, Specification-based, Interactive, Template-based
  - Tool usage guidelines with read-only defaults for security
  - Complete skill templates for common patterns (language experts, framework specialists, tool assistants)
  - Enhanced skill management: create, update, validate, and activate dynamically
  - 972-line comprehensive examples document with real-world use cases
  - 410-line skill specification format documentation
  - Integrated with existing `/activate` and `/skills` commands
  - Skills created via write_file tool immediately added to session's active_skills list
  - Progressive disclosure for efficient context management
  - Compatible with all existing skill system features

- **Starlink-Optimized Network Retry in npm Installer** (`install.js`):
  - Implemented exponential backoff retry logic (2s → 4s → 8s, max 60s)
  - Automatic retry on network drops (up to 3 attempts, 5-minute timeout per attempt)
  - Detects multiple network error types (timeout, ECONNRESET, ETIMEDOUT, ENOTFOUND, DNS failures)
  - Converted to async/await for better error handling and retry management
  - Clear user feedback during retry attempts with progress indicators
  - Network error-specific messages to help diagnose Starlink connection issues

- **Audit Directory Setup in Windows Installer**:
  - Automatically creates `~/.grok/audit/` directory during installation
  - Required for external file access audit logging (JSONL format)
  - Enables compliance tracking out of the box

- **Enhanced Configuration Template in Windows Installer**:
  - Added `[external_access]` section with security defaults (disabled, approval required, audit enabled)
  - Enhanced `[network]` section with retry delay settings for Starlink optimization
  - Added `[logging]` section with default settings
  - Added `[security]` section with shell approval mode
  - Updated to include all v0.1.42 configuration options

- **Expanded Documentation Installation**:
  - `EXTERNAL_FILE_ACCESS_SUMMARY.md` - Master summary of external access feature
  - `EXTERNAL_FILE_REFERENCE.md` - Complete implementation guide (406 lines)
  - `PROPOSAL_EXTERNAL_ACCESS.md` - Technical proposal (803 lines)
  - `TROUBLESHOOTING_TOOL_LOOPS.md` - Tool loop debugging guide
  - `SYSTEM_CONFIG_NOTES.md` - System configuration documentation
  - `CONTRIBUTING.md` - Contribution guidelines
  - All new docs automatically installed by Windows installer

- **Tool Loop Debugging Tools**: Added comprehensive diagnostic and troubleshooting tools for ACP tool loops
</text>

  - New PowerShell script `analyze_tool_loops.ps1` to analyze debug logs and identify loop patterns
  - New bash script `test_tool_loop_debug.sh` for reproducing and debugging tool loop issues
  - New documentation `Doc/TROUBLESHOOTING_TOOL_LOOPS.md` with detailed guide for diagnosing and fixing tool loops
  - Analyzer detects repeated tool calls, finish reason patterns, and provides actionable recommendations
  - Added comprehensive MCP server configuration examples to `config.example.toml` with proper syntax and documentation
  - New PowerShell script `update_system_config.ps1` to safely add MCP section to system config
  - New documentation `Doc/SYSTEM_CONFIG_NOTES.md` explaining system config settings and tool loop iteration limits
  - Test script creates controlled scenarios to verify tool loop behavior
  - Comprehensive examples of good vs bad prompts to prevent tool loops

### Fixed

- **Configuration Syntax**: Fixed `.grok/config.toml` missing required `env` field for MCP servers
  - MCP server configurations now include `env = {}` field (required even if empty)
  - Prevents TOML parse errors that could cause configuration loading failures
  - Updated project config template with proper MCP server structure
  - Added comprehensive MCP server examples to `config.example.toml` with commented templates

### Documentation

- Added comprehensive troubleshooting guide for tool loop issues
  - Explains normal vs abnormal tool loop behavior (1-10 iterations is normal)
  - Documents common causes: configuration issues, vague prompts, AI confusion
  - Provides diagnostic tools and solutions for each issue type
  - Includes prompt engineering best practices to prevent loops
  - Clarifies when to increase `max_tool_loop_iterations` (rarely needed)
  - **Key insight**: The limit is a safety mechanism - increasing it doesn't fix loop problems
- Added system configuration notes documentation
  - Explains configuration hierarchy and priority order
- **Added External File Access Documentation**: Comprehensive guides for referencing files outside project boundaries
  - `EXTERNAL_FILE_ACCESS_SUMMARY.md` - Master summary of all solutions and workarounds
  - `Doc/EXTERNAL_FILE_REFERENCE.md` - Complete guide with step-by-step instructions (406 lines)
  - `Doc/PROPOSAL_EXTERNAL_ACCESS.md` - Technical proposal for future configurable external access feature (803 lines)
  - `.zed/EXTERNAL_FILES_QUICK_REF.md` - Quick reference card with common solutions (171 lines)
  - Documents 5 workarounds: symlinks (recommended), file copying, copy-paste, terminal commands, and Zed multi-root
  - Includes Windows-specific guidance for symlinks without admin rights (Developer Mode, junctions)
  - Security best practices and .gitignore templates
  - Troubleshooting guide for common symlink issues
  - Comparison tables and real-world examples
  - Proposes future configuration-based external directory access with security controls

- **Configurable External Directory Access (Complete Feature)**: Full implementation of secure read-only access to files outside project boundaries
  - **Configuration Schema** (`src/config/mod.rs`):
    - Added `ExternalAccessConfig` struct with all security controls
    - Environment variable support: `GROK_EXTERNAL_ACCESS_ENABLED`, `GROK_EXTERNAL_ACCESS_PATHS`, etc.
    - Default excluded patterns protect 13 types of sensitive files (.env, .ssh/, keys, credentials)
    - Session-based trusted paths with thread-safe storage
  - **Enhanced Security Policy** (`src/acp/security.rs`):
    - Three-tier access validation: Internal, External, ExternalRequiresApproval
    - Glob pattern matching for file exclusions
    - Path canonicalization prevents symlink attacks
    - Session trust management for "Trust Always" decisions
  - **Interactive Approval UI** (`src/cli/approval.rs`):
    - Styled terminal prompts with box drawing characters
    - Four options: [A]llow Once, [T]rust Always, [D]eny, [V]iew Path
    - View file metadata before approving access
    - Batch approval support for multiple files
  - **Read File Integration** (`src/acp/tools.rs`):
    - Integrated approval prompts for external files
    - Session trust support (no persistent storage)
    - Comprehensive logging with tracing
  - **Audit Logging System** (`src/security/audit.rs`):
    - Complete audit trail in JSONL format (~/.grok/audit/external_access.jsonl)
    - Tracks: timestamp, path, operation, decision, user, session_id
    - Username detection with `whoami` crate
    - Query methods: recent logs, date ranges, by path
    - Statistics and analytics: total/allowed/denied, top paths
  - **Configuration Validation** (`grok config validate-external-access`):
    - Verify paths exist and are readable
    - Validate glob pattern syntax
    - Security recommendations
    - Check approval and logging settings
  - **Audit Summary Command** (`grok audit external-access`):
    - View recent access logs with filtering
    - Date range filtering (--from, --to)
    - Path-specific filtering (--path)
    - Statistics dashboard (--summary)
    - CSV export (--export)
    - Top 10 most accessed paths
    - Recent denials with reasons
  - **Security Features**:
    - Read-only access (no write operations)
    - Default-deny with explicit allow-list
    - User approval required by default
    - Complete audit trail for compliance
    - 13 default sensitive file patterns
  - Documents how system config interacts with project config
  - Provides recommendations for setting `max_tool_loop_iterations` based on use case
  - Includes instructions for safely updating system config without breaking existing setup

## [0.1.41] - 2025-02-15

### Changed

- **Version Update**: Updated project version from 0.1.4 to 0.1.41 to reflect ongoing development and improvements.

### Fixed

- **Tool Calling**: Addressed issues with tool calling functionality to ensure proper execution and response handling.
  - Fixed errors in tool message processing to align with the expected format by the Grok API.
  - Improved reliability of tool execution in multi-turn conversations.

## [0.1.4] - 2025-02-10

### Added

- **Apple Silicon Support**: Added native ARM64 (aarch64) build for macOS
  - New release artifact: `grok-cli-macos-aarch64.zip` for Apple Silicon Macs
  - Updated GitHub Actions workflow to build for `aarch64-apple-darwin` target
  - All 4 platforms now supported:
    - `grok-cli-windows-x86_64.zip` (Windows x64)
    - `grok-cli-macos-x86_64.zip` (macOS Intel)
    - `grok-cli-macos-aarch64.zip` (macOS Apple Silicon) ← **NEW**
    - `grok-cli-linux-x86_64.zip` (Linux x64)
  - Native performance on M1/M2/M3 Macs without Rosetta translation
  - Improved build matrix in CI/CD pipeline for cross-platform compilation

### Fixed

- **CRITICAL: Tool Loop Exit Condition**: Fixed ACP requests not exiting properly, causing 40+ minute timeouts
  - **Bug #1**: Empty tool_calls array `Some([])` was not detected, causing infinite loops
  - **Bug #2**: `finish_reason` field from API was being ignored - the model was saying "I'm done" but we kept looping!
  - Now properly checks `finish_reason: "stop"` to exit immediately when model signals completion
  - Added `MessageWithFinishReason` wrapper to preserve finish_reason from API responses
  - Tool loop now exits in 1-5 iterations for most requests instead of hitting max iterations
  - Enhanced logging shows finish_reason in debug output: `📋 Finish reason: Some("stop")`
  - All chat completion callers updated to handle finish_reason properly
  - Impact: Requests that took 40 minutes now complete in seconds
  - Confidence: VERY HIGH - this was the root cause of excessive looping

- **Configuration Loading**: Fixed `config.toml` not being loaded in hierarchical mode
  - `Config::load_hierarchical()` now loads both `config.toml` and `.env` files
  - System config now properly loaded from `%APPDATA%\grok-cli\config.toml` on Windows
  - Project config now properly loaded from `.grok/config.toml` in project directories
  - Previously only `.env` files were loaded, causing config.toml settings to be ignored
  - Configuration source now correctly displayed in interactive mode session info
  - Added comprehensive loading order: defaults → system config → system env → project config → project env → environment variables
  - Created `fix_config_syntax.ps1` script to fix common TOML syntax errors (e.g., numbers with commas)
  - All user-configured settings (including `max_tool_loop_iterations`) now load correctly

### Added

- **Context Discovery Enhancement**: Context files now walk up directory tree to find project root
  - Context discovery now matches configuration discovery behavior
  - Works from any subdirectory within a project
  - Automatically finds project root by detecting `.git`, `Cargo.toml`, `package.json`, or `.grok/`
  - No longer requires running grok from project root for context loading
  - Applies to all context file types: `.zed/rules`, `.grok/context.md`, `GEMINI.md`, etc.
  - Created PROJECT_CONTEXT_GUIDE.md (560 lines) - comprehensive guide to context and config discovery

- **Troubleshooting Documentation**: Created comprehensive TROUBLESHOOTING.md guide
  - Version conflict resolution (multiple installations)
  - Configuration hierarchy explanation and debugging
  - Network issue handling (Starlink compatibility)
  - Installation troubleshooting
  - Common error messages and solutions
  - Quick reference commands
  - Configuration priority diagram

- **Version Conflict Detection**: Installer now detects and removes old Cargo installations
  - Automatic detection of old `~/.cargo/bin/grok.exe` during installation
  - Interactive prompt to remove conflicting versions
  - Prevents version mismatch issues (e.g., PowerShell showing 0.1.3 while 0.1.4 is installed)
  - Enhanced user feedback about version conflicts

- **Cleanup Scripts**: Added scripts for removing old installations
  - PowerShell script: `scripts/cleanup_old_install.ps1`
    - Detects both Cargo and AppData installations
    - Shows version information for each
    - Interactive removal with confirmation
    - Verifies correct version after cleanup
  - Batch script: `scripts/cleanup_old_install.bat`
    - Windows-native alternative to PowerShell
    - Same functionality in batch format
    - Works without PowerShell execution policy issues

### Fixed

- **MCP Implementation Recovery (2025-02-10)**: Verified and restored MCP functionality after system crash
  - Added missing `[[bin]]` definition for `github_mcp` in Cargo.toml
  - Verified all MCP source code intact (client.rs, config.rs, protocol.rs, github_mcp.rs)
  - Successfully rebuilt github_mcp binary (5.8 MB)
  - Tested all core MCP functionality:
    - Initialize handshake ✅
    - Tool listing ✅
    - Tool execution (search_repos) ✅
  - GitHub search tool working with top repositories
  - Network resilience features confirmed operational
  - Created comprehensive status documentation (MCP_STATUS.md, MCP_RECOVERY_SUMMARY.md)
  - Zero data loss from crash - only build configuration needed update
  - Status: Fully operational and production-ready

### Added

- **Installer Enhancements**: Comprehensive installation package with complete documentation and examples
  - Added documentation installation: 12 documentation files now included
  - Added example skills: rust-expert and cli-design skills with complete implementations
  - Added LICENSE file to installation for legal compliance
  - Added config.example.toml with all 139 settings documented
  - Added MAX_TOOL_LOOP_ITERATIONS.md: 346-line comprehensive error resolution guide
  - Created install_additional_files() function for documentation/examples installation
  - Created copy_dir_recursive() helper for recursive directory copying
  - Enhanced post-install feedback showing documentation paths
  - Installation now includes: TOOLS.md, settings.md, ZED_INTEGRATION.md, WEB_TOOLS_SETUP.md
  - Installation now includes: SKILLS_QUICK_START.md, SKILL_SECURITY.md, SKILL_SPECIFICATION.md
  - Complete installation package increased from ~12 MB to ~15 MB (25% increase for full docs)
  - Created INSTALLER_REQUIREMENTS.md (562 lines) - detailed requirements specification
  - Created INSTALLER_CHECKLIST.md (315 lines) - comprehensive verification checklist
  - Created INSTALLER_SUMMARY.md (422 lines) - status and improvements documentation
  - Installation directory structure now includes docs/, examples/, and LICENSE
  - Users now have offline access to all documentation and examples

- **Configurable Tool Loop Iterations**: Added `max_tool_loop_iterations` setting to prevent infinite loops
  - New configuration option: `acp.max_tool_loop_iterations` (default: 25, previously hardcoded to 10)
  - Configurable via environment variable: `GROK_ACP_MAX_TOOL_LOOP_ITERATIONS`
  - Configurable via config file: `config.toml` under `[acp]` section
  - Increased default from 10 to 25 iterations to handle more complex multi-step tasks
  - Improved error message now shows the limit and suggests solutions
  - Helps users resolve "Max tool loop iterations reached" errors
  - Documented in TOOLS.md, CONFIGURATION.md, settings.md, and README.md
  - Recommended values: 25 (default), 50 (complex tasks), 100+ (very complex operations)

- **Skills System Enhancements - Progressive Disclosure**: Implemented on-demand skill activation
  - Added `active_skills` field to `InteractiveSession` for session-level skill state management
  - Skills are no longer loaded into context at startup (reduces token usage)
  - New interactive commands:
    - `/skills` - List all available skills with activation status
    - `/activate <skill>` - Activate a skill for the current session
    - `/deactivate <skill>` - Deactivate an active skill
  - Active skills automatically included in system prompt for each message
  - Skills show in session info with count: "Skills: X available, Y active"
  - Added autocomplete suggestions for skill commands
  - Example skills provided in `examples/skills/`:
    - `rust-expert` - Expert Rust development guidance
    - `cli-design` - CLI design and UX best practices
  - Allows users to control which skills are active, reducing context size and improving performance

- **Skill Security Validation System**: Comprehensive security framework to protect against malicious skills
  - Created `skills::security` module with pattern-based threat detection
  - Added `SkillSecurityValidator` with 4 validation levels: Safe, Warning, Suspicious, Dangerous
  - Automatic validation when activating skills in interactive mode
  - New CLI command: `grok skills validate <name>` for security scanning
  - Detects 15+ dangerous patterns: command injection, data exfiltration, prompt injection
  - Detects 8+ suspicious patterns: file operations, network access, shell commands
  - Blocks DANGEROUS and SUSPICIOUS skills by default for user safety
  - Validates SKILL.md content, scripts/, and references/ directories
  - Detects encoded content (base64, hex) that may hide malicious payloads
  - Comprehensive security documentation in `Doc/SKILL_SECURITY.md` (562 lines)
  - Inspired by recent CVE-2025-53109 & CVE-2025-53110 (Claude Desktop RCE vulnerabilities)
  - Added 5 security validation tests
  - Total tests: 88 (up from 83)

- **Web Tools Error Handling**: Enhanced error handling and configuration checking for web tools
  - Added `is_web_search_configured()` to check if Google API credentials are set
  - Added `get_available_tool_definitions()` to filter out unconfigured tools
  - Web search tool now shows helpful setup instructions when API keys are missing
  - Web fetch tool includes detailed error messages for network issues
  - Added timeout (30s) to web fetch requests to prevent hanging
  - Unconfigured web tools are automatically filtered from tool list
  - Added 5 new tests for web tool error handling and configuration checking
  - Total tests: 83 (up from 78)

### Fixed

- **Input cursor positioning**: Fixed cursor appearing outside the input box when typing long text
  - Implemented horizontal scrolling for input text that exceeds box width
  - Cursor now stays properly positioned within the visible box area
  - Text automatically scrolls as you type beyond the visible area
  - Works correctly on window resize

- **Web Tools Failure in Command Line Mode**: Fixed web search/fetch tools failing without helpful error messages
  - Tools now provide clear setup instructions when API keys are missing
  - Better error messages explain network failures and how to resolve them
  - Unconfigured web tools no longer appear in available tools list
  - Added comprehensive test coverage for web tool error scenarios

### Changed

- **Test Race Conditions**: Fixed failing tests in `utils::context` module
  - Added `serial_test = "3.0"` dependency to dev-dependencies
  - Applied `#[serial]` attribute to tests that manipulate global environment variables
  - All 78 tests now passing reliably without race conditions

- **Migrated to `grok_api` crate**: Replaced local API implementation with published `grok_api = "0.1.0"` crate from crates.io
  - Removed local `src/api` module
  - Created `grok_client_ext` compatibility wrapper to maintain existing API surface
  - All existing functionality preserved with minimal changes
  - Benefits: Better maintenance, version management, and reusability across projects
  - All 78 tests passing successfully

## [0.1.2] - 2026-01-13

### Added

- **Automatic Tool Execution**: Grok can now execute file operations automatically during conversations!
  - Integrated tool calling support into interactive and single-query chat modes
  - Grok can create files and directories directly without manual copy-pasting
  - Supported tools:
    - `write_file` - Create or overwrite files with content
    - `read_file` - Read file contents
    - `replace` - Find and replace text in files
    - `list_directory` - List directory contents
    - `glob_search` - Find files matching glob patterns
    - `save_memory` - Save facts to long-term memory
    - `run_shell_command` - Execute shell commands (cargo, git, etc.)
  - Automatic PowerShell syntax conversion: bash-style `&&` converted to `;` on Windows
  - Automatic parent directory creation: creates `.grok/` and other nested directories automatically
  - Security-restricted to current directory and subdirectories
  - Visual feedback with ✓ confirmation for each operation
  - Example: Ask "Create a new Rust project structure" and files are created automatically
  - Comprehensive documentation in `docs/FILE_OPERATIONS.md` (402 lines)
  - Works seamlessly with session persistence and context discovery

- **Chat Session Logging**: Comprehensive conversation logging system
  - Automatic logging of all chat sessions (ACP and interactive modes)
  - Dual format output: JSON (machine-readable) and TXT (human-readable)
  - Unique session IDs with timestamps for easy retrieval
  - Full metadata tracking (timestamps, roles, optional data)
  - Automatic log rotation based on size limits
  - Network-resilient with proper error handling for Starlink drops
  - Configuration via environment variables:
    - `GROK_CHAT_LOGGING_ENABLED` - Enable/disable logging (default: true)
    - `GROK_CHAT_LOG_DIR` - Custom log directory
    - `GROK_CHAT_LOG_MAX_SIZE_MB` - Max file size before rotation
    - `GROK_CHAT_LOG_ROTATION_COUNT` - Number of files to keep
    - `GROK_CHAT_LOG_INCLUDE_SYSTEM` - Include system messages
  - Default location: `~/.grok/logs/chat_sessions/`
  
- **Chat History Commands**: New `grok history` command suite
  - `grok history list` - List all saved chat sessions with previews
  - `grok history view <session-id>` - View complete conversation transcript
  - `grok history search "query"` - Search through all sessions with highlighting
  - `grok history clear --confirm` - Clear all chat history
  - Rich terminal formatting with colored output
  - Session metadata display (start time, duration, message count)
  - Context-aware search results with line previews

- **Documentation**: New comprehensive chat logging guide
  - `docs/CHAT_LOGGING.md` - Complete feature documentation (415 lines)
  - Configuration examples and environment variable reference
  - Usage examples and troubleshooting guide
  - Privacy and security best practices
  - API reference for programmatic access
  - Updated `CONFIGURATION.md` with chat logging settings

### Changed
- **ACP Module Visibility**: Made `security` and `tools` modules public for use in chat commands
  - Enables tool execution in regular chat mode (not just ACP/Zed integration)
  - Maintains security policies across all modes

- **Rust 2024 Edition Upgrade**: Updated from Rust 2021 to Rust 2024 edition
  - Improved safety requirements for environment variable operations
  - Updated all unsafe operations to comply with edition 2024 standards
  - All tests passing with new edition requirements

- **Chat Command Enhancement**: Enhanced chat commands to support tool calling
  - Added tool definitions to API requests
  - Integrated tool call response parsing and execution
  - Improved error handling for tool operations

- **Shell Command Compatibility**: Fixed PowerShell command chaining issues
  - Automatic conversion of bash-style `&&` to PowerShell `;` separator
  - Enables natural command syntax across all platforms
  - Example: `cargo new project && git init` works on Windows now

- **File Operations Enhancement**: Fixed directory creation for nested paths
  - Parent directories now created before path resolution
  - Fixes "file not found" errors when writing to `.grok/context.md` and similar nested paths
  - Added `working_directory()` getter to SecurityPolicy for proper path handling

- **Dependency Updates**: Updated all major dependencies to latest versions
  - `tokio` updated to 1.49.0 (from 1.40.0) - Async runtime improvements
  - `reqwest` locked to 0.13.1 with `native-tls-vendored` for Windows compatibility
  - `clap` updated to 4.5.54 (from 4.5.20) - CLI parsing improvements
  - `toml` updated to 0.9.11 (from 0.9.8) - TOML parsing updates
  - `anyhow` updated to 1.0.100 - Error handling improvements
  - `uuid` updated to 1.19.0 - UUID generation updates
  - `chrono` updated to 0.4.42 - Date/time handling improvements
  - `thiserror` updated to 2.0.17 - Error derive macro improvements
  - All dependencies tested and working with Rust 2024 edition

- **TLS Backend Configuration**: Optimized for Windows 11 development
  - Using `native-tls-vendored` instead of `rustls` for reqwest 0.13.1
  - Avoids CMake/NASM build dependencies on Windows
  - Uses native Windows SChannel API for TLS
  - Faster build times (~30s vs ~60s with rustls)
  - Automatic Windows certificate store integration
  - Reliable network operations for Starlink connectivity
  - See `docs/TLS_BACKEND_WINDOWS.md` for detailed TLS backend options

- **Security: Replaced Unmaintained Dependencies**
  - Replaced `dotenv` (unmaintained) with `dotenvy` (maintained)
  - Replaced `atty` (unmaintained) with `std::io::IsTerminal` (stdlib)
  - Removed `term_size` (unmaintained) - using `terminal_size` instead
  - TLS implementation using Windows-native SChannel (native-tls-vendored)
  - All security advisories resolved
  - Zero vulnerabilities in dependency tree

### Documentation
- Added `docs/TLS_BACKEND_WINDOWS.md` - Comprehensive TLS backend configuration guide
- Added `TLS_UPDATE_SUMMARY.md` - Quick reference for TLS backend changes
- Documented native-tls vs rustls trade-offs for Windows development
- Included instructions for switching to rustls if pure Rust TLS is required

### Added
- **Shell Command Permission System**: Comprehensive security for `!` commands
  - Interactive approval prompts with allow/deny/always options
  - Session-level allowlist (temporary permissions)
  - Persistent allowlist saved to `~/.grok/shell_policy.json`
  - Automatic blocklist for dangerous commands (rm, shutdown, format, etc.)
  - Command root extraction for intelligent permission management
  - Approval modes: Default (prompt) and YOLO (always allow)
  - Configuration via `GROK_SHELL_APPROVAL_MODE` environment variable
  - Inspired by Gemini CLI's security model

- **Shell Command Execution**: Local command execution in interactive mode
  - Execute shell commands with `!` prefix (e.g., `!ls`, `!git status`)
  - Cross-platform support (Windows cmd, Unix sh)
  - Real-time stdout/stderr output
  - Exit code reporting for failed commands
  - Commands never sent to AI - executed locally only
  - Integrated with permission system for safety

- **Configuration Consolidation**: Unified `.env`-based configuration
  - Migrated from mixed TOML/env to pure `.env` files
  - Hierarchical loading: project `.grok/.env` → system `~/.grok/.env` → defaults
  - 50+ environment variables for all settings
  - Clear configuration priority rules
  - Project-specific overrides in `.grok/.env`
  - System-wide settings in `~/.grok/.env`
  - Removed redundant TOML config files

- **Hierarchical Configuration System**: Multi-tier config loading (Task 12)
  - Project-level: `.grok/.env` in project root
  - System-level: `~/.grok/.env` in home directory
  - Built-in defaults with proper fallback
  - Configuration source tracking and display
  - Proper field-level merging with serde defaults
  - Environment variable overrides (highest priority)

- **Session Persistence**: Implemented full session save/load functionality
  - Added `/save <name>` command to save current conversation session
  - Added `/load <name>` command to load a previously saved session
  - Added `/list` command to show all saved sessions
  - Sessions stored as JSON in `~/.grok/sessions/`
  - Full conversation history and context preserved across sessions

- **Context File Integration**: Automatic project context loading
  - Detects and loads project context files on startup (GEMINI.md, .grok/context.md, .ai/context.md, CONTEXT.md)
  - Context automatically injected into system prompt to ground the agent
  - Support for multiple context file locations with priority order
  - File size validation (5 MB max) and error handling
  - Visual feedback when context is loaded

- **Extension Loading System**: Complete extension framework implementation
  - Extension discovery from `~/.grok/extensions/` directory
  - Extension manifest parsing (`extension.json`)
  - Hook-based extension API with `before_tool` and `after_tool` hooks
  - Extension Manager and Hook Manager for lifecycle management
  - Configuration-based extension enabling/disabling

- **Comprehensive Documentation**:
  - `SECURITY.md` - Shell command security guide (460+ lines)
  - `INTERACTIVE.md` - Interactive mode complete guide (400+ lines)
  - `QUICKSTART.md` - Quick start guide (378+ lines)
  - `CONFIGURATION.md` - Configuration guide (458+ lines)
  - Updated all existing documentation with new features
  - Support for extension dependencies
  - Example logging-hook extension included

- **Hierarchical Configuration Loading**: Three-tier configuration priority system
  - Project-local settings (`.grok/config.toml` in project root)
  - System-level settings (`~/.grok/config.toml` or `%APPDATA%\.grok`)
  - Built-in defaults
  - Automatic project root detection (walks up directory tree)
  - Config merging with proper priority: project → system → defaults
  - Environment variable overrides still take highest priority
  - Supports per-project customization while maintaining global preferences

- **Enhanced Context Rules Discovery**: Multi-editor context file support
  - Expanded context file discovery to include editor-specific files:
    - `.zed/rules` (Zed editor)
    - `.gemini.md` (Gemini CLI)
    - `.claude.md` (Claude AI)
    - `.cursor/rules` (Cursor editor)
    - `AI_RULES.md` (generic)
  - Support for loading and merging multiple context files
  - Visual feedback shows all loaded context sources
  - Annotated context with source file information
  - Compatible with existing GEMINI.md and other formats

### Changed
- Enhanced interactive session startup with context loading feedback
- Improved session info display to show loaded context files
- CLI now uses hierarchical config loading by default
- Context loading supports merging multiple files with source annotations
- Added comprehensive extension system documentation (docs/extensions.md)

### Technical Details
- Added `Config::load_hierarchical()` for cascading configuration
- Added `Config::find_project_config()` to walk directory tree
- Added `Config::merge_configs()` for proper config priority merging
- Enhanced `src/utils/context.rs` with multi-file support:
  - `load_and_merge_project_context()` for merging multiple files
  - `get_all_context_file_paths()` to list all available contexts
- Added `src/hooks/loader.rs` for extension loading and management
- Extended `Config` with `ExtensionsConfig` structure
- Session persistence uses Serde serialization with proper error handling
- Network resilience built into context loading (Starlink-aware)

### Documentation
- Added comprehensive extension system documentation (docs/extensions.md)
- Created example extension with full README (examples/extensions/logging-hook/)
- Created progress report (docs/PROGRESS_REPORT.md)
- Created quick reference guide (docs/QUICK_REFERENCE.md)
- Documented context file integration in code comments
- Added task tracking updates in .zed/tasks.json

## [0.1.1] - 2024-01-XX

### Fixed
- **Critical**: Resolved "failed to deserialize response" error in Zed editor integration (TWO ROOT CAUSES)
  
  **Issue 1: Clap Argument Definitions**
  - Added missing `#[arg(...)]` attributes to all Clap command-line argument definitions
  - Fixed `ConfigAction::Init` force flag parsing
  - Fixed `AcpAction::Server` port and host argument parsing
  - Fixed `AcpAction::Test` address argument parsing
  - Fixed `CodeAction` boolean flags and optional parameters
  - Fixed `SettingsAction` optional parameters
  
  **Issue 2: Protocol Serialization Mismatch**
  - Fixed protocol version serialization (was returning "2024-04-15" instead of echoing client's version)
  - Added camelCase field names for JSON-RPC protocol (protocolVersion, agentCapabilities, etc.)
  - Added custom serializer to output protocol version as integer when numeric
  - Fixed protocol version to echo back client's version instead of hardcoding
  - Added flexible protocol version parser to handle both integer and string formats
  - Updated all ACP protocol structs to use camelCase for Zed compatibility

- All commands now work correctly with proper flag and option parsing
- ACP protocol now fully compatible with Zed editor's JSON-RPC expectations

### Added
- Comprehensive Zed integration documentation (ZED_INTEGRATION.md)
- Technical fix documentation (FIXES.md)
- Quick fix guide (QUICKFIX_ZED.md)
- Complete project summary (SUMMARY.md)
- This CHANGELOG file

### Changed
- Updated README.md with fix notification and improved Zed integration section
- Improved ACP server command with default host value (127.0.0.1)
- Enhanced error messages and help text for all commands

### Documentation
- Added complete Zed editor setup guide with STDIO and Server mode instructions
- Added troubleshooting section with common issues and solutions
- Added quick reference guide for all commands
- Updated README with links to all new documentation

## [0.1.0] - 2024-01-XX

### Added
- Initial release of Grok CLI
- Beautiful interactive terminal interface inspired by Gemini CLI
- Adaptive ASCII art logo with multiple size variants
- Rich interactive mode with context-aware prompts
- Chat completion with Grok AI (grok-2-latest, grok-2, grok-1)
- Code operations (explain, review, generate, fix)
- Agent Client Protocol (ACP) support for Zed editor integration
- Configuration management system with TOML config files
- Settings management with interactive browser
- Health monitoring and diagnostics
- Network resilience features with Starlink optimizations
- Colored output with professional color scheme
- Progress indicators and status displays
- Session management and conversation history
- Temperature and token control
- Multi-language code support
- Security policy engine for shell command execution
- MCP (Model Context Protocol) integration
- Web search and fetch capabilities
- File search with glob patterns
- Content search with ripgrep integration
- Persistent memory system
- Comprehensive error handling and retry logic
- Windows, Linux, and macOS support

### Features

#### Core Functionality
- Interactive chat sessions with context tracking
- Single-shot queries for quick answers
- System prompts for specialized behavior
- Streaming responses with real-time display
- Token usage monitoring
- Session saving and restoration

#### Code Intelligence
- Code explanation with language detection
- Code review with focus areas (security, performance, style)
- Code generation from natural language
- Code fixing with issue descriptions
- Multi-file analysis support

#### Developer Tools
- ACP server mode for editor integration
- ACP STDIO mode for subprocess communication
- Configuration initialization and validation
- API key management
- Network connectivity testing
- Health checks (API, config, network)

#### Network Features
- Starlink satellite internet optimizations
- Smart retry logic with exponential backoff
- Connection drop detection and recovery
- Configurable timeouts and retry limits
- Network health monitoring

#### UI/UX
- Adaptive terminal width detection
- Unicode support with fallback to ASCII
- Colored output with theme support
- Progress bars for long operations
- Banner and tips system
- Status indicators
- Error formatting with context

#### Configuration
- TOML-based configuration system
- Environment variable support
- Per-user config files
- Config validation
- Interactive settings editor
- Import/export capabilities

### Fixed
- **ACP Protocol Serialization**: Fixed `SessionUpdate` enum tag from `sessionUpdate` to `type`
  - Corrected JSON output to match ACP specification
  - All 68 tests now passing

### Technical Details
- Built with Rust 2024 edition
- Async runtime with Tokio
- HTTP client with reqwest (rustls-tls)
- CLI parsing with Clap 4.x
- Comprehensive test suite (68 tests passing)
- Structured logging with tracing
- JSON serialization with serde
- Terminal UI with ratatui and crossterm
- File operations with walkdir and glob
- Regex search with ripgrep integration
- Secure random with rand
- Date/time handling with chrono

### Dependencies
- Rust 1.70 or later
- X/Grok API key from x.ai
- Internet connection (with resilience for Starlink)

### Installation
- Source build with Cargo
- Windows, Linux, and macOS support
- Optional PATH configuration

### Known Issues
- None at this time

### Security
- API keys stored securely in config or environment
- Shell command execution requires trusted directory approval
- Localhost-only binding for ACP server by default
- Policy engine for tool execution control

---

## Release Notes

### Version 0.1.1
This is a critical bug fix release that resolves Zed editor integration issues. The "failed to deserialize response" error was caused by two separate issues:

1. **Command-line parsing**: Missing Clap attributes causing argument parsing failures
2. **Protocol serialization**: Field name and type mismatches between grok-cli and Zed's expectations

Both issues have been completely resolved. All users experiencing integration errors should upgrade immediately. The fixes are backward compatible with no breaking changes to existing functionality.

### Version 0.1.0
This is the initial public release of Grok CLI, featuring a complete implementation of the Agent Client Protocol for Zed editor integration, beautiful terminal UI, and comprehensive AI capabilities powered by X/Grok API.

---

## Upgrade Guide

### From 0.1.0 to 0.1.1

1. Pull latest changes from repository
2. Rebuild: `cargo clean && cargo build --release`
3. No configuration changes required
4. Test with: `grok config init --force`
5. Verify Zed integration: `grok acp capabilities`

No breaking changes - all existing configurations remain compatible.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details on:
- How to report issues
- How to submit pull requests
- Code style guidelines
- Testing requirements

---

## Links

- **Repository**: https://github.com/microtech/grok-cli
- **Issues**: https://github.com/microtech/grok-cli/issues
- **Discussions**: https://github.com/microtech/grok-cli/discussions
- **Author**: John McConnell (john.microtech@gmail.com)
- **License**: MIT

---

**Maintained by**: John McConnell  
**Project Start**: 2024  
**Current Version**: 0.1.1