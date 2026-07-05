
# Multi-Agent Orchestration

Grok-CLI includes a fully wired multi-agent system that lets the LLM spawn, coordinate, and collect results from parallel sub-agents — each making independent xAI API calls.

## Architecture

```
GrokAcpAgent (main session)
│
├── spawn_agent("task")          → single sub-agent, serial, returns result
│
├── fork_agent(["A","B","C"])    → 3 parallel tokio tasks, waits for all
│   ├── Agent A  ──→ call_subagent_api() ──→ grok-3-mini
│   ├── Agent B  ──→ call_subagent_api() ──→ grok-3-mini
│   └── Agent C  ──→ call_subagent_api() ──→ grok-3-mini
│
└── delegate_plan_step("step", parent_id)  → child tracked under parent
```

All agents are tracked in the global `AgentManager` (an `Arc<RwLock<HashMap>>`) and activity events are emitted to the Zed UI in real time.

## LLM-Callable Tools

All tools are registered with JSON schemas and callable by the xAI model during a tool loop.

### `spawn_agent`

Spawn a single focused sub-agent and wait for its result.

```json
{
  "task":       "Write unit tests for src/auth.rs",
  "context":    "(optional extra context)",
  "max_tokens": 2048
}
```

- Registers in `AgentManager` with `Running` status
- Makes a real `grok-3-mini` API call (up to 3 Starlink retries)
- Updates status to `Completed` or `Failed`
- Emits `Spawned` → `Joined` activity events to Zed
- Returns the sub-agent's response text

### `fork_agent`

Spawn multiple sub-agents **in parallel** and wait for all results.

```json
{
  "tasks": [
    "Summarise the memory module",
    "List all TODO comments in src/",
    "Check for unused dependencies"
  ]
}
```

- Registers all agents immediately
- Launches one `tokio::spawn` task per agent (true parallel execution)
- Each task has a 180-second timeout (Starlink-safe)
- Waits for all tasks then returns a structured summary:

```
## Fork Results (3/3 succeeded)

### Agent `8bb0253e` ✅
(result text)

---

### Agent `4a64eda3` ✅
(result text)

---
```

- If all tasks fail, returns an `Err` with details so the LLM can react
- Partial failures are included in the summary with ❌ markers

### `join_agents`

Collect results from previously spawned agents by ID.

```json
{
  "agent_ids": ["8bb0253e-...", "4a64eda3-..."]
}
```

Returns per-agent status:
- ✅ `Completed` — includes result text
- ❌ `Failed` — includes error message
- ⏳ `Running` — still in progress
- ⚫ `Cancelled` — was cancelled
- ❓ `Not found` — ID not in registry

### `list_agents`

List all tracked sub-agents, optionally filtered by parent.

```json
{ "parent_id": "optional-parent-uuid" }
```

### `get_agent_status`

Get the full status and result of one agent.

```json
{ "agent_id": "8bb0253e-7204-40ea-8f40-..." }
```

### `cancel_agent`

Cancel a running sub-agent.

```json
{ "agent_id": "8bb0253e-7204-40ea-8f40-..." }
```

### `delegate_plan_step`

Delegate a plan step to a child sub-agent with proper parent tracking.

```json
{
  "task":      "Refactor the auth module",
  "parent_id": "plan-abc-123"
}
```

The child agent is registered under `parent_id` so `list_agents(parent_id)` correctly returns it. Uses `call_subagent_api` directly — no double-registration in the manager.

## Messaging Between Agents

### `send_message` (file-backed, persistent)

```json
{ "target": "agent-id-or-channel", "message": "Hello from main agent" }
```

Appended atomically to `.grok/messages/{target}.jsonl`. Starlink-safe (write to `.tmp`, then rename).

### `send_message_in_memory` (fast, in-process)

```json
{ "from": "main", "to": "agent-b", "message": "Your turn" }
```

Uses the global `AgentMessageBus` (`Arc<RwLock<HashMap>>`). Faster than file-based but not persistent across process restarts.

### `receive_messages`

```json
{ "target": "agent-b" }
```

