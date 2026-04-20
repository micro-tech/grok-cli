# Reasoning Protocol Layer (RPL) — Architecture Document

> **Status:** Design specification — `src/rpl/` scaffolded, implementation in progress.  
> **Author:** john mcconnell <john.microtech@gmail.com>  
> **Repository:** https://github.com/micro-tech/grok-cli

---

## 1. Overview

The Reasoning Protocol Layer (RPL) is a structured introspection subsystem that sits alongside the `CpuRouter`'s tool-execution loop. Its purpose is to capture *why* the agent made each decision — which tools it considered, what memory it consulted, how uncertain it was — in a machine-readable, version-stamped envelope called a `ReasoningTrace`. This trace is produced silently during normal operation and is never surfaced to the user unless an explicit debug flag is set.

Without the RPL, the agent's decision process is opaque: tool selections, skill arbitration scores, and memory lookups exist only as ephemeral in-process state. The RPL makes that state observable without altering the observable behaviour of any API surface. It provides the foundation for future self-correction, Bayesian belief integration, and memory-aware planning while keeping the current request/response contract fully backward compatible.

---

## 2. Goals and Constraints

### 2.1 Primary Goals

| Goal | Description |
|------|-------------|
| **Structured reasoning** | Every agent turn produces a typed, versioned `ReasoningTrace` that records goals, tool evaluations, memory references, and the final plan. |
| **Observability** | `trace_id` links RPL traces to `tracing::debug!` logs emitted by `CpuRouter`, enabling end-to-end correlation without invasive instrumentation. |
| **Privacy by default** | Traces are suppressed (`suppressed = true`) and never appear in ACP `SessionUpdate` messages unless an explicit debug flag is active. |
| **Determinism** | The RPL is a pure observer — it records decisions but does not alter them. The same request must produce the same `RouterResponse` with or without the RPL present. |

### 2.2 Non-Goals

- **Model fine-tuning** — Traces are not training data pipelines. The RPL does not feed back into model weights.
- **Backend replacement** — The RPL does not route requests, call the Grok API, or act as a middleware proxy. It observes the `CpuRouter` lifecycle via hooks only.
- **User-visible chain-of-thought** — The RPL is an engineering telemetry layer, not a user-facing explanation system.

### 2.3 Hard Constraints

1. Traces are suppressed by default. A trace must never reach `SessionUpdate::AgentMessageChunk` or any ACP response unless `RplConfig::expose_in_acp` is explicitly set to `true` by an operator.
2. No user data (prompt text, file contents, personal identifiers) may be written into a trace field unless it has passed through a redaction gate. Memory consideration entries are subject to this rule.
3. The RPL API must be fully backward compatible. Removing the RPL from a build (feature-flagged or otherwise) must leave `CpuRouter::route_with_tools` functionally identical.
4. `schema_version` must be incremented for every breaking change to `ReasoningTrace` field semantics.

---

## 3. Position in the Architecture

### 3.1 Component Diagram

```
User / Zed Editor
      │
      ▼
  ACP Layer (protocol.rs)
      │   PromptRequest
      ▼
  AppRouter / CpuRouter  ◄──── RplLayer (Reasoning Protocol Layer)
      │                              │
      │                         ReasoningTrace
      ▼                           (schema.rs)
  Backend (Grok API)
      │
      ▼
  Tool Loop ──────────────► RplLayer.on_tool_selection()
      │
      ▼
  Skill Arbitration ◄────── ReasoningTrace.tool_evaluations + uncertainty
      │
      ▼
  MemoryManager ◄─────────── ReasoningTrace.memory_considerations
```

The `RplLayer` is instantiated once per turn and passed into `CpuRouter::route_with_tools_traced`. It is not on the hot path — the router calls its hooks fire-and-forget style; hook failures are logged and swallowed so they can never propagate a panic into the agent loop.

### 3.2 Data Flow for a Single Turn

1. **ACP receives `PromptRequest`** — The ACP layer decodes the incoming JSON-RPC message and hands a typed `PromptRequest` to `AppRouter`.
2. **`CpuRouter::route_with_tools` is called** — The router selects a backend (currently always `BackendKind::Grok`) and enters the tool-execution loop.
3. **`RplLayer::on_pre_evaluate()` creates a `ReasoningTrace`** — A UUID v4 `trace_id` is minted. The `phase` is set to `ReasoningPhase::PreEvaluation`. The inferred `goal` and `context` fields are populated from the session state if available.
4. **Each tool selection calls `RplLayer::on_tool_selection()`** — The tool name, its score, and any rejection reason are appended to `tool_evaluations`. The `uncertainty` field is updated after each iteration.
5. **`RplLayer::on_complete()` is called on loop exit** — The trace is finalized (`phase = Complete`), `validate()` is run against it, and the trace is logged at the configured `ReasoningLogLevel`. If `suppressed = true` (the default), the trace never leaves the process boundary.
6. **Skill arbitration consults the trace for the next turn** — `AutoActivationEngine::check_with_reasoning` accepts an `Option<&ReasoningTrace>`. When a trace is present, the engine can use `tool_evaluations` scores and `uncertainty` to weight skill confidence without re-running scoring from scratch.

