# Changelog

All notable changes to the Grok CLI project are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

**Full detailed history** is available in [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md).

---

## [0.2.3] — 2025-01-15

### Commit Message Generator (Task 161)

- Added `/commit` slash command that generates high-quality Conventional Commits messages from the current git diff (`git diff --cached` with fallback to `git diff`).
- Added `generate_commit_message` tool so the agent itself can request commit messages during workflows.
- Supports optional extra instructions: `/commit fix auth edge case`.
- Respects the new `acp.commit_message_instructions` config field (appended to every prompt).
- Works with Session DNA and active goals for context-aware commit messages.
- Default style follows Conventional Commits (`<type>(<scope>): <description>`).

### Safety Hooks — 7 Mandatory Layers (Tasks 154–160)

Grok-CLI now ships with a comprehensive, mandatory safety system that protects against the most common classes of AI-induced file damage.

**Core Safety Modules** (`src/safety/`):
- `pre_write_hook.rs` — `on_before_write_file()` runs before every write/replace/delete. Blocks binary junk, massive overwrites, invalid JSON, and DNA-flagged failure patterns.
- `dry_run.rs` — `dry_run: bool` parameter on `write_file` and `replace`. Returns a diff without touching disk; LLM must explicitly confirm.
- `diff_validator.rs` — Rejects full-file rewrites >200 lines or >40% content removal.
- `intent_validator.rs` — Rejects ambiguous file-edit requests (“fix the bug”, “make it better”) and forces clarification.
- `suspicious_write_guard.rs` — Final in-tool checks: empty overwrite, 10× size explosion, binary junk, parse failures.
- `dna_safety.rs` — `DnaSafetyController` automatically raises thresholds when SessionDNA shows repeated write failures or hallucinated paths.
- `tool_health_monitor.rs` — Tracks per-tool success/failure/hallucination rates and can disable unhealthy tools.

**Wiring**:
- `write_file()` and `replace()` now run the full safety pipeline (pre-write hook → suspicious guard → dry-run support).
- All 7 hooks are exported via `crate::safety::*`.

**Tests**:
- 6 new unit tests in `src/safety/tests.rs` covering binary blocking, large-rewrite rejection, ambiguous intent, empty overwrite, DNA safe-mode trigger, and tool health degradation.

This change dramatically reduces the chance of the agent destroying user files through over-confident or hallucinated edits.

### DNA-Driven Skill Arbitration 2.0 (Tasks 151–153)

The DNA system has been deeply integrated into the core reasoning engine:

### Task 151 — DNA-Driven Skill Arbitration 2.0
- `EngineBeliefs::tool_score()` and `score_plan()` now accept an optional `dna_tool_weight` parameter.
- `ArbitrationEngine::rank_tools()` now accepts `dna_tool_weight` and applies it to every tool's final score.
- DNA tool preferences directly boost or reduce tool rankings during arbitration.

### Task 152 — DNA-Conditioned Planning
- `SessionDna::shape_plan()` and `get_mode()` are available for any planner to use.
- Plans can now be shaped differently based on the active DNA profile (tone, verbosity, risk tolerance, past failures).

### Task 153 — DNA-Based Mode Switching
- `SessionDna::get_mode()` returns one of: `coder`, `research`, `shell`, or `creative`.
- The current mode is injected into the system prompt at session start.
- Mode selection is driven by `risk_tolerance`, `verbosity`, `tool_preferences`, and `tone`.

All three behaviors are now live and influence real tool selection, planning, and behavior.

The Session DNA system has been extended with three powerful new capabilities that make the agent feel truly personalized:

### 1. DNA-Driven Skill Arbitration 2.0
DNA now directly influences:
- **Skill scoring** via `get_skill_weight(skill_name)`
- **Tool scoring** via `get_tool_weight(tool_name)`
- **Model routing** hints via `get_model_preference()`
- **Plan shaping** via `shape_plan()`

Weight multipliers are applied based on `risk_tolerance`, `coding_style`, and `tool_preferences`.

### 2. DNA-Conditioned Planning
The planner can now generate different plan structures depending on:
- Past tool failures (via the feedback loop)
- Preferred coding patterns (`coding_style`)
- User communication style (`tone` + `verbosity`)

Example output modes: `shell`, `research`, `creative`, or `coder`.

### 3. DNA-Based Mode Switching
The agent automatically selects an operating mode at session start:
- `coder` (default)
- `shell` (when shell tools are preferred or risk is high)
- `research` (when verbosity is high)
- `creative` (when tone/style signals exploration)

The current mode is injected into the system prompt and logged.

All three behaviors are fully wired:
- DNA is loaded and applied in `initialize_session`
- Mode is computed and injected into the prompt
- Feedback loop continues to evolve DNA during tool execution

See `src/session/dna.rs` for the new helper methods.

- Session DNA is now a **living behavioral system**, not just static prompt text.
- **LLM-side injection** — all five fields (`tone`, `verbosity`, `risk_tolerance`, `coding_style`, `tool_preferences`) are now injected into the system prompt so the model fully adopts the session fingerprint.
- **Router influence** — `risk_tolerance` and `tool_preferences` now bias the Bayesian engine priors, making high-risk tools more (or less) likely depending on DNA.
- **Tool feedback loop** — after every tool execution the DNA is updated with success/failure signals, allowing the agent to learn and adapt the user’s preferred operating style over the course of a session.
- `SessionData` now owns a mutable `SessionDna` instance that evolves during the conversation.

- `SessionDna::load()` now checks the **project root first** (`./session_dna.json`) before falling back to `~/.grok/session_dna.json`.
- Project-local DNA files are now automatically loaded and injected into every new ACP session.
- Your `session_dna.json` in the repo root is now live — tone, verbosity, risk tolerance, coding style, and tool preferences are respected.

