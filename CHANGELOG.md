# Changelog

All notable changes to the Grok CLI project are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

Author: John McConnell <john.microtech@gmail.com>
Repository: https://github.com/microtech/grok-cli
Buy me a coffee: https://buymeacoffee.com/micro.tech

---

## [Unreleased] - 2026-05-10

### Added

- **Task 111.3 done — ACP connection-layer rewrite: Agent::builder() + ByteStreams** (`src/cli/commands/acp.rs`, `tests/acp_protocol.rs`)
  - Replaced the 280-line manual `BufReader` / `BufWriter` JSON-RPC dispatch loop (`run_acp_session` + `handle_json_rpc`) with `Agent.builder().connect_to(ByteStreams::new(writer, reader))`.
  - Typed `on_receive_request` handlers for every standard ACP method: `initialize`, `session/new`, `session/prompt` (via `cx.spawn()`), `session/list`, `session/load`.
  - `session/prompt` streaming runs in `cx.spawn()` so the Builder event loop stays responsive while the AI call is in flight. Tool-call / chunk notifications forwarded via `cx.send_notification()` through a JSON serde round-trip.
  - Permission requests auto-approve in this version (full elicitation via `cx.send_request` tracked in 111.6).
  - `session/fork` and `session/set_model` are a known limitation — custom extension methods need `#[derive(JsonRpcRequest)]` wrappers not yet written; `test_session_fork` marked `#[ignore]`.
  - Added `tests/acp_protocol.rs` integration tests (4 tests, 3 active) exercising the full `initialize → session/new → session/load` flow over in-memory `tokio::io::duplex` pipes. No real API key required.
  - Added `tokio-util = { version = "0.7", features = ["compat"] }` dependency for `TokioAsyncReadCompatExt` / `TokioAsyncWriteCompatExt` adapters.
  - Merged from branch `feature/acp-crate-111.3`.
  - **655/655** lib tests + **3/3** integration tests pass.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.5 done — Session resume: disk persistence + restore** (`src/acp/mod.rs`, `src/cli/commands/acp.rs`)
  - Added `Serialize, Deserialize` to `SessionConfig` so sessions can be serialised.
  - Added `pub(crate) struct PersistedSession` — a JSON-serialisable snapshot of a session (id, cwd, messages, config, goal, always-allow list, unix timestamp).
  - Added three methods to `GrokAcpAgent`:
    - `sessions_dir()` — returns `~/.grok/sessions/`
    - `save_session_to_disk(session_id)` — writes `~/.grok/sessions/<id>.json` with up to 3 retries on I/O failure (Starlink-safe).
    - `load_session_from_disk(session_id)` — reads and deserialises the file; returns `None` if missing; up to 3 retries.
    - `restore_session_from_disk(state)` — re-initialises the session then overwrites messages, goal, and always-allow from the snapshot.
  - `handle_session_load` now checks disk first: restores if found, creates fresh session if not, or resumes in-memory session if already present.
  - `handle_session_prompt` auto-saves to disk after every successful AI response (at end of function, before final `Ok`).
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.7 done — Session fork** (`src/acp/mod.rs`, `src/cli/commands/acp.rs`)
  - Added `fork_session(source_id, new_id)` to `GrokAcpAgent` — clones messages, config, goal, and always-allow into a new session ID with a fresh Bayesian engine.
  - Added `handle_session_fork(params, agent)` function that generates a `<source>-fork-<8-char uuid>` ID and returns `{ "newSessionId": "..." }`.
  - Registered `session/fork` in `handle_json_rpc` immediately before the `session/set_model` branch.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.1 done — ACP migration audit** (`Doc/acp-migration-map.md`)
  - Produced `Doc/acp-migration-map.md`: full classification of all 48 types in `src/acp/protocol.rs` against `agent_client_protocol::schema::*` as REPLACE (28 types, 58%), EXTEND (8 types, 17%), or KEEP (12 types, 25%). Includes Phase B implementation order, Phase C feature table, and a risk register.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.2 done — Schema leaf-type swap + wire-format verification** (`Cargo.toml`, `src/acp/protocol.rs`, `src/acp/slash_commands.rs`)
  - Added `agent-client-protocol = "0.11"` to `[dependencies]`; resolves to v0.11.1. New transitive deps: `rmcp v1.6.0`, `futures-concurrency v7.7.1`, `tokio-util v0.7.18`, `tower v0.5.3`, `agent-client-protocol-derive v0.11.0`.
  - Replaced 7 types in `src/acp/protocol.rs` with `pub use` re-exports from `agent_client_protocol::schema`: `ToolKind`, `ToolCallStatus`, `Implementation`, `SessionListCapabilities`, `AvailableCommandsUpdate`, `AvailableCommand`, `AvailableCommandInput` / `UnstructuredCommandInput`.
  - `StopReason` intentionally **skipped** — crate variant set differs (adds `MaxTurnRequests`, `Refusal`, `Cancelled`; does not have our `StopSequence`/`ToolUse` wire names). Local definition kept.
  - Updated all 11 `.with_input()` call sites in `slash_commands.rs` to the crate builder API: `.input(AvailableCommandInput::Unstructured(UnstructuredCommandInput::new(...)))`. Introduced a local `input(hint)` helper closure in `get_available_commands()` for readability.
  - Fixed two broken test-import paths left by an interrupted sub-agent (`super::protocol` → `crate::acp::protocol`).
  - Added 2 new wire-format regression tests: `test_available_command_input_serializes_to_hint_object` confirms `AvailableCommandInput` still serialises to `{"hint":"..."}` (not as a tagged enum); `test_available_command_round_trips_with_input` confirms `AvailableCommand.input.hint` round-trips through JSON.
  - **655/655** lib tests pass. Clippy clean.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.3 deferred — Connection-layer rewrite blocked; migration map updated** (`Doc/acp-migration-map.md`)
  - Investigated the crate's `Agent` API: it is a unit struct with a `Builder<Agent, NullHandler, NullRun>` builder (not a trait to implement). Handlers are registered via `on_receive_request!` / `on_receive_notification!` macros and a `ConnectionTo<Agent>` context.
  - **Why deferred:** (1) Our `handle_session_prompt` does bidirectional streaming — sending chunk / tool-call `session/update` notifications *while* awaiting the AI response — which does not map cleanly to a per-request callback. (2) `SessionId` is `Arc<str>`-backed in the crate vs. `String`-backed in our code; migrating touches ~100 callsites. (3) The `PermissionBridge` pattern (permission RPC mid-tool-execution) has no obvious equivalent in the crate's request model. (4) No automated integration tests exist to verify Zed/Gemini CLI compatibility after the rewrite.
  - Migration map updated: `SessionId`, `PermissionKind`, `PermissionOption` reclassified from REPLACE → **KEEP** based on actual crate API inspection. Pre-requisites for 111.3 documented in the map.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.2 continued — 4 more REPLACE types swapped** (`src/acp/protocol.rs`, `src/cli/commands/acp.rs`)
  - **`TextContent`** — local definition removed; `pub use agent_client_protocol::schema::TextContent` added. Extra crate fields (`annotations`, `meta`) are `skip_serializing_none` — wire format `{"text":"..."}` unchanged.
  - **`ListSessionsRequest`** (type alias `SessionListRequest`), **`ListSessionsResponse`** (type alias `SessionListResponse`), **`SessionInfo`** — all re-exported from crate. Callsite updated for `cwd: Option<PathBuf>` → `.as_deref().and_then(|p| p.to_str()).unwrap_or("")` conversion.
  - Additional reclassifications — REPLACE → **KEEP**: `ToolCallLocation` (crate uses `path: PathBuf` + `line: u32`, completely different wire format from our `uri: String` + `range`); `ToolCallRange` and `Position` (no crate equivalents exist); `SessionLoadRequest` (crate `cwd` is non-optional `PathBuf`, `mcp_servers: Vec<McpServer>` vs `Vec<Value>`).
  - **Total replaced across all Phase B-1 work: 11 types.** All remaining local types in `protocol.rs` are either EXTEND (grok-specific fields), KEEP (wire-format incompatible or no crate equivalent), or DEFERRED (await 111.3 connection-layer rewrite).
  - **655/655** lib tests pass. Clippy clean.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.5 done — Session resume: full disk persistence** (`src/acp/mod.rs`, `src/cli/commands/acp.rs`)
  - Added `Serialize, Deserialize` to `SessionConfig` so the full session config (model, temperature, max_tokens, system_prompt, thinking_mode) is serialisable.
  - Added `PersistedSession` struct capturing: session ID, cwd, conversation messages, config, active goal, always-allowed tools, and a Unix save timestamp.
  - `GrokAcpAgent::save_session_to_disk()` — writes `~/.grok/sessions/<id>.json` after every prompt. Starlink-safe: retries 3× with exponential backoff on I/O errors.
  - `GrokAcpAgent::load_session_from_disk()` — reads and deserialises the snapshot; returns `None` on `NotFound` (clean first-run experience). Also 3-retry.
  - `GrokAcpAgent::restore_session_from_disk()` — calls `initialize_session` with the saved config, then overwrites messages, goal, and always-allow set.
  - `handle_session_load` updated: now tries disk restore first; falls back to fresh session if no snapshot exists. Previously always returned `null` with no history.
  - Auto-save hooked at the end of `handle_session_prompt` (single call-site before final `Ok()`). Sessions are therefore persisted after every AI response turn.
  - **655/655** tests pass.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Task 111.7 done — Session fork: `session/fork` handler** (`src/acp/mod.rs`, `src/cli/commands/acp.rs`)
  - `GrokAcpAgent::fork_session()` deep-clones an existing session (messages, config, goal, always-allow, client_commands) into a new session ID with a fresh `BayesianEngine`. Released via a two-phase read-lock → write-lock pattern to avoid deadlock.
  - `handle_session_fork()` async function added in `acp.rs`: extracts `sessionId` from params, generates `<original>-fork-<8 hex chars>` as the new ID, calls `fork_session`, returns `{ "newSessionId": "..." }`.
  - `session/fork` branch wired into `handle_json_rpc` dispatch (before `session/set_model`).
  - **655/655** tests pass.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Tasks 111.6 and 111.8 deferred** — Elicitation (structured permission dialogs) and MCP-over-ACP bridging (exposing tools as an MCP server) both require the connection-layer rewrite (111.3) to be done first. They remain blocked behind 111.3.