---

## 4. CPU / State-Machine Lifecycle Mapping

The `CpuRouter` tool loop implicitly passes through several phases. The RPL makes those phases explicit via `ReasoningPhase` and maps a hook to each one.

| CPU Phase            | RPL Phase         | `RplLayer` Hook                     | Status       |
|----------------------|-------------------|-------------------------------------|--------------|
| Request received     | `PreEvaluation`   | `on_pre_evaluate()`                 | Planned      |
| Tool call emitted    | `ToolSelection`   | `on_tool_selection()`               | Planned      |
| Memory query         | `MemoryLookup`    | *(future: `on_memory_lookup()`)*    | Future       |
| Final response built | `ActionPlanning`  | *(future: `on_action_plan()`)*      | Future       |
| Loop exit            | `Complete`        | `on_complete()`                     | Planned      |

**Phase transitions are strictly forward.** A trace that has reached `Complete` must not be mutated. Any attempt to call an earlier-phase hook on a completed trace is a no-op with a logged warning.

---

## 5. Reasoning Schema and Envelope Fields

### 5.1 `ReasoningTrace` — Top-Level Envelope

Defined in `src/rpl/schema.rs`.

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | `u32` | Monotonically increasing schema version. Increment on any breaking field change. Current: `1`. |
| `trace_id` | `String` | UUID v4. Primary correlation key. Links this trace to `tracing::debug!` output from `CpuRouter` via the `trace_id` field in structured logs. |
| `goal` | `Option<String>` | Inferred user intent extracted from the prompt. May be `None` if intent inference is disabled or unavailable. Subject to redaction before persistence. |
| `context` | `Option<String>` | Relevant session context snapshot (e.g. active skill names, working directory). Not the full system prompt. |
| `tool_evaluations` | `Vec<ToolEvaluation>` | Ordered list of every tool considered during this turn, including those that were rejected. See §5.2. |
| `memory_considerations` | `Vec<MemoryConsideration>` | Memory entries retrieved and their relevance scores. See §5.3. |
| `plan` | `Option<String>` | A short human-readable description of the planned action sequence, set by `on_action_plan()` (future). |
| `uncertainty` | `f32` | Confidence measure in the range `[0.0, 1.0]`. `0.0` = maximally uncertain, `1.0` = fully confident. Updated after each tool selection iteration. |
| `created_at` | `DateTime<Utc>` | ISO 8601 UTC timestamp at trace construction. Used for latency analysis and log correlation. |
| `phase` | `ReasoningPhase` | The lifecycle phase at which this trace was captured. One of `PreEvaluation`, `ToolSelection`, `MemoryLookup`, `ActionPlanning`, `Complete`. |
| `suppressed` | `bool` | When `true` (the default), the trace is excluded from all ACP output and user-visible responses. Only written to internal diagnostic logs. |

### 5.2 `ToolEvaluation` — Per-Tool Score Record

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | `String` | The function name as registered in the tool registry (e.g. `"read_file"`, `"list_directory"`). |
| `score` | `f32` | Normalized selection score in `[0.0, 1.0]` assigned by the skill arbitration engine before the tool call was issued. |
| `selected` | `bool` | `true` if this tool was actually called this iteration; `false` if it was considered but rejected. |
| `rejection_reason` | `Option<String>` | Human-readable explanation of why the tool was not selected. `None` when `selected = true`. |
| `iteration` | `u32` | The tool-loop iteration number (zero-based) during which this evaluation occurred. |

### 5.3 `MemoryConsideration` — Per-Entry Memory Record

| Field | Type | Description |
|-------|------|-------------|
| `memory_key` | `String` | Identifier of the memory entry (e.g. episodic episode ID, long-term fact key). |
| `memory_kind` | `String` | Which memory store the entry came from: `"short_term"`, `"long_term"`, `"episodic"`, `"working"`, `"skill"`, or `"tool"`. |
| `relevance` | `f32` | Relevance score in `[0.0, 1.0]` assigned by the memory retrieval subsystem for this turn's query. |
| `used` | `bool` | `true` if this entry was injected into the system prompt or tool context; `false` if retrieved but ultimately excluded. |
| `redacted` | `bool` | `true` if the entry content was redacted before being stored in the trace. Always `true` when the entry contains user-generated text (future enforcement). |

