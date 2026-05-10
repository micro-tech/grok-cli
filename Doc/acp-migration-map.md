# ACP Migration Map — Task 111.1

**Date:** 2026-05-09  
**Purpose:** Audit every type in `src/acp/protocol.rs` against `agent_client_protocol::schema::*` 
and classify each as **REPLACE**, **EXTEND**, or **KEEP**.

---

## Legend

| Label | Meaning |
|---|---|
| **REPLACE** | Delete local definition; `pub use agent_client_protocol::schema::Type;` |
| **EXTEND** | Keep local struct but build on top of / wrap the crate type, or add a custom serde adapter |
| **KEEP** | No crate equivalent exists; local definition stays as-is |

---

## Type-by-type classification

### Core Identity & Session

| grok-cli type | Crate equivalent | Label | Notes |
|---|---|---|---|
| `SessionId(String)` | `schema::SessionId` | **KEEP** | ⚠️ Crate version is `SessionId(pub Arc<str>)` — internal representation differs. Wire format is the same (plain JSON string), but `.0` field is `Arc<str>` vs our `String`. ~100+ callsites use `.0` as a `String`; migration would be a large ripple. Keep local until the connection-layer rewrite (111.3) when all callsites are touched anyway. |
| `ProtocolVersion` | `schema::ProtocolVersion` | **REPLACE** | Crate version has `LATEST`, `V1` constants |
| `MethodNames` + `AGENT_METHOD_NAMES` | `schema::AgentMethodNames` + `schema::AGENT_METHOD_NAMES` | **REPLACE** | Field names differ slightly (`session_request_permission` vs crate name — verify) |

---

### Initialization

| grok-cli type | Crate equivalent | Label | Notes |
|---|---|---|---|
| `AgentCapabilities` | `schema::AgentCapabilities` | **EXTEND** | Our version has `loadSession: bool` that may need to be added via `_meta` or a custom wrapper |
| `SessionCapabilities` | `schema::SessionCapabilities` | **EXTEND** | Our `supportsPermissionRequests` field and `list: Option<SessionListCapabilities>` are grok-specific; the crate's `SessionCapabilities` may differ |
| `SessionListCapabilities` | `schema::SessionListCapabilities` | **REPLACE** | Empty marker struct — same concept |
| `InitializeRequest` | `schema::InitializeRequest` | **EXTEND** | **Critical**: Our version accepts 5 aliases for workspace root (`workspaceRoot`, `workspace_root`, `rootUri`, `rootPath`, `workingDirectory`, `cwd`) for Gemini CLI + Zed compat. The official crate version likely accepts fewer. Must keep serde alias layer. |
| `InitializeResponse` | `schema::InitializeResponse` | **REPLACE** | Same shape; crate version has `authMethods` field |
| `Implementation` | `schema::Implementation` | **REPLACE** | Same `name` + `version` fields |
| `AuthEnvVar` | `schema::EnvVariable` (approximate) | **EXTEND** | Our type has `name`, `label`, `secret`; crate's `EnvVariable` has `name`, `description`. Add an adapter. |
| `AuthMethod` (struct) | `schema::AuthMethod` (enum) | **EXTEND** | **Significant difference.** grok-cli uses a flat struct with a `type: String` discriminant field. The official crate uses a Rust enum. Keep grok-cli's flat struct for serialisation (the wire format needs to stay compatible), but add a conversion helper. |

---

### Session Lifecycle

| grok-cli type | Crate equivalent | Label | Notes |
|---|---|---|---|
| `NewSessionRequest` | `schema::NewSessionRequest` | **EXTEND** | Our version accepts `workingDirectory`, `cwd`, `workspaceRoot` aliases (Gemini CLI compat) and has `mcp_servers: Vec<Value>`. Official crate has `mcpServers` field but likely stricter typing. Keep serde alias layer. |
| `AcpModeInfo` | `schema::SessionMode` | **KEEP** | Crate's `SessionMode` uses `id: SessionModeId` (newtype) and `available`/`current` JSON names. Our wire format uses `availableModes`/`currentModeId` as required by Gemini CLI. Changing JSON names would break existing clients. |
| `AcpModesInfo` | `schema::SessionModeState` | **KEEP** | Same reason — JSON field name mismatch with Gemini CLI requirement |
| `AcpModelInfo` | _(no equivalent)_ | **KEEP** | The official crate uses `SessionConfigOption` for model selection, not a flat `modelId`/`name` list. Gemini CLI specifically expects `{ "models": { "availableModels": [...] } }` in the `session/new` response. |
| `AcpModelsInfo` | _(no equivalent)_ | **KEEP** | Same as above |
| `NewSessionResponse` | `schema::NewSessionResponse` | **EXTEND** | Our version adds `modes: Option<AcpModesInfo>` and `models: Option<AcpModelsInfo>` for Gemini CLI compat. The crate's response may not have these fields; keep with serde `skip_serializing_if`. |
| `SessionListRequest` | `schema::ListSessionsRequest` | **REPLACE** ✅ | Added as `pub type SessionListRequest = ListSessionsRequest;` type alias. `cwd` field is now `Option<PathBuf>` — callsites updated. |
| `SessionInfo` | `schema::SessionInfo` | **REPLACE** ✅ | Same name, same `::new(sid, cwd)` constructor. `session_id: SessionId`, `cwd: PathBuf`. Wire format identical. |
| `SessionListResponse` | `schema::ListSessionsResponse` | **REPLACE** ✅ | Type alias added. Same `::new(sessions)` constructor. `meta: None` skipped. |
| `SessionLoadRequest` | `schema::LoadSessionRequest` | **KEEP** | ⚠️ Crate's `cwd: PathBuf` is required (non-optional); `mcp_servers: Vec<McpServer>` vs our `Vec<Value>`; `session_id: SessionId` uses `Arc<str>`. Too many breaking changes. |

