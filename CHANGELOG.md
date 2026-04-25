# Changelog

All notable changes to the Grok CLI project are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

Author: John McConnell <john.microtech@gmail.com>
Repository: https://github.com/microtech/grok-cli
Buy me a coffee: https://buymeacoffee.com/micro.tech

---

## [Unreleased]

### Fixed — `read_file` Diagnostics, JSONC Validation & Tool-Loop Error Reporting

**Root cause of hallucinations when reading `.zed/task_list.json`**

Three separate issues combined to cause Grok to "make stuff up" instead of
reporting a real error when asked to read a JSON task file:

1. `route_with_tools_traced` used a bare `format!("Tool '{}' failed: {}", …)`
   string on tool failure — the LLM received no recovery suggestions and no
   structured guidance, so it fell back to fabricating an answer.
2. Neither tool-execution loop (`route_with_tools` or `route_with_tools_traced`)
   called `log_tool_error` / `log_tool_success`, so failures left no audit trail
   in `.grok/logs/grok-tool-error-log.log`.
3. `.zed/task_list.json` (and similar editor config files) uses **JSONC** format
   — trailing commas after the last property — which `serde_json` (a strict
   parser) rejects.  Previously there was no validation at all, so the LLM
   silently received potentially malformed bytes.

#### Changes — `src/router/cpu_router.rs`

- **`route_with_tools`**: replaced the bare `unwrap_or_else` error string with
  the full `format_tool_error_for_llm(…)` call **and** added timing +
  `log_tool_success` / `log_tool_error` calls around every tool dispatch.
- **`route_with_tools_traced`**: same fix — replaced `format!("Tool '{}' failed:
  …")` with `format_tool_error_for_llm(…)` and added success/error logging.
  This was the primary source of the hallucination bug.

Both loops now write a structured entry to
`.grok/logs/grok-tool-error-log.log` on every tool call (success or failure),
force-flushed on errors so Starlink drops never lose a record.

#### Changes — `src/tools/file_tools.rs`

- **Two-stage JSON integrity check** added to `read_file`:
  - *Stage 1*: strict `serde_json` parse — valid JSON files pass through
    unchanged.
  - *Stage 2*: JSONC cleanup — trailing commas are stripped with a regex and
    the file is re-parsed.  Files that are valid JSONC (Zed / VS Code config
    style) are returned **verbatim** with no warning, preserving the original
    content for the LLM.
  - *Stage 3*: genuinely malformed content — a `READ_FILE_WARNING:` banner is
    prepended so the LLM knows the file *was* read but the data is corrupt,
    preventing fabrication.
- **`strip_jsonc_trailing_commas()`** helper added (uses the already-imported
  `Regex` crate; no new dependencies).
- **Four new `#[test]` functions** added to `mod tests`:

  | Test | What it verifies |
  |---|---|
  | `read_json_file_valid_json_returns_content` | valid JSON returned verbatim and re-parseable |
  | `read_json_file_malformed_json_returns_warning` | truly broken JSON yields `READ_FILE_WARNING` |
  | `read_json_file_jsonc_trailing_commas_no_warning` | JSONC files pass through cleanly |
  | `read_file_empty_file_returns_empty_string` | empty file → `Ok("")`, never an error |
  | `read_file_denied_for_untrusted_path` | untrusted path → `Err` mentioning access denial |

All 14 `file_tools` tests and 40 router/tool-error tests pass; Clippy reports
zero new errors (pre-existing warnings only).

*Source: AI (Claude Sonnet 4.6) — 2025-07-16*


### Added — Dedicated Tool-Execution Diagnostic Logger

- **`src/utils/tool_logger.rs`** — New module providing a persistent, per-project
  tool-execution log written to `<project-root>/.grok/logs/grok-tool-error-log.log`.
  - `log_tool_error()` — writes a full diagnostic block on every tool failure,
    including: ISO 8601 timestamp, tool name, JSON arguments (truncated at 512 B),
    error message, call duration in µs, current working directory, and the complete
    list of trusted directories from the active security policy.  Entry is
    force-flushed to disk immediately so Starlink drops / process crashes never
    lose a record.
  - `log_tool_success()` — writes a compact one-liner for successful calls
    (`TOOL-OK  ─ write_file  path="src/main.rs"  result=1024B  55µs`) for
    confirming that file writes actually reached disk.
  - `log_note()` — free-form diagnostic note (session-level events).
  - `log_session_start()` — banner written at the top of each new ACP session
    so log entries are cleanly delimited per session.
  - Human-readable hints appended automatically for the most common failure
    categories: Access Denied, Path Not Found, OS Permission, Network Timeout,
    Network Error.
  - Thread-safe via `OnceCell<Mutex<Option<File>>>` — safe to call from async
    contexts and parallel tool loops.
  - 10 unit tests covering hint categorisation, argument truncation, path
    construction, and entry formatting.

- **`src/acp/security.rs`** — Added `trusted_directories() -> &[PathBuf]` accessor
  to `SecurityPolicy` so the tool logger can include the trust list in error entries,
  making "Access denied" root causes immediately visible without digging through
  config files.

- **`src/utils/mod.rs`** — Registered `pub mod tool_logger` so the module is
  reachable as `crate::utils::tool_logger` throughout the codebase.