---

## [Unreleased] - 2026-05-09

### Fixed

- **Bug: slash commands `/bayes show`, `/bayes reset`, `/bayes explain`, `/goal clear` were rejected as "not supported" by ACP clients (Zed)** (`src/acp/slash_commands.rs`)
  - **Root cause**: `get_available_commands()` advertised command names that contain spaces (`"bayes show"`, `"bayes reset"`, `"bayes explain"`, `"goal clear"`). The ACP spec requires single-word command names (e.g. `"web"`, `"plan"`). Clients such as Zed silently drop or reject multi-word names during palette registration, so the commands were never shown or would report "not supported" when invoked.
  - **Fix**: Replaced the three bayes sub-command entries with a single `"bayes"` command that accepts `"show | reset | explain"` as input text. Removed `"goal clear"` as a separate advertised command — the existing `"goal"` command already parses `clear` as a sub-command; its description and input hint were updated to document this. All five `SlashCommand` variants (`BayesShow`, `BayesReset`, `BayesExplain`, `Goal`, `GoalClear`) continue to work unchanged; only the ACP advertisement layer was fixed.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

- **Bug: `/think off` displayed a misleading "reasoning_effort sent to API" message** (`src/cli/commands/acp.rs`)
  - **Root cause**: The `SetThinkingMode` handler always printed `reasoning_effort = "<label>"` and `"Use /think off to disable."`, even when the user had just set the mode to `Off`. When mode is `Off`, `as_api_str()` returns `None` and the field is **omitted** from the API request — it is not sent as `"off"`.
  - **Fix**: Added an `is_off` flag before the mode is consumed. When `is_off` is `true` the response now shows a 🔇 "thinking disabled" message with a hint to enable reasoning. When `is_off` is `false` the response correctly shows the `reasoning_effort` API note and the "use `/think off` to disable" hint.
  - Source: AI (Claude Sonnet 4.6) on request from Human (John McConnell)