### 5.4 `ReasoningPhase` — Lifecycle Enum

```
pub enum ReasoningPhase {
    PreEvaluation,   // Trace created; goal and context set
    ToolSelection,   // Inside the tool-execution loop
    MemoryLookup,    // Memory entries being retrieved (future)
    ActionPlanning,  // Final response being assembled (future)
    Complete,        // Loop exited; trace validated and logged
}
```

---

## 6. Logging and Observability

### 6.1 `ReasoningLogLevel`

Defined in `src/rpl/logging.rs`. Controls the verbosity of `log_trace()`.

| Level | Behaviour |
|-------|-----------|
| `Off` | No output. The `log_trace()` call is a no-op. Useful in production when traces are consumed programmatically rather than written to logs. |
| `Summary` | **Default.** Emits a single `tracing::info!` line per completed trace: `trace_id`, `phase`, `uncertainty`, tool count, and elapsed milliseconds. |
| `Debug` | Emits one `tracing::debug!` line per `ToolEvaluation` entry plus the summary line. |
| `Trace` | Emits the full `ReasoningTrace` serialized as pretty-printed JSON via `tracing::trace!`. Includes `memory_considerations`. |

### 6.2 Enabling Full Trace Logs

Set the `RUST_LOG` environment variable before launching `grok-cli`:

```
# Summary only (default behaviour, explicitly set)
RUST_LOG=grok_cli::rpl=info

# Per-tool evaluation lines
RUST_LOG=grok_cli::rpl=debug

# Full JSON trace dump
RUST_LOG=grok_cli::rpl=trace
```

A future `RplConfig` field (`log_level: ReasoningLogLevel`) in `config.toml` will allow persistent configuration without environment variables. Until that field is implemented, `RUST_LOG` is the only control surface.

### 6.3 Correlating RPL Traces with Router Logs

The `CpuRouter` emits a structured `tracing::debug!` event at the end of every tool loop iteration:

```
tracing::debug!(
    iteration = iteration + 1,
    max = max_iterations,
    tools = tool_calls.len(),
    "tool loop iteration complete"
);
```

When the RPL is active, `on_pre_evaluate()` injects the same `trace_id` value into the router's tracing span as a field. This means filtering `RUST_LOG` output by `trace_id` will show both the router-level iteration events and the RPL-level evaluation events for a single turn, in chronological order.

---

## 7. Suppression and Privacy

### 7.1 The `suppressed` Field

Every `ReasoningTrace` is constructed with `suppressed: true`. This is a hard default enforced in `RplLayer::on_pre_evaluate()` — there is no constructor path that creates an unsuppressed trace automatically.

Setting `suppressed = false` requires an explicit operator action:
- Setting `RplConfig::expose_in_acp = true` in the runtime configuration, **or**
- Passing the `--debug-rpl` flag on the CLI (future).

These gates are intentionally separate from `RUST_LOG`. Raising the log verbosity does not unsuppress traces — it only increases the detail written to the process's own log sink, which is never transmitted over the ACP wire.

### 7.2 ACP Boundary Enforcement

Traces must never appear in `SessionUpdate` messages. Specifically:

- `SessionUpdate::AgentMessageChunk` — must not contain trace JSON embedded in content.
- `SessionUpdate::ToolCall` — the `raw_output` field must not be populated with trace data.
- `SessionUpdate::ToolCallUpdate` — same restriction.

`validate()` in `src/rpl/validation.rs` includes a `ValidationError::AcpLeakDetected` variant that fires if a serialized trace is found within a `SessionNotification` payload during integration testing. This check is only active in `#[cfg(test)]` builds; at runtime the suppression gate is the primary defence.

### 7.3 Future Extension: Redaction Rules

A `redaction: Vec<RedactionRule>` field is reserved in `RplConfig` for future implementation. Redaction rules will be applied to `MemoryConsideration` entries and the `goal` / `context` fields before a trace is written to any log sink. Initially, rules will support:

- **Field nullification** — replace the field value with `None`.
- **Pattern masking** — replace regex-matched substrings with `[REDACTED]`.
- **Full entry removal** — drop `MemoryConsideration` entries where `memory_kind` matches a configured deny-list.

Until this system is implemented, any `MemoryConsideration` that contains user-generated text must have `redacted: true` set manually by the caller.