### Changed

- **`src/acp/mod.rs`** — `GrokAcpAgent::handle_chat_completion()` now calls
  `tool_logger::log_tool_error()` in the `Err` arm of the tool-dispatch loop
  (in addition to the existing `warn!` tracing event) and
  `tool_logger::log_tool_success()` in the `Ok` arm.  Failure entries capture
  the working directory and trusted directories at call time so the log is
  self-contained for debugging.

- **`src/acp/mod.rs`** — `GrokAcpAgent::initialize_session()` now calls
  `tool_logger::log_session_start()` so each ACP session is clearly delimited
  in the tool log with session ID, CWD, and model name.

### Why

When Grok is used as an ACP agent inside the Zed editor the existing
`grok-errors.log` only showed that a tool failed but not **why**.  The two most
common failures observed were:

1. `Access denied: External access is disabled in configuration` — Grok was
   launched from a directory that did not include the file the AI tried to edit.
2. `Failed to resolve path '…': os error 3` — the path was relative but not
   relative to the project root Grok was started from.

The new log makes both cases immediately diagnosable: the `CWD` and `Trusted`
fields show exactly which directory was active and which directories were
considered safe, so the fix (re-launching from the correct project root or
`@`-mentioning a file) is obvious without reading source code.


### Added — Reasoning Protocol Layer (Tasks 86–92)

- **Task 86 — RPL Architecture Document** (`docs/rpl_architecture.md`):
  Full architecture document defining RPL goals, constraints, CPU lifecycle
  mapping, reasoning schema fields, logging levels, suppression rules, and
  integration points.

- **Task 87 — Reasoning Schema** (`src/rpl/schema.rs`, `src/rpl/validation.rs`):
  Structured `ReasoningTrace` type with fields: `trace_id` (UUID v4
  correlation ID), `goal`, `context`, `tool_evaluations`, `memory_considerations`,
  `plan`, `uncertainty` [0,1], `phase`, `suppressed`. Includes non-short-
  circuiting `validate()` function that collects all `ValidationError`s.

- **Task 88 — CPU/State Machine Integration** (`src/router/cpu_router.rs`, `src/rpl/layer.rs`):
  `CpuRouter` gains optional `RplLayer` via `with_rpl()` builder. New
  `route_with_tools_traced()` method calls `on_pre_evaluate()`,
  `on_tool_selection()`, and `on_complete()` hooks at correct CPU lifecycle
  points. Determinism guard uses UUID call_id in tracing spans.

- **Task 89 — Skill Arbitration Integration** (`src/skills/auto_activate.rs`):
  `AutoActivationEngine` gains `check_with_reasoning()` (tool-name keyword
  boost +15, uncertainty penalty −10) and `check_with_fallback()` (halves
  confidence when uncertainty ≥ 0.9).

- **Task 90 — Reasoning Trace Logging** (`src/rpl/logging.rs`):
  `ReasoningLogLevel` enum (Off / Summary / Debug / Trace) with `log_trace()`
  function emitting structured `tracing` events. Default level is `Summary`.
  `Trace` level JSON-serialises the full trace.

- **RPL module** (`src/rpl/mod.rs`, `src/rpl/layer.rs`): `RplLayer` (stateless,
  shareable) and `RplConfig` (log_level, lenient_validation). 56 new unit tests.

- **Task 92 — RPL Integration Test Suite** (`tests/rpl_integration.rs`):
  12 integration tests covering schema round-trips, suppression in production
  and debug modes, redaction of sensitive goals, validation rejection, skill
  arbitration stability, and regression prevention.

### Added — Reasoning Engine (Tasks 91, 93–101)

- **Task 91 — Suppression & Privacy Controls** (`src/rpl/suppression.rs`):
  `SuppressionLayer` with `guard()` (blocks suppressed traces in production),
  `redact()` (applies `RedactionConfig` rules to all string fields).
  `RedactionConfig::default_rules()` covers API keys, secrets, and passwords.
  `RedactionRule` wraps compiled regex patterns for safe, testable redaction.

