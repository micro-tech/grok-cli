# Grok CLI — Reasoning Systems Overview

> **Author:** john mcconnell <john.microtech@gmail.com>
> **Repository:** https://github.com/micro-tech/grok-cli
> **Buy me a coffee:** https://buymeacoffee.com/micro.tech

---

## 1. Introduction

Grok CLI ships with two complementary reasoning systems that together give the agent structured
introspection and goal-directed intelligence. They are designed to be independent, composable, and
privacy-safe by default — each system can operate without the other, but they are most powerful
when used together.

The **Reasoning Protocol Layer (RPL)** (`src/rpl/`) is a *passive observability and tracing layer*
introduced across Tasks 86–92. It wraps the `CpuRouter`'s tool-execution loop via lightweight
fire-and-forget hooks, recording every tool selection, memory lookup, uncertainty score, and phase
transition into a typed, version-stamped `ReasoningTrace` envelope. The RPL never alters the
observable behaviour of the router — it only watches and records. Traces are suppressed by default
and never reach the ACP wire unless an operator explicitly enables exposure.

The **Reasoning Engine** (`src/engine/`) is an *active decision-making finite-state machine (FSM)*
introduced across Tasks 93–101. It sits between the `CpuRouter` and the backend API call, answering
three questions on every agent turn: *What does the user actually want?* (goal inference),
*How should we achieve it?* (multi-step planning), and *Did it work?* (self-correction). Where the
RPL asks "what happened and why?", the Reasoning Engine asks "what should happen next?".

The key distinction that every developer must keep in mind: **the RPL watches what happens; the
Engine decides what to do.** The RPL is a passive observer with no side-effects on routing; the
Engine is an active planner that mutates `ReasoningEngineState`, writes facts to `LongTermMemory`
via `MemoryBridge`, and updates `BayesianEngine` priors via `EngineBeliefs`. Both systems share the
same `ReasoningLogLevel` hierarchy and the same suppression/privacy rules so their observability
outputs can be correlated and audited uniformly.

---

## 2. Quick Links

| Document | Location | Purpose |
|----------|----------|---------|
| RPL Architecture | `docs/rpl_architecture.md` | Full RPL design spec, schema, suppression rules, validation |
| Engine Architecture | `docs/engine_architecture.md` | Full Engine design spec, FSM, Bayesian integration, self-correction |
| Reasoning Quick Start | `Doc/REASONING_QUICK_START.md` | How to use and extend both systems with code examples |
| Project Layout | `project_layout.md` | Full source tree reference for all modules |

---

## 3. RPL at a Glance

The Reasoning Protocol Layer is a structured introspection subsystem that sits *alongside* the
`CpuRouter`'s tool-execution loop. It does not modify routing behaviour; it records it.

### What it does

- **Wraps `route_with_tools_traced`** — the `RplLayer` is instantiated once per agent turn and
  passed into `CpuRouter::route_with_tools_traced`. Its hooks (`on_pre_evaluate`,
  `on_tool_selection`, `on_complete`) fire at defined lifecycle points.
- **Captures a `ReasoningTrace`** — a typed, UUID-stamped envelope containing the inferred goal,
  all `ToolEvaluation` records, `MemoryConsideration` references, the current `ReasoningPhase`,
  and the final `uncertainty` score.
- **Emits structured logs** — using the `tracing` crate at the verbosity level configured by
  `ReasoningLogLevel`. Log lines are tagged with `trace_id` so they can be correlated with
  `CpuRouter` iteration events in the same log stream.

### Key types

| Type | Module | Role |
|------|--------|------|
| `ReasoningTrace` | `src/rpl/schema.rs` | Top-level trace envelope (UUID, phase, tool evals, uncertainty) |
| `RplLayer` | `src/rpl/layer.rs` | Holds `RplConfig`; exposes the lifecycle hooks |
| `ReasoningLogLevel` | `src/rpl/logging.rs` | `Off` / `Summary` / `Debug` / `Trace` verbosity enum |
| `SuppressionLayer` | `src/rpl/suppression.rs` | Guards traces at the ACP boundary; enforces redaction |
| `RedactionConfig` | `src/rpl/redaction.rs` | Ordered list of `RedactionRule` patterns applied before log emission |

### Default behaviour

Every `ReasoningTrace` is constructed with `suppressed = true`. This hard default is enforced in
`RplLayer::on_pre_evaluate()` — there is no constructor path that creates an unsuppressed trace
automatically. A trace never leaves the process boundary unless an operator explicitly sets
`RplConfig::expose_in_acp = true` or (in a future release) passes `--debug-rpl` on the CLI.

### How to enable debug traces

```rust
use grok_cli::rpl::{RplLayer, RplConfig, ReasoningLogLevel};

let layer = RplLayer::new(RplConfig {
    log_level: ReasoningLogLevel::Debug,
    lenient_validation: false,
    ..RplConfig::default()
});
```

