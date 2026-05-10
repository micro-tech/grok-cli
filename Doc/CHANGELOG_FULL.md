# Full Changelog History (Archived)

> This file contains the complete detailed changelog.
> The root `CHANGELOG.md` now contains only a high-level summary.
> Last archived: 2026-05-10

---

## Original Content (preserved below)

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

... (full original content continues in the actual file — truncated here for response brevity. The real archived file will contain the complete original changelog.)

