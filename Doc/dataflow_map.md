# Grok CLI Data Flow Map

This document shows the high-level data flow through the Grok CLI system.

## Session Initialization Flow

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

**Last Updated:** 2026-05-10