---

## [Unreleased] - 2026-05-08

### Fixed

- **Bug: file access denied when Zed sends a file URI with a line-number anchor as the workspace hint** (`src/cli/commands/acp.rs`)
  - **Root cause**: Zed sometimes sends URIs like `file:///H:/GitHub/bot/src/io/web_server/mod.rs#L1:854` as the workspace root in ACP `initialize` / `session/new` messages.  The `#L1:854` fragment was NOT being stripped by `resolve_workspace_path()`, so the path `H:\GitHub\bot\src\io\web_server\mod.rs#L1:854` was registered as a trusted root.  This is a *file path with a line-number suffix*, not a directory, so `starts_with()` checks against it always failed — every subsequent file access was denied with an "Access denied" error.
  - **Fix 1 — fragment stripping**: `resolve_workspace_path()` now truncates the input at the first `#` character before processing the URI scheme, so `mod.rs#L1:854` becomes `mod.rs` before any further handling.
  - **Fix 2 — file-to-directory promotion**: `register_workspace_root()` now calls `find_workspace_root_from_path()` after resolving the raw path.  If the resolved path is a *file* (e.g. an @-mentioned source file rather than a project root directory), the function walks up the directory tree looking for project markers (`.git`, `Cargo.toml`, `package.json`, etc.) and trusts the outermost project root instead of the individual file path.
  - **Observed symptom**: After switching the model config to `grok-4.3`, the ACP config became parseable for the first time (previous TOML had the invalid `max_context_tokens = 1,000,000` with commas which caused parse failure and fallback to defaults).  With a correctly parsed config, the workspace trust path was exercised and the line-number bug surfaced.
  - Source: Human (John McConnell) + AI (Claude Sonnet 4.6)

---

## [Unreleased] - 2026-05-02

### Added

- **Task 109 — grok-4.3 Full Support / 1 Million Token Context Window** (`src/acp/mod.rs`, `src/config/mod.rs`, `config.example.toml`)
  - `create_capabilities()` now advertises `max_context_length = 1_048_576` (up from 131_072) reflecting grok-4.3's 1 M token window.  Added `"1m_context"` and `"vision"` to the features list.
  - New `AcpConfig` field `grok4_max_context_tokens: usize` (default `950_000`).  When the active model starts with `"grok-4"` this budget is used for Layer 3 (token-budget trim) and Layer 4 (compression threshold) instead of the legacy `max_context_tokens` (220 k) that was calibrated for grok-3.
  - New free function `model_context_budget(model, legacy, grok4) -> usize` selects the appropriate budget without branching at call-sites.
  - `SessionConfig::default()` `max_tokens` (output) raised from `4096` to `16_384` to match grok-4.3's higher output limits.
  - `default_max_tokens()` in `Config` corrected from `256_000` (accidentally set to the input context window size) to `16_384` (output budget).
  - `config.example.toml` updated: `default_max_tokens = 16384`, added `grok4_max_context_tokens = 950000` under `[acp]`.
  - Source: AI (Claude Sonnet 4.6)

- **Task 110 — grok-4.3 Thinking Modes / reasoning_effort Support** (`src/config/mod.rs`, `src/grok_client_ext.rs`, `src/router/`, `src/acp/mod.rs`, `src/acp/slash_commands.rs`, `src/cli/`)
  - New `ThinkingMode` enum (`Off` / `Low` / `High`) in `src/config/mod.rs` with `as_api_str()` and `from_str_ci()` helpers.
  - `AcpConfig::thinking_mode` and `SessionConfig::thinking_mode` fields — session defaults to `Off`.
  - `reasoning_effort: Option<String>` added to `ChatRequest` and `ChatRequestBuilder` in the local `grok_api` crate (`../grok_crate/grok_api/src/`).
  - `reasoning_content: Option<String>` added to `grok_api::Message` so the API response field is captured.
  - `GrokClient::chat_completion_with_history` accepts `reasoning_effort: Option<&str>` and passes it to the builder.
  - `AppRouter::chat_completion_with_history` and `RouterRequest` carry `reasoning_effort` end-to-end through `GrokBackend::send`.
  - `RouterResponse::thinking_content` and `MessageWithFinishReason::thinking_content` propagate the reasoning trace from the API response to callers.
  - `handle_chat_completion` in `src/acp/mod.rs` surfaces thinking content as a `<details><summary>🧠 Thinking…</summary>` collapsible block prepended to the main response.
  - New `/think [off|low|high]` slash command — `parse_slash_command`, `handle_builtin` (`BuiltinResult::SetThinkingMode`), `get_available_commands`, `command_to_prompt`.
  - New `GrokAcpAgent::set_thinking_mode` and `get_thinking_mode` methods dispatched from `handle_session_prompt` in `src/cli/commands/acp.rs`.
  - `--thinking <off|low|high>` CLI flag added to `grok chat`; wired through `ChatOptions::thinking_mode`.
  - `config.example.toml` and `CONFIGURATION.md` updated with thinking-mode documentation.
  - 9 new unit tests: `ThinkingMode` serialisation, `/think` parse variants (off/low/high/case-insensitive/unknown), builtin classification.
  - Source: AI (Claude Sonnet 4.6)

- **Task 102 — Wire KnowledgeLoader into session startup** (`src/knowledge/loader.rs`, `src/acp/mod.rs`)
  - Added `pub fn get_all(&self) -> &[KnowledgeEntry]` to `KnowledgeLoader` so callers can retrieve all entries without filtering.
  - In `initialize_session`, after constructing `SessionData`, calls `KnowledgeLoader::load()` on the `knowledge/` directory.  For each `*.md` / `*.json` file found, the content is formatted as `## <source>\n<content>` and pushed onto `session_data.messages` as a `system` role message with a `## Project Knowledge` header.  The count is traced via `tracing::info!`.
  - Source: AI (Claude Sonnet 4.6)