---

## 8. Integration Points Summary

| Call Site | Integration | Notes |
|-----------|-------------|-------|
| `CpuRouter::route_with_tools` | Calls `RplLayer` hooks (`on_pre_evaluate`, `on_tool_selection`, `on_complete`) at the appropriate loop boundaries. | Hook failures are caught and logged; they must never propagate into `RouterError`. |
| `CpuRouter::route_with_tools_traced` | Variant of `route_with_tools` that returns `(RouterResponse, ReasoningTrace)`. | Used by callers that need to inspect the trace after the turn completes (e.g. integration tests, the planning layer). |
| `AutoActivationEngine::check_with_reasoning` | Accepts `Option<&ReasoningTrace>`. When `Some`, uses `tool_evaluations` scores and the `uncertainty` value to weight skill confidence during activation scoring. | Falls back to standard `check()` behaviour when `None`. No API break. |
| `MemoryStore` *(future)* | Will call into the active `RplLayer` to append `MemoryConsideration` entries as memory lookups occur inside `reload_context()` and `build_system_prompt()`. | Gated behind Task 97 (Memory-aware reasoning). |
| `BayesEngine` *(future)* | Will read `uncertainty` from the current trace to seed its prior distribution for the next iteration. | Gated behind Task 95. |

---

## 9. Validation

`src/rpl/validation.rs` exposes a single public function:

```
pub fn validate(trace: &ReasoningTrace) -> Result<(), ValidationError>
```

`validate()` is called by `RplLayer::on_complete()` before the trace is logged or returned to the caller. It enforces:

| Check | `ValidationError` variant |
|-------|--------------------------|
| `schema_version` is non-zero | `InvalidSchemaVersion` |
| `trace_id` is a valid UUID v4 | `InvalidTraceId` |
| `uncertainty` is in `[0.0, 1.0]` | `UncertaintyOutOfRange(f32)` |
| No duplicate `tool_name` + `iteration` pairs in `tool_evaluations` | `DuplicateToolEvaluation` |
| `phase` is `Complete` on call | `IncompleteTrace(ReasoningPhase)` |
| Trace not serialized into ACP payload *(test only)* | `AcpLeakDetected` |

Validation failures are logged at `warn!` level and do not abort the turn. The trace is still returned to the caller with a `validation_error: Option<String>` field populated (future addition to the schema).

---

## 10. Future Work

The following tasks extend the RPL into a full reasoning and planning subsystem. Each is tracked in the project task list.

| Task | Description | ID |
|------|-------------|-----|
| **Bayesian belief integration** | Wire `BayesEngine` (currently in `src/bayes/`) into the RPL so that prior beliefs from `belief_graph.rs` seed the `uncertainty` field and are updated via `updater.rs` after each tool result. | Task 95 |
| **Planning layer** | Implement `on_action_plan()` hook and the `ActionPlanning` phase. The planner will decompose the inferred `goal` into a sequence of tool calls recorded in `ReasoningTrace::plan` before the first call is issued. | Task 96 |
| **Memory-aware reasoning** | Connect `MemoryStore::reload_context()` to the RPL so that every memory retrieval automatically appends a `MemoryConsideration` entry, including relevance scoring from the episodic and long-term stores. | Task 97 |
| **Self-correction loops** | Use `tool_evaluations` from the previous turn's trace to detect repeated tool failures and inject corrective context into the next iteration's system prompt. | Task 99 |
| **Redaction rules** | Implement the `RedactionRule` engine described in §7.3 so that memory content and inferred goals are sanitized before any trace is written to disk or transmitted. | Backlog |
| **Persistent trace store** | Optionally serialize completed traces to a local SQLite database keyed by `trace_id` for offline analysis and replay. | Backlog |

---

## Appendix A — File Inventory

| File | Purpose |
|------|---------|
| `src/rpl/mod.rs` | Module root; re-exports public types |
| `src/rpl/schema.rs` | `ReasoningTrace`, `ToolEvaluation`, `MemoryConsideration`, `ReasoningPhase` |
| `src/rpl/validation.rs` | `validate()`, `ValidationError` |
| `src/rpl/layer.rs` | `RplLayer`, `RplConfig` — the hook entry points |
| `src/rpl/logging.rs` | `ReasoningLogLevel`, `log_trace()` |

---

## Appendix B — Changelog

| Date | Change | Author |
|------|--------|--------|
| 2025-07-10 | Initial document created | john mcconnell (AI-assisted) |

---

*Buy me a coffee — [buymeacoffee.com/micro.tech](https://buymeacoffee.com/micro.tech)*