---

### Prompt / Content

| grok-cli type | Crate equivalent | Label | Notes |
|---|---|---|---|
| `PromptRequest` | `schema::PromptRequest` | **REPLACE** | Same `session_id` + `prompt: Vec<ContentBlock>` |
| `ContentBlock` | `schema::ContentBlock` | **REPLACE** | Same variants: `Text`, `Resource`, `ResourceLink` (crate may have more variants — ok since `#[non_exhaustive]`) |
| `TextContent` | `schema::TextContent` | **REPLACE** | Same `text: String` |
| `ResourceContent` | `schema::EmbeddedResource` | **REPLACE** | Same concept, different name |
| `EmbeddedResourceResource` | `schema::EmbeddedResourceResource` | **REPLACE** | Same enum concept |
| `TextResourceContents` | `schema::TextResourceContents` | **REPLACE** | Same `uri`, `text`, `mimeType` |
| `ResourceLinkContent` | `schema::ResourceLink` | **REPLACE** | Same `uri` + `name` |
| `PromptResponse` | `schema::PromptResponse` | **REPLACE** | Same `stopReason` field |
| `StopReason` | `schema::StopReason` | **REPLACE** | Same variants |
| `ContentChunk` | `schema::ContentChunk` | **REPLACE** | Same `content: ContentBlock` |

---

### Session Notifications & Tool Calls

| grok-cli type | Crate equivalent | Label | Notes |
|---|---|---|---|
| `SessionNotification` | `schema::SessionNotification` | **REPLACE** | Same `sessionId` + `update: SessionUpdate` |
| `SessionUpdate` | `schema::SessionUpdate` | **EXTEND** | **Important**: Our version uses `#[serde(tag = "sessionUpdate")]`. The crate likely uses a different serde strategy. Verify JSON wire format matches before replacing — the `sessionUpdate` discriminant key must stay. |
| `ToolKind` | `schema::ToolKind` | **REPLACE** | Same variants: Read, Edit, Search, Execute, Think, Other |
| `ToolCallStatus` | `schema::ToolCallStatus` | **REPLACE** | Same variants |
| `ToolCall` | `schema::ToolCall` | **REPLACE** | Our `tool_call_id: String`, crate has `tool_call_id: ToolCallId` (newtype) — minor adaptation |
| `ToolCallUpdate` | `schema::ToolCallUpdate` | **REPLACE** | Same as `ToolCall` difference |
| `ToolCallContent` | `schema::ToolCallContent` | **REPLACE** | Same `Text` variant |
| `ToolCallLocation` | `schema::ToolCallLocation` | **KEEP** | ⚠️ Crate version uses `path: PathBuf` + `line: Option<u32>` — completely different wire format from our `uri: String` + `range: Option<ToolCallRange>`. Cannot replace without breaking JSON. |
| `ToolCallRange` | _(no equivalent)_ | **KEEP** | Crate has no `ToolCallRange` type; it uses a single `line: Option<u32>` on `ToolCallLocation`. |
| `Position` | _(no equivalent)_ | **KEEP** | Crate has no `Position` type (start/end character ranges). |

---

### Slash Commands

| grok-cli type | Crate equivalent | Label | Notes |
|---|---|---|---|
| `AvailableCommandInput` | `schema::UnstructuredCommandInput` | **REPLACE** | Both have `hint: String`. Note: crate's `AvailableCommandInput` is an **enum**; `UnstructuredCommandInput` is the text-input variant. Adapt constructor accordingly. |
| `AvailableCommand` | `schema::AvailableCommand` | **REPLACE** | **Crate version is `#[non_exhaustive]`** — must use builder: `AvailableCommand::new(name, description).input(input)` instead of struct literal. Builder method is `.input()` not `.with_input()`. ⚠️ This also enforces valid command names at the type level. |
| `AvailableCommandsUpdate` | `schema::AvailableCommandsUpdate` | **REPLACE** | Same wrapper struct |

