# Full Reasoning Engine — Architecture Document

> **Status:** Design specification — `src/engine/` scaffolded, implementation pending Tasks 93–99.
> **Author:** john mcconnell \<john.microtech@gmail.com\>
> **Repository:** https://github.com/micro-tech/grok-cli
> **Buy me a coffee:** https://buymeacoffee.com/micro.tech

---

## Table of Contents

1. [Overview](#1-overview)
2. [Component Responsibilities](#2-component-responsibilities)
3. [CPU Phase Mapping](#3-cpu-phase-mapping)
4. [Core State Model](#4-core-state-model)
5. [Interfaces to MemoryManager and Skill Arbitration](#5-interfaces-to-memorymanager-and-skill-arbitration)
6. [Bayesian Integration](#6-bayesian-integration)
7. [Self-Correction Loops](#7-self-correction-loops)
8. [Observability](#8-observability)
9. [Future Work](#9-future-work)
10. [Appendix A — File Inventory](#appendix-a--file-inventory)
11. [Appendix B — Changelog](#appendix-b--changelog)

---

## 1. Overview

### 1.1 What the Reasoning Engine Does

The **Full Reasoning Engine** (`src/engine/`) is an *active decision-making* subsystem that sits between the `CpuRouter` and the backend API call. Where the `CpuRouter` routes requests and executes tools mechanically, the Reasoning Engine provides goal-directed intelligence: it interprets the user's intent, generates and ranks hypotheses, decomposes goals into ordered execution plans, selects tools with Bayesian confidence scores, writes useful facts back to long-term memory, and detects when a plan has failed and must be revised.

In concrete terms, the engine answers three questions on every agent turn:

1. **What does the user actually want?** — Goal inference via `EngineState::AnalyzeGoal` and `EngineBeliefs`.
2. **How should we achieve it?** — Multi-step plan construction via `PlanBuilder` and `MemoryBridge`.
3. **Did it work?** — Outcome evaluation and bounded self-correction via `CorrectionEngine`.

### 1.2 What the Reasoning Engine Does NOT Do

| Out of scope | Rationale |
|---|---|
| ACP protocol encoding / decoding | Handled exclusively by `src/acp/`. The engine never touches `SessionUpdate`, `PromptRequest`, or any JSON-RPC framing. |
| Binary CLI argument parsing | Handled by `src/bin/` and `src/cli/`. The engine has no concept of `argv`. |
| Backend HTTP calls | Handled by `CpuRouter` and `src/router/backends/`. The engine emits `PlanStep::ModelCall` entries; the router executes them. |
| Rendering output to the terminal | Handled by `src/display/`. The engine produces structured `RouterResponse` and trace data only. |
| Credential or secrets management | Handled by `src/security/`. The engine reads from `MemoryBridge`; it never touches `.env` files. |

### 1.3 How the Engine Differs from the RPL

This distinction is critical for every developer working on the project.

| Dimension | Reasoning Protocol Layer (`src/rpl/`) | Reasoning Engine (`src/engine/`) |
|---|---|---|
| **Role** | Passive observer — records what happened | Active planner — decides what to do |
| **Mode** | Fire-and-forget hooks called by `CpuRouter` | Invoked synchronously before and during the tool loop |
| **State** | Stateless — one `ReasoningTrace` per turn, immutable once `on_complete()` fires | Stateful per turn — `ReasoningEngineState` evolves through a finite-state machine |
| **Output** | `ReasoningTrace` (UUID-linked telemetry envelope) | `Vec<PlanStep>` (executable plan) + revised `RouterRequest` |
| **Side effects** | Structured log emission via `tracing`; no mutation of the request or response | Writes to `LongTermMemory` via `MemoryBridge`; updates `BayesianEngine` priors via `EngineBeliefs` |
| **Failure mode** | Swallows errors; logs a warning; never propagates panics | Returns `EngineState::Failed(msg)` so the router can surface a graceful error |
| **Coupling** | Reads the trace it created; does not call `BayesianEngine` or `LongTermMemory` | Owns `EngineBeliefs` (wraps `BayesianEngine`) and calls `MemoryBridge` (wraps `LongTermMemory`) |

**In plain English:** the RPL asks *"what happened and why?"*; the Reasoning Engine asks *"what should happen next?"*

---

## 2. Component Responsibilities

### 2.1 Submodule Table

| Module | Primary Type | Responsibility |
|---|---|---|
| `state.rs` | `ReasoningEngineState` | Core FSM: stores the goal, hypotheses, plan, step cursor, selected tools, memory references, uncertainty score, and revision count. Owns the `EngineState` enum transition logic. |
| `beliefs.rs` | `EngineBeliefs` | Wraps `BayesianEngine`. Translates raw text and tool outcomes into posterior probability updates. Scores hypotheses and plan steps using the live belief distribution. |
| `planner.rs` | `PlanBuilder` | Decomposes an inferred goal into an ordered `Vec<PlanStep>`. Consults `EngineBeliefs` for step weights and `MemoryBridge` for factual pre-conditions. Supports plan revision (`revise_plan`). |
| `memory_bridge.rs` | `MemoryBridge` | Queries `LongTermMemory::search()` for facts relevant to the current goal. Decides when session results are worth writing back (`should_write_memory`). Calls `LongTermMemory::save_fact()`. |
| `arbitration.rs` | `ArbitrationEngine` | Performs joint tool selection: combines `AutoActivationEngine::check_with_reasoning()` scores with `EngineBeliefs` posterior probabilities to produce a ranked tool list. Provides a fallback step when no tool clears the confidence threshold. |
| `correction.rs` | `CorrectionEngine` | Examines step outcomes and uncertainty scores after each `ExecuteStep`. Triggers `PlanBuilder::revise_plan()` when a failure or low-confidence outcome is detected. Enforces the `max_revisions` guard to prevent infinite loops. |
| `observability.rs` | `EngineObserver` | Emits structured `tracing` events for every FSM state transition, plan revision, and step completion. Tags every event with `engine_id` for cross-correlation with the RPL `trace_id`. Respects the same `ReasoningLogLevel` hierarchy as `src/rpl/logging.rs`. |

### 2.2 Type Ownership Map

```
src/engine/
├── state.rs          ReasoningEngineState
│                       ├── EngineState          (FSM enum)
│                       ├── Vec<Hypothesis>      (ranked interpretations)
│                       └── Vec<PlanStep>        (executable action list)
│
├── beliefs.rs        EngineBeliefs
│                       └── BayesianEngine       (src/bayes/engine.rs)
│
├── planner.rs        PlanBuilder
│                       └── produces Vec<PlanStep>
│
├── memory_bridge.rs  MemoryBridge
│                       └── LongTermMemory       (src/memory/long_term.rs)
│
├── arbitration.rs    ArbitrationEngine
│                       ├── AutoActivationEngine (src/skills/auto_activate.rs)
│                       └── EngineBeliefs
│
├── correction.rs     CorrectionEngine
│                       └── calls PlanBuilder::revise_plan()
│
└── observability.rs  EngineObserver
                        └── ReasoningLogLevel    (src/rpl/logging.rs)
```

---

## 3. CPU Phase Mapping

### 3.1 Position in the Full Stack

The diagram below shows where the Reasoning Engine sits relative to every other major subsystem. Components marked `[passive]` only observe; components marked `[active]` mutate state or produce artefacts consumed downstream.

```
User Prompt
    │
    ▼
ACP Layer  (src/acp/)
    │  PromptRequest
    ▼
CpuRouter::route_with_tools_traced()   (src/router/cpu_router.rs)
    │
    ├──► RplLayer::on_pre_evaluate()                [RPL - passive]
    │         └── creates ReasoningTrace { trace_id: UUID-v4, phase: PreEvaluation }
    │
    ├──► ReasoningEngine::analyze_goal()            [Engine - active]
    │         │
    │         ├──► EngineBeliefs::update_from_text()
    │         │         └── BayesianEngine::update_from_text(input)
    │         │
    │         ├──► PlanBuilder::build_plan()
    │         │         └── produces Vec<PlanStep>
    │         │
    │         └──► MemoryBridge::relevant_facts()
    │                   └── LongTermMemory::search(goal_keywords)
    │
    ├──► [For each tool call in the plan]
    │         ├──► ArbitrationEngine::rank_tools()  [Engine - active]
    │         │         ├── AutoActivationEngine::check_with_reasoning(rpl_trace)
    │         │         └── EngineBeliefs::score_tool(tool_name)
    │         │
    │         └──► RplLayer::on_tool_selection()    [RPL - passive]
    │                   └── appends ToolEvaluation to ReasoningTrace
    │
    ├──► CorrectionEngine::should_correct()         [Engine - active]
    │         └── (if yes) PlanBuilder::revise_plan()
    │                           └── increments revision_count; returns Err when
    │                               revision_count >= max_revisions
    │
    ├──► MemoryBridge::write_summary()              [Engine - active, conditional]
    │         └── LongTermMemory::save_fact(MemorySource::Inferred)
    │
    └──► RplLayer::on_complete()                    [RPL - passive]
              └── validates + logs ReasoningTrace via log_trace()
```

### 3.2 EngineState to CPU Action Mapping

The `EngineState` enum drives the FSM. Each variant maps to a discrete set of CPU-level operations. Transitions are always forward-only except for the `RevisePlan` back-edge into `CommitPlan`.

| `EngineState` | CPU Action | Next State(s) |
|---|---|---|
| `AnalyzeGoal` | Parse user intent from the last user message; call `EngineBeliefs::update_from_text()`; populate `state.goal` | `ExpandOptions` |
| `ExpandOptions` | Generate `Vec<Hypothesis>` from `BayesianEngine::best_intent()` and keyword heuristics; rank by initial posterior | `EvaluateOptions` |
| `EvaluateOptions` | Score each `Hypothesis` with `EngineBeliefs::score_hypothesis()`; set `state.uncertainty`; check `BayesianEngine::needs_clarification()` | `CommitPlan` or `AskClarification` |
| `AskClarification` | Emit a clarification request as a `ModelCall` plan step; pause FSM until next user turn | `AnalyzeGoal` (next turn) |
| `CommitPlan` | Call `PlanBuilder::build_plan()`; write ordered `Vec<PlanStep>` to `state.plan`; set `current_step_index = 0` | `ExecuteStep(0)` |
| `ExecuteStep(n)` | Execute `state.plan[n]` — one of `UseTool`, `QueryMemory`, `ModelCall`, or `NoOp`; update `EngineBeliefs` with outcome | `ExecuteStep(n+1)`, `RevisePlan`, or `Complete` |
| `RevisePlan` | Increment `revision_count`; abort with `Failed` if `revision_count >= max_revisions`; call `PlanBuilder::revise_plan()` with new evidence | `CommitPlan` |
| `Complete` | Optionally call `MemoryBridge::write_summary()`; return final `RouterResponse`; hand `ReasoningTrace` to `RplLayer::on_complete()` | — (terminal) |
| `Failed(msg)` | Log the failure message at `Error` level; return a graceful error payload to `CpuRouter`; the RPL trace is still completed | — (terminal) |

### 3.3 State Transition Diagram

```
                        ┌─────────────┐
                        │ AnalyzeGoal │◄──── (new turn)
                        └──────┬──────┘
                               │
                               ▼
                       ┌───────────────┐
                       │ ExpandOptions │
                       └───────┬───────┘
                               │
                               ▼
                      ┌─────────────────┐
                      │ EvaluateOptions │
                      └────┬────────────┘
                           │
              ┌────────────┴─────────────┐
              │ high confidence          │ low confidence
              ▼                          ▼
        ┌────────────┐         ┌──────────────────┐
        │ CommitPlan │         │ AskClarification │
        └─────┬──────┘         └──────────────────┘
              │
              ▼
      ┌───────────────┐
      │ ExecuteStep(n)│◄──────────────────┐
      └───┬───────────┘                   │
          │                               │
    ┌─────┼────────────┐                  │
    │     │            │                  │
    ▼     ▼            ▼                  │
┌──────┐ ┌──────┐  ┌────────────┐        │
│ n+1  │ │ Done │  │ RevisePlan │────────┘
└──┬───┘ └──┬───┘  └──────┬─────┘ (if revision_count < max_revisions)
   │        │              │
   │        │              └──► Failed(msg)  (if revision_count >= max_revisions)
   │        ▼
   │   ┌──────────┐
   └──►│ Complete │
       └──────────┘
```

---

## 4. Core State Model

### 4.1 `ReasoningEngineState`

`ReasoningEngineState` is the single source of truth for one agent turn. It is constructed at the start of `ReasoningEngine::analyze_goal()` and lives for the duration of the turn. It is never serialised to disk mid-turn, but its final snapshot informs `MemoryBridge::write_summary()` decisions.

| Field | Type | Description |
|---|---|---|
| `engine_id` | `String` | UUID v4 generated at construction. Linked to the RPL `trace_id` for end-to-end correlation. Format: `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`. |
| `schema_version` | `u32` | Forward-compatibility version. Mirrors `RPL_SCHEMA_VERSION` (currently `1`). Increment when `PlanStep` or `Hypothesis` layouts change. |
| `state` | `EngineState` | Current FSM state. Only transitions via `ReasoningEngineState::advance()`. |
| `goal` | `Option<String>` | Inferred user intent as a short declarative string, e.g. `"list files in /tmp"`. `None` until `AnalyzeGoal` completes. |
| `hypotheses` | `Vec<Hypothesis>` | Ranked list of candidate interpretations, highest confidence first. Populated in `ExpandOptions`; pruned in `EvaluateOptions`. |
| `plan` | `Vec<PlanStep>` | Ordered sequence of actions produced by `PlanBuilder`. Mutated only by `CommitPlan` and `RevisePlan`. |
| `current_step_index` | `usize` | Index into `plan` pointing at the step currently being executed. Advanced after each successful `ExecuteStep`. |
| `selected_tools` | `Vec<String>` | Tool names committed by `ArbitrationEngine::select_tool()` for this turn. Used by `RplLayer::on_tool_selection()` for RPL trace enrichment. |
| `memory_references` | `Vec<String>` | Memory entry IDs consulted via `MemoryBridge::relevant_facts()`. Stored for auditing and write-back decisions. |
| `uncertainty` | `f32` | Overall confidence measure in `[0.0, 1.0]`. Mirrors `BayesianEngine::probability("low_confidence")` after each `EngineBeliefs` update. `0.0` = fully confident; `1.0` = maximally uncertain. |
| `revision_count` | `u32` | Number of times `RevisePlan` has been entered in this turn. Reset to `0` on a new turn. |
| `max_revisions` | `u32` | Hard cap on `revision_count`. Default `3`. When `revision_count >= max_revisions`, the engine transitions to `Failed`. Configurable per-session. |
| `created_at` | `DateTime<Utc>` | Wall-clock time at state construction. Set once; never mutated. |
| `updated_at` | `DateTime<Utc>` | Wall-clock time of the most recent `advance()` call. Updated on every FSM transition. |

### 4.2 `EngineState` Enum

```rust
/// The finite-state machine driving one agent reasoning turn.
///
/// Transitions advance monotonically with the single exception of
/// `RevisePlan` → `CommitPlan` (the self-correction back-edge).
#[derive(Debug, Clone, PartialEq)]
pub enum EngineState {
    /// Parse the user's raw input into a goal string.
    AnalyzeGoal,
    /// Generate candidate hypotheses from the current belief distribution.
    ExpandOptions,
    /// Score and rank hypotheses; decide whether to commit or ask for clarification.
    EvaluateOptions,
    /// Emit a clarification question and await the next user turn.
    AskClarification,
    /// Invoke PlanBuilder to produce the ordered Vec<PlanStep>.
    CommitPlan,
    /// Execute plan[n]. Contains the step index for progress tracking.
    ExecuteStep(usize),
    /// A step failed or confidence dropped; rebuild the plan with new evidence.
    RevisePlan,
    /// All steps complete; optionally write memory; return final response.
    Complete,
    /// Unrecoverable failure. Contains a human-readable error message.
    Failed(String),
}
```

### 4.3 `Hypothesis`

A `Hypothesis` represents one plausible interpretation of the user's goal. Multiple hypotheses may exist simultaneously; the highest-confidence one drives `PlanBuilder`.

| Field | Type | Description |
|---|---|---|
| `id` | `String` | UUID v4 identifier. Stable across `EvaluateOptions` scoring iterations. |
| `description` | `String` | Short declarative description of the interpretation, e.g. `"User wants to search the web for Rust documentation"`. |
| `confidence` | `f32` | Posterior probability in `[0.0, 1.0]` assigned by `EngineBeliefs::score_hypothesis()`. |
| `intent_key` | `String` | The `BayesianEngine` intent key driving this hypothesis, e.g. `"intent_search"`, `"intent_edit"`, `"intent_shell"`, `"intent_question"`. |
| `supporting_tools` | `Vec<String>` | Tool names that best serve this hypothesis, populated from `BayesianEngine::best_intent()` and `ArbitrationEngine::rank_tools()`. |

### 4.4 `PlanStep` and `StepAction`

A `PlanStep` is one discrete action in the execution plan. The `StepAction` variant determines which subsystem handles it.

```rust
/// A single executable step in the reasoning plan.
#[derive(Debug, Clone)]
pub struct PlanStep {
    /// Position in the plan (0-indexed). Stable across revisions.
    pub index: usize,
    /// The action to perform.
    pub action: StepAction,
    /// Execution outcome, set after the step runs.
    pub status: StepStatus,
    /// Optional human-readable note for observability.
    pub note: Option<String>,
}

/// The concrete work a PlanStep performs.
#[derive(Debug, Clone)]
pub enum StepAction {
    /// Execute a registered tool. `tool_name` must match a key in the tool registry.
    UseTool { tool_name: String, args: serde_json::Value },
    /// Query LongTermMemory for relevant facts. Populates `state.memory_references`.
    QueryMemory { keywords: Vec<String> },
    /// Make a model API call via CpuRouter::route(). Used for sub-goal resolution.
    ModelCall { prompt: String },
    /// Intentional no-op. Used as a placeholder during plan construction
    /// or when all meaningful work is complete but the plan list is not yet closed.
    NoOp,
}

/// The execution outcome of a PlanStep.
#[derive(Debug, Clone, PartialEq)]
pub enum StepStatus {
    /// Not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Done,
    /// Execution failed. Contains the error string.
    Failed(String),
    /// Skipped due to a plan revision making this step obsolete.
    Skipped,
}
```

---

## 5. Interfaces to MemoryManager and Skill Arbitration

### 5.1 `MemoryBridge`

`MemoryBridge` is the engine's read/write interface to the persistent memory tiers. It wraps `LongTermMemory` (persisted to `~/.grok/memory.json` and mirrored to `~/.grok/memory.md`) and is also aware of the `MemorySource` enum values (`User`, `Inferred`, `System`) used by the underlying store.

The bridge deliberately does **not** expose the full `LongTermMemory` API. It provides a narrow, goal-aware surface that prevents the engine from over-reading or over-writing memory.

```
/// Query long-term memory for facts relevant to the current engine goal.
///
/// Internally calls LongTermMemory::search(keywords) where keywords are
/// extracted from state.goal. Results are ranked by MemoryEntry::relevance.
/// Returns at most MAX_FACTS_IN_PROMPT entries (currently 20, set in long_term.rs).
MemoryBridge::relevant_facts(
    state: &ReasoningEngineState,
    long_term: &LongTermMemory,
) -> Vec<MemoryEntry>

/// Decide whether the completed turn has produced facts worth persisting.
///
/// Returns true when:
///   - state.revision_count == 0  (clean first-attempt success)
///   - state.uncertainty < 0.4    (high-confidence outcome)
///   - state.plan contains at least one StepStatus::Done UseTool step
///
/// This heuristic is intentionally conservative to avoid polluting
/// long-term memory with noisy or low-confidence inferences.
MemoryBridge::should_write_memory(state: &ReasoningEngineState) -> bool

/// Summarise the completed turn and write the result to LongTermMemory.
///
/// Calls LongTermMemory::save_fact(fact, MemorySource::Inferred).
/// The fact string is a compact declarative sentence derived from
/// state.goal and the outcomes of Done plan steps.
/// Returns Err if the underlying atomic write to disk fails.
MemoryBridge::write_summary(
    state: &ReasoningEngineState,
    memory: &mut LongTermMemory,
) -> Result<(), MemoryBridgeError>
```

**Memory tier access policy:**

| Tier | `MemoryBridge` access | Rationale |
|---|---|---|
| `ShortTerm` | None | The conversation window is managed by `CpuRouter` and the ACP session; the engine reads goal context from `state.goal`, not from raw messages. |
| `LongTerm` | Read (`relevant_facts`) and Write (`write_summary`) | Persistent facts directly inform goal planning and should capture confirmed inferences. |
| `Episodic` | Read-only (future) | Past session summaries are useful for long-horizon context but are not yet surfaced by `MemoryBridge`. |
| `Working` | None | Project context files loaded from `.grok/context.md` are injected at the ACP layer, before the engine is invoked. |

### 5.2 `ArbitrationEngine`

`ArbitrationEngine` performs the joint tool-ranking step that neither the RPL (passive) nor `AutoActivationEngine` alone (no Bayesian signal) can perform. It fuses two independent signals:

1. **`AutoActivationEngine::check_with_reasoning()`** — keyword score (weight 30), regex pattern score (weight 40), file-extension score (weight 25), with an RPL tool-name boost of +15 and an uncertainty penalty of −10 when `ReasoningTrace::uncertainty > 0.6`.
2. **`EngineBeliefs::score_tool(tool_name)`** — posterior probability from `BayesianEngine::probability(intent_key)` mapped onto the tool's primary intent.

The fused score is a weighted average: `0.6 * auto_activation_score + 0.4 * bayesian_score`.

```
/// Rank all candidate tools for the current plan step.
///
/// Calls AutoActivationEngine::check_with_reasoning() passing the live
/// RplLayer trace so skill scoring can use tool_evaluations and uncertainty.
/// Then weights each SkillMatch::confidence with EngineBeliefs::score_tool().
/// Returns the full ranked list, highest fused score first.
ArbitrationEngine::rank_tools(
    plan: &[PlanStep],
    rpl_trace: &ReasoningTrace,
) -> Vec<RankedTool>

/// Select a single tool for the current step.
///
/// Picks the top-ranked tool from rank_tools() only if its fused score
/// exceeds (1.0 - uncertainty) as a dynamic confidence threshold.
/// Returns None when no tool clears the threshold, prompting a fallback.
ArbitrationEngine::select_tool(
    ranked: &[RankedTool],
    uncertainty: f32,
) -> Option<RankedTool>

/// Produce a fallback PlanStep when select_tool() returns None.
///
/// The fallback is always StepAction::ModelCall with a conservative prompt
/// instructing the model to answer from its own knowledge.
/// reason is logged at Info level via EngineObserver for post-hoc diagnosis.
ArbitrationEngine::fallback_tool(
    plan: &[PlanStep],
    reason: &str,
) -> Option<PlanStep>
```

**`RankedTool` fields:**

| Field | Type | Description |
|---|---|---|
| `tool_name` | `String` | Registered tool name (matches the tool registry key). |
| `auto_activation_score` | `u8` | Raw `SkillMatch::confidence` from `AutoActivationEngine` (range `[0, 100]`). |
| `bayesian_score` | `f32` | Posterior probability from `EngineBeliefs` (range `[0.0, 1.0]`). |
| `fused_score` | `f32` | Weighted combination: `0.6 * (auto_activation_score / 100.0) + 0.4 * bayesian_score`. |
| `reasons` | `Vec<String>` | Human-readable justifications from `SkillMatch::reasons` plus a Bayesian score annotation. |

---

## 6. Bayesian Integration

### 6.1 How `EngineBeliefs` Wraps `BayesianEngine`

`EngineBeliefs` is a thin, goal-aware adapter over `BayesianEngine` (`src/bayes/engine.rs`). Its job is to translate engine-level events (user messages, tool outcomes, step results) into the probability-update API that `BayesianEngine` already provides.

The underlying `BayesianEngine` maintains a `HashMap<String, f32>` prior distribution over intent keys (`intent_question`, `intent_edit`, `intent_shell`, `intent_search`) and meta-states (`need_clarification`, `low_confidence`, `is_vague`). The distribution is persisted to `~/.grok/bayes_profile.json` and loaded on startup, so beliefs improve over time.

**Default configured thresholds (from `src/bayes/engine.rs`):**

| Threshold | Default Value | Effect |
|---|---|---|
| `clarification_threshold` | `0.4` | `BayesianEngine::needs_clarification()` fires when `P(need_clarification) > 0.4`. Engine transitions to `AskClarification`. |
| `uncertainty_threshold` | `0.6` | `BayesianEngine::is_high_uncertainty()` fires when `P(need_clarification) > 0.6` or `P(low_confidence) > 0.6`. Influences `ArbitrationEngine` penalty and `CorrectionEngine` trigger. |
| `vagueness_threshold` | `0.6` | `BayesianEngine::is_vague()` fires when `P(is_vague) > 0.6`. Planner adds a clarifying `ModelCall` step. |

### 6.2 Update Events

`EngineBeliefs` calls the following `BayesianEngine` methods at defined engine lifecycle points:

| Engine event | `EngineBeliefs` call | Underlying `BayesianEngine` method |
|---|---|---|
| User message received (start of `AnalyzeGoal`) | `update_from_text(input)` | `BayesianEngine::update_from_text(&str)` — keyword-likelihood spike against `intent_*` priors using the configured `intent_likelihood_weight`. |
| Tool call succeeded | `update_from_tool_result(tool_name, success: true)` | `BayesianEngine::update_profile(tool_name)` — boosts the prior for the matched intent by `profile_learning_rate` (default `0.10`, i.e. 10%), then re-normalises and persists to disk. |
| Tool call failed | `update_from_tool_result(tool_name, success: false)` | `BayesianEngine::update_from_tool_failure()` — applies the `likelihood_from_tool_failure()` vector, raising `P(low_confidence)`. |
| Model confidence score available | `update_from_model_confidence(score)` | `BayesianEngine::update_from_model_confidence(f32)` — likelihood update weighted by the model's self-reported score in `[0.0, 1.0]`. |

### 6.3 How Beliefs Flow into the Plan

After each `EngineBeliefs` update, the engine reads back three derived quantities:

1. **`BayesianEngine::best_intent()`** — the intent key with the highest posterior probability. Drives `Hypothesis::intent_key` assignment in `ExpandOptions`.
2. **`BayesianEngine::probability("low_confidence")`** — mapped directly onto `ReasoningEngineState::uncertainty`. When `uncertainty > uncertainty_threshold`, `CorrectionEngine::should_correct()` becomes eligible to fire.
3. **`BayesianEngine::probability(intent_key)`** — used by `ArbitrationEngine` as the `bayesian_score` component of the fused tool ranking score.

The belief graph can be visualised at any point via `BayesianEngine::visualize()`, which renders an ASCII bar chart — useful in `EngineObserver` `Trace`-level logs.

---

## 7. Self-Correction Loops

### 7.1 When `CorrectionEngine::should_correct()` Fires

The `CorrectionEngine` evaluates three independent trigger conditions after every `ExecuteStep` transition. Any single true condition is sufficient to return `true` from `should_correct()`.

| Trigger | Source | Description |
|---|---|---|
| **Low confidence outcome** | `state.uncertainty > uncertainty_threshold` | `BayesianEngine::is_low_confidence()` returns `true` after the step's belief update. The step may have technically succeeded but the engine's confidence in the result is insufficient to continue the current plan. |
| **Step failure** | `plan[current_step_index].status == StepStatus::Failed(_)` | The step execution returned an error. This may be a transient network failure (retried at the `CpuRouter` level), a missing tool, or a model refusal. |
| **User feedback signal** | Explicit signal injected into the next `AnalyzeGoal` call | When the user's follow-up message contains correction language (e.g. "that's wrong", "no, I meant…"), `BayesianEngine::update_from_text()` raises `P(need_clarification)` above the threshold and the engine re-enters `AnalyzeGoal` for the new turn. This is turn-boundary correction, not mid-turn correction. |

### 7.2 The Bounded Loop Safeguard

The `max_revisions` field on `ReasoningEngineState` is the primary safeguard against infinite self-correction loops. Its default value is `3`. The enforcement logic in `CorrectionEngine` is as follows:

```
fn should_correct(state: &ReasoningEngineState, step_result: &StepStatus) -> bool {
    // Hard stop: never correct if we are already at the cap.
    if state.revision_count >= state.max_revisions {
        return false;  // Engine will transition to Failed on the next step failure.
    }

    match step_result {
        StepStatus::Failed(_) => true,
        StepStatus::Done => {
            state.uncertainty > UNCERTAINTY_THRESHOLD  // 0.6
        }
        _ => false,
    }
}
```

When `should_correct()` returns `false` because the cap has been reached and the current step has also failed, `CorrectionEngine` returns a `CorrectionDecision::FailGracefully(reason)` that causes `ReasoningEngineState::advance()` to transition into `EngineState::Failed(msg)`.

### 7.3 The Full Correction Cycle

```
ExecuteStep(n)
    │
    ▼ (step completes with Done or Failed)
CorrectionEngine::should_correct()
    │
    ├── false  (confidence OK, no failure, or cap reached gracefully)
    │       │
    │       └──► ExecuteStep(n+1)  or  Complete
    │
    └── true
            │
            ▼
        revision_count += 1
            │
            ├── revision_count >= max_revisions?
            │           │
            │           └── YES ──► EngineState::Failed("max revisions reached")
            │
            └── NO
                    │
                    ▼
            PlanBuilder::revise_plan(state, new_evidence)
                    │
                    │   (re-evaluates remaining steps with updated
                    │    EngineBeliefs posterior; may drop, reorder,
                    │    or replace steps; does NOT reset current_step_index
                    │    to 0 — only the remaining steps are rebuilt)
                    │
                    ▼
            EngineState::CommitPlan
                    │
                    ▼
            ExecuteStep(current_step_index)   ← resumes from same position
```

**Key invariant:** `revision_count` counts plan revisions per turn, not step retries. A single step may not be retried more than once without incrementing `revision_count`. This ensures the total number of backend calls for one user turn is bounded by `(initial_plan_length + max_revisions * revised_plan_length)`.

---

## 8. Observability

### 8.1 `EngineObserver` and the RPL Logging Hierarchy

`EngineObserver` emits structured `tracing` events using the same `ReasoningLogLevel` enum defined in `src/rpl/logging.rs`. This means the verbosity of both the RPL trace events and the engine state events is controlled by a single `RplConfig::log_level` setting, keeping operator configuration simple.

Every event emitted by `EngineObserver` carries the `engine_id` field (a UUID v4 string identical to `ReasoningEngineState::engine_id`). Because `CpuRouter::route_with_tools_traced()` also tags its events with a `call_id` UUID, and `RplLayer::on_pre_evaluate()` mints a `trace_id` UUID for the `ReasoningTrace`, operators can correlate a single user turn across three separate log streams using the shared UUID value.

### 8.2 Event Levels and Fields

| Event type | `tracing` macro | Fields | `ReasoningLogLevel` minimum |
|---|---|---|---|
| FSM state transition | `tracing::debug!` | `engine_id`, `from_state`, `to_state`, `uncertainty`, `revision_count` | `Debug` |
| Plan revision | `tracing::info!` | `engine_id`, `revision_count`, `max_revisions`, `reason`, `new_plan_len` | `Summary` |
| Step completion | `tracing::trace!` | `engine_id`, `step_index`, `action_kind`, `status`, `duration_ms` | `Trace` |
| Correction triggered | `tracing::info!` | `engine_id`, `trigger`, `step_index`, `uncertainty` | `Summary` |
| Memory write | `tracing::debug!` | `engine_id`, `fact_id`, `source` (`"inferred"`), `fact_len` | `Debug` |
| Belief update | `tracing::trace!` | `engine_id`, `event_kind`, `best_intent`, `uncertainty_after` | `Trace` |
| Engine failure | `tracing::error!` | `engine_id`, `state`, `reason`, `revision_count` | `Off` (always emitted) |

The `tracing::error!` call for `EngineState::Failed` is always emitted regardless of `log_level`, because a reasoning failure is never suppressed — it indicates a degraded user experience that operators must be able to diagnose.

### 8.3 Suppression and Privacy

The RPL `SuppressionLayer` rules apply to engine observability output in the same way they apply to `ReasoningTrace` fields:

- `state.goal` (the inferred user intent string) is treated as potentially sensitive. It must not appear in `Summary`-level logs. It may appear in `Debug` and `Trace` level logs.
- `state.memory_references` (memory entry IDs) are not sensitive by themselves, but the associated `MemoryEntry::fact` text must not be logged directly. Log only the entry `id` and `relevance` score.
- `EngineBeliefs` belief updates must never log raw user input text at any level. Only derived quantities (`best_intent`, `uncertainty`) are logged.
- Events carrying `engine_id` are correlated with `trace_id` in the `ReasoningTrace`. If the RPL trace is suppressed (`ReasoningTrace::suppressed = true`), the engine events remain in the `tracing` log but are never forwarded to ACP `SessionUpdate` messages.

### 8.4 Enabling Full Engine Trace Logs

To enable `Trace`-level engine observability during development, configure `RplConfig` when constructing `CpuRouter`:

```rust
use grok_cli::rpl::{RplConfig, RplLayer, ReasoningLogLevel};

let rpl = RplLayer::new(RplConfig {
    log_level: ReasoningLogLevel::Trace,
    lenient_validation: true,
});

let router = CpuRouter::new(backends).with_rpl(rpl);
```

Then set the `RUST_LOG` environment variable to enable the `tracing` subscriber output:

```sh
RUST_LOG=grok_cli=trace cargo run
```

At `Trace` level, every FSM transition, every belief update, and every step completion is logged with full field payloads. This level is **not safe for production** — it may log inferred goal text and belief distributions that could contain user-identifying patterns.

---

## 9. Future Work

The following items are tracked in the task backlog and are explicitly out of scope for the initial `src/engine/` implementation (Tasks 93–99).

### 9.1 Persistent Reasoning State Across Sessions

Currently `ReasoningEngineState` is ephemeral — it is constructed at the start of a turn and discarded at the end. A future enhancement would serialise the final state snapshot to `~/.grok/sessions/<session_id>/engine_state.json`, enabling multi-turn goal tracking where the engine can remember which hypotheses were evaluated and which plan steps succeeded in previous turns.

This is architecturally similar to how `BayesianEngine` already persists its prior distribution to `~/.grok/bayes_profile.json` via `update_profile()` — the same atomic-write pattern from `src/memory/long_term.rs` would apply.

### 9.2 Parallel Step Execution for Independent `PlanStep`s

The current design executes `PlanStep` entries strictly in order (`ExecuteStep(n)` → `ExecuteStep(n+1)`). Many real plans contain steps that are logically independent — for example, a memory query and a web search can run concurrently without ordering constraints. A future `PlanBuilder` would annotate steps with dependency edges (similar to the `task_list.json` dependency model) and the executor would use `tokio::join!` or a task-pool to run independent steps in parallel, reducing per-turn latency.

### 9.3 LLM-Generated Hypotheses

The initial `ExpandOptions` implementation generates hypotheses from a fixed keyword→intent mapping (the same signal used by `BayesianEngine::update_from_text()`). A more powerful approach would issue a lightweight `ModelCall` step — using a small, fast model — to generate a diverse set of natural-language hypotheses before scoring them. This would improve handling of ambiguous or domain-specific requests that the keyword heuristics cannot parse reliably.

### 9.4 Online Belief Updates from User Feedback

`BayesianEngine::update_profile()` currently updates priors only when a tool is used successfully. A future enhancement would also fire an update when the user explicitly confirms or rejects a response (e.g. via a thumbs-up / thumbs-down UI in the Zed extension). This would provide a stronger learning signal than tool-success alone, accelerating personalisation of the prior distribution to each user's working style.

### 9.5 Hypothesis Confidence Calibration

The initial `EvaluateOptions` state scores hypotheses using raw `BayesianEngine::probability()` values, which are not calibrated probabilities — they are posterior beliefs that may be systematically over- or under-confident depending on the prior distribution shape. A calibration layer (Platt scaling or isotonic regression over logged hypothesis outcomes) would improve the reliability of the `uncertainty` field and make `CorrectionEngine` thresholds more semantically meaningful.

---

## Appendix A — File Inventory

| Path | Purpose |
|---|---|
| `src/engine/mod.rs` | Module root — re-exports primary types; documents the engine API surface |
| `src/engine/state.rs` | `ReasoningEngineState`, `EngineState`, `Hypothesis`, `PlanStep`, `StepAction`, `StepStatus` |
| `src/engine/beliefs.rs` | `EngineBeliefs` — wraps `BayesianEngine`; provides `update_from_text`, `update_from_tool_result`, `score_hypothesis`, `score_tool` |
| `src/engine/planner.rs` | `PlanBuilder` — `build_plan()` and `revise_plan()` |
| `src/engine/memory_bridge.rs` | `MemoryBridge` — `relevant_facts()`, `should_write_memory()`, `write_summary()` |
| `src/engine/arbitration.rs` | `ArbitrationEngine` — `rank_tools()`, `select_tool()`, `fallback_tool()` |
| `src/engine/correction.rs` | `CorrectionEngine` — `should_correct()`, `CorrectionDecision` |
| `src/engine/observability.rs` | `EngineObserver` — structured log emission at configurable `ReasoningLogLevel` |
| `src/rpl/schema.rs` | `ReasoningTrace`, `ToolEvaluation`, `MemoryConsideration`, `ReasoningPhase`, `RPL_SCHEMA_VERSION` |
| `src/rpl/layer.rs` | `RplLayer`, `RplConfig` — passive RPL hooks called by `CpuRouter` |
| `src/rpl/logging.rs` | `ReasoningLogLevel`, `log_trace()` — shared by RPL and engine observability |
| `src/rpl/validation.rs` | `validate()`, `ValidationError` — non-short-circuiting schema validation |
| `src/bayes/engine.rs` | `BayesianEngine` — belief updating, intent scoring, profile persistence |
| `src/bayes/belief_graph.rs` | `BeliefGraph`, `BeliefNode` — probability distribution storage and normalisation |
| `src/memory/long_term.rs` | `LongTermMemory` — persistent fact store with search and atomic writes |
| `src/memory/types.rs` | `MemoryEntry`, `ChatMessage`, `EpisodeSummary`, `MemoryKind`, `MemorySource` |
| `src/skills/auto_activate.rs` | `AutoActivationEngine`, `SkillMatch` — keyword/pattern/extension skill scoring |
| `src/router/cpu_router.rs` | `CpuRouter` — tool loop, `route_with_tools_traced()`, RPL hook calls |
| `docs/rpl_architecture.md` | RPL architecture document — the passive-layer counterpart to this document |
| `docs/engine_architecture.md` | This document |

---

## Appendix B — Changelog

| Date | Author | Change |
|---|---|---|
| 2025-07-14 | john mcconnell (AI-assisted) | Initial draft — covers Tasks 93.1, 93.2, 93.3; all nine sections written; grounded in actual `src/rpl/`, `src/bayes/`, `src/memory/`, `src/skills/`, and `src/router/` implementations. |