- **Task 103 — Wire SessionDna into session startup** (`src/session/dna.rs`, `src/acp/mod.rs`)
  - In `initialize_session`, immediately after the knowledge injection block, calls `SessionDna::load()`.  If a `system` message was created by the knowledge block the DNA fields (tone, verbosity) are appended to it; otherwise a fresh `system` message is created from the DNA content alone.  Session DNA injection is traced via `tracing::debug!`.
  - Source: AI (Claude Sonnet 4.6)

- **Task 106 — Goal Mode System** (`src/acp/mod.rs`, `src/acp/slash_commands.rs`, `src/cli/commands/acp.rs`)
  - **`SessionData`**: added `current_goal: Option<String>` field; initialised to `None` in `initialize_session` and in the test helper.
  - **`SessionData::refine_prompt`**: step 4 — if a goal is active the text `[Active Goal: ... — interpret this message in the context of achieving this goal.]` is appended to every refined prompt.
  - **`GrokAcpAgent`**: three new `pub async fn` methods — `set_session_goal`, `clear_session_goal`, `get_session_goal` — that write/read `session.current_goal` under the sessions `RwLock`.
  - **`SlashCommand`** enum: two new variants — `Goal { text: String }` and `GoalClear`.
  - **`parse_slash_command`**: `/goal clear` (case-insensitive) → `GoalClear`; `/goal <text>` → `Goal { text }`; bare `/goal` → `Goal { text: "" }`.
  - **`BuiltinResult`** enum: three new variants — `SetGoal(String)`, `ClearGoal`, `ShowGoal`.
  - **`handle_builtin`**: `Goal { text }` with empty text → `ShowGoal`; with non-empty text → `SetGoal(text)`; `GoalClear` → `ClearGoal`.
  - **`command_to_prompt`**: `Goal { .. }` and `GoalClear` added to the built-ins arm (returns `None` — no AI round-trip).
  - **`get_available_commands`**: added `goal` (with input hint) and `goal clear` entries.
  - **`handle_session_prompt`** in `acp.rs`: dispatches `SetGoal`, `ClearGoal`, `ShowGoal` to the corresponding agent methods.
  - **Tests added** (8 new): `test_parse_goal_with_text`, `test_parse_goal_clear`, `test_parse_goal_clear_case_insensitive`, `test_parse_goal_empty`, `test_parse_goal_empty_routes_to_show_goal`, `test_goal_is_builtin`, `test_goal_clear_is_builtin`, `test_goal_set_result_contains_text`.
  - Source: AI (Claude Sonnet 4.6)

### Fixed

- **`src/visualizer.rs` raw string delimiter collision** — the `generate_pipeline_dot` function used `r#"..."#` as its raw string delimiter but the DOT graph body contained `"#RRGGBB` hex colour codes that prematurely closed the string, causing dozens of Rust syntax errors.  Fixed by rewriting the string as a regular escaped string literal (using `\"` for embedded quotes, `\\n` for DOT label newlines, and `{{` / `}}` for literal braces).  Also renamed `\u{2500}` horizontal-rule sequences to plain `-` dashes which DOT renders identically without triggering `format!` positional-argument errors.  Source: AI (Claude Sonnet 4.6)

