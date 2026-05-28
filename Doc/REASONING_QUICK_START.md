# Reasoning Systems — Quick Start Guide

> **Author:** john mcconnell <john.microtech@gmail.com>
> **Repository:** https://github.com/micro-tech/grok-cli
> **Buy me a coffee:** https://buymeacoffee.com/micro.tech

---

## 1. Overview

The **Reasoning Protocol Layer (RPL)** is a passive observability subsystem that wraps every `CpuRouter` tool-execution turn and produces a typed, suppressed `ReasoningTrace` — a machine-readable record of which tools were considered, how confident the agent was, and what memory was consulted. The **Reasoning Engine** is an active decision-making finite-state machine (FSM) that sits before the tool loop and answers three questions on every turn: *what does the user want, how do we achieve it, and did it work?* You would use the RPL when you need telemetry, auditability, or cross-turn correlation; you would use the Engine when you need goal-directed planning, Bayesian tool selection, or bounded self-correction.

---

## 2. Prerequisites

- **Rust 2024 edition** — the project targets the 2024 edition; ensure your `rustup` toolchain is up to date (`rustup update stable`).
- **`grok-cli` compiled** — run `cargo build` from the repository root before attempting any of the examples below. The `src/rpl/` and `src/engine/` modules must be present and compiling cleanly.
- **Familiarity with `src/router/cpu_router.rs`** — both systems hook into `CpuRouter::route_with_tools_traced`. Read that file before extending either system.
- **Network resilience** — this project targets a Starlink connection. All integration code you write must include timeouts and retry logic; see `src/router/backends/` for the established pattern.

---

## 3. Using the RPL in a New Call Site

### Step 1: Create an `RplLayer`

```rust
use grok_cli::rpl::{RplLayer, RplConfig, ReasoningLogLevel};

// Production default: Summary logging, lenient validation.
let layer = RplLayer::new(RplConfig {
    log_level: ReasoningLogLevel::Summary,  // one info! line per completed trace
    lenient_validation: true,               // out-of-range fields are clamped, not rejected
});
```

> **Tip:** Use `RplLayer::with_default_config()` as a shorthand that applies the same production defaults shown above.

---

### Step 2: Wrap your `route_with_tools_traced` call

```rust
use grok_cli::router::CpuRouter;

// Attach the RPL layer to the router before the call.
let router = CpuRouter::new(backends).with_rpl(layer);

// route_with_tools_traced returns the normal router response AND the trace.
let (response, trace) = router
    .route_with_tools_traced(request, &context, 10)   // 10 = max tool iterations
    .await?;

// At this point:
//   trace.phase      == ReasoningPhase::Complete
//   trace.suppressed == true  (safe — nothing leaked to the user)
```

The `10` argument is the maximum number of tool-loop iterations, matching the `max_iterations` parameter already used across the codebase. Respect the existing constant from `src/router/cpu_router.rs` rather than hard-coding a magic number.

---

### Step 3: Guard the trace before exposing it

```rust
use grok_cli::rpl::SuppressionLayer;

// production() builds a guard that respects suppressed = true.
let guard = SuppressionLayer::production();

if let Some(visible_trace) = guard.guard(&trace) {
    // This block is only reached when the trace is explicitly unsuppressed
    // (e.g. --debug-rpl flag or RplConfig::expose_in_acp = true).
    let redacted = guard.redact(visible_trace);
    println!("Trace ID : {}", redacted.trace_id);
    println!("Uncertainty: {:.3}", redacted.uncertainty);
}
// If suppressed == true (the default), guard() returns None and nothing is printed.
```

Never bypass the `SuppressionLayer`. Traces may contain goal text or tool arguments derived from user input; the guard is the last line of privacy enforcement before any log sink.

---

## 4. Using the Reasoning Engine

### Step 1: Create engine state

