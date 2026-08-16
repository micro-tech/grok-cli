# Performance Measurement & Profiling (Task 266)

This document describes how to measure per-turn latency, allocations, and regressions in grok-cli.

## Quick Start (Recommended)

Enable timing output with the `GROK_PERF` environment variable:

```bash
# Non-interactive
GROK_PERF=1 grok chat "hello world"

# Interactive (TUI)
GROK_PERF=1 grok chat --interactive
```

You will see lines like:
```
[perf] single-chat turn took 1.23s
[perf] interactive turn took 0.87s
[perf] cli-interactive turn took 1.45s
[perf] explorer turn took 2.10s
```

These timings are emitted from the main hot paths:
- `src/display/interactive.rs` (rich TUI)
- `src/cli/commands/chat.rs` (CLI interactive + single-shot + explorer)

## Timing Locations (Implemented in Task 266)

| Path                        | Label                        | File                              |
|-----------------------------|------------------------------|-----------------------------------|
| Interactive TUI             | `interactive turn`           | `display/interactive.rs`          |
| CLI interactive loop        | `cli-interactive turn`       | `cli/commands/chat.rs`            |
| Single non-interactive chat | `single-chat turn`           | `cli/commands/chat.rs`            |
| Explorer mode               | `explorer turn`              | `cli/commands/chat.rs`            |

All use `std::time::Instant` and only print when `GROK_PERF` is set (zero cost when disabled).

## Building a Simple Benchmark

You can create a repeatable smoke test:

```bash
# 5-turn baseline
for i in {1..5}; do
  GROK_PERF=1 grok chat "What is 2+2? Turn $i" 2>&1 | grep '\[perf\]'
done
```

For a more structured test, add a test that exercises the paths (see `tests/perf_smoke.rs` example below).

## Allocation & Memory Profiling

### Using `dhat` (heap profiler)

1. Add to `Cargo.toml` (dev only):
   ```toml
   [dev-dependencies]
   dhat = "0.3"
   ```

2. Run with:
   ```bash
   cargo run --features dhat-heap -- chat "test message"
   ```

3. Analyze `dhat-heap.json` with `dh_view` or the web viewer.

### Manual allocation counting

Temporarily wrap hot paths with a counter (example in `src/utils/perf.rs`).

## Flamegraphs & CPU Profiling

### On Linux (perf + flamegraph)

```bash
cargo install flamegraph
cargo flamegraph -- chat "complex query with tools"
# Opens flamegraph.svg
```

### On macOS

Use `cargo-instruments` or Instruments.app.

### Build timings

```bash
cargo build --release --timings
# Open target/cargo-timings/cargo-timing.html
```

## Baseline Storage

Store repeatable numbers in this file or `Doc/PERF_BASELINES.md`:

Example entry (fill in with real data):

```
Date: 2025-...
Commit: abc1234
Machine: MacBook M2, 16GB
Rust: 1.82

Scenario                     | Median | P95  | Notes
-----------------------------|--------|------|-------
Single chat (no tools)       |  850ms | 1.1s |
Interactive first turn       | 1200ms | 1.8s |
10-turn interactive (mixed)  |  980ms | 1.4s |
ACP stdio (tool-using)       |  650ms | 950ms| (if measured)
```

## Adding More Instrumentation

- Use the `perf_guard!` macro from `src/utils/perf.rs` for scoped timing.
- For micro-benchmarks, add a `[[bench]]` in `Cargo.toml` + `criterion`.
- For CI, you can parse the `[perf]` lines and fail on regressions.

## Related Tasks

- Task 263: Static tool definitions (big allocation win)
- Task 264: Arc<str> history + reduced clones
- Task 265: Static suggestions (per-keystroke savings)
- Tasks 267–272: Further hot-path and build optimizations

When making changes, always run with `GROK_PERF=1` before/after to quantify impact.
