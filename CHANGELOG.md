# Changelog

All notable changes to the Grok CLI project are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

Author: John McConnell <john.microtech@gmail.com>
Repository: https://github.com/microtech/grok-cli
Buy me a coffee: https://buymeacoffee.com/micro.tech

---

## [0.1.7-pre] - 2026-03-14

### Fixed

- **Slash commands broken after grok restart** (`src/acp/mod.rs`, `src/cli/commands/acp.rs`)
  - **Root cause**: When grok-cli restarts, Zed re-uses the session ID from the previous
    connection. The agent had no record of that session, so every `session/prompt` returned
    `"Session not found: <id>"` and all slash commands silently failed.
  - **Fix**: Added `GrokAcpAgent::session_exists()` helper. In `handle_json_rpc`, when a
    `session/prompt` arrives with an unknown session ID the agent now auto-creates a fresh
    session under that ID and immediately re-sends `available_commands_update` so the client's
    command palette is repopulated.
  - Source: AI (Claude Sonnet 4.6)

- **"Loading or resuming sessions is not supported by this agent." message in Zed** (`src/acp/protocol.rs`, `src/cli/commands/acp.rs`)
  - **Root cause**: The `initialize` response did not advertise `loadSession: true` or
    `sessionCapabilities.list: {}`. Zed checks for these fields at startup and displays the
    "not supported" banner when they are absent.
  - **Fix**: `AgentCapabilities` now sets `loadSession: true` and `SessionCapabilities` now
    includes `list: {}` by default.
  - Implemented `session/list` handler — returns the currently active in-memory sessions
    (empty list on a fresh start). Registered the new `list_sessions()` helper on
    `GrokAcpAgent`.
  - Implemented `session/load` handler — re-registers the workspace root as trusted,
    re-creates the session in memory if it no longer exists, re-sends
    `available_commands_update`, and responds with `null` (no history to replay since
    grok-cli does not persist conversations across restarts). This satisfies the ACP spec
    and suppresses Zed's warning banner.
  - Source: AI (Claude Sonnet 4.6)

### Changed

- **AI-assisted slash commands now forward tool-call updates to the client**
  (`src/cli/commands/acp.rs`)
  - Previously, AI-powered slash commands (`/web`, `/explain`, `/review`, `/plan`, `/test`,
    `/fix`) called `handle_chat_completion` with `event_sender = None`. This meant Zed saw
    no activity while the model was running tools (e.g. `web_search`) and could appear to
    hang.
  - These commands now go through the same `tokio::select!` loop as normal chat prompts,
    forwarding `ToolCall` / `ToolCallUpdate` notifications, permission requests, and client
    messages (including `session/cancel`) in real time.
  - Source: AI (Claude Sonnet 4.6)

---

## [0.1.6] - 2026-03-11

### Fixed

- **ACP permission outcome wire format fix** (`src/acp/protocol.rs`)
  - **Root cause**: `OutcomeDetail::Selected { option_id }` was serializing to `{"outcome":"selected","option_id":"..."}` (snake_case) instead of `{"outcome":"selected","optionId":"..."}` (camelCase) as required by the ACP spec.
  - Serde's `rename_all = "camelCase"` at the **enum** level only renames variant names, not fields inside struct variants. The field needed an explicit `#[serde(rename = "optionId")]` annotation.
  - This was a silent bug: the agent correctly sent `session/request_permission` requests, but when a client echoed back `{"optionId":"proceed_always"}` the agent could not deserialize it, causing every "Always Allow" permission response to fall through to the cancel path.
  - Fixed by adding `#[serde(rename = "optionId")]` to the `option_id` field in `OutcomeDetail::Selected`.
  - All 132 unit + integration tests pass; Clippy reports zero warnings. (Source: AI)