- **Task 93 — Full Reasoning Engine Architecture Document** (`docs/engine_architecture.md`):
  Comprehensive architecture specification for the `src/engine/` subsystem covering
  all nine required sections:
  - **§1 Overview** — Engine responsibilities, explicit non-goals (ACP layer, CLI binary,
    HTTP calls, display, secrets), and a detailed comparison table distinguishing the
    active Reasoning Engine from the passive RPL layer across seven dimensions (role,
    mode, state, output, side-effects, failure mode, coupling).
  - **§2 Component Responsibilities** — Submodule table for all seven `src/engine/`
    files (`state.rs`, `beliefs.rs`, `planner.rs`, `memory_bridge.rs`, `arbitration.rs`,
    `correction.rs`, `observability.rs`) plus an ASCII type-ownership map showing
    which engine types wrap which existing crate types.
  - **§3 CPU Phase Mapping** — Full-stack ASCII call-flow diagram showing engine
    position between `ACP Layer` and `RplLayer::on_complete()`; `EngineState`→CPU
    action→next-state table for all nine FSM variants; annotated forward-only state
    transition diagram with the `RevisePlan`→`CommitPlan` back-edge.
  - **§4 Core State Model** — Field-by-field table for `ReasoningEngineState` (13
    fields including `engine_id` UUID v4, `schema_version` mirroring `RPL_SCHEMA_VERSION=1`,
    `uncertainty` f32 [0,1], `max_revisions` default 3); inline Rust definitions for
    `EngineState`, `Hypothesis`, `PlanStep`, `StepAction` (`UseTool`, `QueryMemory`,
    `ModelCall`, `NoOp`), and `StepStatus`.
  - **§5 Interfaces** — `MemoryBridge` interface (3 methods) with memory-tier access
    policy table; `ArbitrationEngine` interface (3 methods) with `RankedTool` field
    table and fused-score formula (`0.6 * auto_activation + 0.4 * bayesian`); grounded
    in real `AutoActivationEngine` weights (keyword=30, pattern=40, extension=25) and
    `SkillMatch` struct from `src/skills/auto_activate.rs`.
  - **§6 Bayesian Integration** — Maps engine lifecycle events to actual
    `BayesianEngine` methods (`update_from_text`, `update_from_model_confidence`,
    `update_from_tool_failure`, `update_profile`); documents real threshold values
    (clarification=0.4, uncertainty=0.6, vagueness=0.6) from `src/bayes/engine.rs`;
    explains how `best_intent()`, `probability("low_confidence")`, and per-intent
    posteriors flow into `Hypothesis::confidence` and `ArbitrationEngine` scoring.
  - **§7 Self-Correction Loops** — Three trigger conditions (low confidence, step
    failure, user feedback signal); `should_correct()` pseudocode with `max_revisions`
    cap enforcement; full correction cycle ASCII diagram showing detect→revise→
    re-evaluate→continue-or-fail path and the key invariant that `revision_count`
    counts plan revisions not step retries.
  - **§8 Observability** — Event-level table (seven event types with `tracing` macro,
    fields, and minimum `ReasoningLogLevel`); suppression and privacy rules for
    `state.goal`, `memory_references`, and belief update events; code snippet for
    enabling `Trace`-level logs in development.
  - **§9 Future Work** — Five backlog items: persistent state across sessions,
    parallel independent step execution via `tokio::join!`, LLM-generated hypotheses,
    online belief updates from user feedback, and hypothesis confidence calibration
    (Platt scaling / isotonic regression).
  - **Appendix A** — Complete file inventory cross-referencing all `src/engine/`,
    `src/rpl/`, `src/bayes/`, `src/memory/`, `src/skills/`, and `src/router/` files.

- **Task 94 — Core State Model** (`src/engine/state.rs`, `src/engine/mod.rs`):
  `ReasoningEngineState` FSM with `EngineState` enum (8 states), `PlanStep`,
  `StepAction`, `StepStatus`, `Hypothesis`, `TransitionError`, `PlanError`.
  Enforced state transitions reject invalid moves from terminal states.
  17 unit tests. `pub mod engine` added to `src/lib.rs`.

- **Task 95 — Bayesian Belief Integration** (`src/engine/beliefs.rs`):
  `EngineBeliefs` wraps `BayesianEngine`, manages `ToolBelief` scores,
  processes `Evidence` variants (UserText, ToolSuccess, ToolFailure,
  ModelConfidence). `sync_to_state()` propagates uncertainty and hypotheses.
  19 unit tests.

- **Task 96 — Planning Layer** (`src/engine/planner.rs`):
  `PlanBuilder` with keyword→tool heuristic `ToolHint`s (7 default rules),
  `build_plan()`, `build_model_only_plan()`, and `revise_plan()` for
  post-failure re-planning. 9 unit tests.

- **Task 97 — Memory Bridge** (`src/engine/memory_bridge.rs`):
  `MemoryBridge` queries `LongTermMemory` for goal-relevant facts, decides
  write-back eligibility (uncertainty + Complete state + no revision),
  and builds summary strings for memory persistence. 8 unit tests.

- **Task 98 — Skill Arbitration Integration** (`src/engine/arbitration.rs`):
  `ArbitrationEngine` with 8 built-in `ToolCapability` entries, `rank_tools()`
  combining plan + RPL trace scores, uncertainty-aware `select_tool()`
  (falls back to cheapest tool at high uncertainty), and `commit_selection()`
  writing `ToolEvaluation`s to the RPL trace. 11 unit tests.

- **Task 99 — Self-Correction Loops** (`src/engine/correction.rs`):
  `CorrectionEngine` detects `StepFailed`, `HighUncertainty`, `EmptyPlan`,
  `ExternalFeedback` triggers and applies targeted recovery plans.
  `correct_until_stable()` provides a bounded loop safeguard. 23 unit tests.

- **Task 100 — Engine Observability** (`src/engine/observability.rs`):
  `EngineObserver` with `ObserverConfig` (log_level, redaction, suppressed).
  Logs state transitions at Info/Debug/Trace, plan revisions, step completions,
  and corrections (always warn). `is_safe_to_log()` and `redact_state()` for
  privacy validation. 22 unit tests.

- **Task 101 — End-to-End Integration Tests** (`tests/engine_integration.rs`):
  15 integration tests exercising the full engine lifecycle: goal analysis,
  Bayesian updates, plan building, memory write decisions, arbitration
  ranking, self-correction, observability, suppression, redaction, and
  regression checks.