Or via the environment before launching grok-cli:

```
# Per-tool evaluation lines
RUST_LOG=grok_cli::rpl=debug cargo run -- chat "hello"

# Full JSON trace dump
RUST_LOG=grok_cli::rpl=trace cargo run -- chat "hello"
```

### Full lifecycle code example

```rust
// 1. Create the layer (one per turn)
let layer = RplLayer::with_default_config();

// 2. Pre-evaluation hook — mints the trace_id, sets phase = PreEvaluation
let mut trace = layer.on_pre_evaluate(Some("read the config"), None);

// 3. Tool selection hook — appends a ToolEvaluation record
layer.on_tool_selection(&mut trace, "read_file", true, None);

// 4. Completion hook — finalises phase, runs validate(), emits the log line
layer.on_complete(&mut trace);

// Postconditions:
// trace.phase   == ReasoningPhase::Complete
// trace.suppressed == true  (never leaks to user output)
// tracing::info! fired with trace_id + uncertainty + tool count
```

---

## 4. Reasoning Engine at a Glance

The Full Reasoning Engine is an active decision-making FSM that provides goal-directed intelligence
on top of the `CpuRouter`'s mechanical tool execution.

### What it does

The engine drives `ReasoningEngineState` through a strict forward-only FSM:

```
AnalyzeGoal → ExpandOptions → EvaluateOptions → CommitPlan → ExecuteStep → Complete
                                    │                              │
                                    └──── AskClarification ←──────┘
                                                                   │
                                                              RevisePlan ──► Failed
```

At each state the engine updates Bayesian priors (`EngineBeliefs`), consults long-term memory
(`MemoryBridge`), selects tools with joint confidence scores (`ArbitrationEngine`), and checks
whether a completed step requires a plan revision (`CorrectionEngine`).

### Key types

| Type | Module | Role |
|------|--------|------|
| `ReasoningEngineState` | `src/engine/state.rs` | Core FSM: goal, hypotheses, plan, step cursor, uncertainty, revision count |
| `PlanBuilder` | `src/engine/planner.rs` | Decomposes a goal into an ordered `Vec<PlanStep>`; supports `revise_plan` |
| `EngineBeliefs` | `src/engine/beliefs.rs` | Wraps `BayesianEngine`; translates outcomes into posterior probability updates |
| `CorrectionEngine` | `src/engine/correction.rs` | Detects low-confidence / failed steps; triggers bounded plan revision |
| `ArbitrationEngine` | `src/engine/arbitration.rs` | Joint tool selection combining skill scores and Bayesian posteriors |
| `MemoryBridge` | `src/engine/memory_bridge.rs` | Queries `LongTermMemory`; decides when to write facts back |
| `EngineObserver` | `src/engine/observability.rs` | Emits structured `tracing` events; tags every event with `engine_id` |

### Self-correction

Self-correction is bounded by `max_revisions` (default `3`). The `CorrectionEngine` evaluates
three trigger conditions after every `ExecuteStep`: low-confidence outcome (`uncertainty >
uncertainty_threshold`), a `StepStatus::Failed` result, or a user feedback signal in the next
turn. When `revision_count >= max_revisions` and the current step has also failed, the engine
transitions to `EngineState::Failed` rather than looping indefinitely.

Use `correct_until_stable()` for the safe bounded loop in tests and integration code:

```rust
use grok_cli::engine::{CorrectionEngine, CorrectionOutcome};

let corrector = CorrectionEngine::default();
// Runs at most max_revisions correction cycles, then stops.
let outcomes = corrector.correct_until_stable(&mut state, 5);
for (trigger, outcome) in &outcomes {
    println!("Corrected: {trigger} → {outcome:?}");
}
```

### Full lifecycle code example

```rust
use grok_cli::engine::{ReasoningEngineState, EngineState, PlanBuilder};

// 1. Initialise state with a goal
let mut state = ReasoningEngineState::new()
    .with_goal("list all Rust files")
    .with_max_revisions(3);

// 2. Build a plan from the available tool surface
let builder = PlanBuilder::default();
state.plan = builder.build_plan(
    "list all Rust files",
    &["list_directory", "search_content"],
);

// 3. Commit the plan — FSM transitions from EvaluateOptions to CommitPlan
state.transition(EngineState::CommitPlan).unwrap();

// Postconditions:
// state.plan contains [UseTool("list_directory"), NoOp]
// state.state == EngineState::CommitPlan
// Engine is ready for the first ExecuteStep transition
```

---

## 5. How RPL and Engine Work Together

The two systems are designed to interoperate at several well-defined seams.

### Shared call site: `route_with_tools_traced`

