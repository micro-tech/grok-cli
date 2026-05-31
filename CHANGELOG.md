# Changelog

All notable changes to the Grok CLI project are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

**Full detailed history** is available in [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md).

---

## [Unreleased]

### Bayesian Belief Stabilization

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