- **ACP file-reading broken in Zed** (`src/cli/commands/acp.rs`, `src/config/mod.rs`, `src/acp/protocol.rs`)
  - **Root cause 1 — Permission gate silently blocked all tools**: `acp.require_permission` defaulted to
    `true`, causing the agent to send a `session/request_permission` JSON-RPC request to Zed before every
    tool call.  Zed does not implement this method and returns a JSON-RPC error response; the agent was
    treating that error as a user "cancel", injecting `"User rejected the tool execution."` into every
    tool result and preventing any file read or directory listing from completing.
    - Changed `acp.require_permission` default to `false` (matches the documented intent for clients that
      don't yet support the permission dialog).
    - Updated `.grok/config.toml` to explicitly set `require_permission = false` with an explanatory
      comment.
    - When a client returns a JSON-RPC error for `session/request_permission`, the agent now auto-approves
      the tool call (`proceed_once`) instead of silently cancelling it, and logs a `WARN` suggesting the
      config flag.
    - Added `PermissionOutcome::proceed_once()` convenience constructor (mirrors the existing `::cancel()`).
    - Both the `handle_session_prompt` select-loop path and the `handle_json_rpc` outer-loop path received
      the same fix so behaviour is consistent regardless of when the response arrives.
    - Permission-response matching now accepts both string and numeric JSON-RPC response IDs for broader
      client compatibility.
  - **Root cause 2 — Windows `file:///` URI mis-parsed as UNC path**: `resolve_workspace_path` stripped
    only 7 bytes from `file:///H:/GitHub/project` (removing `file://`, leaving `/H:/…`).  After replacing
    `/` with `\` on Windows the result was `\H:\…`, which Windows treats as a UNC path prefix.
    `PathBuf::canonicalize()` failed, the fallback path was never added to the trusted-directory list, and
    every subsequent file access for that workspace was denied.
    - The Windows normalisation block now also detects the `\X:\path` pattern (backslash + drive-letter +
      colon) produced by decoding a Windows file URI and strips the leading backslash → `X:\path`.
    - Git-bash / WSL `\x\path` → `X:\path` conversion is preserved as before.

### Added

- **ACP Gemini-style permission UI** (`src/acp/mod.rs`, `src/cli/commands/acp.rs`, `src/config/mod.rs`)
  - Implements the interactive `session/request_permission` RPC as specified in the ACP protocol.
  - The agent now pauses before every tool execution to request explicit user permission via the client (e.g. Zed).
  - Three outcome options are supported:
    - **Proceed Once**: Executes the current tool call; subsequent calls for the same tool will prompt again.
    - **Proceed Always**: Executes the current tool call and adds the tool to an `"always_allow"` set for the duration of the session, suppressing future prompts for that specific tool.
    - **Cancel**: Rejects the tool execution; the agent receives a failure message and continues its loop gracefully.
  - **Non-blocking Bidirectional Communication**: Refactored the ACP session handler to use a background reader task, allowing the agent to wait for user permission without deadlocking the JSON-RPC stream.
  - **New Configuration Flags**:
    - `acp.require_permission` (default: `true`): Enable or disable the permission gate.
    - `acp.permission_timeout_secs` (default: `60`): How long to wait for a user response before failing the tool call.
  - **Resilience**: Automatically cancels pending permissions on network drops or IO errors, preventing the agent from hanging.
  - Comprehensive unit and integration tests covering all permission outcomes and timeout scenarios.
  - Source: AI (Claude Sonnet 4.6) — implemented as Task #29 and #30 in the `.zed/task_list.json`.

---

## [0.1.61-pre] - 2026-03-06

### Added

- **ACP Slash Commands** (`src/acp/slash_commands.rs`, `src/acp/protocol.rs`, `src/cli/commands/acp.rs`)
  - Implements the ACP `available_commands_update` session notification as specified at
    <https://agentclientprotocol.com/protocol/slash-commands>.
  - After every `session/new` the agent automatically sends an
    `available_commands_update` notification so clients (e.g. Zed) can populate
    their `/` command palette with Grok's capabilities.
  - **Ten slash commands** are advertised and handled:
    | Command | Type | Description |
    |---------|------|-------------|
    | `/help` | built-in | List all available commands and usage |
    | `/web <query>` | AI-assisted | Research a topic / search the web |
    | `/explain [subject]` | AI-assisted | Thorough explanation of code or a concept |
    | `/review [target]` | AI-assisted | Comprehensive code review (bugs, security, performance, style) |
    | `/plan <description>` | AI-assisted | Detailed step-by-step implementation plan |
    | `/test [target]` | AI-assisted | Write, run, or debug tests |
    | `/fix [problem]` | AI-assisted | Diagnose and fix a bug or error |
    | `/model [name]` | built-in | Switch the active Grok model; lists available models if no name given |
    | `/clear` | built-in | Wipe conversation history for the current session |
    | `/context` | built-in | Show session ID, model, temperature, token limit, and message count |
  - **Built-in commands** (`/help`, `/clear`, `/model`, `/context`) are resolved
    entirely on the agent side with zero AI round-trips.
  - **AI-assisted commands** rewrite the raw `/command text` into a structured,
    richly-instructed prompt before forwarding to the Grok API, resulting in
    more focused and complete model responses.
  - New protocol types added to `src/acp/protocol.rs`:
    `AvailableCommandInput`, `AvailableCommand`, `AvailableCommandsUpdate`,
    and a new `SessionUpdate::AvailableCommandsUpdate` variant.
  - New session helpers on `GrokAcpAgent`: `clear_session_history`,
    `get_session_config`, `get_session_message_count`, `set_session_model`.
  - 17 unit tests covering the parser, prompt builder, builtin dispatcher, and
    formatting helpers — all passing.
  - Source: AI (Claude Sonnet 4.6) — triggered by user request to implement ACP
    slash-command advertisement as specified in the ACP protocol documentation.

- **Hooks settings exposed in `/settings` and `/hooks` command wired (Task 26)**
  - `tools.enable_hooks` is now visible and editable in the **Tools** category
    of the `/settings` menu. Toggling it to `true` activates before/after
    tool-call hook execution; the `/hooks` command immediately reflects the
    change.
  - Three new **Experimental** settings surface the extensions subsystem that
    powers custom hooks:
    - `experimental.extensions.enabled` — master toggle for loading extensions.
    - `experimental.extensions.extension_dir` — path to the extensions folder
      (defaults to `~/.grok/extensions` when left blank).
    - `experimental.extensions.enabled_extensions` — comma-separated list of
      extension names to load on startup.
  - `get_value()` and `set_value()` in `src/config/mod.rs` now handle all four
    new keys so that `grok config set tools.enable_hooks true` (and the
    equivalent extension keys) round-trip correctly through the config layer.
  - Created `.zed/task_list.json` as the canonical task-tracking file going
    forward; Task 26 is recorded there with all five subtasks marked **done**.
  - Source: AI (Claude Sonnet 4.6) — triggered by missing hooks/settings
    entries reported by user.
- **ACP Workspace Initialization**: Automatically reads workspace directory when ACP session starts
  - When started in ACP mode with workspace root, grok-cli now automatically reads the top-level directory
  - Directory contents are logged to the session for immediate context awareness
  - AI agent has project structure information from the first interaction
  - Uses existing security policy to ensure only trusted directories are accessed
  - Non-breaking: directory reading failure logs warning but doesn't prevent session initialization
  - Improves initial AI responses by providing project context upfront

- **Context Discovery Enhancement**: Context files now walk up directory tree to find project root
  - Context discovery now matches configuration discovery behavior
  - Works from any subdirectory within a project
  - Automatically finds project root by detecting `.git`, `Cargo.toml`, `package.json`, or `.grok/`
  - No longer requires running grok from project root for context loading
  - Applies to all context file types: `.zed/rules`, `.grok/context.md`, `GEMINI.md`, etc.
  - Created PROJECT_CONTEXT_GUIDE.md (560 lines) - comprehensive guide to context and config discovery

- **Context file display improvements in session startup info (Task 25)**
  - Context files now show their **full absolute path** (e.g.
    `H:\GitHub\grok-cli\context.md`) instead of just the bare filename.
    This makes it immediately clear which file on disk was loaded, especially
    useful when multiple context sources (project + global `~/.grok`) are
    active at the same time.
  - When `ui.hide_context_summary` is `false` (the default), the first three
    non-empty lines of each context file are printed as a dimmed preview
    directly beneath the path. Lines longer than 80 characters are truncated.
    Set `ui.hide_context_summary = true` in your config to suppress the preview.
  - Load confirmation messages emitted by `load_project_context_for_session`
    also now show full paths instead of bare filenames.
  - Source: AI (Claude Sonnet 4.6) — triggered by user feedback that filename-
    only display made it impossible to tell which `context.md` was loaded.

- **`grok acp stdio --workspace <path>` flag for explicit project root**
  - Zed (and other ACP clients) sometimes launch the `grok` binary from the
    user's home directory rather than the project root, causing every file
    access to be denied. The new `--workspace` flag lets you tell grok exactly
    which directory to trust at startup — before any protocol messages arrive.
  - In your Zed agent settings, pass `--workspace ${workspaceFolder}` and Zed
    will substitute the open project's root automatically.
  - Two environment-variable fallbacks are also checked (in order):
    1. `GROK_WORKSPACE_ROOT` — grok-specific override
    2. `WORKSPACE_ROOT` — generic convention used by some CI systems
  - Example Zed agent config (`~/.config/zed/settings.json`):
    ```json
    {
      "agent": {
        "command": "grok",
        "args": ["acp", "stdio", "--workspace", "${workspaceFolder}"]
      }
    }
    ```
  - At startup grok now logs the CWD (or the explicit workspace root) to
    `tracing` at INFO level so it is always clear which directory is trusted.

### Fixed

- **ACP Mode — Cross-project file access denied when using Zed resource links**
  - **Root cause:** When Grok is launched as an ACP server for project A but the
    user @-mentions files from project B in Zed, project B's directory was never
    added to the trusted paths — only the directory where `grok` was started was
    trusted. Every `read_file` / `list_directory` call for project B would return
    "Access denied: External access is disabled in configuration".
  - **Fix (`src/cli/commands/acp.rs`):** `handle_session_prompt` now inspects
    every `ResourceLink` and `Resource` block in the incoming `session/prompt`
    message. For each `file://` URI it finds, it calls the new
    `trust_workspace_from_uri` helper which:
    1. Decodes the URI using the existing `resolve_workspace_path` logic
       (handles `file://`, forward-slash Windows paths, Git-bash paths, etc.)
    2. Walks up the directory tree from the resolved path looking for common
       project-root markers (`.git`, `Cargo.toml`, `package.json`, `.grok`, etc.)
       via the new `find_workspace_root_from_path` helper
    3. Registers the discovered workspace root as a trusted directory so all
       subsequent `read_file` / `list_directory` / `glob_search` calls for that
       project succeed without requiring external-access config changes
  - **Fix (`src/acp/security.rs`):** `validate_path_access` now includes a
    detailed diagnostic when access is denied — showing the resolved path,
    the full list of currently-trusted directories, and a tip on how to fix it.
    This replaces the terse "Access denied: …" message that gave the AI model
    nothing useful to tell the user.

- **ACP Mode — "Request timeout after 30 seconds" — root cause diagnosed and mitigated**
  - **Root cause 1 (grok_api crate bug):** `grok_api ≤ 0.1.2` hardcodes the
    literal `30` in its `from_reqwest` error formatter regardless of the actual
    configured `timeout_secs`. The message "Request timeout after 30 seconds"
    is therefore always misleading — the real HTTP timeout driving the request
    is `config.timeout_secs` (default 300 s). This is a bug in the upstream
    crate and cannot be fixed without a crate update or fork.
  - **Root cause 2 (connect_timeout config is dead code):** `NetworkConfig.
    connect_timeout` is read from `.grok/config.toml` but was never passed to
    the `grok_api` HTTP client. The crate hardcodes `connect_timeout(10 s)`
    internally. Changing `connect_timeout` in config had zero effect on API
    calls. Added prominent warning comments in config to prevent confusion.
  - **Root cause 3 (retry delays too short for Starlink):** ACP retry backoff
    was `2 → 4 → 8 s` over 3 attempts — far too short for a Starlink satellite
    handover which can take 20–60 s to recover.

- **ACP retry logic hardened for Starlink satellite drops**
  - `MAX_API_RETRIES` raised from **3 → 5** in `handle_chat_completion`
  - `BASE_RETRY_DELAY_SECS` raised from **2 s → 5 s**; delays now follow
    `5 → 10 → 20 → 40 → 60 s` (capped at 60 s via `MAX_RETRY_DELAY_SECS`)
  - Total maximum wait before giving up: **~135 s** vs the previous **~14 s**
  - Retry log now labels each failure as `TIMEOUT` or `NETWORK DROP` and
    prints `real_timeout=Ns` so it is clear which configured timeout applies
  - Error message when all retries are exhausted now includes a diagnostic tip
    explaining the grok_api "30 seconds" bug and suggesting `timeout_secs` as
    the knob to adjust

- **`.grok/config.toml` — explicit timeout settings added**
  - `timeout_secs = 300` and `max_retries = 5` now appear explicitly at the
    top of the project config so they are visible and easy to tune
  - `[network]` section added with `connect_timeout`, `read_timeout`, and
    Starlink-specific retry parameters
  - Every timeout field annotated with comments explaining what it controls,
    its environment-variable override, and the grok_api crate limitations

---

## [0.1.5] - 2026-02-28

### Fixed

- **ACP Workspace Access — Project root always accessible from startup**
  - `SecurityPolicy::new()` and `with_working_directory()` now pre-populate
    `trusted_directories` with the CWD at construction time so the project root
    is trusted before any `session/new` or `initialize` message arrives
  - Fixed silent data loss: if `canonicalize()` failed the workspace root was
    silently discarded; now a normalised-but-un-canonicalized path is used as
    fallback so the directory is always registered
  - Added robust `resolve_workspace_path()` helper that handles every path
    format Zed and other ACP clients may send:
    - `file:///H:/GitHub/project` — `file://` URI scheme (URL-decoded)
    - `H:/GitHub/project` — Windows path with forward slashes
    - `/h/GitHub/project` — Git-bash / WSL style path on Windows
    - `/home/user/project` — standard Unix path
  - `InitializeRequest` now parses `workspaceRoot`, `workspace_root`,
    `rootUri`, and `rootPath` fields so clients that send the project root
    during `initialize` (before `session/new`) are handled correctly
  - `handle_initialize` now calls `register_workspace_root()` immediately
  - `handle_session_new` falls back to re-trusting the CWD when no workspace
    root is provided
  - Renamed test `test_empty_trusted_directories` →
    `test_working_directory_auto_trusted` to reflect the corrected behaviour
  - Added `test_path_outside_working_directory_not_auto_trusted` to confirm
    untrusted directories remain blocked

### Added

- **Skill Auto-Activation Engine** (`src/skills/auto_activate.rs`)
  - Skills now activate automatically based on conversation context — no
    manual `/activate` required
  - Three trigger types declared in `SKILL.md` frontmatter:
    - **Keywords** — case-insensitive word/phrase matches (`"rust"`, `"cargo"`)
    - **Regex patterns** — full Rust `regex` patterns on the user message
      (e.g. `fn\s+\w+`)
    - **File extensions** — activate when the project contains matching file
      types (e.g. `.rs`, `.py`)
  - Confidence scoring: keywords +30 pts, patterns +40 pts, file extensions
    +25 pts, capped at 100; per-skill `min_confidence` threshold (default 50)
  - New `auto-activate` YAML frontmatter block in `SKILL.md`
  - New `/auto-skills [on|off]` interactive command to toggle globally
  - Security validation runs before every auto-activation
  - Already-active skills are never suggested twice in the same session
  - `InteractiveSession` gains `auto_skills_enabled: bool` (serialized,
    default `true`) — persists across `/save` and `/load`
  - New types: `AutoActivateConfig`, `AutoActivationEngine`, `SkillMatch`
  - 11 new unit tests covering all trigger paths, scoring, thresholding,
    sort order, case-insensitivity, and invalid-regex safety

- **`/hooks` command in interactive mode**
  - Added missing `/hooks` command handler in `handle_special_commands`
  - `print_hooks_info()` displays hooks system status and configuration
  - `list_hooks()` and `hook_count()` methods added to `HookManager` API
  - Shows hooks enable status, extensions config, and usage tips
  - Help menu updated to include `/hooks`

- **Dynamic Skill Builder v2.0** — create and activate custom skills on-the-fly
  - Complete rewrite with dynamic skill creation capabilities
  - Create skills from natural language descriptions or structured YAML/JSON
  - Interactive step-by-step guided skill building
  - Clone and extend existing skills with automatic adaptation
  - Immediate activation in current session without restart
  - Security validation with automatic tool permission checking
  - Four creation modes: Natural Language, Specification, Interactive, Template
  - `SKILL_SPEC.md` format with validation rules and examples

### Changed

- **Installer updated to v0.1.5** across all components
- Version bumped in `Cargo.toml`, `package.json`, `src/bin/installer.rs`,
  and `README.md`
- All 110 unit tests passing

---

## [0.1.42] - 2026-02-20

### Added

- **Configurable External Directory Access** — full implementation of secure
  read-only access to files outside the project boundary
  - `ExternalAccessConfig` struct in `src/config/mod.rs` with env var support:
    `GROK_EXTERNAL_ACCESS_ENABLED`, `GROK_EXTERNAL_ACCESS_PATHS`, etc.
  - 13 default excluded patterns protect sensitive files
    (`.env`, `.ssh/`, keys, credentials, etc.)
  - Three-tier path validation: Internal / External / ExternalRequiresApproval
  - Interactive approval UI (`src/cli/approval.rs`) with styled terminal
    prompts: Allow Once, Trust Always, Deny, View Path
  - Complete audit trail in JSONL format at `~/.grok/audit/external_access.jsonl`
  - `grok config validate-external-access` command to verify configuration
  - `grok audit external-access` command with `--summary`, `--from`, `--to`,
    `--path`, and `--export` (CSV) flags
  - Session-based trusted paths for "Trust Always" decisions
  - Windows installer now creates `~/.grok/audit/` directory automatically

- **Shared `GrokClient` initializer** — `initialize_client()` utility to
  eliminate duplicated client setup across commands

- **File-backup-hook extension** — sample hook and documentation showing
  how to auto-backup files before write operations

- **Enhanced installer config template** — added `[external_access]`,
  `[network]`, `[logging]`, and `[security]` sections with all v0.1.42
  defaults pre-filled

### Fixed

- `audit.rs` — fixed compile error causing `cargo test` failures
- Windows installer — fixed old binary not being removed before replacement

### Changed

- `health` command refactored to use shared `initialize_client()` helper
- Updated project context documentation and Grok config defaults
- Expanded documentation installed by the Windows installer:
  `EXTERNAL_FILE_ACCESS_SUMMARY.md`, `EXTERNAL_FILE_REFERENCE.md`,
  `PROPOSAL_EXTERNAL_ACCESS.md`, `TROUBLESHOOTING_TOOL_LOOPS.md`,
  `SYSTEM_CONFIG_NOTES.md`, `CONTRIBUTING.md`

---

## [0.1.41] - 2026-02-18

### Added

- **Native tool message support** via `grok_api` v0.1.2
  - Replaced user-message workaround with native `role: "tool"` +
    `tool_call_id` field
  - Improves compatibility with Grok API's expected message format
  - Eliminates tool results appearing as user messages

- **`finish_reason` support** — chat completion loop now correctly handles
  `"stop"` and `"end_turn"` finish reasons to break the tool loop early

- **Tool loop diagnostics and configurable iteration limit**
  - `acp.max_tool_loop_iterations` config key (default 10)
  - `Doc/TROUBLESHOOTING_TOOL_LOOPS.md` — guide for diagnosing and fixing
    runaway tool loops; includes good vs bad prompt examples
  - `Doc/SYSTEM_CONFIG_NOTES.md` — explains config hierarchy and priority
  - `analyze_tool_loops.ps1` PowerShell script to parse debug logs
  - `test_tool_loop_debug.sh` bash script to reproduce loop scenarios

### Changed

- `grok_api` dependency updated to v0.1.2 from crates.io
- Deprecated `.grok/` docs removed; documentation moved to `Doc/`
- Hierarchical config loading improved — project → system → defaults cascade
  more reliably
- Config display updated with current defaults
- `fix_config_syntax.ps1` script added to repair malformed TOML configs
- MCP server configuration syntax fixed: `env = {}` is now required even
  when empty; comprehensive examples added to `config.example.toml`

---

## [0.1.4] - 2026-02-15

### Added

- **macOS Apple Silicon (aarch64) support** — CI now builds and packages
  `aarch64-apple-darwin` binaries in the release workflow

- **Agent Skills System** — progressive skill loading with session-level
  activation/deactivation
  - Skills stored as directories under `~/.grok/skills/<name>/SKILL.md`
  - YAML frontmatter: `name`, `description`, `license`, `allowed-tools`,
    `compatibility`, `metadata`
  - `grok skills list` — list all available skills
  - `grok skills show <name>` — display skill details and instructions
  - `grok skills new <name>` — scaffold a new skill from template
  - `grok skills validate <name>` — security scan with four levels:
    Safe / Warning / Suspicious / Dangerous
  - `/skills`, `/activate <name>`, `/deactivate <name>` interactive commands
  - Skills injected into system prompt when active (zero token cost when
    inactive)
  - `SkillSecurityValidator` — detects dangerous shell patterns, prompt
    injection, encoded payloads, and restricts tool permissions

- **Web tools** — `web_search` and `web_fetch` enabled in tool execution
  - Switched from Google Search API to DuckDuckGo (no API key required)
  - DuckDuckGo fallback with graceful degradation on failures
  - Detailed error messages included in tool failure responses
  - `read_multiple_files` — read several files in a single tool call
  - `list_code_definitions` — list functions/types in a source file

- **Improved context discovery** — context loader now walks up to the
  project root to find `.grok/context.md`, `GEMINI.md`, `.claude.md`,
  `.zed/rules`, and other context files

- **Windows installer enhancements**
  - Bundled documentation installed to `~/.grok/docs/`
  - Extended config template with network, logging, and security sections
  - Cleanup scripts for removing old `grok` installations

- **Async tool execution** — all tool handlers are now `async`, enabling
  concurrent web requests without blocking the runtime

### Fixed

- MCP client restored after crash; MCP configuration docs added
- Old grok binary correctly removed before replacement on Windows
- Web search errors now include full error details for diagnosis
- Project root markers added to all integration tests to prevent false
  path-trust failures

### Changed

- `max_tool_loop_iterations` made configurable (was hard-coded)
- Release workflow refactored to produce clean per-platform artifacts
- Obsolete documentation removed; `Doc/` established as canonical docs dir
- Network module updated with improved retry logic and timeout handling

---

## [0.1.3] - 2026-02-05

### Added

- **GitHub Actions release workflow** — builds Windows (x86_64), Linux
  (x86_64), and macOS (x86_64) binaries on every tagged release
- **Binary-only terminal module** (`src/terminal/`) — isolates `crossterm`
  / `ratatui` code into the binary crate to avoid duplicate compilation
- `grok` shell wrapper and `install.sh` install script for Unix systems

### Fixed

- `grok_api` pinned to v0.1.0 with compatibility shims to stabilise the
  build while the upstream crate API stabilises
- CI updated to stable Rust toolchain (was using beta)
- Ubuntu CI: added `libssl-dev` and other native dependencies
- Unused lint warnings demoted to allow in Cargo.toml to keep CI green

### Changed

- Project renamed to `grok-cli-acp` in `package.json` to reflect the
  ACP-first focus
- Documentation reorganised: some files moved to `Doc/`
- Release workflow updated to build artifacts from `target/release/` and
  produce correct archive names per platform
- Env parsing and imports refactored for cleaner module boundaries

---

## [0.1.2] - 2026-01-25

### Added — Initial Public Release

This is the bootstrap release that established the full project structure.

#### Core CLI
- `grok chat` — single-shot and interactive chat with Grok AI
- `grok query` — quick one-liner query mode
- `grok interactive` — full interactive REPL (default when no subcommand)
- `grok code` — code explain, review, and generate subcommands
- `grok health` — API connectivity and config diagnostic checks
- `grok config` — configuration management (show, set, validate)
- `grok settings` — live settings display and editing
- `grok history` — browse and replay past chat sessions

#### ACP / Zed Integration
- `grok acp stdio` — ACP server over stdin/stdout for Zed editor
- `grok acp server` — TCP ACP server mode
- `grok acp test` — connectivity test against a running ACP server
- `grok acp capabilities` — show agent capabilities JSON
- Full JSON-RPC protocol: `initialize`, `session/new`, `session/prompt`
- Session management with configurable temperature, tokens, and model

#### Agent Tools
- `read_file` — read file content with security policy enforcement
- `write_file` — write file content (trusted directories only)
- `list_directory` — list directory contents
- `replace` — targeted text replacement in files
- `glob_search` — find files by glob pattern
- `search_file_content` — regex search across files (ripgrep-style)
- `run_shell_command` — execute shell commands with approval mode
- `save_memory` — persist facts to `~/.grok/memory.md`
- `web_search` — search the web (Google Search API, later DuckDuckGo)
- `web_fetch` — fetch and return URL content as text

#### Security
- `SecurityPolicy` with trusted-directory allow-list (deny by default)
- Shell command approval modes: `prompt`, `auto`, `yolo`
- Path canonicalization to prevent symlink escapes
- Environment variable isolation for API keys

#### Configuration
- Three-tier hierarchical config: project (`.grok/config.toml`) →
  system (`~/.grok/config.toml`) → built-in defaults
- Full `config.toml` / `.env` support with environment variable overrides
- Configurable model, temperature, max tokens, timeout, retries, rate limits
- MCP (Model Context Protocol) client configuration
- Telemetry (opt-in, local only)

#### Context & Session
- Auto-loads `.grok/context.md`, `GEMINI.md`, `.claude.md`, `.zed/rules`
  and injects them into the system prompt
- Session persistence — `/save <name>`, `/load <name>`, `/list`
- Chat logging to `~/.grok/logs/chat_sessions/` in JSON and plain-text

#### Interactive Mode
- Rich prompt with model name, directory, and context-usage indicator
- Tab-completion and command suggestions
- `/help`, `/clear`, `/model`, `/system`, `/tools`, `/status`, `/reset`,
  `/history`, `/version`, `/config`, `/settings`, `/hooks` commands
- Shell passthrough via `!<command>` prefix
- Welcome banner with tips, session info, and directory warnings

#### Network (Starlink-optimised)
- Exponential backoff retry: 2 s → 4 s → 8 s, capped at 60 s
- Per-request timeout with configurable `timeout_secs`
- Network connectivity test (`grok test-network`)
- `install.js` npm installer with async retry logic for unreliable links

#### Platform
- Windows 11 native binary (`grok.exe`) with Windows installer
- Linux x86_64 binary
- macOS x86_64 binary (aarch64 added in v0.1.4)
- MCP GitHub integration server (`github_mcp` binary)

#### Documentation (shipped with binary)
- `README.md` — full feature overview and quickstart
- `CONFIGURATION.md` — all config keys with defaults and examples
- `CONTRIBUTING.md` — contribution guidelines
- `docs/` — API reference, interactive mode guide, tool reference,
  Zed integration guide, extensions guide, settings reference
- `.env.example` and `.grok/.env.example` — annotated environment templates

---

## Links

- **Repository**: https://github.com/microtech/grok-cli
- **Issues**: https://github.com/microtech/grok-cli/issues
- **Buy Me a Coffee**: https://buymeacoffee.com/micro.tech