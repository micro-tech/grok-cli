# ACP Structured Feedback (Tasks 128–130)

This document describes the three new structured `SessionUpdate` types added to the ACP integration so that Zed (and other ACP clients) can display rich agent state.

## Overview

Previously the ACP path only emitted generic text and tool-call updates. With Tasks 128–130 we now emit three new typed updates:

| Update Type              | Purpose                              | Config Toggle                  | Status     |
|--------------------------|--------------------------------------|--------------------------------|------------|
| `ThinkingUpdate`         | Real-time reasoning traces           | —                              | Implemented |
| `ContextUsageUpdate`     | Token / context window feedback      | `acp.show_context_usage`       | Implemented |
| `AgentActivityUpdate`    | Sub-agent lifecycle (spawn/fork/join)| —                              | Protocol ready |

## 1. ThinkingUpdate (Task 129)

**Purpose**: Allow the client to display the model’s chain-of-thought in a collapsible section.

**Structure** (`src/acp/protocol.rs`):
```rust
pub struct ThinkingUpdate {
    pub content: String,
    pub is_final: bool,
}
```

**Emission points**:
- When `thinking_content` is present in a Grok response → emit with `is_final = false`
- At the end of a turn that produced thinking → emit with `is_final = true`

**Client rendering suggestion**:
```markdown
<details>
<summary>🧠 Thinking…</summary>

{thinking_content}

</details>

{normal response}
```

## 2. ContextUsageUpdate (Task 130)

**Purpose**: Give the editor a live view of how much of the context window is being used.

**Structure**:
```rust
pub struct ContextUsageUpdate {
    pub estimated_tokens: usize,
    pub context_limit: usize,
    pub message_count: usize,
}
```

**Emission**:
- After every final response (no more tool calls)
- After every tool-loop iteration (so the meter updates while the agent is working)
- Only emitted when `acp.show_context_usage = true` (default)

**Client usage**:
Zed can show a small meter in the status bar or footer:
```
Context: 47k / 950k  (5%)
```

## 3. AgentActivityUpdate (Task 128)

**Purpose**: Enable future multi-agent trees in the UI.

**Structure**:
```rust
pub struct AgentActivityUpdate {
    pub agent_id: String,
    pub parent_id: Option<String>,
    pub status: AgentActivityStatus,   // Running | Completed | Failed | Cancelled
    pub description: String,
}
```

**Current state**:
- Protocol type + helper method `emit_agent_activity()` are implemented.
- Actual emission points inside `spawn_agent` / `fork_agent` / `join_agents` are deferred until Task 26 (Multi-Agent Orchestration) is completed.

## Configuration

```toml
[acp]
show_context_usage = true   # default
```

No toggle currently exists for thinking traces (they are always sent when present). A future `acp.stream_thinking` flag is planned.

## Data Flow Diagram

```
handle_chat_completion
          │
          ├── thinking_content? ──▶ ThinkingUpdate (is_final=false)
          │
          ├── after API call ─────▶ ContextUsageUpdate (if enabled)
          │
          ├── tool loop iteration ─▶ ContextUsageUpdate
          │
          └── final response ─────▶ ThinkingUpdate (is_final=true) + Text
```

## Future Work

- Task 129.4: Add `acp.stream_thinking` toggle for partial thinking chunks.
- Task 128.3/128.4: Wire `emit_agent_activity()` into the new multi-agent tools.
- Live partial thinking streaming from the Grok backend.

## Related Files

- `src/acp/protocol.rs` — new update structs
- `src/acp/mod.rs` — emission logic in `handle_chat_completion`
- `src/config/mod.rs` — `AcpConfig::show_context_usage`
- `dataflow_map.md` — high-level ACP feedback flow