```rust
use grok_cli::engine::{ReasoningEngineState, EngineState};

let mut state = ReasoningEngineState::new()
    .with_goal("read the README and summarise it")
    .with_max_revisions(3);  // default; set lower in tests, higher for complex tasks

// After construction the FSM is in EngineState::AnalyzeGoal.
assert_eq!(state.current_state(), EngineState::AnalyzeGoal);
```

---

### Step 2: Build a plan

```rust
use grok_cli::engine::PlanBuilder;

let builder = PlanBuilder::default();
let available_tools = &["read_file", "list_directory", "web_search"];

// build_plan decomposes the goal into an ordered Vec<PlanStep>.
// It consults EngineBeliefs for step weights and MemoryBridge for pre-conditions.
state.plan = builder.build_plan(
    &state.goal.clone().unwrap(),
    available_tools,
);

// Commit the plan — transitions FSM from AnalyzeGoal → CommitPlan.
state.transition(EngineState::CommitPlan).expect("CommitPlan transition");

// state.plan now contains concrete PlanStep entries, e.g.:
//   [UseTool("read_file", ...), NoOp]
```

---

### Step 3: Execute with self-correction

```rust
use grok_cli::engine::{CorrectionEngine, CorrectionOutcome};

let corrector = CorrectionEngine::default();

// correct_until_stable drives the ExecuteStep → correction loop until either:
//   (a) all steps reach StepStatus::Done, or
//   (b) revision_count reaches max_revisions.
// The second argument is a per-call iteration cap (distinct from max_revisions).
let outcomes = corrector.correct_until_stable(&mut state, 5);

for (trigger, outcome) in &outcomes {
    match outcome {
        CorrectionOutcome::Revised   => println!("Plan revised after: {trigger}"),
        CorrectionOutcome::Stable    => println!("Plan stable after:  {trigger}"),
        CorrectionOutcome::FailedMax => println!("Max revisions hit:  {trigger}"),
    }
}

// On success the FSM is in EngineState::Complete.
```

> **Bounded by design:** `CorrectionEngine` will never call `revise_plan` more times than `state.max_revisions`. If the cap is reached and the current step has also failed, the engine transitions to `EngineState::Failed` — handle that variant in your call site.

---

## 5. Adding a Custom Redaction Rule

Redaction rules are applied to `MemoryConsideration` entries, the `goal` field, and the `context` field before any trace is written to a log sink. Rules are evaluated in order; the first matching rule wins.

```rust
use grok_cli::rpl::{RedactionConfig, RedactionRule, SuppressionLayer};

// Build a rule that masks e-mail addresses before they reach any log sink.
let email_rule = RedactionRule::new(
    "email",                                                    // rule name (for diagnostics)
    r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",       // regex pattern
    "[EMAIL REDACTED]",                                         // replacement text
).expect("email regex is valid");

// Add it to the default rule set (which already masks common secrets).
let mut config = RedactionConfig::default_rules();
config.add_rule(email_rule);

// Build a SuppressionLayer that applies the custom rules.
// false = not in expose mode (production); rules apply to every visible trace.
let layer = SuppressionLayer::new(false, config);
```

Use `RedactionConfig::default_rules()` as your starting point — it includes redaction for common patterns (tokens, UUIDs used as credentials, filesystem paths). Only add rules for domain-specific patterns your call site introduces.

---

## 6. Enabling Debug Traces

### Via environment variable (recommended for short sessions)

```/dev/null/shell.sh#L1-5
# Summary line only (production default, one info! per turn)
RUST_LOG=grok_cli::rpl=info cargo run -- chat "hello"

# Per-tool evaluation lines
RUST_LOG=grok_cli::rpl=debug cargo run -- chat "hello"

# Full JSON trace dump + engine FSM events
RUST_LOG=grok_cli::rpl=trace,grok_cli::engine=debug cargo run -- chat "hello"
```

### Via code (for integration tests or persistent dev configuration)

