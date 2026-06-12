# Integration Test Harness — grok-cli

> **Task 148** — Fully Automated Integration Test Harness

This document describes the structure, coverage, and usage of the grok-cli
integration test harness.  Every test runs **offline** (no network, no API key),
uses isolated `tempfile::TempDir` state, and is deterministic.

---

## Quick start

```powershell
# Run the full integration harness
cargo test --test task_tools_tests --test file_tools_tests --test subsystem_tests --test cli_smoke_tests

# Or use the Makefile target (requires make / GNU Make for Windows)
make test-integration

# Run all tests including unit tests
cargo test

# Run a single suite with output
cargo test --test cli_smoke_tests -- --nocapture
```

---

## Test suites

| File | Tests | Subtask | What's covered |
|---|---|---|---|
| `tests/task_tools_tests.rs` | 18 | 148.6 | Task lifecycle, JSON formats, .bak recovery, atomic save |
| `tests/file_tools_tests.rs` | 23 | 148.4 + 148.5 | File I/O tools, security/path policy, path traversal |
| `tests/subsystem_tests.rs` | 20 | 148.7 + 148.8 | Memory, Bayesian router, config defaults, tool registry |
| `tests/cli_smoke_tests.rs` | 24 | 148.9 | CLI tool listing, error formatting, arbitration, settings |
| **Total** | **85** | | |

---

## Suite details

### `task_tools_tests` — Task lifecycle (148.6)

Tests in `tests/integration/task_tools_tests.rs`, wrapped by `tests/task_tools_tests.rs`.

Covers:
- `task_create → task_get → task_update` full happy path
- Format A (`{"tasks":[…]}`) and Format C (plain array) loading — both normalise transparently
- `.bak` recovery when the live `task_list.json` is corrupted
- Atomic save: `.tmp` file is cleaned up, no partial writes survive
- Input validation: empty/whitespace title, invalid priority, invalid status
- Descriptive errors for missing file and unknown task ID
- Subtask decimal IDs (`1.1`, `1.2`, `1.3`) created and individually retrievable
- Second save creates a `.bak` snapshot of the prior state

### `file_tools_tests` — File tools + security (148.4 + 148.5)

Tests in `tests/integration/file_tools_tests.rs`, wrapped by `tests/file_tools_tests.rs`.

Covers:
- `read_file`: existing file, missing file, valid JSON verbatim, outside-trust denial
- `write_file`: creates file, creates parent directories, outside-trust denial
- `replace`: substitutes text, errors on missing old_string and missing file
- `list_directory`: returns file names, errors on missing dir
- `glob_search`: `*.txt` match, no-match response
- `search_file_content`: pattern found, pattern absent
- **Security** (148.5): inside-trust access OK, absolute system path denied (`C:\Windows\...`), path traversal `../../etc/passwd` denied

### `subsystem_tests` — Memory, Bayes, config, registry (148.7 + 148.8)

Tests in `tests/subsystem_tests.rs`.

Covers:
- **Long-term memory**: `save_fact` persists across reload, duplicate deduplication, keyword search, `to_prompt_section` output, empty-fact guard
- **Bayesian engine**: construction from defaults and from config, threshold values, belief update after text input
- **Config**: `AcpConfig` sane defaults (token budgets, `ThinkingMode`, `auto_compress`), `ThinkingMode` serde round-trip and `from_str_ci`
- **Tool registry shape**: `get_tool_definitions()` coverage, full-schema `function.name` + `function.description` validation
- **Tool arbitration**: Execute / NeedMoreInfo / Reject outcomes for all three cases

### `cli_smoke_tests` — CLI API surface (148.9)

Tests in `tests/cli_smoke_tests.rs`.

Covers:
- `get_tool_definitions()` > 10 entries, core tools present
- `get_full_tool_definitions()` all entries have valid shape
- `format_tool_error_for_llm` — header, tool name, error text, Starlink hint, access-denied copy, "Do NOT repeat" instruction
- `AcpConfig::default()` — loop iterations, auto-compress, context token budgets, thinking mode, compression threshold
- Tool arbitration: `task_create`, `task_get`, `read_file` happy/missing-field paths; unknown tool rejection; `fork_agent` consistency with tool list
- `save_memory` empty/whitespace rejection

---

## Shared helpers

`tests/integration/helpers.rs` provides utilities reused across file-based suites:

```rust
make_security(dir: &TempDir) -> SecurityPolicy   // policy rooted at temp dir
make_ctx(dir: &TempDir) -> ToolContext            // context from policy
write_task_list_a(dir, tasks)                     // Format A: {"tasks":[…]}
write_task_list_c(dir, tasks)                     // Format C: plain JSON array
write_fixture(dir, relative_path, content)        // general file fixture helper
```

---

## Adding new tests

1. Pick the right suite file (or create a new one in `tests/`).
2. All tests must be `#[test]` (sync) or `#[tokio::test]` (async).
3. Every test must:
   - Use `tempfile::TempDir` for any file-system state.
   - Make **no network calls** (tag any that do with `#[ignore]`).
   - Be fully deterministic.
4. Follow the mandatory three subtasks per task: error checking, cargo test coverage, documentation.

---

## Coverage

Run with [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) (target: >80% line coverage):

```powershell
# Install once
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir target/coverage \
  --exclude-files "src/bin/*" "src/main.rs"

# Open report
start target/coverage/tarpaulin-report.html
```

Or with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):

```powershell
cargo install cargo-llvm-cov
cargo llvm-cov --html --output-dir target/llvm-cov
start target/llvm-cov/index.html
```

---

## CI

A `Makefile` target is provided for CI pipelines:

```makefile
make test-integration   # runs all four integration suites
make test-all           # cargo test (unit + integration)
make test-coverage      # tarpaulin HTML report
```

See `Makefile` in the project root.