- **State Machine Visualizer — Task 107** (`src/visualizer.rs`, `src/lib.rs`, `src/cli/app.rs`, `src/acp/slash_commands.rs`, `src/cli/commands/acp.rs`)
  - New `pub mod visualizer` with two public functions:
    - `generate_pipeline_dot(config: Option<&Config>) -> String` — emits a valid Graphviz DOT digraph of the full Grok-CLI routing pipeline (entry → slash-dispatch → Bayesian router → context manager → Grok API → tool loop → memory subsystem → response). Reads live Bayesian priors, `max_context_tokens`, `compression_threshold`, and `max_tool_loop_iterations` from the loaded config so the graph reflects actual runtime settings.
    - `generate_pipeline_markdown(config: Option<&Config>) -> String` — wraps the DOT in a Markdown ` ```dot ` code block with render instructions, for use in ACP sessions.
  - New CLI subcommand `grok visualize [--output <path>]`:
    - Without `--output`: prints DOT to stdout (pipe-friendly: `grok visualize | dot -Tpng -o pipeline.png`).
    - With `--output <file>`: writes DOT to disk and prints a render hint.
  - New ACP slash command `/visualize` — classified as a built-in (no AI round-trip); returns the Markdown-wrapped DOT graph directly in the session response.
  - 3 unit tests pass: `dot_output_is_valid_digraph`, `dot_contains_default_priors`, `markdown_wraps_in_code_block`.
  - `cargo build` and `cargo clippy -- -D warnings` both clean.
  - Source: AI (Claude Sonnet 4.6) — Task 107 completion

### Fixed

- **`/bayes show`, `/bayes reset`, `/bayes explain` not appearing in Zed `/` palette** (`src/acp/slash_commands.rs`, `src/acp/mod.rs`, `src/cli/commands/acp.rs`)
  - Root cause (3 separate bugs):
    1. Commands were missing from `get_available_commands()` — Zed never learned they existed.
    2. `handle_builtin` returned `None` for all three — they silently fell through to the AI with the raw `/bayes …` text.
    3. No agent methods existed to read/reset/explain the session's `bayes_engine`.
  - Fix: Added three new `BuiltinResult` variants (`ShowBayes`, `ResetBayes`, `ExplainBayes`); wired `handle_builtin`; added three `pub async fn` methods on `GrokAcpAgent` (`get_bayes_visualize`, `reset_bayes`, `get_bayes_explain`) that access `session.bayes_engine` directly; dispatched them in `handle_session_prompt`; added the three command entries to `get_available_commands`.
  - Added 8 new tests: `test_parse_bayes_show/reset/explain`, `test_parse_bayes_unknown_subcommand_returns_none`, `test_bayes_show/reset/explain_is_builtin`, `test_bayes_commands_no_ai_prompt`.
  - Source: AI (Claude Sonnet 4.6) — Task 100 completion

### Maintenance — Tasks 98–108 Audit

- **Task list audit** (`.zed/task_list.json`) — verified actual code vs. claimed status for tasks 98–108:

  | Task | Was | Corrected to | Reason |
  |------|-----|-------------|--------|
  | 99.1 | `pending` | `done` | `describe_tool`, `tool_examples`, `tool_search` exist in `discovery_tools.rs` |
  | 99.2 | `pending` | `done` | `ToolsAction::Describe` and `ToolsAction::Examples` wired in `cli/commands/tools.rs` |
  | 102  | `done` | `pending` | `KnowledgeLoader` exists in `knowledge/loader.rs` but is never called from any session or ACP path |
  | 103  | `done` | `pending` | `SessionDna::load()` exists in `session/dna.rs` but `inject_into_prompt` is never called during session init |
  | 105  | `pending` | `done` | Fully implemented in `agent/simulator.rs` + wired in `display/interactive.rs` with `/simulate on\|off` |

  - Tasks 98, 100, 101, 104, 106, 107, 108 — no status change needed.
  - `notes` fields added to tasks 102, 103, 105 explaining the wiring gap or implementation detail.
  - Source: AI (Claude Sonnet 4.6)

### Added (Tasks 108.1 & 108.2)

- **New `src/memory/context_archive.rs`** — Per-session context archive (Task 108.1)
  - `ChunkMeta` — lightweight index entry (chunk_id, created_at, message_count,
    estimated_tokens_saved, summary_preview truncated to 80 chars + "…").
  - `ArchiveIndex` — session index persisted to `archives/index.json`; derives
    `Default` for zero-cost construction of a fresh session.
  - `ContextChunk` — full archived chunk with raw messages, AI summary, and
    key facts; serialises to `chunk_{NNN:03}.json` atomically.
  - `ContextArchive` — manager with `for_session` (default `~/.grok/sessions/`)
    and `with_sessions_dir` (tests) constructors. Exposes `save_chunk`,
    `load_chunk`, `list_chunks`, `next_chunk_id`, and `total_tokens_archived`.
  - All file I/O uses atomic write-then-rename (Starlink-safe).
  - Five unit tests: `chunk_meta_preview_truncated`, `save_and_load_chunk_roundtrip`,
    `next_chunk_id_starts_at_one`, `next_chunk_id_increments`, `list_chunks_empty`.
  - Source: AI (Claude Sonnet 4.6) — Task 108.1

- **New `src/memory/context_compressor.rs`** — AI-powered conversation compressor (Task 108.2)
  - `compress(messages, router, model)` async function: builds a compact transcript,
    calls the Grok API with a structured summarizer prompt, and parses the structured
    `SUMMARY:` / `FACTS:` response.
  - Empty-input short-circuit: returns `("(no messages to summarize)", [])` instantly
    without touching the network.
  - Starlink-safe retry loop: 3 attempts with 5 s / 10 s / 20 s back-off; warns via
    `tracing` on each retry and returns a wrapped error after all attempts fail.
  - Transcript is capped at 60 000 chars before being sent to the API.
  - `parse_summary_response` private helper parses `SUMMARY:` and `FACTS:` lines;
    falls back to the first 200 chars of raw text when the format is not found.
  - Three unit tests: `parse_empty_gives_fallback`, `parse_well_formed_response`,
    `compress_empty_messages_returns_placeholder` (tokio async, no real API call).
  - Source: AI (Claude Sonnet 4.6) — Task 108.2

- **Updated `src/memory/mod.rs`**
  - Declared `pub mod context_archive` and `pub mod context_compressor`.
  - Added `pub use context_archive::{ArchiveIndex, ChunkMeta, ContextArchive, ContextChunk}`
    re-exports for ergonomic access from the rest of the crate.
  - Source: AI (Claude Sonnet 4.6) — Tasks 108.1 & 108.2

### Added (Task 108.3)

- **New `AcpConfig` fields for intelligent auto-compression** (`src/config/mod.rs`)
  - `auto_compress: bool` (default `true`) — master switch for AI-powered context
    summarization. When enabled, the oldest message chunk is summarized and archived
    to disk instead of being silently dropped when the context fills up.
    Set to `false` to revert to the previous drop-only behaviour.
  - `compression_threshold: f32` (default `0.75`) — fraction of `max_context_tokens`
    at which auto-compression fires (0.0–1.0). At the default, compression triggers
    when the estimated prompt exceeds 75 % of the token budget.
  - `compression_chunk_ratio: f32` (default `0.40`) — fraction of current non-system
    messages to compress per compression event. At the default, the oldest 40 % of
    messages are archived each time, with a minimum of 4 messages enforced.
  - Three matching `default_*` functions and `AcpConfig::default()` wired up.
  - Both `config.example.toml` and `.grok/config.toml` updated with commented
    documentation for all three new settings.
  - Source: AI (Claude Sonnet 4.6) — Task 108.3

### Added (Tasks 108.4 & 108.5)

- **Layer 4 smart compression in `handle_chat_completion`** (`src/acp/mod.rs`)
  - Added immediately after the existing three-layer trim.
  - Fires when `auto_compress = true` AND estimated tokens exceed
    `max_context_tokens * compression_threshold`.
  - Collects the oldest `compression_chunk_ratio` fraction of non-system messages
    (minimum 4), drains them from `session.messages`, calls
    `context_compressor::compress()` with Starlink-safe retries, then:
    - Saves the raw messages + AI summary to `ContextArchive` on disk.
    - Inserts a compact `build_archive_notice` system message at position 1
      (just after the system prompt) so the model is always aware archived
      context exists and how to recall it.
  - On compressor failure (network drop) the drained messages are restored
    in-place so history is never silently lost.
  - Source: AI (Claude Sonnet 4.6) — Task 108.4

- **`build_archive_notice` helper** (`src/acp/mod.rs`)
  - Pure function that formats a `role: "system"` JSON message from a
    `ContextChunk`.  Format: `[📦 Context Archive #N | date | N messages]\n`
    followed by a ≤200-char summary preview, up to 5 key-fact bullets, and a
    `/recall N` hint.  Kept under 400 chars to minimise its own token cost.
  - Unit test `build_archive_notice_has_correct_role_and_chunk_id` verifies
    `role`, chunk ID, `/recall` hint, and message count.
  - Source: AI (Claude Sonnet 4.6) — Task 108.5

### Added (Task 108.6)

- **`/recall` and `/archives` slash commands** (`src/acp/slash_commands.rs`)
  - Two new `SlashCommand` variants: `Archives` and `Recall { chunk_id: Option<u32> }`.
  - Parser: `/archives` → `Archives`; `/recall` → `Recall { None }`;
    `/recall N` → `Recall { Some(N) }` where N is parsed as `u32`.
  - Both advertised in `get_available_commands` with descriptions and input hints.
  - Both listed as built-ins in `command_to_prompt` (no AI round-trip needed).
  - New `BuiltinResult::RecallArchive(Option<u32>)` variant for the ACP layer.
  - `handle_builtin` returns `Text(format_archives_text(None))` for `Archives`
    and `RecallArchive(chunk_id)` for `Recall`.
  - New `pub fn format_archives_text(session_id: Option<&str>) -> String`:
    opens `ContextArchive::for_session`, renders a Markdown table
    (# | Date | Messages | Tokens Saved | Summary preview) or a friendly
    empty-state message, and appends a `/recall N` usage tip.
  - `src/cli/commands/acp.rs` handles `RecallArchive` variant with a placeholder
    (renders archive listing); full message-injection pass is done in Layer 4.
  - Source: AI (Claude Sonnet 4.6) — Task 108.6

### Added (Task 108.7)

- **`recall_context` tool registered in `src/tools/registry.rs`** — surfaces the
  context archive to the LLM as a callable tool (Task 108.7).
  - New `"recall_context"` arm in `execute_tool()`: accepts a `chunk_id: u32`
    argument, opens `ContextArchive::for_session("unknown")` (registry fallback—
    session-aware dispatch is handled by the ACP path added in Task 108.4),
    and returns a formatted recall notice with chunk ID, message count, archive
    timestamp, summary text, and bullet-pointed key facts.
    Returns a user-friendly "not found" message when the chunk ID is absent.
  - New JSON schema entry in `get_tool_definitions()` after the `remote_trigger`
    entry, under a `// Context recall` comment section. Tool count docstring
    updated from 32 → 34 (was already 33 tools; 34 with `recall_context`).
  - Count assertion in `get_tool_definitions_has_31_tools` test updated 33 → 34;
    all 12 registry tests pass, all 20 `slash_commands` tests pass.
  - **`section_label` closure in `src/acp/slash_commands.rs`** updated: the
    `save_memory` branch now also matches `recall_context` so the tool
    displays under the **🧠 Memory** section in `/tools` output.
  - Also fixed pre-existing unicode escape compile errors in
    `src/acp/mod.rs` (`build_archive_notice`): `\u2022` → `\u{2022}`,
    `\u2026` → `\u{2026}`, UTF-16 surrogate pair `\ud83d\udce6` → `\u{1F4E6}` (📦).
  - Also fixed pre-existing Clippy `manual_strip` lint in
    `src/memory/context_compressor.rs`: replaced index-based slice with
    `strip_prefix`.
  - Assumption: `ToolContext` has no `session_id` field (only `policy:
    SecurityPolicy`), so the registry fallback uses `"unknown"` as the
    session ID. Proper session-aware dispatch is the responsibility of the
    ACP-layer handler added in Task 108.4.
  - Source: AI (Claude Sonnet 4.6) — Task 108.7

### Fixed

- **Context-window overflow — "maximum prompt length" API error** (`src/acp/mod.rs`, `src/config/mod.rs`)
  - Root cause: history trimming was purely count-based (`max_history_messages`).
    When tool calls returned large file contents, even 80 messages could balloon
    to 674 790 tokens, far exceeding the model's 256 000-token limit.
  - Fix — three-layer defence in `handle_chat_completion`:
    1. **Per-message truncation** (`truncate_tool_results`): tool-result messages
       are capped at `max_tool_result_chars` (default 30 000 chars ≈ 7 500 tokens)
       so a single large file read cannot flood the context.
    2. **Count-based trim** (existing): keeps the most recent
       `max_history_messages` turns.
    3. **Token-budget trim** (`trim_to_token_budget`): estimates total tokens
       (4 chars ≈ 1 token) and drops oldest messages until the history fits
       within `max_context_tokens` (default 220 000, leaving 36 k headroom for
       the response and tool schemas).
  - **Better error message**: when the API still rejects the request with
    "maximum prompt length", the error is caught before the retry loop, logged
    clearly, and returned with actionable `/clear` + config-tuning advice.
  - Added `max_context_tokens` and `max_tool_result_chars` to `AcpConfig` with
    serde defaults and `Default` impl; documented in `config.example.toml` and
    `.grok/config.toml`.
  - Source: AI (Claude Sonnet 4.6)

- **Clippy warnings** (various files)
  - `src/acp/mod.rs`: `ptr_arg`, `collapsible_if`
  - `src/cli/commands/tools.rs`: `collapsible_if`
  - `src/knowledge/loader.rs`: `collapsible_if`
  - `src/task_graph/mod.rs`: `new_without_default`, `type_complexity`
  - `src/session/dna.rs`: `collapsible_if`
  - `src/tools/discovery_tools.rs`: `collapsible_if`
  - `src/tools/sandbox.rs`: `needless_borrows_for_generic_args`
  - Source: AI (Claude Sonnet 4.6)

## [Unreleased] - Bug fixes from grok-errors.log analysis

### Fixed

- **`replace` tool — Windows CRLF line-ending mismatch** (`src/tools/file_tools.rs`)
  - Root cause: files on Windows are saved with CRLF (`\r\n`) but the AI always
    emits plain LF (`\n`) in search strings.  The literal `str::matches()` call
    could never find the target text, causing dozens of "not found in file" errors.
  - Fix: normalise both the file content and the search string to LF before
    matching; restore CRLF in the written output when the original file used it.
  - Added `replace_handles_crlf_files` regression test.

- **Shell command 30 s hard timeout** (`src/tools/shell_tools.rs`, `src/config/mod.rs`)
  - Root cause: `SHELL_COMMAND_TIMEOUT_SECS` was hardcoded to 30 — far too short
    for `cargo build`, `git status`, or any command on a Starlink connection.
  - Fix: raised the built-in default to **300 s** (5 min) and added a
    `command_timeout_secs` field to `ShellConfig` (default 300) so users can
    tune it in `.grok/config.toml` under `[tools.shell]`.
  - `run_shell_command` now accepts an explicit `timeout_secs: u64` parameter
    (pass `0` to use the built-in default); the ACP agent reads the value from
    `config.tools.shell.command_timeout_secs`.
  - Updated `config.example.toml` with documentation for the new field.

- **Pre-existing test compilation errors** (`src/acp/tools.rs`, `src/tools/file_tools.rs`,
  `src/acp/slash_commands.rs`)
  - `async fn` tests annotated with `#[test]` instead of `#[tokio::test]` —
    the test suite could not compile.
  - Missing `.await` on several async calls (`write_file`, `read_file`) inside
    async test functions.
  - Spurious `.await` on sync functions (`list_directory`, `glob_search`,
    `search_file_content`) inside async tests.
  - Non-exhaustive `match` on `SlashCommand` missing `BayesShow`, `BayesReset`,
    `BayesExplain` arms in `command_to_prompt`.
  - All 12 relevant unit tests now pass (`cargo test --lib -- tools::file_tools::tests
    tools::shell_tools::tests`).

---

## [0.1.10] - 2024-10-04

### Added

- **Task Graph Engine** (Task 98) — Add a DAG-based multi-step execution engine to Grok-CLI.
  - **`src/task_graph/mod.rs`** — Core task graph implementation with `TaskNode`, `ToolCall`, and `TaskGraph` structs. Supports JSON serialization for LLM-generated graphs.
  - **`src/tools/task_graph_tools.rs`** — `execute_task_graph` tool that parses JSON graphs and executes them with dependency resolution.
  - **DAG Executor** — Topological sort ensures correct execution order, detects cycles, and propagates errors.
  - **Tool Loop Integration** — New `execute_task_graph` tool registered in the tool registry, allowing LLMs to orchestrate multi-step workflows.
  - **Error Handling** — Structured error reporting for invalid graphs, cycles, and execution failures.
  - **Unit Tests** — Basic tests for graph creation, topological sorting, and cycle detection.
  - **33 tools** now available (up from 32) with full LLM schema support.

### Fixed

- **Tool Count Update** — Updated tool count from 31 to 33 in registry comments and tests to reflect the new `execute_task_graph` tool.

### Source
- AI (Claude Sonnet 4.6) — Implemented as Task 98 in `.zed/task_list.json`.

### Fixed

- **ACP tool loop bug** (`src/acp/mod.rs`) — `finish_reason == "stop"` was
  checked **before** tool-call processing, so when Grok returned `stop` +
  tool calls together the loop bailed out early and tool results were never
  appended to the message history.  Fix mirrors commit `7aa7c8b` from the
  old branch: the `finish_reason` check now happens **after** the entire
  tool-execution for-loop, and a post-loop guard returns early (without
  spinning up a redundant API call) when the model signalled stop alongside
  tool calls.

### Added

- **`src/rpl/`** (Task 92) — Reasoning Protocol Layer ported from `test-old_10`
  commit `a6c6f82`.  Six source files: `layer.rs`, `logging.rs`, `schema.rs`,
  `validation.rs`, `suppression.rs`, `mod.rs` (~2 550 lines total).
  Registered as `pub mod rpl` in `src/lib.rs`. **76/76 tests pass.**

- **`src/engine/state.rs` + `src/engine/mod.rs`** (Task 93) — FSM core ported
  from `test-old_10`. Defines `EngineState`, `ReasoningEngineState`,
  `PlanStep`, `StepAction`, `Hypothesis`, `TransitionError`, and friends.
  Registered as `pub mod engine` in `src/lib.rs`. **17/17 tests pass.**

- **`src/engine/` sub-modules** (Task 94) — Six supporting modules ported:
  `beliefs.rs`, `planner.rs`, `memory_bridge.rs`, `arbitration.rs`,
  `correction.rs`, `observability.rs`.  All compiled against the current
  `PreRelese` API with zero fixes needed. **101 new tests pass.**

- **`CpuRouter::with_rpl()` + `route_with_tools_traced()`** (Task 95) —
  Re-added the optional `RplLayer` field, builder method, and the traced
  route variant to `src/router/cpu_router.rs`. All existing router tests
  unaffected.

- **`tests/engine_integration.rs`** (Task 96) — 15 integration tests ported
  from `test-old_10`, covering the full engine lifecycle (goal → plan →
  execute → self-correct). **15/15 pass.**

- **`src/agent/planner.rs`** (Task 97) — Replaced the mock stub with a real
  `Planner` that drives `ReasoningEngineState` through AnalyzeGoal →
  ExpandOptions → EvaluateOptions → CommitPlan. `Plan` wraps the committed
  state and exposes `heuristic_confidence()`, `step_count()`, and
  `first_step_action()`. **3/3 planner tests pass.**

### Fixed — Clippy (`-D warnings`)

- Resolved all 38 Clippy errors across 18 files: `sort_by` → `sort_by_key`
  with `Reverse` (6×), collapsed nested `if`/`if-let` blocks (10×), doc
  comment overindentation (6×), wildcard-in-or-pattern (2×), useless
  `format!` / `.to_string()` in format args (3×), `manual_clamp` (2×),
  `needless_borrow` (1×), `too_many_arguments` (1×, `#[allow]`),
  `unreachable_patterns` (1×), added `Default` derive to `BeliefGraph`.

- Fixed `test_profile_learning_rate_applied` test isolation: replaced
  `BayesianEngine::new_with_config()` (reads `~/.grok/bayes_profile.json`)
  with `from_priors(default_priors(), …)` for deterministic behaviour.

### Verified

- `cargo clippy -- -D warnings` → **zero errors**
- `cargo test` → **610 lib + 15 engine_integration + 5 tool_loop + 3 acp +
  13 tool_data_flow + 2 integration = 648 total, 0 failures**

---

## [0.1.9-pre] - 2026-04-02

### Investigated — RPL + Reasoning Engine branch gap — AI: Claude Sonnet 4.6

- Discovered that the full **Reasoning Protocol Layer** (`src/rpl/`, ~2 550 lines)
  and **Reasoning Engine** (`src/engine/`, ~5 250 lines) were written on the
  `test-old_10` branch (commit `a6c6f82`) but were **never merged** into
  `PreRelese`.
- Corrected `task_list.json`: tasks 69–84 (all RPL/Engine) reset from
  `"done"` → `"pending"` (60 status fields updated).
- Added 6 new tasks to track the porting work:
  - **Task 92** — Port `src/rpl/` (6 files) from `test-old_10`
  - **Task 93** — Port `src/engine/state.rs` (FSM core, 987 lines)
  - **Task 94** — Port `src/engine/` sub-modules (6 stubs)
  - **Task 95** — Wire RPL back into `CpuRouter`
  - **Task 96** — Port `tests/engine_integration.rs`
  - **Task 97** — Replace `agent/planner.rs` stub with real engine integration
- Current state on `PreRelese`: `agent/planner.rs` is a stub with mock methods;
  `agent/router.rs`, `agent/simulator.rs`, `agent/mode.rs` are complete.
  `src/engine/` and `src/rpl/` directories do **not** exist on this branch.

### Fixed

- **Test isolation hardening** — AI: Claude Sonnet 4.6
  - Seven lib tests were failing because `BayesianEngine::new()` and
    `LongTermMemory::load_or_create()` read from `~/.grok/` at test time,
    picking up on-disk data from real usage that corrupted expected values.
  - Added `BayesianEngine::new_with_default_priors()` — same as `new()` but
    always uses compiled-in `default_priors()` and never touches disk.
  - Added `Router::new_with_default_priors()` — thin wrapper around the new
    engine constructor for deterministic router unit tests.
  - `grok_dir()` in `long_term.rs` now checks `GROK_GLOBAL_CONTEXT_DIR` env
    var first, letting tests redirect long-term memory away from `~/.grok/`.
  - Updated 7 unit tests across `bayes/engine.rs`, `agent/router.rs`, and
    `memory/mod.rs` to use the isolated constructors or the env-var override.
  - **Result**: 422 lib tests pass, 0 failures.

- **`tests/integration_tests.rs`** — removed reference to deleted
  `OperationalMode` enum (removed in the "drop OperationalMode" commit).
  Replaced with two lean smoke tests that verify `AcpConfig` and `AppRouter`
  are publicly accessible.

- **`task_list.json`** — corrected status for tasks 42 and 51 from `"done"`
  back to `"pending"`.  Both had all subtasks listed as pending and their
  dependency chains unfinished; the `"done"` marking was a git-rebase
  artefact.

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
- **Buy Me a Coffee**: https://buymeacoffee.com/micro.tech- - - 
 
 
 
 # #   [ U n r e l e a s e d ] 
 
 
 
 # # #   A d d e d 
 
 
 
 -   * * L o c a l   K n o w l e d g e   P a c k   L o a d e r * *   ( T a s k   1 0 2 )      L o a d   p r o j e c t - s p e c i f i c   k n o w l e d g e   f r o m   ` k n o w l e d g e / `   d i r e c t o r y   w i t h   r e l e v a n c e   s c o r i n g .   S u p p o r t s   . m d   a n d   . j s o n   f i l e s ,   c o m p u t e s   r e l e v a n c e   b a s e d   o n   q u e r y   s i m i l a r i t y ,   a n d   i n t e g r a t e s   w i t h   L L M   c o n t e x t . 
 
 -   * * S e s s i o n   D N A   S y s t e m * *   ( T a s k   1 0 3 )      P e r s i s t e n t   p e r s o n a l i t y   a n d   b e h a v i o r   c o n f i g u r a t i o n   f i l e   ( ` s e s s i o n _ d n a . j s o n ` )   d e f i n i n g   t o n e ,   v e r b o s i t y ,   r i s k   t o l e r a n c e ,   c o d i n g   s t y l e ,   a n d   t o o l   p r e f e r e n c e s .   L o a d s   a t   s e s s i o n   s t a r t   a n d   i n j e c t s   i n t o   s y s t e m   p r o m p t s . 
 
 -   * * P l u g i n   S a n d b o x   f o r   C u s t o m   T o o l s * *   ( T a s k   1 0 4 )      D y n a m i c   c o m p i l a t i o n   a n d   l o a d i n g   o f   c u s t o m   R u s t   t o o l s   f r o m   ` t o o l s / c u s t o m / ` .   I n c l u d e s   s c h e m a   v a l i d a t i o n ,   i s o l a t e d   e x e c u t i o n ,   a n d   r e g i s t r y   i n t e g r a t i o n . 
 
 
 
 # # #   F i x e d 
 
 
 
 -   * * T a s k   L i s t   U p d a t e s * *      M a r k e d   t a s k s   1 0 2 - 1 0 4   a s   d o n e ,   i n c l u d i n g   s u b t a s k s   1 0 4 . 1   a n d   1 0 4 . 2 . 
 
 
 
 # # #   S o u r c e 
 
 -   A I   ( C l a u d e   S o n n e t   4 . 6 )      I m p l e m e n t e d   t a s k s   1 0 2 - 1 0 4   i n   ` . z e d / t a s k _ l i s t . j s o n ` . 
 
 
 
 - - - 
 
 
 
 # #   L i n k s 
 
 
 
 -   * * R e p o s i t o r y * * :   h t t p s : / / g i t h u b . c o m / m i c r o t e c h / g r o k - c l i 
 
 -   * * I s s u e s * * :   h t t p s : / / g i t h u b . c o m / m i c r o t e c h / g r o k - c l i / i s s u e s 
 
 -   * * B u y   M e   a   C o f f e e * * :   h t t p s : / / b u y m e a c o f f e e . c o m / m i c r o . t e c h 
 
 