`CpuRouter::route_with_tools_traced()` is the single call site where both systems connect to the
router. The method accepts an `RplLayer` (for passive tracing) and an optional
`&mut ReasoningEngineState` (for active planning). The engine runs first — it mutates the
`RouterRequest` and produces the execution plan — then the RPL hooks observe the resulting tool
selections and record them into the `ReasoningTrace`.

### Log correlation via `engine_id` / `trace_id`

Every `EngineObserver` event is tagged with `engine_id`, which is set to the same UUID value as
`ReasoningTrace::trace_id`. This means a single grep or log query on a UUID will return both
RPL-level trace events and Engine-level FSM transition events for the same agent turn, in
chronological order.

### `ArbitrationEngine` writes into the RPL trace

`ArbitrationEngine::commit_selection()` appends a `ToolEvaluation` entry directly into the
`ReasoningTrace` that `RplLayer` is currently building. This is the primary mechanism by which
engine-computed confidence scores become part of the permanent trace record, enabling post-hoc
analysis of why a particular tool was chosen over alternatives.

### Shared suppression and privacy rules

`SuppressionLayer` guards both systems with the same policy. The `EngineObserver` respects the
same `ReasoningLogLevel` enum defined in `src/rpl/logging.rs`. Raising or lowering the log level
via `RUST_LOG` or `RplConfig` affects both the RPL trace output and the engine observability
events uniformly. Neither system ever emits user data to the ACP wire without passing through the
redaction gate.

---

## 6. Test Coverage Summary

| Test File | Tests | What it covers |
|-----------|-------|----------------|
| `tests/rpl_integration.rs` | 12 | RPL schema, suppression, redaction, CPU lifecycle, skill arbitration |
| `tests/engine_integration.rs` | 15 | Full engine lifecycle, Bayesian updates, planning, memory, arbitration, self-correction |
| `src/rpl/*.rs` (unit) | 84 | All RPL module internals (schema, hooks, logging, suppression, validation, redaction) |
| `src/engine/*.rs` (unit) | 116 | All engine module internals (state, beliefs, planner, memory bridge, arbitration, correction, observability) |
| **Total** | **227** | Full reasoning system coverage |

Run the full suite with:

```
cargo test --lib && cargo test --test rpl_integration --test engine_integration
```

---

## 7. Configuration

Both systems expose typed configuration structs. All structs implement `Default` so only the fields
that differ from the safe production defaults need to be set explicitly.

### RPL configuration

| Struct / Field | Type | Default | Description |
|----------------|------|---------|-------------|
| `RplConfig::log_level` | `ReasoningLogLevel` | `Summary` | Verbosity of the log line emitted by `on_complete()` |
| `RplConfig::lenient_validation` | `bool` | `true` | If `true`, validation errors are logged as warnings rather than returning `Err` |
| `RplConfig::expose_in_acp` | `bool` | `false` | Gate that allows traces to cross the ACP boundary — keep `false` in production |

### Engine observability configuration

| Struct / Field | Type | Default | Description |
|----------------|------|---------|-------------|
| `ObserverConfig::log_level` | `ReasoningLogLevel` | `Summary` | Matches the RPL hierarchy; controls `EngineObserver` verbosity |
| `ObserverConfig::redaction` | `RedactionConfig` | default rules | Applied to goal and hypothesis text before emission |
| `ObserverConfig::suppressed` | `bool` | `true` | Mirrors the RPL suppression gate for engine events |

### Self-correction configuration

| Struct / Field | Type | Default | Description |
|----------------|------|---------|-------------|
| `CorrectionConfig::uncertainty_threshold` | `f64` | `0.6` | `should_correct()` fires when `state.uncertainty` exceeds this value |

### Memory bridge configuration

| Struct / Field | Type | Default | Description |
|----------------|------|---------|-------------|
| `MemoryBridgeConfig::write_uncertainty_threshold` | `f64` | `0.5` | `should_write_memory()` only writes facts when `uncertainty` is below this value |
| `MemoryBridgeConfig::max_facts` | `usize` | `20` | Upper bound on the number of facts retrieved per turn |

### Arbitration configuration

| Struct / Field | Type | Default | Description |
|----------------|------|---------|-------------|
| `ArbitrationConfig::fallback_uncertainty_threshold` | `f64` | `0.7` | Confidence level below which the fallback `NoOp` step is inserted |
| `ArbitrationConfig::rpl_weight` | `f64` | `0.4` | Weight given to RPL trace tool-evaluation scores in the joint ranking |
| `ArbitrationConfig::plan_weight` | `f64` | `0.6` | Weight given to `EngineBeliefs` posterior scores in the joint ranking |

---

*This document is auto-maintained. Update it whenever a new module is added to `src/rpl/` or
`src/engine/`, and increment `schema_version` in `src/rpl/schema.rs` for any breaking change to
`ReasoningTrace` field semantics. Record all changes in `CHANGELOG.md`.*