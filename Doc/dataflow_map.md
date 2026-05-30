# Grok CLI Data Flow Map

This document shows the high-level data flow through the Grok CLI system.

## ACP / Zed Startup Flow (Tasks 121–126)

```
grok acp stdio
        │
        ▼
┌──────────────────────────────┐
│  GrokAcpAgent::new()         │
│  (extremely lightweight)     │
└──────────────────────────────┘
        │
        ├─► OnceLock<AppRouter>          (empty)
        ├─► OnceLock<SecurityManager>    (empty)
        ├─► OnceLock<HookManager>        (empty)
        └─► OnceLock<Capabilities>       (empty)
        │
        ▼
   Wait for Zed initialize / session/new
        │
        ▼
   First prompt or tool call
        │
        ▼
   Lazy initialization on demand:
        ├─► AppRouter::new()          (only if API key present)
        ├─► SecurityManager::new() + trust CWD
        └─► HookManager::new()
```

**Key optimization**: No expensive work happens until the first real request. The agent can respond to `initialize` declaring its auth requirements in < 50 ms.

```
User starts session
        │
        ▼
┌──────────────────────┐
│  initialize_session  │
└──────────────────────┘
        │
        ├─► Load Hierarchical Config (.grok/config.toml + system)
        │
        ├─► Load Session DNA (session_dna.json)
        │       └── Inject tone, verbosity, risk_tolerance, coding_style
        │
        ├─► Load Knowledge (knowledge/*.md + *.json)
        │
        ├─► Load Context Archive (if resuming)
        │
        └─► Create SessionData + BayesianEngine
```

## Prompt Processing Flow

```
User Input
    │
    ▼
┌──────────────────────┐
│   Slash Command?     │◄── /save, /load, /goal, /think, /visualize, /bayes, etc.
└──────────────────────┘
    │ Yes                  │ No
    ▼                      ▼
Builtin Handler       ┌──────────────────────┐
    │                 │   Context Manager    │
    ▼                 └──────────────────────┘
Return immediately           │
                        ┌────┴────┐
                        ▼         ▼
                 System Prompt   User Message
                        │
                        ▼
                ┌──────────────────────┐
                │   Layered Trimming   │
                │  (token budget)      │
                └──────────────────────┘
                        │
                        ▼
                ┌──────────────────────┐
                │  Auto-Compression?   │◄── If context > threshold
                └──────────────────────┘
                        │
                        ▼
                ┌──────────────────────┐
                │   Grok API Call      │
                │  (+ reasoning_effort)│
                └──────────────────────┘
                        │
                        ▼
                ┌──────────────────────┐
                │   Tool Loop          │
                │  (max iterations)    │
                └──────────────────────┘
                        │
                        ▼
                Response + Thinking Content
```

## Key Components

| Component              | File(s)                          | Responsibility |
|------------------------|----------------------------------|----------------|
| Session DNA            | `src/session/dna.rs`             | Personality injection |
| Knowledge Loader       | `src/knowledge/loader.rs`        | Project knowledge injection |
| Context Archive        | `src/memory/context_archive.rs`  | Long-term memory chunks |
| Context Compressor     | `src/memory/context_compressor.rs` | AI summarization |
| Task Graph             | `src/task_graph/`                | Multi-step workflows |
| ACP Layer              | `src/cli/commands/acp.rs`        | Zed / ACP protocol |
| Bayesian Router        | `src/bayes/`                     | Smart model routing |

---

**Last Updated:** 2026-05-10 (ACP lazy initialization – tasks 121-126)

---

## Multi-Agent Orchestration Flow (Task 127)

```
ReasoningEngineState (Planner)
        │
        ▼
┌──────────────────────────────┐
│  PlanBuilder::build_plan()   │
│  • Detects complex goals     │
│  • High uncertainty?         │
│  • Emits DelegateToSubAgent  │
└──────────────────────────────┘
        │
        ▼
StepAction::DelegateToSubAgent
        │
        ▼
┌──────────────────────────────┐
│  AgentManager (global)       │
│  • spawn() → Running         │
│  • fork_agent()              │
│  • join_agents()             │
└──────────────────────────────┘
        │
        ├─► In-Memory MessageBus
        │      send_message_in_memory()
        │      receive_messages()
        │
        └─► Tool Execution
               spawn_agent / fork / join
```

**Key Components Added**

| Component           | Responsibility                              |
|---------------------|---------------------------------------------|
| `AgentManager`      | Central registry + lifecycle tracking       |
| `AgentMessageBus`   | Fast in-process messaging between agents    |
| `DelegateToSubAgent`| Native plan step for sub-agent delegation   |
| Orchestration Tools | `fork_agent`, `join_agents`, `list_agents`, etc. |