### Documentation

- `docs/rpl_architecture.md` — RPL architecture specification (Task 86)
- `docs/engine_architecture.md` — Reasoning Engine architecture specification (Task 93)
- `docs/REASONING_SYSTEMS.md` — Overview index linking both architecture docs.
  New index document covering both the RPL and Reasoning Engine in a single reference
  page. Sections: Introduction (RPL vs Engine distinction), Quick Links table, RPL at
  a Glance (key types, default suppression behaviour, full lifecycle code example),
  Reasoning Engine at a Glance (FSM diagram, key types, self-correction bounds,
  lifecycle code example), How RPL and Engine Work Together
  (`route_with_tools_traced` call site, `engine_id`/`trace_id` correlation,
  `ArbitrationEngine::commit_selection` writing into RPL trace, shared
  `SuppressionLayer` rules), Test Coverage Summary (227 tests across 4 files),
  and Configuration tables for all five config structs (`RplConfig`,
  `ObserverConfig`, `CorrectionConfig`, `MemoryBridgeConfig`, `ArbitrationConfig`).
  Source: AI-assisted, human-reviewed.
- `Doc/REASONING_QUICK_START.md` — Developer quick-start guide for RPL and Engine.
  Covers: Prerequisites (Rust 2024, `cargo build`, Starlink network resilience note),
  step-by-step RPL usage (`RplLayer` creation → `route_with_tools_traced` wrapping →
  `SuppressionLayer` guard), Engine usage (state creation → `PlanBuilder::build_plan`
  → `CorrectionEngine::correct_until_stable`), adding a custom `RedactionRule` +
  `RedactionConfig`, enabling `Trace`-level logs (env-var and in-code variants),
  running the test suites (unit, integration, Clippy), and a Troubleshooting table
  covering 8 common failure modes with causes and fixes.
  Source: AI-assisted, human-reviewed.
- `project_layout.md` — Complete project directory and module reference
- `dataflow_map.md` — Updated with RPL and Engine data flow diagrams

### Changed

- `src/lib.rs` — Added `pub mod rpl` and `pub mod engine`
- `src/router/cpu_router.rs` — Added `with_rpl()` builder, `route_with_tools_traced()`, determinism guard
- `src/skills/auto_activate.rs` — Added `check_with_reasoning()` and `check_with_fallback()` methods
- `src/rpl/mod.rs` — Added `pub mod suppression` and complete re-exports
- `src/engine/mod.rs` — Complete re-exports for all engine submodule types

---

## [Unreleased] - 2026-04-19

### Added

- **`docs/rpl_architecture.md`** — Comprehensive architecture document for the Reasoning Protocol Layer (RPL). AI: Claude Sonnet 4.6
  - **Task 86 completed** (subtasks 86.1, 86.2, 86.3 all done).
  - Documents RPL goals and constraints: structured reasoning, observability, privacy by default, and determinism. Non-goals explicitly stated (no model fine-tuning, no backend replacement).
  - Defines the position of `RplLayer` in the overall architecture with an ASCII component diagram showing data flow from ACP → `CpuRouter` → Tool Loop → Skill Arbitration → `MemoryManager`.
  - Maps CPU/state-machine phases to `ReasoningPhase` enum values and their corresponding `RplLayer` hook entry points (`on_pre_evaluate`, `on_tool_selection`, `on_complete`).
  - Fully documents the `ReasoningTrace` envelope schema (`schema_version`, `trace_id`, `goal`, `context`, `tool_evaluations`, `memory_considerations`, `plan`, `uncertainty`, `created_at`, `phase`, `suppressed`) plus `ToolEvaluation` and `MemoryConsideration` sub-types.
  - Documents `ReasoningLogLevel` (`Off` / `Summary` / `Debug` / `Trace`) and explains `RUST_LOG` control surface and `trace_id` correlation with `CpuRouter` structured logs.
  - Documents suppression and privacy contract: `suppressed = true` by default, hard ACP boundary enforcement, and the future redaction rules extension point.
  - Lists all integration points: `CpuRouter::route_with_tools`, `CpuRouter::route_with_tools_traced`, `AutoActivationEngine::check_with_reasoning`, and future `MemoryStore` / `BayesEngine` hooks.
  - Includes a validation table (`validate()` checks and `ValidationError` variants).
  - Future work section tracks Tasks 95 (Bayesian belief integration), 96 (Planning layer), 97 (Memory-aware reasoning), and 99 (Self-correction loops).
- **`.zed/task_list.json`** — Task 86 and subtasks 86.1–86.3 marked as `done`.

---

## [0.1.9-pre] - 2026-04-02

### Fixed

