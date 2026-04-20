# Grok CLI — Project Layout

> **Version:** 0.1.9-pre | **Author:** John McConnell <john.microtech@gmail.com>
> **Repository:** https://github.com/micro-tech/grok-cli | **Last updated:** 2025

---

## Table of Contents

1. [Root Directory](#1-root-directory)
2. [Source Tree (`src/`)](#2-source-tree-src)
3. [Integration Tests (`tests/`)](#3-integration-tests-tests)
4. [Documentation (`docs/` and `Doc/`)](#4-documentation-docs-and-doc)
5. [Scripts, Examples, and Support Directories](#5-scripts-examples-and-support-directories)
6. [Architecture Layers](#6-architecture-layers)
7. [Key Integration Points](#7-key-integration-points)
8. [Development Notes](#8-development-notes)

---

## 1. Root Directory

The root of the workspace contains the Cargo manifest, all top-level documentation files,
configuration templates, and supporting tooling metadata.

### Root Files

| File / Directory                   | Description                                                               |
|------------------------------------|---------------------------------------------------------------------------|
| `.gitignore`                       | Git exclusion rules (target/, .env, .zed/, .grok/, Cargo.lock, etc.)     |
| `.zed/`                            | Zed editor config and AI task list (`task_list.json` — 101 tasks)        |
| `.grok/`                           | Project-local Grok CLI config and memory cache (git-ignored)             |
| `Cargo.toml`                       | Workspace manifest — package metadata, all dependencies, binary targets   |
| `Cargo.lock`                       | Pinned dependency versions for reproducible builds                        |
| `README.md`                        | Project overview, feature guide, and quick-start instructions             |
| `CHANGELOG.md`                     | Version history, release notes, and change rationale                      |
| `CONFIGURATION.md`                 | Full configuration reference (all keys, types, defaults)                  |
| `CONTRIBUTING.md`                  | Contribution guidelines, PR process, and code style expectations          |
| `FIXES.md`                         | Documented known fixes, workarounds, and patched regressions              |
| `SETUP.md`                         | Installation and first-run setup instructions                             |
| `TESTING_TOOLS.md`                 | Guide for running and writing tests                                       |
| `TROUBLESHOOTING.md`               | Common problems and resolution steps                                      |
| `SKILL_BUILDER_ANNOUNCEMENT.md`    | Announcement document for the skills system launch                        |
| `config.example.toml`             | Annotated example configuration file (copy to `~/.config/grok/config.toml`) |
| `context.md`                       | Rust best practices, coding conventions, and project-level context notes  |
| `dataflow_map.md`                  | Visual data-flow diagram across all major subsystems                      |
| `plan.md`                          | High-level project plan (stub; see `.zed/task_list.json` for active tasks)|
| `project_layout.md`                | **THIS FILE** — definitive project directory and module map               |
| `icon.svg`                         | Project icon (used in npm package and GitHub metadata)                    |
| `install.js`                       | Node.js install helper for npm-based distribution                         |
| `package.json`                     | npm package metadata for Node.js distribution wrapper                     |
| `package-lock.json`                | Locked npm dependency versions                                            |
| `LICENSE`                          | Project license file                                                      |
| `settings.rs`                      | Stray settings module at root (pending relocation to `src/`)              |
| `out.txt`                          | Scratch/build output capture (not committed to source control)            |
| `test_acp_output.json`             | ACP protocol test fixture / captured output                               |

### Root Directories

| Directory       | Description                                                                  |
|-----------------|------------------------------------------------------------------------------|
| `bin/`          | Node.js binary shim (`grok.js`) for npm-based invocation on Windows/macOS   |
| `config/`       | Additional loose config files (`mod.rs` — may be merged into `src/config/`) |
| `docs/`         | Technical/architecture reference documents (internal, developer-facing)      |
| `Doc/`          | User-facing documentation, quick-starts, and command references              |
| `examples/`     | Runnable usage examples for skills and extension hooks                       |
| `scripts/`      | PowerShell, Bash, and Python build/test/release utility scripts              |
| `src/`          | Rust library and binary source code (see Section 2)                          |
| `tests/`        | Rust integration tests (see Section 3)                                       |
| `target/`       | Cargo build output (git-ignored)                                             |
| `node_modules/` | npm dependencies (git-ignored)                                               |

---

## 2. Source Tree (`src/`)

The `src/` directory contains the full Rust codebase, organized into a flat-module
architecture declared in `src/lib.rs`. All modules are re-exported from the library
crate; `src/main.rs` provides the thin binary entry point.

### Top-Level Source Files

| File                   | Description                                                                   |
|------------------------|-------------------------------------------------------------------------------|
| `src/lib.rs`           | Crate root — `pub mod` declarations for every module; public API surface      |
| `src/main.rs`          | Binary entry point — bootstraps tokio runtime, loads `.env`, calls `cli::run` |
| `src/grok_client_ext.rs` | Extended `GrokClient` wrapper with retry logic and Starlink-resilient timeouts |

---

### `src/acp/` — Agent Client Protocol

The `acp` module implements the Agent Client Protocol, which is grok-cli's integration
layer with Zed editor's AI extension API. It manages JSON-RPC sessions over stdin/stdout,
enforces path-level security on all file access requests, handles the full set of
slash-command interactions (`/help`, `/tools`, `/clear`, etc.), and exposes a thin shim
that re-exports tool implementations from `src/tools/`. This is the primary entry point
when grok-cli is invoked as a Zed context-server process.

| File                 | Description                                                              |
|----------------------|--------------------------------------------------------------------------|
| `mod.rs`             | ACP session handler — message dispatch loop, session lifecycle           |
| `protocol.rs`        | ACP message types, JSON-RPC envelope serialization/deserialization       |
| `security.rs`        | Path security enforcement — sandboxing allowed read/write directories    |
| `slash_commands.rs`  | Handler logic for `/help`, `/tools`, `/clear`, and other slash commands  |
| `tools.rs`           | Thin re-export shim routing ACP tool calls into `src/tools/`             |

**Integration:** Depends on `src/tools/` (tool dispatch), `src/security/` (path policy),
and `src/router/` (forwarding chat completions to the xAI API).

---

### `src/agent/` — Agent Routing and Simulation

The `agent` module provides higher-level agent orchestration logic sitting above the raw
router. It classifies the current operational mode, routes parsed intents to appropriate
action handlers, and runs pre-execution plan simulations with risk analysis to surface
potentially destructive operations before they run. This module acts as the "middle brain"
between the CLI's command parsing and the low-level API router.

| File          | Description                                                                    |
|---------------|--------------------------------------------------------------------------------|
| `mod.rs`      | Module root — re-exports `AgentRouter`, `AgentSimulator`, `AgentMode`          |
| `mode.rs`     | `AgentMode` enum — operational mode detection (interactive, autonomous, ACP)   |
| `planner.rs`  | High-level agent planner — translates goals into ordered action sequences      |
| `router.rs`   | Intent-to-action routing — maps classified intents to concrete tool/API calls  |
| `simulator.rs`| Plan simulation and risk analysis — dry-runs plans before execution            |

**Integration:** Consumes `src/bayes/` for intent classification input and feeds
execution results back to `src/memory/` for episodic archiving.

---

### `src/bayes/` — Bayesian Inference Engine

The `bayes` module is a self-contained probabilistic reasoning engine. It classifies
user intent using a Bayesian belief graph updated at each conversational turn, maintains
persistent per-user prior distributions in a profile file, and exposes likelihood
functions for intent categories. The `BayesianEngine` is consumed directly by
`src/engine/beliefs.rs` (Reasoning Engine integration) and by
`src/skills/auto_activate.rs` (skill affinity scoring).

| File              | Description                                                               |
|-------------------|---------------------------------------------------------------------------|
| `mod.rs`          | Re-exports `BayesianEngine` and `BeliefGraph`                             |
| `belief_graph.rs` | Directed belief-node graph — models causal relationships between intents  |
| `engine.rs`       | `BayesianEngine` — inference entry point, intent classification API       |
| `likelihoods.rs`  | Likelihood functions P(evidence | intent) for each intent category        |
| `priors.rs`       | Prior probability distributions, seeded from user profile                 |
| `profile.rs`      | Persistent user profile — serialized to `~/.grok/profile.json`            |
| `updater.rs`      | Bayes update formula — posterior = likelihood × prior / evidence          |

**Integration:** `src/engine/beliefs.rs` wraps `BayesianEngine` as `EngineBeliefs`.
`src/skills/auto_activate.rs::check_with_reasoning` queries this engine for intent
scores during skill affinity evaluation.

---

### `src/bin/` — Additional Binary Targets

The `bin` directory under `src/` contains supplementary binary entry points registered
in `Cargo.toml` alongside the main `grok` binary.

| File              | Description                                                                   |
|-------------------|-------------------------------------------------------------------------------|
| `github_mcp.rs`   | GitHub MCP (Model Context Protocol) server binary — `[[bin]] name = "github_mcp"` |
| `banner_demo.rs`  | Standalone demo binary for rendering terminal banners and ASCII art           |
| `docgen.rs`       | Documentation generator binary — renders CLI help into Markdown               |
| `installer.rs`    | Standalone installer binary for platform-specific install steps               |
| `home_server.md`  | Notes/specification for a planned local home-server integration (not a binary)|

---

### `src/cli/` — CLI Argument Parsing and Command Handlers

The `cli` module owns all command-line surface area. It defines the top-level `clap`
application, maps subcommands to handler functions, and manages user-facing approval
prompts for destructive operations. Each `grok <subcommand>` corresponds to a dedicated
file under `commands/`.

| File                     | Description                                                            |
|--------------------------|------------------------------------------------------------------------|
| `mod.rs`                 | Module root — exports `run()` as the binary's main entry function      |
| `app.rs`                 | Clap `App` definition — all subcommands, flags, and argument dispatch  |
| `approval.rs`            | Interactive approval prompts for destructive or privileged operations  |
| `commands/mod.rs`        | Re-exports all subcommand handler modules                              |
| `commands/acp.rs`        | `grok acp` — start ACP context-server session for Zed integration      |
| `commands/audit.rs`      | `grok audit` — display and export security audit log                   |
| `commands/chat.rs`       | `grok chat` / `grok query` — interactive and one-shot chat modes       |
| `commands/code.rs`       | `grok code` — code-focused assistant mode with file context            |
| `commands/config.rs`     | `grok config` — read, write, and validate configuration keys           |
| `commands/health.rs`     | `grok health` — API connectivity and environment health check          |
| `commands/history.rs`    | `grok history` — browse and search conversation history                |
| `commands/settings.rs`   | `grok settings` — interactive TUI settings editor                      |
| `commands/setup.rs`      | `grok setup` — guided first-run configuration wizard                   |
| `commands/skills.rs`     | `grok skills` — list, activate, deactivate, and inspect skills         |

---

### `src/config/` — Configuration Loading and Types

The `config` module handles hierarchical configuration loading from multiple sources:
compiled defaults → system-wide config → user config (`~/.config/grok/config.toml`) →
project-local `.grok/config.toml` → environment variables. It defines all typed
configuration structs and performs validation at startup.

| File                   | Description                                                          |
|------------------------|----------------------------------------------------------------------|
| `mod.rs`               | `Config` struct, `load()`, hierarchical merge logic, validation      |
| `operational_mode.txt` | Plain-text notes on operational mode semantics (development artifact)|

---

### `src/context/` — Project Context Discovery

The `context` module is responsible for discovering and loading project-local context
files that are injected into system prompts. It searches for `.grok/rules`,
`.zed/rules`, `.claude.md`, and similar convention files, enabling per-project AI
persona customization.

| File     | Description                                                                      |
|----------|----------------------------------------------------------------------------------|
| `mod.rs` | Context discovery logic — `.grok/rules`, `.zed/rules`, `.claude.md` file loader |

---

### `src/display/` — Terminal Output Formatting

The `display` module provides all terminal rendering infrastructure: ASCII logo art with
adaptive width detection, session banners, the interactive REPL display loop, contextual
usage tips, and reusable TUI components. It uses `ratatui` and `crossterm` for rich
terminal rendering.

| File / Directory          | Description                                                          |
|---------------------------|----------------------------------------------------------------------|
| `mod.rs`                  | Module root — re-exports all display primitives                      |
| `ascii_art.rs`            | ASCII logo and header art, adaptive to terminal width                |
| `banner.rs`               | Startup banners — version, model, and context info panels            |
| `interactive.rs`          | Interactive session display loop — streaming response renderer       |
| `terminal.rs`             | Terminal capability detection and size/color utilities               |
| `tips.rs`                 | Contextual tips — shown based on usage patterns                      |
| `components/mod.rs`       | Re-exports all reusable TUI components                               |
| `components/input.rs`     | Multi-line input widget with history navigation                      |
| `components/settings_list.rs` | TUI settings list/editor widget (used by `grok settings`)        |

---

### `src/engine/` — Reasoning Engine *(Tasks 93–101)*

> **New in 0.1.9-pre.** The `engine` module is the active decision-making core of
> grok-cli. Where the RPL layer (below) is purely passive and observational, the
> Reasoning Engine is a full finite-state machine (FSM) that actively drives the
> conversation loop. It decomposes user goals into multi-step plans, updates Bayesian
> beliefs on each observation, bridges working context into long-term memory, arbitrates
> tool selection across competing options, runs self-correction loops when tool calls
> fail or produce unexpected results, and emits structured observability events for
> debugging and telemetry.

| File                | Description                                                                        |
|---------------------|------------------------------------------------------------------------------------|
| `mod.rs`            | Module root — flat re-exports of all engine types; `ReasoningEngine` facade        |
| `state.rs`          | `ReasoningEngineState` FSM — `Idle → Analyzing → Planning → Executing → Correcting`; `PlanStep` types |
| `beliefs.rs`        | `EngineBeliefs` — wraps `BayesianEngine`; translates engine observations into Bayes updates |
| `planner.rs`        | `PlanBuilder` — multi-step goal decomposition; produces ordered `PlanStep` sequences |
| `memory_bridge.rs`  | `MemoryBridge` — queries and writes `LongTermMemory`; surfaces relevant facts into working context |
| `arbitration.rs`    | `ArbitrationEngine` — joint tool selection; scores candidate tools using RPL traces and skill affinities |
| `correction.rs`     | `CorrectionEngine` — self-correction loops; detects failed steps and synthesizes retry strategies |
| `observability.rs`  | `EngineObserver` — structured debug logging; emits `EngineEvent` records for tracing/telemetry |

**Integration:** `EngineBeliefs` wraps `src/bayes/BayesianEngine`. `MemoryBridge` reads
and writes `src/memory/long_term::LongTermMemory`. `ArbitrationEngine` collaborates with
`src/skills/auto_activate::AutoActivationEngine` for skill-aware tool scoring.
`CorrectionEngine` reads `ReasoningTrace` records produced by `src/rpl/`.

---

### `src/hooks/` — Extension Hook System

The `hooks` module implements the external extension system. Users can register TOML
hook manifests that point to executable scripts or binaries. Hooks are fired at defined
lifecycle points (pre-tool, post-tool, pre-response, post-response), enabling third-party
extensions without modifying grok-cli source.

| File        | Description                                                                 |
|-------------|-----------------------------------------------------------------------------|
| `mod.rs`    | Hook system public API — `HookManager`, lifecycle event definitions         |
| `loader.rs` | Hook manifest discovery, loading from `~/.config/grok/hooks/`, and execution |

**See also:** `examples/extensions/` for sample hook implementations.

---

### `src/mcp/` — Model Context Protocol Support

The `mcp` module implements the client side of Anthropic's Model Context Protocol,
enabling grok-cli to connect to external MCP servers (including the bundled
`github_mcp` binary) and consume their tool and resource capabilities as first-class
tools within the conversation loop.

| File          | Description                                                                |
|---------------|----------------------------------------------------------------------------|
| `mod.rs`      | Module root — `McpManager`, server registry, tool discovery               |
| `client.rs`   | Async MCP client — JSON-RPC transport over stdio or TCP                   |
| `config.rs`   | `McpServerConfig` — per-server connection parameters and auth settings     |
| `protocol.rs` | MCP message types, capability negotiation, and resource/tool schemas       |

---

### `src/memory/` — Four-Tier Memory Hierarchy

The `memory` module implements grok-cli's persistent memory architecture across four
distinct tiers: short-term (in-session message ring buffer), long-term (persisted facts
and learned preferences), episodic (full session archives), and working memory (active
project context). A `MemoryStore` facade provides a unified API across all tiers.
Skill-specific affinity scores and per-tool call statistics are tracked in dedicated
sub-stores.

| File              | Description                                                                  |
|-------------------|------------------------------------------------------------------------------|
| `mod.rs`          | `MemoryStore` facade — unified read/write API across all memory tiers        |
| `short_term.rs`   | In-session message ring buffer — bounded history for context window management |
| `long_term.rs`    | `LongTermMemory` — persistent key/fact store, serialized to `~/.grok/memory.json` |
| `episodic.rs`     | Session archives — full conversation snapshots stored in `~/.grok/sessions/` |
| `skill_memory.rs` | Per-skill affinity scores — updated by `AutoActivationEngine` after each turn |
| `tool_memory.rs`  | Tool call history and success/failure statistics — informs `ArbitrationEngine` |
| `types.rs`        | Shared types: `MemoryEntry`, `ChatMessage`, `SessionMetadata`                |
| `working.rs`      | Working memory — active project context assembled from `src/context/`       |

**Integration:** `src/engine/memory_bridge.rs` queries `LongTermMemory` and
`WorkingMemory` directly. `src/skills/auto_activate.rs` reads and updates
`skill_memory`.

---

### `src/router/` — CPU-Side Request Router

The `router` module is the network-facing core of grok-cli. `CpuRouter` owns the
primary tool-call loop: it sends requests to the xAI Grok API, handles streaming
responses, processes tool-call deltas, dispatches tool execution back to `src/tools/`,
and loops until the model produces a final response or the iteration cap is reached.
`AppRouter` is the high-level public facade used by CLI commands. The RPL layer hooks
are called synchronously within `cpu_router.rs`'s tool loop.

| File / Directory        | Description                                                              |
|-------------------------|--------------------------------------------------------------------------|
| `mod.rs`                | Re-exports `CpuRouter`, `AppRouter`, `RouterRequest`, `RouterResponse`   |
| `app_router.rs`         | `AppRouter` — public facade; entry point for all CLI command routing     |
| `backend.rs`            | `Backend` trait — abstraction over any HTTP API backend                  |
| `cpu_router.rs`         | `CpuRouter` + `route_with_tools_traced()` — main tool loop with RPL hooks |
| `request.rs`            | `RouterRequest`, `ToolDefinition` — outbound request types               |
| `response.rs`           | `RouterResponse`, `UsageStats` — inbound response types                  |
| `router_error.rs`       | `RouterError` enum — typed error variants for all routing failures       |
| `backends/mod.rs`       | Backend registry — selects the active backend by configuration           |
| `backends/grok.rs`      | `GrokBackend` — concrete HTTP backend for the xAI Grok API               |

**Integration:** `CpuRouter` calls `RplLayer` hooks (from `src/rpl/`) at each tool loop
iteration. Tool execution is delegated to `src/tools/registry::execute_tool()`.

---

### `src/rpl/` — Reasoning Protocol Layer *(Tasks 86–92)*

> **New in 0.1.9-pre.** The `rpl` module is a **passive observability layer** that
> wraps the `CpuRouter` tool loop without altering its control flow. It captures a
> structured `ReasoningTrace` at each step, validates the trace for anomalies (e.g.
> repeated tool calls, missing rationale, suspiciously short reasoning), applies
> configurable log-level filtering, and supports optional PII/secret redaction before
> traces are written to the observability backend. The RPL layer is the data source
> consumed by the active Reasoning Engine.

| File              | Description                                                                        |
|-------------------|------------------------------------------------------------------------------------|
| `mod.rs`          | Module root — flat re-exports; `RplLayer` as the primary integration surface       |
| `schema.rs`       | `ReasoningTrace`, `ToolEvaluation`, `ReasoningPhase` — all trace data types        |
| `validation.rs`   | `validate()` — non-short-circuiting checks; accumulates all anomalies per trace    |
| `layer.rs`        | `RplLayer` — lifecycle hooks: `on_tool_call`, `on_tool_result`, `on_final_response`|
| `logging.rs`      | `ReasoningLogLevel`, `log_trace()` — structured trace emission via `tracing`       |
| `suppression.rs`  | `SuppressionLayer`, `RedactionConfig` — pattern-based field redaction for PII/secrets |

**Integration:** `src/router/cpu_router.rs` instantiates `RplLayer` and calls its hooks
in the tool loop. `src/engine/correction.rs` reads `ReasoningTrace` records to detect
failure patterns. `src/skills/auto_activate.rs::check_with_reasoning` accepts a
`ReasoningTrace` reference for RPL-aware skill scoring.

---

### `src/security/` — Security Policy and Audit

The `security` module defines the runtime security policy (which paths and shell
commands are permitted) and maintains a structured audit log of all privileged
operations. The audit log supports CSV export for external review.

| File       | Description                                                                   |
|------------|-------------------------------------------------------------------------------|
| `mod.rs`   | `SecurityPolicy` — path allowlists, shell command permissions, policy checks  |
| `audit.rs` | Append-only audit log with timestamped entries and CSV export capability      |

---

### `src/skills/` — Skills System

The `skills` module implements grok-cli's persona/specialization system. Skills are
TOML manifests that define a focused AI persona (e.g., "rust-expert", "cli-design")
with custom system prompts, tool restrictions, and activation keywords. The
`AutoActivationEngine` scores incoming messages against all loaded skills using Bayesian
intent signals and RPL trace context to automatically select the most appropriate skill
for each turn.

| File               | Description                                                                    |
|--------------------|--------------------------------------------------------------------------------|
| `mod.rs`           | Re-exports all skill types: `Skill`, `SkillConfig`, `SkillRegistry`            |
| `auto_activate.rs` | `AutoActivationEngine` — RPL-aware automatic skill scoring and selection       |
| `config.rs`        | `Skill`, `SkillConfig` types — skill manifest deserialization                  |
| `manager.rs`       | `load_skill()`, `list_skills()`, `find_skill()` — skill lifecycle management   |
| `registry.rs`      | `SkillRegistry`, `SkillManifest` — in-memory registry of loaded skills         |
| `security.rs`      | `SkillSecurityValidator` — validates skill manifests don't escalate permissions|

**Integration:** `AutoActivationEngine::check_with_reasoning` accepts a `ReasoningTrace`
(from `src/rpl/`) and queries `BayesianEngine` (from `src/bayes/`) for intent signals.
`skill_memory` (in `src/memory/`) persists affinity scores between sessions.

---

### `src/terminal/` — Binary-Only Terminal I/O

The `terminal` module provides terminal I/O primitives used exclusively by the binary
(`src/main.rs`) and CLI commands. It is **not** re-exported from `src/lib.rs` to keep
the library crate free of binary-specific I/O concerns. It handles terminal detection,
raw-mode input reading, progress reporting, and output rendering.

| File          | Description                                                                |
|---------------|----------------------------------------------------------------------------|
| `mod.rs`      | Module root — terminal detection, capability flags, size utilities         |
| `display.rs`  | Output rendering helpers — paging, colour output, diff display             |
| `input.rs`    | Raw-mode keyboard input reader — line editing, history recall              |
| `progress.rs` | Spinner and progress-bar primitives wrapping `indicatif`                   |

---

### `src/tools/` — Tool Implementations

The `tools` module contains every tool that grok-cli can invoke on the model's behalf,
plus the central `execute_tool()` dispatcher that routes tool-call JSON to the correct
implementation. Tools are organized by domain and registered with JSON schema definitions
for injection into the model's tool-call context. `ToolContext` carries the active
`SecurityPolicy` and `SessionId` into every tool call.

| File                 | Description                                                                   |
|----------------------|-------------------------------------------------------------------------------|
| `mod.rs`             | Re-exports all tools and `execute_tool()` entry point                         |
| `registry.rs`        | `execute_tool()` dispatcher + JSON tool-schema definitions for the API        |
| `tool_context.rs`    | `ToolContext` — wraps `SecurityPolicy`, `SessionId`, and working directory    |
| `tool_error.rs`      | `ToolError` enum — 9 typed variants covering permission, I/O, network, etc.  |
| `file_tools.rs`      | 8 file-system tools: read, write, append, delete, list, move, copy, stat      |
| `shell_tools.rs`     | `run_shell_command` — sandboxed shell execution with allowlist/denylist       |
| `web_tools.rs`       | `web_search` (xAI search API) + `web_fetch` (URL content retrieval)          |
| `memory_tools.rs`    | `save_memory` — persist facts to `LongTermMemory` from within a session       |
| `agent_tools.rs`     | Agent communication tools — spawn sub-agents, delegate tasks                  |
| `system_tools.rs`    | System utilities — env vars, working directory, process info                  |
| `discovery_tools.rs` | Project structure discovery — directory trees, file search, grep              |
| `lsp_tools.rs`       | LSP-integration tools — symbol lookup, diagnostics, go-to-definition queries  |
| `mcp_tools.rs`       | MCP pass-through tools — delegates calls to connected MCP servers             |
| `notebook_tools.rs`  | Notebook / scratchpad tools — temporary working notes within a session        |
| `plan_tools.rs`      | Plan manipulation tools — create, update, and query the active plan           |
| `skill_tools.rs`     | Skill management tools — list, activate, and inspect skills from the model    |
| `task_tools.rs`      | Task management tools — read and update `task_list.json` from the model       |

---

### `src/utils/` — Shared Utilities

The `utils` module provides cross-cutting utilities shared across multiple modules.
Network helpers are built to handle Starlink satellite connectivity — short outages,
variable latency, and unexpected connection resets — with configurable retry policies
and exponential back-off.

| File                    | Description                                                              |
|-------------------------|--------------------------------------------------------------------------|
| `mod.rs`                | Re-exports all utilities                                                 |
| `auth.rs`               | API key loading and validation from `.env` / environment variables       |
| `chat_logger.rs`        | Conversation logging to disk with rotation and redaction                 |
| `client.rs`             | Base HTTP client builder — timeout settings, retry middleware wiring     |
| `context.rs`            | Context file loading utilities — reads and merges context documents      |
| `history_compressor.rs` | Conversation history compression — summarises old turns to save context  |
| `network.rs`            | Starlink-resilient network helpers — retry-with-backoff, timeout guards  |
| `rate_limiter.rs`       | API rate limiter — token-bucket algorithm for xAI API rate limits        |
| `session.rs`            | Session ID generation and management using UUID v4                       |
| `shell_permissions.rs`  | Shell command allowlist/denylist enforcement helpers                     |
| `telemetry.rs`          | Structured telemetry emission — wraps `tracing` spans for key operations |

---

## 3. Integration Tests (`tests/`)

All integration tests live at the crate root's `tests/` directory and are compiled as
separate test binaries by Cargo. Run with:
`cargo test --test <filename_without_extension>`

| File                       | Task(s) | What It Covers                                                          |
|----------------------------|---------|-------------------------------------------------------------------------|
| `integration_tests.rs`     | General | End-to-end CLI invocation, config loading, chat round-trips, tool dispatch |
| `acp_permission_flow.rs`   | ACP     | ACP session lifecycle, path security enforcement, permission escalation scenarios |
| `rpl_integration.rs`       | 92      | RPL layer hook invocation sequence, `ReasoningTrace` capture, validation failures, redaction |
| `engine_integration.rs`    | 101     | Reasoning Engine FSM state transitions, plan execution, self-correction loops, memory bridge queries |
| `tool_loop_integration.rs` | General | Multi-turn tool loop correctness, iteration cap enforcement, tool error propagation |

---

## 4. Documentation (`docs/` and `Doc/`)

### `docs/` — Technical / Architecture Reference

Internal developer documentation covering system design and architecture decisions.

| File                     | Description                                                                   |
|--------------------------|-------------------------------------------------------------------------------|
| `agent_docs.md`          | Generated CLI command documentation (output of `cargo run --bin docgen`)     |
| `rpl_architecture.md`    | Reasoning Protocol Layer architecture — design decisions, hook protocol, schema |
| `engine_architecture.md` | Reasoning Engine architecture — FSM design, module responsibilities, data flows |

### `Doc/` — User-Facing Documentation

End-user guides, quick-starts, and command references.

| File / Directory                      | Description                                                      |
|---------------------------------------|------------------------------------------------------------------|
| `CONFIG_QUICK_START.md`               | Getting started with `config.toml` in 5 minutes                 |
| `EXTERNAL_ACCESS_QUICK_START.md`      | Guide for granting grok-cli access to external file paths        |
| `HOOKS_AND_EXTENSIONS.md`             | Extension system guide — writing and registering hooks           |
| `MAX_TOOL_LOOP_ITERATIONS.md`         | Configuring the `max_tool_loop_iterations` cap and its tradeoffs |
| `QUICK_REFERENCE.md`                  | One-page CLI quick reference card                                |
| `SECURITY.md`                         | Security model, path sandboxing, audit log, and threat model     |
| `SKILLS_QUICK_START.md`               | Creating and activating skills in 5 minutes                      |
| `extensions.md`                       | Extension development guide — hook manifest format and lifecycle |
| `commands/hooks-command.md`           | Per-command reference for `grok hooks`                           |
| `ai-generated-summaries/`             | AI task completion summaries (populated post task-runner runs)   |

---

## 5. Scripts, Examples, and Support Directories

### `scripts/` — Build, Test, and Release Utilities

| File                            | Description                                                    |
|---------------------------------|----------------------------------------------------------------|
| `build.ps1`                     | Windows PowerShell build script (cargo build + post-steps)     |
| `release.ps1`                   | PowerShell release automation (version bump, tag, publish)     |
| `release.sh`                    | Bash release script for Linux/macOS CI                         |
| `test_acp.ps1` / `.py` / `.sh`  | ACP protocol smoke tests in three environments                 |
| `test_acp_simple.sh`            | Minimal ACP handshake test                                     |
| `test_simple_acp.sh`            | Alternate ACP test script                                      |
| `test_tool_loop_chat.sh`        | Tool-loop chat integration smoke test                          |
| `test_tool_loop_debug.sh`       | Tool-loop debug trace capture                                  |
| `test_env.sh`                   | Environment variable validation script                         |
| `analyze_tool_loops.ps1`        | Post-run analysis of tool loop traces from `out.txt`           |
| `cleanup_old_install.bat`       | Batch script to remove stale Windows installation artifacts    |
| `cleanup_old_install.ps1`       | PowerShell equivalent of the above                             |
| `fix_config_syntax.ps1`         | Automated TOML config syntax fixer                             |
| `update_system_config.ps1`      | Updates system-wide grok config on Windows                     |
| `verify_installer_v0.1.41.ps1`  | Post-install verification script for v0.1.41 baseline          |
| `BUILD_README.md`               | Build system documentation                                     |
| `fix-github-build.md`           | Notes on fixing GitHub Actions build pipeline issues           |

### `examples/` — Usage Examples

| Directory / File                             | Description                                           |
|----------------------------------------------|-------------------------------------------------------|
| `examples/skills/README.md`                  | Overview of all bundled skill examples                |
| `examples/skills/SKILL_SPEC.md`              | Skill manifest format specification                   |
| `examples/skills/SKILL_BUILDER_QUICKSTART.md`| Quick-start guide for building new skills             |
| `examples/skills/skill-builder-examples.md`  | Annotated skill-builder workflow examples             |
| `examples/skills/rust-expert/`               | Rust development specialist skill                     |
| `examples/skills/cli-design/`                | CLI UX and argument-design specialist skill           |
| `examples/skills/skill-builder/`             | Meta-skill for building other skills interactively    |
| `examples/skills/zed-task-manager/`          | Zed task-list management skill                        |
| `examples/extensions/file-backup-hook/`      | Hook example: auto-backup files before tool writes    |
| `examples/extensions/logging-hook/`          | Hook example: append all tool calls to a log file     |
| `examples/extensions/project-setup-hook/`    | Hook example: run project-setup scripts on activation |

### `bin/` and `config/` (Root)

| File            | Description                                                           |
|-----------------|-----------------------------------------------------------------------|
| `bin/grok.js`   | Node.js binary shim — resolves platform binary path for npm installs  |
| `config/mod.rs` | Loose config module (pending merge into `src/config/`)                |

---

## 6. Architecture Layers

The following diagram shows the module dependency stack from user input to API output.
Arrows indicate the direction of function calls and data flow.

```
┌─────────────────────────────────────────────────────────┐
│                  CLI / Binary Layer                      │
│   src/main.rs  ·  src/cli/  ·  src/terminal/            │
│   src/display/ ·  src/grok_client_ext.rs                 │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│              ACP / Protocol Layer                        │
│         src/acp/  ·  src/mcp/                           │
│     (Zed context-server & MCP client sessions)          │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│              Router / CPU Layer                          │
│     src/router/   (CpuRouter + AppRouter)               │
│     ┌─────────────────────────────────┐                 │
│     │         RPL Layer               │  ← src/rpl/     │
│     │  (passive observability hooks)  │                 │
│     │  ReasoningTrace captured here   │                 │
│     └─────────────────────────────────┘                 │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│              Agent / Orchestration Layer                 │
│         src/agent/  ·  src/context/                     │
│     (intent routing, plan simulation, mode detection)   │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│              Reasoning Engine Layer                      │
│                  src/engine/                            │
│  FSM: Idle → Analyzing → Planning → Executing →         │
│       Correcting → Idle                                 │
│  (active goal decomposition, tool arbitration,          │
│   self-correction, structured observability)            │
└────────┬──────────────────┬──────────────────┬──────────┘
         │                  │                  │
┌────────▼───────┐  ┌───────▼──────┐  ┌────────▼────────┐
│  Bayesian      │  │   Memory     │  │  Skills /       │
│  Inference     │  │  Hierarchy   │  │  Arbitration    │
│  src/bayes/    │  │ src/memory/  │  │  src/skills/    │
│                │  │              │  │  src/hooks/     │
└────────────────┘  └──────┬───────┘  └─────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│                  Tools Layer                             │
│                 src/tools/                              │
│  file · shell · web · memory · agent · system ·         │
│  discovery · lsp · mcp · notebook · plan · skill · task  │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│             Shared / Cross-cutting Layer                 │
│  src/utils/    src/security/    src/config/             │
│  (network resilience, rate limiting, auth, telemetry,   │
│   audit log, policy enforcement)                        │
└─────────────────────────────────────────────────────────┘
```

---

## 7. Key Integration Points

The following describes how the major subsystems connect at runtime:

- **`CpuRouter` → `RplLayer` hooks:**
  `src/router/cpu_router.rs::route_with_tools_traced()` holds an `RplLayer` instance.
  Before each tool call it calls `layer.on_tool_call(...)`, after each tool result it
  calls `layer.on_tool_result(...)`, and at conversation end it calls
  `layer.on_final_response(...)`. The layer captures a `ReasoningTrace` at each step
  without modifying the router's control flow — purely observational.

- **`ReasoningEngineState` ↔ `ReasoningTrace`:**
  The Reasoning Engine FSM (`src/engine/state.rs`) transitions states based on
  `ReasoningTrace` records emitted by the RPL layer. `CorrectionEngine`
  (`src/engine/correction.rs`) reads traces to identify repeated failures,
  truncated reasoning, or anomalous tool selections and synthesizes a corrective
  strategy before the next FSM iteration.

- **`EngineBeliefs` wraps `BayesianEngine`:**
  `src/engine/beliefs.rs::EngineBeliefs` holds a reference to
  `src/bayes/engine::BayesianEngine`. After each conversation turn the engine calls
  `EngineBeliefs::update(observation)`, which translates the observation into Bayesian
  evidence and calls the underlying `BayesianEngine::update()`, keeping the FSM's
  belief state in sync with the probabilistic intent model.

- **`MemoryBridge` → `LongTermMemory`:**
  `src/engine/memory_bridge.rs::MemoryBridge` holds a handle to
  `src/memory/long_term::LongTermMemory`. At the start of each engine cycle it calls
  `bridge.fetch_relevant(goal)` to surface related past facts into the working context
  window, and at the end it calls `bridge.commit(new_facts)` to persist any new
  information the model produced during the turn.

- **`AutoActivationEngine::check_with_reasoning` uses `ReasoningTrace`:**
  `src/skills/auto_activate.rs::AutoActivationEngine::check_with_reasoning` accepts
  the current `ReasoningTrace` alongside the raw message text. It uses the trace's
  `ToolEvaluation` records and `ReasoningPhase` classification to weight skill affinity
  scores — skills that align with the model's stated reasoning phase receive a scoring
  bonus, improving skill selection accuracy beyond pure keyword matching.

- **`ArbitrationEngine` ↔ `AutoActivationEngine`:**
  When the Reasoning Engine's planner produces a `PlanStep` requiring tool selection,
  `src/engine/arbitration.rs::ArbitrationEngine` calls
  `AutoActivationEngine::check_with_reasoning` to retrieve the active skill's tool
  restrictions and affinity weights. It combines these signals with `tool_memory`
  statistics (historical success rates per tool) and the current `ReasoningTrace`
  to score all candidate tools and select the highest-scoring viable option.

- **`SecurityPolicy` threads through `ToolContext`:**
  Every tool execution receives a `ToolContext` (`src/tools/tool_context.rs`) that
  carries the current `SecurityPolicy` and `SessionId`. Both `ACP security`
  (`src/acp/security.rs`) and shell permission checks (`src/utils/shell_permissions.rs`)
  delegate to the same `SecurityPolicy` instance, ensuring consistent enforcement
  across all tool invocation paths.

- **`network.rs` resilience applied globally:**
  `src/utils/network.rs` provides `retry_with_backoff(op, config)` — a generic
  async retry wrapper with exponential back-off and jitter. Both `GrokBackend`
  (`src/router/backends/grok.rs`) and `web_tools.rs` wrap all outbound HTTP calls
  in this helper, ensuring the application survives Starlink link drops gracefully.

---

## 8. Development Notes

### Language and Edition
- **Rust 2024 edition** (`edition = "2024"` in `Cargo.toml`)
- Follows Rust idiomatic conventions: `Result`/`Option` error handling throughout,
  `thiserror` for typed error enums, `anyhow` for application-level error propagation
- Use `cargo clippy -- -D warnings` before every commit to enforce lint cleanliness

### Platform
- **Primary platform:** Windows 11
- **Line endings:** CRLF (`git config core.autocrlf true` on Windows)
- Windows-specific dependencies: `winreg = "0.56"` (under `[target.'cfg(windows)'.dependencies]`)
- All file paths use `std::path::PathBuf`; avoid hardcoded separators
- PowerShell (`.ps1`) scripts are the canonical build/release tooling on Windows

### Network Resilience (Starlink)
- All outbound network calls MUST use `src/utils/network.rs::retry_with_backoff()`
- Default policy: 3 retries, 2s base delay, 30s max delay, ±20 % jitter
- `reqwest` client is configured with `connect_timeout(10s)` and `timeout(60s)`
- Never use bare `client.get(...).send().await?` without the retry wrapper
- Test network failure paths with `mockito` — see `tests/integration_tests.rs`

### Running Tests

```
# Run all library unit tests
cargo test --lib

# Run a specific integration test binary
cargo test --test integration_tests
cargo test --test rpl_integration
cargo test --test engine_integration
cargo test --test acp_permission_flow
cargo test --test tool_loop_integration

# Run everything with output
cargo test -- --nocapture

# Run with a specific filter
cargo test --test engine_integration fsm_transitions
```

### Key Environment Variables

| Variable            | Description                                                     |
|---------------------|-----------------------------------------------------------------|
| `GROK_API_KEY`      | **Required.** xAI API key for Grok model access                 |
| `RUST_LOG`          | Log filter string (e.g. `grok_cli=debug,reqwest=warn`)          |
| `GROK_CONFIG_PATH`  | Override path for `config.toml` (default: `~/.config/grok/`)   |
| `GROK_MEMORY_PATH`  | Override path for memory store (default: `~/.grok/`)            |
| `GROK_MAX_RETRIES`  | Override default network retry count (default: `3`)             |
| `GROK_LOG_LEVEL`    | RPL reasoning log level: `off`, `summary`, `full`               |

Store all API keys in a `.env` file at the project root (git-ignored). Load with
`dotenvy::dotenv().ok()` — already called in `src/main.rs` before argument parsing.

### Dependency Highlights

| Crate              | Version    | Role                                              |
|--------------------|------------|---------------------------------------------------|
| `grok_api`         | 0.1.3      | Local path dependency — xAI Grok API client types |
| `clap`             | 4.6.1      | CLI argument parsing (derive feature)             |
| `tokio`            | 1.52.1     | Async runtime (full features)                     |
| `reqwest`          | 0.13.2     | HTTP client (native-tls-vendored, http2)          |
| `serde` / `serde_json` | 1.0    | JSON serialization throughout                     |
| `tracing`          | 0.1        | Structured logging and spans                      |
| `ratatui`          | 0.30       | TUI framework for interactive display             |
| `crossterm`        | 0.29       | Cross-platform terminal control                   |
| `anyhow`           | 1.0        | Application-level error propagation               |
| `thiserror`        | 2.0        | Typed error enum derivation                       |
| `chrono`           | 0.4        | Timestamps and session dating (serde feature)     |
| `uuid`             | 1.22.0     | Session and entry ID generation (v4)              |

---

*This document is the authoritative map of the grok-cli source tree.*
*Update it whenever directories are added, renamed, or removed.*
*See `CHANGELOG.md` for version history and `.zed/task_list.json` for active work items.*