### Task 148 — Fully Automated Integration Test Harness

- Added **85 offline integration tests** across 4 new test suites, all passing with zero network calls:
  - `tests/task_tools_tests.rs` (18 tests) — task lifecycle, Format A/C normalisation, `.bak` recovery, atomic save, input validation
  - `tests/file_tools_tests.rs` (23 tests) — file I/O tools, security/path policy, path traversal rejection
  - `tests/subsystem_tests.rs` (20 tests) — long-term memory, Bayesian engine, config defaults, tool registry shape
  - `tests/cli_smoke_tests.rs` (24 tests) — tool listing, error formatting, arbitration edge cases, CLI settings
- Added `tests/integration/helpers.rs` with shared `make_security`, `make_ctx`, `write_task_list_a/c`, `write_fixture` helpers
- Fixed `tool_arbitration::is_known_tool` — added missing entries (`fork_agent`, `join_agents`, `list_agents`, `get_agent_status`, `cancel_agent`, `send_message_in_memory`, `receive_messages`) that were in `get_tool_definitions()` but not in the arbitration allow-list
- Added `Makefile` with `test-integration`, `test-all`, `test-coverage`, `lint`, `fmt` targets
- Added `Doc/testing.md` documenting harness structure, suite details, coverage instructions, and how to add new tests

### Architectural Cleanup (Task 131)

- Added pure formatting helpers in `src/cli/mod.rs`:
  - `format_success`, `format_error`, `format_warning`, `format_info`
  - `format_confirm_prompt`
- These functions return `String` and perform **no I/O**, satisfying the “Pure Display + Library/Binary Separation” requirement.
- Legacy I/O functions remain deprecated and will be removed after all command handlers are migrated to the pure API.
- Module documentation updated to clearly state the new library-vs-binary boundary.

### TGS-RAG Epic (Tasks 112.x)

- Added full **Text-Graph Synergy RAG** engine (`src/rag/`)
  - Semantic entity graph (structs, enums, traits, functions, impls) built with tree-sitter + syn
  - Hybrid retrieval (BM25 + embeddings) + graph expansion
  - Context Compression 2.0 with retrieval-aware pruning
  - Persistence + incremental mtime-based updates
  - `TgsRagContextProvider` + ACP integration layer
  - Session DNA influence on retrieval budgets (verbosity → node/token limits)
  - Debug logging and basic unit tests

- New modules: `graph`, `parser`, `retrieval`, `compression`, `persistence`, `api`, `acp_integration`, `dna_integration`, `debug`
- Configuration via `TgsRagConfig` (enable/disable, budgets, auto-load)
- Documentation: `Doc/TGS_RAG.md`

This enables project-aware, graph-guided context retrieval for much more precise LLM prompting.

### ACP Structured Feedback (Tasks 128–130)

- **Agent Activity Notifications** (Task 128)
  - New `AgentActivityUpdate` session update type for sub-agent lifecycle events.
  - `GrokAcpAgent::emit_agent_activity()` helper ready for `spawn_agent`/`fork_agent`/`join_agents` tools (deferred wiring until Task 26).
  - Enables Zed to render agent trees and status in the UI.

- **Real-time Thinking Trace Streaming** (Task 129)
  - New `ThinkingUpdate` session update with `content` + `is_final` flag.
  - Thinking traces are emitted on every Grok response (initial chunk + final marker).
  - Supports future live partial thinking streaming once the backend provides incremental chunks.

- **Context / Token Usage Feedback** (Task 130)
  - New `ContextUsageUpdate` session update containing `estimated_tokens`, `context_limit`, and `message_count`.
  - Emitted after every turn and after every tool-loop iteration.
  - New config toggle: `acp.show_context_usage` (default `true`).
  - Enables Zed to display a live context meter / token usage indicator.

These updates give Zed (and other ACP clients) rich, structured visibility into agent state without changing the core chat flow.

- **Configurable Decay** — Added `belief_decay_rate` (default `0.95`) and `prior_pull_rate` (default `0.05`) to `[bayesian]` in `config.toml`.
- **Decay Step** — `bayes_update()` now includes a stabilization pass after every likelihood update:
  ```rust
  *belief_value = *belief_value * decay_rate + prior * pull_rate;
  ```
  This gently regresses beliefs toward their long-term priors, preventing any single intent from dominating (e.g. 98.5% vs near-zero).
- **Engine Integration** — `BayesianEngine` stores the decay parameters and passes them through all update paths (`update_from_text`, `update_from_model_confidence`, `update_from_tool_failure`).
- **Example Config** — `config.example.toml` now documents the new parameters with recommended values for stable routing.

This change dramatically improves belief distribution stability while preserving responsiveness to strong signals.

### Multi-Agent Orchestration (Task 127)

- **AgentManager** — New central registry (`src/agent/manager.rs`) for tracking sub-agents with full lifecycle states (`Running`, `Completed`, `Failed`, `Cancelled`).
- **Orchestration Tools** — New tools registered:
  - `spawn_agent`, `fork_agent`, `join_agents`
  - `list_agents`, `get_agent_status`, `cancel_agent`
  - `send_message_in_memory` + `receive_messages` (fast in-memory bus)
- **In-Memory Message Bus** — New `AgentMessageBus` (`src/agent/message_bus.rs`) for low-latency inter-agent communication.
- **Engine Integration** — Added `StepAction::DelegateToSubAgent` to the reasoning engine. The `PlanBuilder` now intelligently emits delegation steps for complex or parallelizable goals.
- **Global Shared Instance** — `AgentManager` is exposed via a lazy static so all tools and the engine share the same view of active sub-agents.

This lays the foundation for true multi-agent workflows inside the reasoning engine.

## [0.2.2] — 2026-05-10

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