- **`[No response content]` on complex questions via ACP (`@bot`)** — AI: Claude Sonnet 4.6
  - **Root cause:** `RouterResponse` had no `finish_reason` field, so the real
    value from the xAI API (e.g. `"tool_calls"`) was silently discarded.
    `RouterResponse::into_message_with_finish_reason()` then hardcoded
    `finish_reason = Some("stop")` for every response.
  - In `handle_chat_completion` (`src/acp/mod.rs`) the very first check is
    `if finish_reason == Some("stop") { return Ok(response_text) }`.  Because
    finish_reason was always `"stop"`, this early-return fired on the **first
    loop iteration**, before any tool calls were ever executed.  When the model
    had returned tool-calls with no text content, `response_text` was `""` →
    the fallback `"[No response content]"` was sent back to Zed.
  - **Fix — three files changed:**
    1. `src/router/response.rs` — added `finish_reason: Option<String>` field
       to `RouterResponse`; `into_message_with_finish_reason()` now uses
       `self.finish_reason` (falls back to `"stop"` only when absent).
    2. `src/router/backends/grok.rs` — captures `mwfr.finish_reason` from the
       `GrokClient` response and stores it in the returned `RouterResponse`.
    3. `src/router/cpu_router.rs` — added the new `finish_reason` field to the
       inline `RouterResponse` literal (set to `Some("stop")` — that path only
       fires after tool execution completes, so "stop" is correct there).
  - Also fixed a pre-existing one-character typo in
    `src/cli/commands/acp.rs` line 1: `/!` → `//!` (missing `/` on the
    module doc-comment) which caused a hard compile error.
  - Simple messages (e.g. "hi") are unaffected — they never trigger tool calls
    and the model correctly returns `finish_reason = "stop"` with text content.


### Added

- **Tools module restructuring** (`src/tools/`) — AI: Claude Sonnet 4.6
  - Moved all tool implementations out of the monolithic `src/acp/tools.rs`
    (1 166 lines) into a properly structured `src/tools/` module following the
    Grok-CLI Tools Build Instructions spec.
  - **`tool_error.rs`** — `ToolError` enum with nine structured variants:
    `AccessDenied`, `FileNotFound`, `Io`, `InvalidArgument`, `Timeout`,
    `Network`, `InvalidPattern`, `UnknownTool`, and `Other` (anyhow catch-all).
    Both `std::io::Error` and `anyhow::Error` have `#[from]` conversions.
  - **`tool_context.rs`** — `ToolContext` struct wrapping `SecurityPolicy`.
    `Clone + Debug`, cheap to copy without `Arc`. Constructors: `::new(policy)`,
    `::default_for_cwd()`, and `From<SecurityPolicy>`.
  - **`file_tools.rs`** — eight file-system tools with identical signatures to
    the previous `acp::tools` functions so no call-sites needed updating:
    `read_file`, `read_multiple_files`, `list_code_definitions`, `write_file`,
    `replace`, `list_directory`, `glob_search`, `search_file_content`.
    Full external-access approval / audit flow preserved. 9 unit tests.
  - **`shell_tools.rs`** — `run_shell_command` with 30-second hard timeout,
    denylist validation, Windows PowerShell dispatch with `&&`→`;` rewriting.
    2 unit tests.
  - **`web_tools.rs`** — `web_search` (DuckDuckGo HTML scraping) and
    `web_fetch` (URL GET with 30-second timeout, 10 000-char truncation).
    Static regex patterns compiled once via `Lazy`. Starlink-resilient timeouts.
    4 unit tests.
  - **`memory_tools.rs`** — `save_memory` delegating to the long-term memory
    store's atomic write path. 1 unit test.
  - **`registry.rs`** — central `execute_tool(name, args, ctx)` async entry
    point dispatching all 12 named tools. `get_tool_definitions()` and
    `get_available_tool_definitions()` return LLM-facing JSON schemas.
    5 unit tests.
  - **`mod.rs`** — flat re-exports of all tool functions plus `ToolContext` and
    `ToolError` so callers can write `tools::read_file(...)` or go through the
    registry.
  - **`src/acp/tools.rs`** reduced to a single `pub use crate::tools::*;`
    re-export shim — all existing call-sites in `src/acp/mod.rs` continue to
    compile unchanged; all 11 existing ACP tool tests preserved and still pass.
  - **`src/lib.rs`** — added `pub mod tools;` to expose the new module
    publicly.

- **CPU router tool-execution loop** (`src/router/cpu_router.rs`) — AI: Claude Sonnet 4.6
  - New `route_with_tools(req, context, max_iterations)` method implementing
    the full agent/tool loop directly inside `CpuRouter`:
    - Serializes message history to raw JSON so `tool_call_id` fields survive
      round-trips (the typed `grok_api::Message` does not carry this field).
    - Each iteration re-deserializes to typed messages, calls `route()` (which
      already handles Starlink back-off retries via the backend), then checks
      for tool calls.
    - No tool calls → returns the final `RouterResponse` immediately.
    - Tool calls present → dispatches each through `tools::registry::execute_tool`,
      appends results as `"tool"` role messages, and loops.
    - Exhausts `max_iterations` → `RouterError::MaxToolIterations(n)`.
  - 2 new tests: happy path (no tools → text returned) and exhaustion path
    (infinite tool calls → `MaxToolIterations` after 3 iterations).

- **`RouterError` new variants** (`src/router/router_error.rs`) — AI: Claude Sonnet 4.6
  - `ToolError(String)` — hard tool execution failure (tool name + message).
  - `MaxToolIterations(u32)` — loop exhausted its iteration budget; the `u32`
    is the cap that was hit, making error messages self-documenting.

### Test Results
  - **37/37 tools + ACP shim tests pass** (zero failures introduced).
  - **3/3 new CPU router tool-loop tests pass**.
  - Pre-existing failures in `agent::router`, `bayes::engine`, and
    `tests/integration_tests.rs` are unrelated to this change.