Returns all pending messages for the target formatted as `[timestamp] from → to: content`.

## Team Management

### `team_create`

```json
{
  "name":        "review-team",
  "members":     ["reviewer-a", "reviewer-b"],
  "description": "Code review agents"
}
```

Stored in `.grok/teams.json`. Returns error if team already exists (call `team_delete` first).

### `team_delete`

```json
{ "name": "review-team" }
```

## Activity Events (Zed UI)

The following events are emitted as `AgentActivityUpdate` notifications visible in Zed's agent panel:

| Event | Trigger |
|---|---|
| `Spawned` | Agent registered and API call started |
| `Forked` | Agent registered as part of a `fork_agent` batch |
| `Joined` | Agent completed successfully |
| `Cancelled` | Agent failed, timed out, or was explicitly cancelled |

Events flow through a global `ACTIVITY_SENDER` channel registered by the ACP layer at session start.

## Implementation Details

### `call_subagent_api` (private)

The single shared API call helper used by both `spawn_agent` and `fork_agent`. Features:

- Model: `grok-3-mini`
- Timeout: 180 seconds per call
- Retries: up to 3, with exponential backoff (5 → 10 → 20 → 40 → 60s cap)
- Network drop detection via `utils::network::detect_network_drop`
- System prompt: `"You are a focused sub-agent. Complete the given task as concisely and directly as possible."`

### `AgentManager`

Global `Arc<AgentManager>` (lazily initialized via `once_cell`). Thread-safe via `tokio::sync::RwLock`. Each `SubAgent` record contains:

```rust
pub struct SubAgent {
    pub id: String,
    pub parent_id: Option<String>,
    pub task: String,
    pub status: AgentStatus,   // Running | Completed | Failed | Cancelled
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub model: Option<String>,
    pub skill: Option<String>,
}
```

### `AgentMessageBus`

Global `Arc<AgentMessageBus>` (lazily initialized). Channels are `HashMap<String, Vec<AgentMessage>>` behind a `RwLock`. Use `clear(target)` after processing messages to avoid unbounded growth.

## Usage Patterns

### Pattern 1: Parallel research

```
User: Research these three topics in parallel and summarize each
LLM calls: fork_agent(tasks=["explain X", "explain Y", "explain Z"])
Returns: structured results from all 3 agents
```

### Pattern 2: Divide-and-conquer code tasks

```
User: Write tests for every module in src/
LLM calls: fork_agent(tasks=["tests for auth.rs", "tests for config.rs", "tests for router.rs"])
Returns: merged test code from all 3 agents
```

### Pattern 3: Sequential with tracking

```
LLM calls: spawn_agent("summarise memory module") → id_1
LLM calls: spawn_agent("summarise router module") → id_2
LLM calls: join_agents([id_1, id_2]) → combined summary
```

### Pattern 4: Plan delegation

```
LLM creates plan with parent_id = "plan-xyz"
LLM calls: delegate_plan_step("step 1: analyse", parent_id="plan-xyz")
LLM calls: delegate_plan_step("step 2: implement", parent_id="plan-xyz")
LLM calls: list_agents(parent_id="plan-xyz") → shows child progress
```

## Configuration

Sub-agent behavior can be tuned in `.grok/config.toml`:

```toml
[acp]
max_tool_loop_iterations = 50   # increase for complex multi-agent tasks
timeout_secs = 120              # base timeout for main agent calls
```

Sub-agents use their own fixed timeout (180s) and retry logic defined in `src/tools/agent_tools.rs`.

## Source Files

| File | Purpose |
|---|---|
| `src/tools/agent_tools.rs` | All LLM-callable agent tools + `call_subagent_api` |
| `src/agent/manager.rs` | `AgentManager` — registry with full CRUD |
| `src/agent/message_bus.rs` | `AgentMessageBus` — in-memory messaging |
| `src/agent/activity.rs` | Activity event emitter → Zed UI |
| `src/agent/planner.rs` | Bayesian-driven task planner |
| `src/tools/registry.rs` | Tool registration + JSON schema definitions |
