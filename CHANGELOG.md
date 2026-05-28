# Changelog

All notable changes to the Grok CLI project are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

**Full detailed history** is available in [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md).

---

## [Unreleased]
<<<<<<< HEAD

## [0.2.2] — 2026-05-10
=======

### Fixed — Task Tool Data-Flow, Format Detection, Anti-Hallucination & Session Hardening

**Root cause chain that caused Grok to fabricate task information:**

1. `task_get` only handled two task-file layouts; the bot project's plain JSON array
   format was silently parsed as "0 entries" → TOOL ERROR → LLM hallucinated.
2. Even when a TOOL ERROR was returned, the LLM ignored it and invented task data.
3. The system prompt's wording ("return fields verbatim") was misread by the model
   as permission to return JSON even on failure.
4. `chat_logger::init` replaced the global logger unconditionally on every call, so
   a reconnect wiped any active session → "no active session" warning on every turn.
5. `SecurityPolicy::add_trusted_directory` had no dedup check, so the same path was
   pushed 3–4 times per session, polluting every TOOL-ERROR log entry.

#### `src/tools/task_tools.rs`

- **Format C** (`[{…}, {…}, …]`) added to `task_get` — plain top-level JSON array,
  the layout used by the bot project.  Format detection order is now:
  - A: `{"tasks":[…]}` (grok-cli standard)
  - C: `[{…},…]` (plain array — checked before B so arrays are not misidentified)
  - B: `{"0":{…},"1":{…},…}` (numeric-indexed object)
- `total` count in the not-found error now covers all three formats.

#### `src/acp/mod.rs`

- **Anti-hallucination guard**: after every `TOOL ERROR` response is pushed to
  `session.messages`, an immediate `role:"system"` message is injected:
  > ⚠️ STOP — the tool call above returned an error. You MUST NOT fabricate…
  This fires right before the LLM generates its reply and overrides the tendency
  to invent task information.
