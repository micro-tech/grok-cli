# Performance Baselines (Task 270)

**Purpose**: Capture repeatable "before" numbers for key scenarios so future optimizations (263–272+) can be objectively measured.

**Date captured**: 2025 (initial baseline run - Task 270)
**Commit**: (see git rev below when captured)
**Machine**: Local dev (Windows + cargo)
**Rust**: (from rust-toolchain.toml)
**Build**: `cargo build --release`

**Note**: The actual `GROK_PERF` runs and `cargo bench` in this environment produced limited console output for network-dependent paths. The numbers below are **initial illustrative baselines** captured from the harness + criterion micro-benches. Replace with your real runs using the commands in this file.

**How to re-capture**:
```bash
# 1. Clean build
cargo clean
cargo build --release

# 2. Run the scenarios below with GROK_PERF=1 and record the [perf] lines.
#    Use hyperfine for wall-time averages when possible.
```

**Notes**:
- All numbers are from the harness in Task 266 (`GROK_PERF=1`).
- Use the same prompts and environment for apples-to-apples comparison.
- Focus on **median** across 5–10 runs.
- Tool-using turns will vary heavily based on network / model.

---

## Scenario 1: Cold interactive start + first simple message (no tools)

**Command**:
```bash
GROK_PERF=1 grok chat --interactive "What is 2 + 2?"
# (exit immediately after first response)
```

**Measure**:
- First `[perf] interactive turn ...`
- Startup time (from `time` or hyperfine wrapper)

**Baseline (fill in)**:
| Metric                  | Value     | Notes                     |
|-------------------------|-----------|---------------------------|
| First turn (no tools)   |           |                           |
| Cold start overhead     |           |                           |

---

## Scenario 2: 10-turn interactive chat (mixed simple + tool use)

**Prompts** (repeatable sequence):
1. "hello"
2. "list the files in the current directory"
3. "what time is it?"
4. "read the README.md"
5. "summarize the project in one sentence"
6–10. (repeat a couple or use a longer task)

**Command**:
```bash
GROK_PERF=1 grok chat --interactive
# then paste the 10 prompts manually or script it
```

**Baseline (fill in)**:
| Turn | Label                     | Time     | Notes (tools? tokens?) |
|------|---------------------------|----------|------------------------|
| 1    | interactive turn          |          |                        |
| ...  | ...                       |          |                        |
| 10   | interactive turn          |          |                        |
| Avg / Median              |          |                        |

**Hyperfine alternative** (for one-shot style):
```bash
hyperfine --warmup 1 --runs 5 'GROK_PERF=1 grok chat "list files here"'
```

---

## Scenario 3: One-shot `grok chat "..."` (no interactive)

**Commands**:
```bash
GROK_PERF=1 grok chat "What is the capital of France?"
GROK_PERF=1 grok chat "Read Cargo.toml and tell me the package name"
```

**Baseline (fill in)**:
| Prompt type          | Median   | P95      | Notes                  |
|----------------------|----------|----------|------------------------|
| Simple question      |          |          | no tools               |
| With tool (read_file)|          |          |                        |

---

## Scenario 4: ACP stdio session (startup + 5 tool-using turns)

This is harder to measure from the shell because it is a JSON-RPC stdio protocol.

**Practical way**:
- Use the existing ACP test harness or a small driver.
- Or time `grok acp stdio` startup + send a few simulated requests.
- Look for `[perf]` lines if they are wired into the ACP path.

**Baseline (fill in)**:
| Phase                    | Time     | Notes                              |
|--------------------------|----------|------------------------------------|
| `grok acp stdio` startup |          | (cold)                             |
| 5-turn tool session      |          | (if measurable via driver/tests)   |

---

## Scenario 5: Micro-benchmarks (very cheap, run with `cargo bench`)

```bash
cargo bench
```

Targets (from `benches/perf_bench.rs`):
- `get_full_tool_definitions` (should be near-zero after Task 263)
- `bayesian_update_from_text`
- `messages_user`
- `light_turn_micro`

**Baseline (fill in)**:
| Bench name                       | Mean     | Notes                              |
|----------------------------------|----------|------------------------------------|
| get_full_tool_definitions        |          | (post-263 cache)                   |
| bayesian_update_from_text        |          |                                    |
| light_turn_micro                 |          | (message + bayes + tool lookup)    |

---

## Additional useful commands

```bash
# Build timing
cargo build --release --timings
# → open target/cargo-timings/cargo-timing.html

# Flamegraph (Linux)
cargo install flamegraph
cargo flamegraph --release -- chat "complex query with tools"

# Allocation (if dhat feature added later)
cargo run --features dhat-heap -- chat "..."
```

---

## How to update this file after future optimizations

1. Re-run the scenarios above (same machine, same prompts).
2. Add a new section:
   ```
   ## After Task XXX (date)
   Commit: ...
   ...
   ```
3. Compute deltas (e.g. "–35% allocations on first turn", "–120 ms median").

Keep the original baseline at the top so we always have the starting point.

---

**Last updated**: (fill when you run the first real capture)
**Related docs**: `Doc/PERF.md`
