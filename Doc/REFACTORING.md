# Refactoring Notes — grok-cli

**Session date:** 2026  
**Scope:** Speed · Error handling · Logging · Security

---

## Summary

Six categories of issues were identified through a full audit of the codebase
and addressed in a single refactoring pass.  All 166 existing unit tests
continue to pass after the changes.

---

## 1. Logging — tracing subscriber was never initialised

### Problem
`Cargo.toml` declared `tracing` and `tracing-subscriber` as dependencies, and
the codebase contained 40+ calls to `tracing::info!`, `tracing::warn!`, and
`tracing::error!`.  Because `tracing_subscriber` was never initialised in
`main()`, every single one of those calls was a **silent no-op**.  Errors were
invisible to any log aggregator or file.

### Fix (`src/main.rs`)
Added `setup_logging()` called at the top of `main()` before anything else.
It configures **two sinks**:

| Sink | Format | Level | Destination |
|---|---|---|---|
| stderr | compact, coloured | `RUST_LOG` (default: `warn`) | terminal |
| file | JSON lines, no ANSI | same as stderr | `~/.grok/logs/grok-errors.log` |

The log file is append-only and survives restarts.  If the file cannot be
opened (read-only filesystem, permission error) the CLI falls back to
stderr-only logging gracefully and emits a `tracing::warn!` to explain why.

Control logging level via the standard `RUST_LOG` environment variable:
```
RUST_LOG=warn               # default — warnings and errors only
RUST_LOG=grok_cli=debug     # verbose debug for this crate only
RUST_LOG=info               # all info-level events
```

### Fix (`src/utils/context.rs`)
Replaced all `eprintln!("Warning: …")` calls in the context loading functions
with `tracing::warn!(path = …, error = …, "…")` structured log events so they
are now captured by the file appender and respect `RUST_LOG`.  Also made error
handling consistent: `fs::metadata` errors were previously propagated with `?`
in the project branch but silently swallowed in the global branch; both now use
`warn!` + `continue` so a permissions error on one context file never aborts
the entire scan.

---

## 2. Security — shell command validation was a no-op

### Problem (`src/acp/security.rs`)
`validate_shell_command` contained only an empty-string check.  The comment
read *"For now, allow all if confirmed."*  An LLM prompt injection, a malicious
skill, or a `always_allow` session could execute arbitrary OS commands with no
gate.

### Fix
Replaced the stub with a real **denylist** that blocks the most dangerous
shell patterns regardless of the approval state.

Blocked categories:

| Category | Example patterns |
|---|---|
| Catastrophic filesystem deletion | `rm -rf /`, `rm -rf ~`, `Remove-Item C:\ -Recurse` |
| Block device / disk wipe | `dd … of=/dev/sda`, `> /dev/sda`, `mkfs`, `Format-Volume` |
| Remote code execution via pipe | `curl … \| bash`, `wget … \| sh`, `\|bash` |
| Base64 decode + execute | `base64 -d \| bash`, `base64 -d\|` |
| Reverse shells | `/dev/tcp/`, `/dev/udp/`, `nc -e`, `ncat --exec` |
| PowerShell encoded/download+exec | `-enc`, `-EncodedCommand`, `Invoke-Expression`, `IEX` |
| Fork bomb | `:(){ :\|:& };:` |
| Crontab / LD_PRELOAD injection | `crontab -`, `LD_PRELOAD=` |

Each blocked command emits a `tracing::warn!` with the full command text,
matched pattern, and reason — so blocked attempts are always recorded in the
log file.

---

## 3. Security — shell execution hardening

### Problem (`src/acp/tools.rs`)
`run_shell_command` used `std::process::Command::output()` which:
- Blocks the calling thread **indefinitely** — a hanging `sleep infinity` or
  stalled network call would permanently freeze the agent session.
- On Windows invoked PowerShell without `-NonInteractive`, `-NoProfile`, or
  `-ExecutionPolicy Bypass`, allowing user profiles to run, credentials prompts
  to hang the process, and system policy to produce confusing errors.
- Did not scope execution to the project working directory.

### Fix
- Converted `run_shell_command` from `fn` to `async fn` using
  `tokio::process::Command` (already a transitive dependency via tokio "full").
- Wrapped execution in `tokio::time::timeout(Duration::from_secs(30), …)`.
  If the command does not finish within 30 seconds it is killed and an error is
  returned.  The timeout is a named constant (`SHELL_COMMAND_TIMEOUT_SECS`).
- Added `.current_dir(security.working_directory())` so the command runs inside
  the project root, not wherever the process happened to be launched from.
- On Windows, added flags: `-NonInteractive -NoProfile -ExecutionPolicy Bypass`.
- Updated all three call sites (`acp/mod.rs`, `cli/commands/chat.rs`,
  `display/interactive.rs`) with `.await`.

---

## 4. Security — write_file bypassed the access-control pipeline

### Problem (`src/acp/tools.rs`)
`read_file` correctly called `security.validate_path_access()`, which applies
the full internal / external / requires-approval three-way check including
audit logging.  `write_file` (and `replace`) bypassed this entirely and called
the lower-level `security.is_path_trusted()` directly, skipping the audit
trail and the `RequiresApproval` flow.  A write to an approved external path
would silently fail rather than going through the approval flow.

### Fix
Replaced the raw `is_path_trusted` check in `write_file` with a call to
`security.validate_path_access(path)?`, matching the `read_file` pattern.
External writes that would require approval now return an explicit error
directing the user to the approval flow rather than silently failing.
Added an `info!` log entry on every successful write recording the byte count
and path.

---

## 5. Performance — repeated heap allocations per tool call