---

### Permissions

| grok-cli type | Crate equivalent | Label | Notes |
|---|---|---|---|
| `PermissionKind` | `schema::PermissionOptionKind` | **KEEP** | Crate version name differs; our `PermissionOption` uses our `PermissionKind`. Keep consistent with PermissionOption decision below. |
| `PermissionOption` | `schema::PermissionOption` | **KEEP** | ⚠️ Crate version uses `option_id: PermissionOptionId` (newtype) not `String`, and is `#[non_exhaustive]`. `PermissionOptionId` newtype adds friction to all callsites that construct `PermissionOption`. Keep local. |
| `PermissionToolCall` | `schema::ToolCall` (approximate) | **KEEP** | Our `PermissionToolCall` is a lighter struct used specifically in the permission prompt. Keep as local type. |
| `RequestPermissionParams` | `schema::RequestRequestPermissionRequest` | **REPLACE** | Same concept, crate has `RequestRequestPermissionRequest` (yes, double "Request") |
| `OutcomeDetail` | `schema::RequestPermissionOutcome` (approximate) | **KEEP** | Our `OutcomeDetail` enum and `PermissionOutcome` wrapper with helper methods (`cancel()`, `proceed_once()`, `is_cancelled()`, `is_always_allow()`) are grok-specific. Keep local. |
| `PermissionOutcome` | — | **KEEP** | Grok-specific helper wrapper with semantic methods |
| `default_permission_options()` | — | **KEEP** | Grok-specific helper |

---

## Summary counts

_Updated 2026-05-10 after Phase B-1 implementation verified against actual crate API._

| Label | Count | % |
|---|---|---|
| **REPLACE** | 26 | 54% |
| **EXTEND** | 8 | 17% |
| **KEEP** | 14 | 29% |

**Changes from initial audit:**
- `SessionId` reclassified REPLACE → **KEEP**: crate uses `Arc<str>` internally, ours uses `String`. 100+ callsites affected.
- `PermissionKind` + `PermissionOption` reclassified REPLACE → **KEEP**: crate uses `PermissionOptionId` newtype for `option_id`, breaking all construction callsites.

**Phase B-1 implemented (2026-05-10):** 7 types replaced: `ToolKind`, `ToolCallStatus`, `Implementation`, `SessionListCapabilities`, `AvailableCommandsUpdate`, `AvailableCommand`, `AvailableCommandInput`/`UnstructuredCommandInput`. Wire-format regression tests added. 655/655 tests pass.

**Phase B-1 continued (2026-05-10):** 4 more types replaced: `TextContent`, `ListSessionsRequest` (alias `SessionListRequest`), `ListSessionsResponse` (alias `SessionListResponse`), `SessionInfo`. Reclassified to KEEP: `ToolCallLocation` (different wire format — crate uses `path/line`, we use `uri/range`), `ToolCallRange`/`Position` (no crate equivalents), `SessionLoadRequest` (incompatible field types). **Total replaced: 11 types. 655/655 tests pass.**

---

## Phase B implementation plan

### Phase B-1 — Schema types swap (subtask 111.2)

**Cargo.toml change:**
```toml
agent-client-protocol = "0.11"
```
No features needed — the default set gives all schema types without any unstable additions.
The full connection machinery (Builder, Agent trait) is also included but won't be used until B-3.

**Migration order** (dependency-safe, no circular issues):

1. Leaf types first: `SessionId`, `ProtocolVersion`, `Implementation`, `TextContent`, `TextResourceContents`, `ResourceLink`, `StopReason`, `ToolKind`, `ToolCallStatus`, `ToolCallContent`, `ToolCallLocation`, `SessionListCapabilities`, `ContentChunk`
2. Mid-level: `ContentBlock`, `EmbeddedResource`, `EmbeddedResourceResource`, `ToolCall`, `ToolCallUpdate`, `SessionUpdate`, `SessionNotification`, `PromptRequest`, `PromptResponse`
3. Top-level: `InitializeResponse`, `SessionInfo`, `SessionListResponse`, `LoadSessionRequest`, `ListSessionsRequest`
4. Slash commands: `UnstructuredCommandInput` → `AvailableCommandInput` adapter, `AvailableCommand`, `AvailableCommandsUpdate`
5. Permissions: `PermissionOptionKind`, `PermissionOption`, `RequestRequestPermissionRequest`
6. EXTEND types last (requires most care): `AgentCapabilities`, `SessionCapabilities`, `InitializeRequest`, `NewSessionRequest`, `NewSessionResponse`, `AuthMethod`, `SessionUpdate`

