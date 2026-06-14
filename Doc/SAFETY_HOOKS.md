# Safety Hooks Documentation

**Mandatory 7-layer safety system for file operations** (Tasks 154–160)

## Overview

Grok-CLI now includes a comprehensive, mandatory safety pipeline that runs before every `write_file` and `replace` operation. The goal is to dramatically reduce the risk of the agent destroying or corrupting user files through hallucination, over-confidence, or ambiguous instructions.

## The 7 Safety Hooks

| # | Hook | File | Purpose |
|---|------|------|---------|
| 1 | Pre-Write Hook | `pre_write_hook.rs` | `on_before_write_file()` — first gate |
| 2 | Dry-Run Mode | `dry_run.rs` | Simulate writes without touching disk |
| 3 | Diff Validator | `diff_validator.rs` | Reject massive full-file rewrites |
| 4 | Intent Validator | `intent_validator.rs` | Force clarification on ambiguous edits |
| 5 | Suspicious Write Guard | `suspicious_write_guard.rs` | Final in-tool sanity checks |
| 6 | DNA-Aware Safety | `dna_safety.rs` | Raise thresholds based on SessionDNA |
| 7 | Tool Health Monitor | `tool_health_monitor.rs` | Track per-tool reliability |

## Data Flow

See the detailed flow in `dataflow_map.md` under “Safety Hooks Data Flow”.

## Usage

### `write_file` and `replace` signatures (updated)

```rust
pub async fn write_file(
    path: &str,
    content: &str,
    security: &SecurityPolicy,
    dry_run: bool,           // NEW
) -> Result<String>

pub async fn replace(
    path: &str,
    old_string: &str,
    new_string: &str,
    expected_replacements: Option<u32>,
    security: &SecurityPolicy,
    dry_run: bool,           // NEW
) -> Result<String>
```

When `dry_run = true`, the tool returns a message describing what *would* happen without writing anything.

## Testing

Run the dedicated safety tests:

```bash
cargo test --test safety
# or
cargo test safety::
```

## Future Work

- Wire `DiffValidator` and `IntentValidator` into the actual tool dispatch path
- Expose `ToolHealthMonitor` metrics via a `/health tools` command
- Add user-configurable safety strictness levels

---

**Status**: All 7 hooks implemented and the two primary file-writing tools (`write_file`, `replace`) are wired. Remaining hooks are ready for deeper integration.