- **System prompt rewritten** (guidelines 7–9): separated success path ("reply in
  plain English using title, status, priority, description from the tool result")
  from failure path ("reply ONLY with 'I could not retrieve task N. Error: …'
  — Do NOT return any JSON, do NOT guess the task title").
- **History trimmer** now protects the `role:"system"` message at index 0 from
  eviction.  Previously `drain(0..trim_count)` could delete the system prompt
  after a long conversation, causing the LLM to lose all task-management
  instructions and revert to hallucinating.
- **DATA-TRACE logging** added at three points in `handle_chat_completion`:
  1. Before each API call: message-history layout (`[i]role:NB`) written to both
     `tracing::info!` and the persistent tool log.
  2. After the LLM responds: finish reason, tool-call count, text preview.
  3. After each tool result is pushed: tool name, call ID, byte count, preview.
  Enable full per-message body previews with `RUST_LOG=grok_cli::acp=debug`.

#### `src/utils/chat_logger.rs`

- **`init` is now idempotent** — if a `ChatLogger` already exists in `GLOBAL_LOGGER`
  the call is a no-op, so a Zed reconnect no longer wipes an active session and
  causes subsequent `log_user` calls to warn "no active session".
- **`reinit`** added for callers that intentionally want to replace the logger
  (tests, explicit reset scenarios).

#### `src/acp/security.rs`

- **`add_trusted_directory` deduplicates**: checks `!self.trusted_directories.contains`
  before pushing, eliminating the 4× duplicate entries seen in TOOL-ERROR logs.

#### `tests/tool_data_flow.rs` — new integration test file (20 tests)

Pins down every handoff point in the data pipeline:

| Test | What it asserts |
|---|---|
| `task_get_format_a/b/c_finds_task_by_id` | all three layouts return correct JSON |
| `task_get_format_b_key_does_not_equal_id` | inner `id` field used, not outer key |
| `task_get_format_c_finds_string_subtask_id` | `"60.1"` string IDs resolved |
| `tool_result_message_has_correct_role_and_content` | `role:"tool"` + `tool_call_id` set |
| `tool_error_message_contains_actionable_guidance` | TOOL ERROR has "Do NOT repeat" guard |
| `trimmer_never_removes_system_prompt` | system message pinned at index 0 |
| `trimmer_preserves_tool_result_after_evicting_user_turn` | tool result survives trim |
| `full_pipeline_format_a/b/c_task60_survives_to_llm_context` | end-to-end pipeline intact |
| `task_create_then_task_get_round_trip` | write + read consistency |
| `task_get_missing_file_returns_descriptive_error` | no panic on missing file |

**Build result**: `cargo build` clean, **672/672 tests pass**, zero Clippy errors.

*Source: AI (Claude Sonnet 4.6) — 2025-07-16*

---


### Fixed & Hardened — All Tool Implementations + `task_list.json` Rebuild

**Scope**: Every tool module (`agent_tools`, `discovery_tools`, `file_tools`,
`lsp_tools`, `mcp_tools`, `memory_tools`, `notebook_tools`, `plan_tools`,
`shell_tools`, `skill_tools`, `system_tools`, `task_tools`, `web_tools`) plus
the `cpu_router` tool-dispatch loops and the `.zed/task_list.json` data file.

#### Cross-cutting changes (all tool files)

- **`tracing::warn!`** added at every `return Err(…)` site across all 13 tool
  modules — every failure path now emits a structured diagnostic before
  propagating the error.
- **`tracing::debug!`** added on success paths in `system_tools` for lightweight
  observability.
- The `utils::network` helpers (`detect_network_drop`, `calculate_retry_delay`)
  are now wired into every network-calling tool — previously they existed but
  were called by nothing.

#### Per-file changes

**`web_tools.rs`**
- Fixed a **UTF-8 truncation panic**: `&text[..10_000]` (byte-index slice on a
  UTF-8 string) replaced with a `char_indices().nth(10_000)` boundary search
  that is always safe.
- Added **empty-query guard** to `web_search` — an empty string now returns
  `Err` immediately instead of hitting DuckDuckGo.
- Added **3-retry loop** (with Starlink-aware backoff) to both `web_search` and
  `web_fetch` using `detect_network_drop` + `calculate_retry_delay`.
- Added `tracing::warn!` before every non-2xx / parse-error / retry-exhausted
  return in `web_fetch`.
- New unit test: `web_search_empty_query_returns_error`.

**`agent_tools.rs`**
- Added **3-retry loop** around `router.chat_completion(…)` in `spawn_agent`.
- **Atomic write** for `send_message` — data is now written to a `.tmp` file
  and then renamed into place, preventing corruption on a mid-write Starlink
  drop.
- `merge_agent_results` — added **empty-slice guard** (`Ok("No agent results
  to merge.")`); fixed return type to `Result<String>`; applied Clippy
  suggestions (`clamp`, `sort_by_key`).
- `tracing::warn!` added before every `return Err(…)` in all five functions.
- New unit tests: `merge_empty_returns_ok`, `merge_single_passthrough`,
  `merge_prefers_longer`.

**`mcp_tools.rs`**
- Added a **30-second `tokio::time::timeout`** around the entire MCP protocol
  exchange — previously the function had zero built-in timeout and could hang
  indefinitely.
- Added a **2-retry connect loop** (inside the timeout budget) for transient
  MCP server connection failures.
- `tracing::warn!` added at security-validation failure, connect failure
  (per-retry and final), tool-call failure, serialisation failure, and timeout.

**`lsp_tools.rs`**
- `tracing::warn!` added at every error site across `lsp_query`,
  `get_diagnostics`, `get_hover`, `find_definition`, `find_references`, and
  `extract_symbol`.
- `get_diagnostics` now logs a structured warning with `timeout_secs` and
  `path` fields when `cargo check` times out, and warns separately when it
  exits non-zero.

**`discovery_tools.rs`**
- `remote_trigger` now returns `Err` for any HTTP method other than GET, POST,
  or PUT — previously unknown methods silently fell through to POST.
- Added **3-retry loop** with `detect_network_drop` + `calculate_retry_delay`
  to `remote_trigger`.
- `tracing::warn!` added at every error site in `tool_search`, `cron_create`,
  and `remote_trigger`.

**`task_tools.rs`**
- **Atomic write** via `.json.tmp` + `fs::rename` in `save_task_file` — covers
  both `task_create` and `task_update`, preventing `task_list.json` corruption
  on a mid-write process kill.
- `task_update` now **validates the `priority` field** (`high / medium / low`)
  the same way `task_create` already did.
- `tracing::warn!` at every error return site.
- New tests: `update_rejects_invalid_priority`, `update_rejects_invalid_status`.

**`notebook_tools.rs`**
- **Atomic write** via `.ipynb.tmp` + `fs::rename` — notebook is never left in
  a partially-written state.
- Added **empty-source guard** — a blank or whitespace-only cell source now
  returns `Err` immediately.
- `tracing::warn!` at every error site (resolve, trust, cell_type, source,
  read, parse, cells array, mkdir, serialise, write, rename).
- New test: `rejects_empty_source`.

**`plan_tools.rs`**
- `let _ = save_state(…)` replaced with `if let Err(e) = save_state(…)`
  + `tracing::warn!` — state-save failures are no longer silently discarded.
- **Branch-name sanitizer** rejects names containing `..` or starting with `-`
  (guards against argument injection and path-traversal in git commands).
- `tracing::warn!` at every error site in `enter_worktree` and `exit_worktree`.
- New tests: `enter_worktree_rejects_dotdot_branch`,
  `_rejects_dash_prefix_branch`, `_rejects_empty_branch`,
  `_rejects_empty_path`.

**`memory_tools.rs`**
- Added **empty-fact guard** (`fact.trim().is_empty()` check) — `save_memory`
  now returns `Err` instead of silently writing a blank fact.
- `map_err` closure now calls `tracing::warn!` before wrapping the error.
- Replaced trivially-true `assert!(result.is_ok() || result.is_err())` test
  with `save_memory_empty_fact_returns_err` (and a whitespace-only variant).

**`shell_tools.rs`**
- `tracing::warn!` added on process-spawn failure and on timeout.
- `tracing::warn!(exit_code, command)` added when the child process exits
  non-zero (the `Ok(…)` return is preserved for backward compatibility).

**`skill_tools.rs`**
- Added a **32 KB input-size cap warning** (`tracing::warn!` when `input.len()`
  exceeds 32 768 bytes) to prevent context-window overflow.
- `tracing::warn!` at every error site in `execute_skill` and
  `list_available_skills`.
- Replaced trivially-true `assert!(r.is_ok() || r.is_err())` test with
  `list_skills_returns_ok`.

**`system_tools.rs`**
- `sleep_for(0)` now emits `tracing::warn!` — a zero-second sleep is almost
  certainly a caller mistake.
- `tracing::debug!` added for successful `sleep_for` and `synthetic_output`
  calls.
- `tracing::warn!` at error sites in `synthetic_output`.

#### `.zed/task_list.json` rebuild

The file was completely rebuilt by a Node.js script:

| Problem | Fix |
|---|---|
| JSONC trailing commas (invalid `serde_json`) | Stripped — file is now standard JSON |
| IDs jumped from 10 → 26 (gap 11–25) | Old 26–40 renumbered to 11–25; all subsequent IDs shifted |
| Duplicate "Implement Context Engine 2.0" (IDs 34 & 52) | Old 52 removed; old 34 (now 19) retained |
| Duplicate "Implement Agent Health Monitoring" (IDs 56 & 70) | Old 70 removed; old 56 (now 40) retained |
| Tasks 86 & 87 referenced non-existent dep IDs 12 & 22 | Dependencies cleared to `[]` / remapped correctly |
| Status drift — completed work still marked `pending` | Old 58 (Agent Error Logging) → `done`; old 67 (Enhanced Logging) → `done`; old 45 (Tool Failure Recovery) → `in_progress` |

Two new tasks added at the end:
- **Task 85** — "Harden read_file: JSON/JSONC Validation and Error Reporting" (`done`)
- **Task 86** — "Harden All Tool Implementations" (`in_progress`, deps: [85])

Final state: **86 tasks, IDs 1–86, no gaps, no duplicates, valid standard JSON.**
Original backed up to `.zed/task_list.bak2.json`.

**Build result**: `cargo build` clean, **650/650 tests pass**, zero Clippy errors.

*Source: AI (Claude Sonnet 4.6) — 2025-07-16*

---

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
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a

### Performance & Startup

- **ACP stdio instant startup** (Tasks 121–126)
  - `AppRouter` creation is now fully lazy (`OnceLock`) — only instantiated on the first actual chat-completion request (task 123).
  - `SecurityManager` and `HookManager` initialization moved to lazy `OnceLock` getters (task 126). The expensive `new()` + CWD canonicalization + trusted-directory logic no longer runs during `GrokAcpAgent::new()`.
  - Duplicate `SecurityManager::new()` calls removed from the constructor.
  - `start_acp_stdio` path profiled and trimmed (task 125): heavy work (router, security, hooks, capabilities) is now deferred until the first `initialize` / `session/new` or first prompt.
  - Result: `grok acp stdio` now starts in milliseconds even when an API key is present; the agent can immediately respond to Zed’s `initialize` request declaring its auth requirements.

### Bug Fixes

- **ACP protocol `text` → `content` rename** (`src/acp/protocol.rs`)
  `ToolCallContent::Text` was serialised with `"type": "text"`, but
  `agent-client-protocol-schema` ≥ 0.12 renamed that variant to `"content"`.
  Zed was logging `"skipped malformed list entry … unknown variant text"` and
  silently dropping every tool-call content block sent to the editor.
  Fixed by changing `#[serde(rename = "text")]` → `#[serde(rename = "content")]`.
  *(Source: human observation / AI fix)*

- **Slash commands blocked / `PromptResponse` delayed** (`src/acp/mod.rs`, `src/cli/commands/acp.rs`)
  `handle_chat_completion` held the `sessions` write lock for its **entire duration**,
  including all async API calls (which can take 10–300 s).  Any slash command that
  needed even a read lock (`/context`) or a write lock (`/clear`, `/model`, `/think`)
  was blocked for that entire duration.  Additionally, `save_session_to_disk` was
  called *before* `responder.respond(EndTurn)` in all three paths of
  `handle_session_prompt_v2`; if the read lock inside `save_session_to_disk` was
  contested, Zed never received the `PromptResponse` and the turn appeared to hang.

  Two fixes applied:
  1. **Lock-window reduction** — the write lock is now held only for the brief setup
     phase (user-message push, trimming, compression).  The session state is cloned
     out before the lock is released, the API call loop runs with no lock held, and a
     brief write lock is re-acquired only for per-iteration and final state syncs.
  2. **`PromptResponse`-before-save ordering** — `responder.respond(EndTurn)` is now
     called *before* `save_session_to_disk` in all three response paths so Zed always
     closes the turn immediately, regardless of lock contention on the disk save.
  *(Source: human report / AI analysis & fix)*

- **Context-window overflow in multi-turn tool loops** (`src/acp/acp.rs`)
  The token-budget trimming (steps 1-4) ran only **once**, before the tool
  loop, but each loop iteration appends an assistant message plus one or more
  tool-result messages.  After many iterations of large file reads the context
  could balloon to 12 M tokens, triggering a 400 from the API.
  Fixed by re-running `truncate_tool_results`, the count guard, and
  `trim_to_token_budget` at the **top of every loop iteration** before the API
  call.  A `WARN` log is emitted whenever mid-loop trimming fires.
  *(Source: human log report / AI fix)*

### Highlights

- **ACP connection-layer rewrite** (Task 111.3) — Replaced manual JSON-RPC dispatch with `Agent.builder() + ByteStreams`. Full typed handlers for `initialize`, `session/new`, `session/prompt`, etc.
- **Session persistence & fork** (Tasks 111.5, 111.7) — Disk-based session save/restore + `session/fork` support with fresh Bayesian engine.
- **ACP schema migration** (Task 111.1–111.2) — 11 types replaced with `agent-client-protocol` crate re-exports; wire-format verified.
- **ACP startup performance** (Tasks 121–126) — `AppRouter`, `SecurityManager`, and `HookManager` are now created lazily via `OnceLock`. `grok acp stdio` starts instantly and can answer `initialize` before any API key or heavy component is loaded.
- Multiple bug fixes for Zed compatibility (slash commands, thinking mode, file URI handling).

**655/655** lib tests + integration tests pass. Clippy clean.

See [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md) for the complete unreleased notes and all prior versions.

---

## [0.1.10] — 2024-10-04 (Summary)

- Task Graph Engine, Skill Auto-Activation, **Session DNA**, Plugin Sandbox
- External directory access with approval + audit logging
- Chat logging, search, and replay
- ACP workspace access fixes

See [Doc/SESSION_DNA.md](Doc/SESSION_DNA.md) for details on the Session DNA system.

---

## [0.1.9] and earlier

See the full archive in [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md) for detailed entries from v0.1.9 back to the initial public release (v0.1.2).

---

**Links**

- Repository: https://github.com/microtech/grok-cli
- Issues: https://github.com/microtech/grok-cli/issues
- Buy Me a Coffee: https://buymeacoffee.com/micro.tech