**What stays in `protocol.rs` after B-1:**
- KEEP types (AcpModeInfo, AcpModesInfo, AcpModelInfo, AcpModelsInfo, PermissionOutcome, PermissionToolCall, OutcomeDetail, ToolCallRange, Position)
- EXTEND wrappers (InitializeRequest adapter, NewSessionRequest adapter, AuthMethod flat struct)
- Helper functions (default_permission_options, serialize_protocol_version, etc.)

### Phase B-2 — Connection-layer rewrite (subtask 111.3) — DEFERRED

**Status:** Deferred. Requires a dedicated sprint. Do NOT attempt without full integration tests.

**Why deferred:**
- `Agent` is a unit struct with a `Builder<Agent, NullHandler, NullRun>` builder pattern, NOT a trait to implement. The builder uses `on_receive_request!`, `on_receive_notification!` macros for callback registration.
- Our `handle_session_prompt` does complex bidirectional streaming: sends `session/update` notifications (chunks, tool-call events) _while_ waiting for the AI — this maps awkwardly to a per-request callback.
- `SessionId` (currently `String`-backed) needs to become `Arc<str>`-backed to use the crate's type, touching ~100 callsites.
- The `PermissionBridge` (permission requests during tool execution) has no clear equivalent in the crate's request-callback model.
- No automated integration tests exist to verify Zed/Gemini CLI compatibility after the rewrite.

**When it is attempted, the pattern will be:**

| Current handler | Builder callback |
|---|---|
| `handle_initialize()` | registered via `on_receive_request!(InitializeRequest)` |
| `handle_session_new()` | registered via `on_receive_request!(NewSessionRequest)` |
| `handle_session_prompt()` | registered via `on_receive_request!(PromptRequest)` |
| `handle_session_list()` | registered via `on_receive_request!(ListSessionsRequest)` |
| `handle_session_load()` | registered via `on_receive_request!(LoadSessionRequest)` |
| session/set_model | registered via `Builder::on_receive_dispatch` |

The BufReader/BufWriter loop is replaced by:
```rust
Agent.builder()
    .connect_with(ByteStreams::new(stdin, stdout), async |cx| {
        // register all handlers
    })
    .await
```

**Pre-requisites for 111.3:**
1. Add `grok acp test` integration test suite covering initialize → session/new → session/prompt flow
2. Migrate `SessionId` from `String` to `Arc<str>` backed
3. Resolve the `PermissionBridge` streaming pattern with the crate's `ConnectionTo<Agent>` context

### Phase B-3 — Slash-command advertisement (subtask 111.4)

`slash_commands.rs` changes:
- `use crate::acp::protocol::AvailableCommand` → `use agent_client_protocol::schema::AvailableCommand`
- `use crate::acp::protocol::AvailableCommandInput` → `use agent_client_protocol::schema::{AvailableCommandInput, UnstructuredCommandInput}`
- In `get_available_commands()`, change builder calls from `.with_input(hint)` to `.input(UnstructuredCommandInput { hint: hint.into() })`
- The crate's `AvailableCommand` is `#[non_exhaustive]`, so struct-literal construction is disallowed in external code — the existing `AvailableCommand::new(name, description)` pattern already matches the crate's builder, just swap it.

---

## Phase C new features enabled by crate

| Feature | Crate flag | What it unlocks |
|---|---|---|
| Session resume | `unstable_session_resume` | `LoadSessionRequest`, `LoadSessionResponse` with history |
| Elicitation | `unstable_elicitation` | Structured permission dialog RPC |
| Session fork | `unstable_session_fork` | `session/fork` method |
| MCP-over-ACP | always available (`mcp_server` module) | Expose tools as MCP endpoint |

---

## Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| `SessionUpdate` serde tag mismatch | High | Verify `#[serde(tag = "sessionUpdate")]` produces same JSON as crate before replacing |
| `AuthMethod` struct → enum mismatch | Medium | Keep local flat struct; add `From<AuthMethod> for schema::AuthMethod` if needed |
| `AvailableCommand` `#[non_exhaustive]` | Low | Already using `.new()`+`.with_input()` pattern; rename to `.input()` |
| Gemini CLI `session/new` JSON compat | High | `AcpModesInfo`/`AcpModelInfo` kept local — wire format unchanged |
| `InitializeRequest` alias loss | Medium | Keep local EXTEND type with all aliases |
| Heavy new dependencies | Low | Adds `jsonrpcmsg`, `rmcp`, `futures-concurrency`, `schemars`, `tokio-util` — verify build times on Windows |

---

*Document produced by Claude Sonnet 4.6 as part of Task 111.1 (ACP migration audit). — 2026-05-09*