### Added

- **Unified memory module** (`src/memory/`)
  - New four-tier memory hierarchy replacing the scattered `Vec<Value>`,
    `Vec<ConversationItem>`, and bare file-append patterns that existed before.
  - **`types.rs`** — shared types used across all tiers: `ChatMessage` (with
    `to_api_value()`, token estimation, builder constructors for system/user/
    assistant/tool roles), `MemoryEntry` (UUID-keyed persistent fact with tags
    and `MemorySource`), `EpisodeSummary` (completed-session metadata),
    `MemoryKind` enum, and the `estimate_tokens` helper (1 token ≈ 4 chars).
  - **`short_term.rs`** — `ShortTermMemory`: bounded, auto-trimming conversation
    buffer.
    - Configurable limits: `max_messages` (default 50) and `max_tokens` (default
      6 000 estimated tokens).
    - System messages are pinned at index 0 and never trimmed; `push_system()`
      replaces an existing system message rather than appending.
    - `push_tool_result(tool_call_id, content)` for OpenAI-compatible tool
      messages.
    - `clear_keep_system()` mirrors the `/clear` slash-command behaviour.
    - `to_json_messages()` / `From<&ShortTermMemory>` emit the
      `Vec<serde_json::Value>` format expected by `AppRouter` and all legacy
      `chat_completion_with_history` call sites — **zero changes needed at call
      sites**.
    - `recent(n)` for sliding-window summarisation.
    - 22 unit tests.
  - **`long_term.rs`** — `LongTermMemory`: structured, persistent fact store.
    - Dual-file storage: `~/.grok/memory.json` (canonical, machine-readable) +
      `~/.grok/memory.md` (human-readable mirror regenerated on every save).
    - Atomic write-then-rename on every flush — a Starlink drop mid-write never
      corrupts the live store.
    - Exact-text deduplication: saving an identical fact returns the existing ID.
    - `search(query)` — case-insensitive substring match across fact text and
      tags; results sorted newest-first.
    - `by_tags(&[&str])` — filter facts that carry **all** of the supplied tags.
    - `by_source(source)` — filter by `MemorySource` (User / Inferred / System).
    - `to_prompt_section(max_facts)` — Markdown block ready for system-prompt
      injection, capped at 20 facts by default.
    - Free functions `save_fact_to_default_store` and `load_prompt_section` for
      call sites that don't hold a `LongTermMemory` instance.
    - 19 unit tests.
  - **`episodic.rs`** — `EpisodicMemory`: archive of completed sessions.
    - Each session stored in `~/.grok/sessions/<session_id>/` with
      `episode.json` (summary + key facts) and `transcript.json` (full
      `Vec<ChatMessage>`).
    - `save(summary, transcript)` — atomic write for both files.
    - `update_summary(summary)` — patch key facts after AI summarisation without
      re-writing the transcript.
    - `list()` / `refresh()` — sorted most-recent-first; result cached in
      memory between calls.
    - `recent(n)`, `exists(id)`, `delete(id)`.
    - `to_prompt_context(max_episodes)` — Markdown section of recent episodes
      with key facts for system-prompt injection.
    - Backward-compat free functions `save_episode_from_session` and
      `list_episode_ids` so `utils/session.rs` callers keep working.
    - 17 unit tests.
  - **`working.rs`** — `WorkingMemory`: project context injection.
    - Thin typed wrapper over `utils::context` (no duplicated file-discovery
      logic).
    - `load_for_project(dir)` — highest-priority single context file.
    - `load_and_merge(dir)` — all context files merged, deduplicated.
    - `from_content(str)` — construct from pre-loaded text (tests / templates).
    - `to_prompt_section()` — returns the formatted block or an empty string
      when no context is loaded (safe to unconditionally append).
    - `reload()` — re-reads from disk mid-session for `/reload-context`.
    - `append(extra)` / `set_content(content)` — runtime enrichment with skill
      definitions or per-session rules.
    - `display_summary()` — one-liner for the `/context` command.
    - 17 unit tests.
  - **`mod.rs`** — `MemoryStore` unified facade.
    - `new_for_session(model, project_dir, base_system_prompt)` — boots all
      four tiers, builds an enriched system prompt (base + working context +
      long-term facts) and pushes it into short-term memory.
    - `remember(fact, tags)` / `remember_inferred(fact, tags)` — convenience
      wrappers around `LongTermMemory::save_fact`.
    - `save_episode(title)` — archives the current short-term transcript to
      episodic memory.
    - `reload_context()` — reloads working memory and rebuilds the system
      prompt in-place.
    - `build_system_prompt()` — returns the assembled prompt string without
      mutating state (for logging / display).
    - `status_line()` — one-liner suitable for the session footer.
    - `recent_episode_context(n)` — pulls recent episode summaries for
      system-prompt injection.
    - `minimal()` — isolated per-call temp-dir store for unit tests and
      single-shot command handlers.
    - 13 unit tests.
  - **Total: 97 / 97 new memory unit tests pass** (`cargo test --lib memory`).