```rust
use grok_cli::rpl::{RplLayer, RplConfig, ReasoningLogLevel};

let layer = RplLayer::new(RplConfig {
    log_level: ReasoningLogLevel::Trace,   // full JSON trace written via tracing::trace!
    lenient_validation: false,             // strict — validation errors surface immediately
});
```

> **Note:** Raising `log_level` to `Trace` does **not** unsuppress the trace. The trace remains suppressed (`suppressed = true`) unless `RplConfig::expose_in_acp = true` is also set. Log verbosity and ACP exposure are independent controls.

---

## 7. Running the Test Suites

```/dev/null/shell.sh#L1-16
# Unit tests for the RPL module only
cargo test --lib rpl

# Unit tests for the Engine module only
cargo test --lib engine

# RPL integration tests (tests/rpl_integration.rs)
cargo test --test rpl_integration

# Engine integration tests (tests/engine_integration.rs)
cargo test --test engine_integration

# Full unit + integration run (recommended before every PR)
cargo test --lib && cargo test --test rpl_integration --test engine_integration

# With Clippy (required — project enforces zero Clippy warnings)
cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

> On a Starlink connection, integration tests that hit live backends may time out. The test helpers in `tests/common/` expose `with_timeout(Duration)` and `with_retries(u32)` wrappers — use them for any test that makes an outbound network call.

---

## 8. Troubleshooting

| Problem | Likely Cause | Fix |
|---------|--------------|-----|
| `guard()` always returns `None` | `suppressed = true` (the default) + `SuppressionLayer::production()` mode | Switch to `SuppressionLayer::debug()` in your dev environment, or explicitly set `trace.suppressed = false` after construction. |
| `validate()` returns `ValidationError::UncertaintyOutOfRange` | `uncertainty` field outside `[0.0, 1.0]` | Clamp your uncertainty value before passing it to `RplLayer`. Set `lenient_validation: true` in `RplConfig` to auto-clamp in production. |
| `validate()` returns `ValidationError::EmptyTraceId` | `trace_id` was not populated by `on_pre_evaluate()` | Ensure you call `on_pre_evaluate()` before any other hook; it is the only method that mints the `trace_id`. |
| `revise_plan()` returns `CorrectionError::MaxRevisionsExceeded` | `revision_count >= max_revisions` | Increase `max_revisions` on `ReasoningEngineState`, or catch the error and transition to a graceful fallback. |
| FSM transition returns `Err(InvalidTransition)` | Attempted an illegal state jump (e.g. `Complete → AnalyzeGoal`) | Check `src/engine/state.rs` for the allowed transition table. Transitions are strictly forward except `RevisePlan → CommitPlan`. |
| High uncertainty always triggers fallback tool | `uncertainty >= 0.7` in `ArbitrationEngine` — normal behaviour | This is by design. If the threshold is too aggressive for your use case, lower `ArbitrationConfig::fallback_uncertainty_threshold` from `0.7` toward `0.5`. |
| Tests time out in CI / on Starlink | Network drops during backend calls | Wrap outbound calls with `with_timeout(Duration::from_secs(30))` and `with_retries(3)` from `tests/common/`. |
| Trace JSON unexpectedly appears in `SessionUpdate` | `RplConfig::expose_in_acp = true` set in a shared config | Audit `config.toml` and any runtime overrides. The ACP leak detector in `src/rpl/validation.rs` will catch this in `#[cfg(test)]` builds. |

---

## 9. See Also

| Document | Location | Purpose |
|----------|----------|---------|
| RPL Architecture | `docs/rpl_architecture.md` | Full RPL design spec with schema definitions and suppression rules |
| Engine Architecture | `docs/engine_architecture.md` | Full Engine design spec with FSM diagram and Bayesian integration details |
| Reasoning Systems Overview | `docs/REASONING_SYSTEMS.md` | High-level index linking both systems and their test coverage |
| Project Layout | `project_layout.md` | Full source tree reference |
| Configuration Reference | `CONFIGURATION.md` | `config.toml` keys for `RplConfig` and `ObserverConfig` |