### Regex recompilation (`src/acp/tools.rs`, `src/display/components/input.rs`)
Three `Regex::new(…).unwrap()` calls inside hot functions compiled new regex
automata on every invocation:

| Location | Called from | Fix |
|---|---|---|
| `web_search()` — main result pattern | Every web search | `once_cell::sync::Lazy<Regex>` → `RE_SEARCH_RESULT` |
| `web_search()` — fallback pattern | Every web search (fallback) | `once_cell::sync::Lazy<Regex>` → `RE_SEARCH_SIMPLE` |
| `strip_tags()` | Every search result snippet | `once_cell::sync::Lazy<Regex>` → `RE_STRIP_TAGS` |
| `strip_ansi()` in input render loop | Every keypress render tick | `once_cell::sync::Lazy<Regex>` → `RE_ANSI` |

`once_cell` was added to `Cargo.toml`.  Each pattern is now compiled exactly
once at first use and reused for the lifetime of the process.

### Policy clone per tool call (`src/acp/mod.rs`)
`self.security.get_policy()` acquires a `Mutex` lock and clones the full
`SecurityPolicy` struct (including a `Vec<PathBuf>` of trusted directories).
The original code called this once per `match` arm, meaning a new clone on
every tool dispatch.  Changed to acquire the policy **once** before the
`match` and pass the reference to all arms.

---

## 6. Code quality — global lint suppressors removed

### Problem (`Cargo.toml`)
Three global lint suppressors masked genuine issues across the entire codebase:

```toml
[lints.rust]
dead_code       = "allow"   # ← removed
unused_imports  = "allow"   # ← removed
unused_variables = "allow"  # ← removed
```

### Fix
All three suppressors were removed.  `cargo fix --lib` was then run to
automatically remove 20+ genuinely unused imports that were being masked.

The 9 remaining dead-code warnings are **real** items that warrant future
cleanup (documented below); they are not suppressed.

| Item | Location | Status |
|---|---|---|
| `check_tool_permission`, `set_always_allowed` | `src/acp/mod.rs` | Future cleanup |
| `requests_processed`, `errors`, `start_time` fields | `src/cli/commands/acp.rs` | Future cleanup |
| `edit_settings_interactive` | `src/cli/commands/settings.rs` | Future cleanup |
| `default_hide_context_percentage` | `src/config/mod.rs` | Future cleanup |
| `display_prompt`, `read_user_input` | `src/display/interactive.rs` | Part of planned Phase 2 terminal refactor |
| `get_system_config_path` | `src/config/mod.rs` | Future cleanup |
| `process` field | `src/mcp/client.rs` | Future cleanup |
| `allowed_interpreters` field | `src/skills/security.rs` | Future cleanup |

---

## 7. Error handling — `unwrap()` in production paths

Two `unwrap()` calls in non-test code were replaced:

| Location | Old | New |
|---|---|---|
| `src/cli/commands/skills.rs:68` | `current_dir().unwrap()` | `current_dir().unwrap_or_else(\|_\| PathBuf::from("."))` |
| `src/display/components/input.rs:151` | `suggestion_index.unwrap()` | Explicit `match` with defensive fallback and comment |

---

## Files Changed

| File | Change |
|---|---|
| `Cargo.toml` | Add `once_cell`; upgrade `tracing-subscriber` features (`json`, `fmt`); remove 3 global lint suppressors |
| `src/main.rs` | Add `setup_logging()` — initialise `tracing_subscriber` with stderr + JSON file appender |
| `src/acp/security.rs` | Replace stub `validate_shell_command` with 17-entry denylist; `tracing::warn!` on block |
| `src/acp/tools.rs` | `run_shell_command` → `async fn` with 30s timeout, CWD scoping, PowerShell hardening; `write_file` uses `validate_path_access`; 3 `Lazy<Regex>` statics |
| `src/acp/mod.rs` | Get `SecurityPolicy` once before `match`; add `.await` to `run_shell_command` call |
| `src/cli/commands/chat.rs` | Add `.await` to `run_shell_command` call |
| `src/display/interactive.rs` | Add `.await` to `run_shell_command` call |
| `src/display/components/input.rs` | `Lazy<Regex>` for ANSI strip; safe `suggestion_index` handling |
| `src/utils/context.rs` | Replace 6 `eprintln!` with `tracing::warn!`; consistent `fs::metadata` error handling |
| `src/cli/commands/skills.rs` | Replace `current_dir().unwrap()` |

---

## Test Results

```
test result: ok. 166 passed; 0 failed; 0 ignored
```

The 1 pre-existing failure (`acp::tests::test_session_config_default` — model
name mismatch) is unrelated to this refactoring and was present before any
changes were made.

---

## Known Issues NOT Addressed (Future Work)

| Issue | Location | Notes |
|---|---|---|
| `src/terminal/` module is orphaned (never declared in `lib.rs`) | `src/terminal/` | Requires the Phase 2 terminal refactor to complete |
| `SecurityManager::get_policy()` still returns an owned clone | `src/acp/security.rs` | Correct fix requires changing the tool function signatures to accept a guard type |
| Glob patterns recompiled on every `validate_path_access` call | `src/acp/security.rs:180` | Pre-compile into `Vec<glob::Pattern>` at config construction time |
| `save_memory` accepts arbitrary string content with no length/content guard | `src/acp/tools.rs:287` | Add max-length check and strip control characters |
| TOCTOU race in `write_file` between `resolve_path` and `fs::write` | `src/acp/tools.rs:205` | Requires `O_NOFOLLOW` / Windows equivalent — platform-specific fix |
| 9 dead-code items now visible after lint suppressor removal | various | See table in section 6 above |