- **`acp/tools.rs` — `save_memory` migrated to `LongTermMemory`**
  - The bare `OpenOptions::append` implementation is replaced with a call to
    `memory::long_term::save_fact_to_default_store`.
  - Gains atomic writes, deduplication, structured JSON storage, and the
    Markdown mirror — all transparently, with no change to the tool's public
    interface or the model's function-calling schema.

- **CPU Router module** (`src/router/`)
  - New unified AI dispatch layer that routes every inference request through a
    single `CpuRouter` + `GrokBackend` stack instead of talking to the Grok API
    directly at each call site.
  - **`backend.rs`** — async `Backend` trait (via `async-trait`) with `kind()`,
    `is_available()`, and `async send()`. `BackendKind` enum (`Grok`) derives
    `PartialEq`/`Eq` for pattern-matching in the router.
  - **`cpu_router.rs`** — `CpuRouter` dispatches requests to the matching backend
    based on the model-name prefix (`"grok-*"` → `GrokBackend`). Falls back to
    the first available backend for unrecognised prefixes. Manual `Debug` impl so
    `Arc<CpuRouter>` can be used inside `AppRouter`.
  - **`request.rs`** — `RouterRequest` with typed `Vec<grok_api::Message>` and
    `Vec<ToolDefinition>` fields. Builder helpers: `with_temperature()`,
    `with_max_tokens()`, `with_tools()`, `with_json_tools()` (accepts raw
    `Vec<Value>` from existing call sites without a double-serde round-trip).
    `ToolDefinition` / `FunctionDefinition` match the OpenAI/xAI function-calling
    schema so they serialise cleanly to the wire format.
  - **`response.rs`** — `RouterResponse` with `text`, `tool_calls`, `raw` JSON,
    `model`, and `usage` (`UsageStats`). Convenience helpers `has_tool_calls()`,
    `text_or_empty()`, and `into_message_with_finish_reason()` — the last one
    converts a `RouterResponse` back into the `MessageWithFinishReason` type used
    throughout the rest of the codebase, enabling zero-change call sites.
  - **`router_error.rs`** — `RouterError` enum with variants:
    `BackendUnavailable`, `BackendError`, `Serialization`, `Network` (Starlink
    drop / timeout), `Auth` (HTTP 401 — fatal, never retried), `RateLimit`
    (HTTP 429 — retried with back-off), `Unknown`.
  - **`backends/grok.rs`** — `GrokBackend` wraps the existing `GrokClient`:
    - `new(api_key)` and `new_with_timeout(api_key, timeout_secs)` constructors.
    - **Starlink-resilient retry loop**: up to 4 retries with exponential
      back-off (`BASE * 2^attempt`) capped at 30 s plus random jitter (0–500 ms)
      to avoid thundering-herd on reconnect.
    - Smart error classification: auth errors abort immediately; network errors
      and rate-limits are retried; backend/serialisation errors are not.
    - Inner `GrokClient` is configured with `max_retries = 1` so retry logic
      lives entirely in `GrokBackend::send`, not in two layers at once.
    - 12 unit tests covering construction, back-off math, error classification,
      and retryability decisions.
  - **`app_router.rs`** — `AppRouter`: a `Clone`-able (`Arc<CpuRouter>`) shim
    that exposes the **same async method signatures as `GrokClient`**:
    - `chat_completion(message, system_prompt, temperature, max_tokens, model)`
    - `chat_completion_with_history(messages, temperature, max_tokens, model, tools)`
    - Accepts `&[serde_json::Value]` messages and `Option<Vec<Value>>` tools so
      existing call sites compile without touching their method bodies.
    - 3 unit tests: rejects empty key, accepts placeholder key, clone shares Arc.
  - Added `async-trait = "0.1"` to `Cargo.toml`.
  - Registered `pub mod router` in `src/lib.rs`.
  - **19 / 19** new router unit tests pass (`cargo test --lib router`).

- **AppRouter wired into all CLI and display call sites**
  (`src/cli/commands/chat.rs`, `src/cli/commands/code.rs`,
  `src/display/interactive.rs`, `src/utils/client.rs`)
  - Added `initialize_router(api_key, timeout_secs) -> Result<AppRouter>` to
    `utils/client.rs` alongside the legacy `initialize_client` (kept for
    `acp/mod.rs` which has not yet been migrated).
  - **`cli/commands/chat.rs`** — `handle_chat`, `handle_single_chat`, and
    `handle_interactive_chat` now use `AppRouter` instead of `GrokClient`.
    Constructor changed from `initialize_client(key, timeout, retries, limits)`
    to `initialize_router(key, timeout)`. Method call bodies are unchanged.
  - **`cli/commands/code.rs`** — `handle_code_action` and all four inner
    handlers (`handle_code_explain`, `handle_code_review`, `handle_code_generate`,
    `handle_code_fix`) use `AppRouter`. Unused `RateLimitConfig` and
    `initialize_client` imports removed.
  - **`display/interactive.rs`** — `start_interactive_mode` constructs
    `AppRouter::new(api_key, 30)` instead of `GrokClient::new(api_key)`.
    `run_interactive_loop`, `send_to_grok`, and `run_simulation` updated to
    accept `&AppRouter`.

### Pending

- `acp/mod.rs` migration to `AppRouter` (tracked as Task 83) — the ACP session
  handler still constructs `GrokClient` directly; it will be migrated in the
  next pre-release cycle once the session-state refactor is complete.

### Source
- AI (Claude Sonnet 4.6)

---

## [0.1.8] - 2026-04-02

### Added

- **Bayesian Intent Router** (`src/agent/router.rs`, `src/bayes/*`)
  - Implemented a lightning-fast, pre-LLM Bayesian routing layer that intercepts user input before it hits the expensive model.
  - Features:
    - **Belief Graph** (`belief_graph.rs`): Tracks probabilities of different intents (`intent_edit`, `intent_shell`, `intent_search`, `intent_question`) and meta-states (`need_clarification`, `low_confidence`) that sum to 1.0.
    - **Bayesian Updater** (`updater.rs`): Mathematically exact Bayesian updates (prior * likelihood / normalization) with a built-in decay factor (0.1) for unmatched hypotheses to ensure intended actions bubble up.
    - **Text Heuristics** (`likelihoods.rs`): Maps keywords (e.g., "edit", "run", "careful") to high-weight likelihood spikes.
    - **Clarification Gate**: If the probability of `need_clarification` exceeds 0.4 (e.g., user says "be careful, don't delete"), the router intercepts the chat loop, prevents the API call, and asks the user to clarify. It dynamically decays this probability (`reset_clarification`) once triggered so it doesn't get stuck in a loop.
    - **System Hint Injection**: For high-probability intents (like editing or running a shell command), the router invisibly appends a system hint to the prompt (e.g., `[System: High probability of needing tool 'replace'. Please use it if appropriate.]`) guiding the LLM toward the correct tool.
  - Added new configuration flags to `[experimental]` in `config.toml`:
    - `enable_bayesian_router`: Master switch to turn the router on/off (defaults to `false`).
    - `show_belief_graph`: Toggles real-time visual output of the engine's internal state.
  - Added `/bayes` (or `/beliefs`) interactive chat command to toggle the ASCII bar chart visualization on the fly during a session.
  - Full suite of unit tests covering Bayesian math, probability normalization, and router state transitions.

- **Terminal Auth setup wizard** (`src/cli/commands/setup.rs`)
  - Implemented the ACP **Terminal Auth** entry point: `grok setup`
  - Declared in the ACP `initialize` response as `{ "type": "terminal", "args": ["setup"] }`
  - ACP clients such as Zed automatically launch `grok setup` when no API key is configured,
    presenting the interactive wizard inside their built-in terminal
  - Features:
    - Colorful welcome banner with link to `https://console.x.ai/`
    - Detects and offers to replace an already-configured key (`GROK_API_KEY` env var or
      `~/.grok/.env`)
    - Masked input via **crossterm** raw mode — characters echo as `*` as you type, with
      full Backspace and Ctrl-C support
    - Falls back to plain `stdin` when raw mode is unavailable (CI / piped environments)
    - Basic format validation (length ≥ 20 chars, no whitespace, `xai-` prefix warning)
    - Live verification against `https://api.x.ai/v1/models` with **Starlink-resilient**
      exponential back-off (up to 4 retries: 3 s → 6 s → 12 s); auth failures (HTTP 401)
      are treated as fatal and abort immediately
    - Persists the key to `~/.grok/.env` (in-place update preserves other entries)
    - Unix: restricts `.env` file to mode `0600` (owner read/write only)
    - Prints next-step instructions on success
  - Source: AI (Claude Sonnet 4.6)

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

### Added

- **ACP Registry Authentication Requirements** (`src/acp/protocol.rs`, `src/cli/commands/acp.rs`, `src/cli/commands/setup.rs`, `src/cli/app.rs`)
  - The ACP Registry requires agents to support at least one of **Agent Auth** or **Terminal Auth**.
    grok-cli previously only declared `env_var` auth (API key via environment variable), which is
    not accepted by the registry.
  - Added **Terminal Auth** declaration to the `authMethods` array in the `initialize` response:
    ```json
    { "id": "grok-setup", "type": "terminal", "args": ["setup"] }
    ```
    When Zed (or any ACP client) detects the user has no API key configured, it launches
    `grok setup` in its built-in terminal to run the interactive wizard.
  - Added `args: Vec<String>` field to `AuthMethod` (serialised as `"args"`, skipped when empty)
    and a new `AuthMethod::terminal()` constructor alongside the existing `env_var()` one.
  - Implemented **`grok setup`** subcommand (`src/cli/commands/setup.rs`) — an interactive
    terminal wizard that:
    1. Checks whether `GROK_API_KEY` is already configured (env var or `~/.grok/.env`).
    2. Prompts the user to paste their xAI API key (echo disabled on interactive terminals).
    3. Validates the key format (length, no whitespace, warns if missing `xai-` prefix).
    4. Tests the key against `https://api.x.ai/v1/models` with up to 3 Starlink-resilient
       retries (3 → 6 → 12 s back-off). Auth errors (401) abort immediately.
    5. Saves the key to `~/.grok/.env` as `GROK_API_KEY="<key>"`, preserving any other
       existing lines. Sets `0600` permissions on Unix.
    6. Prints a success summary with next-steps hints.
  - Source: AI (Claude Sonnet 4.6) — triggered by ACP Registry auth requirements doc.

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