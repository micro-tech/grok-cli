# Grok-CLI Code Review

**Reviewer:** Grumpy Old Rust Expert  
**Date:** 2026-07-30  
**Version reviewed:** 0.2.5-PreRelease  
**Scope:** Full codebase — security, correctness, performance, architecture, readability

---

> *"I've seen things you wouldn't believe. Code so bad it made me cry in the parking lot.  
> This project is not that — but it is not finished either. Here's what I found."*

---

## Legend

| Symbol | Severity |
|--------|----------|
| 🔴 | Critical — fix before any release |
| 🟠 | High — fix soon |
| 🟡 | Medium — should fix |
| 🔵 | Low / Style — nice to have |
| ✅ | Good — noting it as a positive |

---

## 1. SECURITY

### 🔴 SEC-1 — `write_file` creates directories before path validation

**File:** `src/tools/file_tools.rs` L357–383

```rust
// Creates the directory FIRST ...
if let Some(parent) = absolute_path.parent() {
    fs::create_dir_all(parent).await ...
}
// ... then validates access AFTER
let access_type = security.validate_path_access(path)?;
```

The directory is created in an unauthorized location BEFORE the security check runs.
An attacker-supplied path like `../../outside/project/malicious/` would get the directory
created even if the subsequent security check denies the write. **Flip the order: validate
first, create directory only if the path passes.**

---

### 🔴 SEC-2 — `RequireConfirmation` silently proceeds in `write_file`

**File:** `src/tools/file_tools.rs` L328–333

```rust
SafetyDecision::RequireConfirmation(msg) => {
    // "For now we treat it as a warning but still proceed."
    tracing::warn!("Safety confirmation required: {}", msg);
}
```

The `on_before_write_file` hook explicitly returns `RequireConfirmation` for DELETE operations.
The caller then ignores this and writes anyway. A confirmation guard that never blocks is not
a safety guard. Either wire up real confirmation logic or document it clearly as unimplemented
and gate it behind a feature flag. **As-is, the safety subsystem is theater.**

---

### 🔴 SEC-3 — `TrustAlways` decision is silently dropped in `read_file`

**File:** `src/tools/file_tools.rs` L86–107

```rust
Ok(ApprovalDecision::TrustAlways) => {
    // NOTE: session-trust mutation requires a mutable policy reference...
    // callers that need session-trust should call ...
    path.clone()
}
```

The user clicked "Trust Always." The code acknowledges it in a comment but does nothing.
The next `read_file` call on the same path will prompt again. Users are being lied to by
their own tool. **Fix this or remove the `TrustAlways` option entirely.**

---

### 🟠 SEC-4 — `process::exit(1)` in a library function

**File:** `src/utils/auth.rs` L39

```rust
pub fn require_api_key(...) -> String {
    ...
    process::exit(1);
}
```

Library code must **never** call `process::exit`. It bypasses all Drop implementations,
prevents async runtimes from flushing, skips any registered panic/signal handlers, and
makes the function un-testable. Return `Result<String, anyhow::Error>` and let `main`
decide how to exit. The current design also means the function signature lies — it claims
to return `String` but may never return at all.

---

### 🟠 SEC-5 — `session_dna.json` at the project root

**File:** `src/session/dna.rs` L39

```rust
let local = std::path::Path::new("session_dna.json");
```

Behavioral configuration (tone, risk tolerance, tool preferences) lives in the project root
by default. This file can be accidentally committed to version control and exposes session
behavior to anyone who forks the repo. Use `.grok/session_dna.json` as the project-level
path, or at minimum add `session_dna.json` to `.gitignore`.

---

### 🟡 SEC-6 — Audit logger tests hit the real audit log directory

**File:** `src/security/audit.rs` L364–496

Every `#[test]` function creates `AuditLogger::new(true)` which writes to
`%LOCALAPPDATA%\.grok\audit\external_access.jsonl` — the **real** log file on the
developer's machine. Tests should use `tempfile::TempDir` and inject the path. The
`test_get_statistics` test even calls `clear_logs()` to reset state, which would wipe
a developer's real audit history. Use dependency injection or at minimum `TempDir`.

---

### 🟡 SEC-7 — `AuditLogger::new` creates the audit directory even when disabled

**File:** `src/security/audit.rs` L64–79

When `enabled = false` the logger still creates `~/.grok/audit/` on disk during
construction. Move directory creation inside `log_access` (or make it lazy) so a disabled
logger has zero filesystem side-effects.

---

### 🟡 SEC-8 — New UUID per-call is not a session ID

**File:** `src/tools/file_tools.rs` L42, L65

```rust
let session_id = Uuid::new_v4().to_string();
```

This generates a random ID on every file access, not a real session identifier. The audit
log therefore contains IDs that cannot be correlated across calls in the same session.
Pass in the real session ID from `SessionData` instead.

---

## 2. CORRECTNESS

### 🔴 COR-1 — `detect_starlink_connection` logic is backwards

**File:** `src/utils/network.rs` L147–165

```rust
if let Ok(addrs) = tokio::net::lookup_host("starlink.com:80").await
    && addrs.count() > 0
{
    return true;  // "possible Starlink connection"
}
```

`starlink.com` resolves from ANY internet-connected machine (it's just a public website).
This function will return `true` for literally every non-Starlink connection with working
DNS. This is a useless heuristic that pollutes log output. Remove it or replace with
something meaningful (e.g., checking gateway IP ranges, detecting high latency variance).

---

### 🔴 COR-2 — Cargo.lock is in `.gitignore` for a binary crate

**File:** `.gitignore` (per AGENTS.md instructions)

For an executable crate, `Cargo.lock` **must** be committed to version control. Without
it, `cargo build` on a CI machine or another developer's workstation may pull a different
version of a transitive dependency, causing non-reproducible builds and potential supply
chain surprises. The Cargo team's own guidance is explicit: binaries commit Cargo.lock,
libraries don't.

---

### 🟠 COR-3 — `&&` → `;` PowerShell replacement is dangerously naïve

**File:** `src/tools/shell_tools.rs` L59

```rust
let ps_command = command.replace(" && ", "; ");
```

Bash `&&` means "run next command only if the previous succeeded". PowerShell `;` is
unconditional — it always runs the next statement. Translating `build && test` into
`build; test` means `test` runs even if `build` fails, potentially destroying state or
reporting false success. Use PowerShell's native `&&` operator (available in PowerShell 7+)
or rewrite the command as:

```powershell
if ($?) { test }
```

The current approach will silently give wrong results in error cases.

---

### 🟠 COR-4 — `strip_jsonc_trailing_commas` compiles a regex on every call

**File:** `src/tools/file_tools.rs` L233–238

```rust
fn strip_jsonc_trailing_commas(s: &str) -> String {
    let re = Regex::new(r",(\s*[}\]])").expect("static regex is valid");
    ...
}
```

This function is called on every JSON file read. Regex compilation is expensive.
Use `once_cell::sync::Lazy` (the pattern already used in `web_tools.rs`) or make `re` a
`static`. Same problem in `list_code_definitions` (L274).

---

### 🟠 COR-5 — `RateLimitConfig` is a no-op

**File:** `src/grok_client_ext.rs` L51–55

```rust
/// Set rate limit configuration (for compatibility - currently a no-op)
pub fn with_rate_limits(mut self, config: RateLimitConfig) -> Self {
    self.rate_limit_config = Some(config);
    self
}
```

The `RateLimitConfig` struct is defined, configurable in TOML, shown in `Config::show`,
but never actually enforced. If the API rate-limits requests, the client will receive
429 errors and retry — but there is no token-bucket or request-counter guard at the
client layer. This is misleading to users who configure `rate_limits` in their config.
Either implement it or remove the config knob entirely.

---

### 🟡 COR-6 — Unused parameter `_timeout_secs` in `run_shell_command`

**File:** `src/tools/shell_tools.rs` L49

```rust
pub async fn run_shell_command(
    command: &str,
    security: &SecurityPolicy,
    _timeout_secs: u64,   // ← suppressed warning, never used
) -> Result<String> {
```

The function reads `effective_timeout(security)` internally. The public `_timeout_secs`
parameter is dead weight — callers pass it in but it has no effect. This creates a
misleading API. Remove the parameter and have callers rely on the policy-driven timeout,
or use the parameter.

---

### 🟡 COR-7 — `web_search_returns_result_or_no_results` test always passes

**File:** `src/tools/web_tools.rs` L271–275

```rust
let result = web_search("rust programming language").await;
assert!(result.is_ok() || result.is_err()); // This ALWAYS passes
```

This assertion is logically equivalent to `assert!(true)`. It provides zero value and
gives false confidence. Either assert on the content or mark the test as `#[ignore]` for
offline CI. Delete it if you have nothing meaningful to assert.

---

### 🟡 COR-8 — `STARLINK_ERROR_PATTERNS` includes non-Starlink patterns

**File:** `src/utils/network.rs` L12–30

```
"connection refused",      // This is a SERVER-side rejection, not a network drop
"network error",           // Too generic — matches many non-transient errors
"error sending request",   // Matches auth failures, malformed requests, etc.
"service unavailable",     // Could be legitimate 503 from the API itself
```

The pattern list conflates transient satellite-link drops with server-side errors. A 503
from the Grok API (e.g., model overloaded) is correctly handled by retrying, but
"connection refused" indicates the server is rejecting the connection, not that the
satellite handover dropped. Retrying on "connection refused" without back-off will hammer
a down service. Tighten the list.

---

## 3. PERFORMANCE

### 🟠 PERF-1 — New `reqwest::Client` on every HTTP call

**File:** `src/tools/web_tools.rs` L87, L165

```rust
pub async fn web_fetch(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()...build()?;
    // used once, then dropped
}

async fn duckduckgo_search(query: &str) -> Result<String> {
    let client = reqwest::Client::builder()...build()?;
    // same story
}
```

`reqwest::Client` manages a connection pool. Creating a new one on every call means:
- No connection reuse (TCP handshake + TLS negotiation on every request)
- Repeated DNS resolution
- Memory allocation and cleanup overhead

Create ONE `Arc<reqwest::Client>` at startup and pass/share it. This is a classic
beginner mistake with `reqwest`.

---

### 🟠 PERF-2 — `AuditLogger::log_access` opens a new file handle per entry

**File:** `src/security/audit.rs` L130–140

```rust
let mut file = OpenOptions::new()
    .create(true).append(true)
    .open(&self.log_file_path)...;
writeln!(file, "{}", json)?;
file.flush()?;
```

Opening, writing, flushing, and closing a file on every log entry is expensive. Consider
keeping an `Arc<Mutex<BufWriter<File>>>` open, or at minimum batch-write. For an audit log
that may be called on every file access, this is a hot path.

---

### 🟠 PERF-3 — `get_all_logs` reads the entire log file on every stats call

**File:** `src/security/audit.rs` L193–212, L264–282

`get_statistics`, `get_top_accessed_paths`, `get_logs_for_path`, and `get_logs_in_range`
all call `get_all_logs()`, which reads and deserializes every log entry from disk. A busy
session will produce thousands of entries. Without caching or a rolling-window structure,
the `audit summary` command will slow to a crawl over time.

---

### 🟡 PERF-4 — `AgentManager` holds `RwLock<HashMap>` without capacity

**File:** `src/agent/manager.rs` L40–41

```rust
agents: RwLock::new(HashMap::new()),
```

Minor, but `HashMap::new()` starts with zero capacity and doubles on each resize. Use
`HashMap::with_capacity(16)` or similar if agents are expected to be spawned frequently.

---

### 🟡 PERF-5 — `list_code_definitions` regex compiled on every call

**File:** `src/tools/file_tools.rs` L274–277

Same issue as COR-4 above. Promote to `Lazy<Regex>` static.

---

## 4. ARCHITECTURE

### 🔴 ARCH-1 — Monolithic files that need splitting

The following files are too large for any single human (or AI) to reason about safely:

| File | Lines | Problem |
|------|-------|---------|
| `src/config/mod.rs` | ~3,100 | 25+ structs, all config concerns in one file |
| `src/acp/mod.rs` | ~3,200 | `GrokAcpAgent::handle_chat_completion` alone is ~1,000 lines |
| `src/tools/registry.rs` | ~1,450 | `execute_tool` (552 lines), `get_full_tool_definitions` (755 lines) |

`handle_chat_completion` contains retry logic, tool loop, permission checks, compression,
history management, and model routing — all in one function. Extract each concern into its
own function or module.

Split `config/mod.rs` into: `config/acp.rs`, `config/network.rs`, `config/ui.rs`,
`config/security.rs`, etc.

---

### 🔴 ARCH-2 — `is_known_tool` / `execute_tool` / `get_tool_definitions` must be manually synced

**File:** `src/tools/tool_arbitration.rs` L57–104, `src/tools/registry.rs`

There are THREE places where the full tool list must be maintained:
1. `is_known_tool` — for rejection
2. `execute_tool` — for dispatch
3. `get_tool_definitions` / `get_full_tool_definitions` — for schema export

Adding a tool requires updating all three, in three different files, with no compile-time
enforcement. This WILL cause bugs. Consolidate into a `Tool` trait or a single registry
macro/array that drives all three. At minimum, add a test that asserts
`is_known_tool(name) == true` for every name returned by `get_tool_definitions()`.

---

### 🟠 ARCH-3 — Library/binary separation violations documented but not fixed

**File:** `src/lib.rs` L18–34

The `lib.rs` doc-comment honestly lists the violations: direct `println!`, `eprintln!`,
`indicatif`, `ratatui`, `process::exit`. These have been acknowledged with TODO comments
for what appears to be a long time. 

The `require_api_key` function (SEC-4 above) calling `process::exit` is a direct result of
this. Mark a date by which these will be addressed, or accept that the crate is a binary
with a thin library veneer and remove the pub API.

---

### 🟠 ARCH-4 — Stray backup file `mod .rs_bak` with a space in the filename

**File:** `src/acp/mod .rs_bak`

A backup file with a space in the name (`mod .rs_bak`) is committed to the repository.
This is not source code — it is editor garbage. Delete it. It causes issues with tools
that glob for `*.rs` files and is confusing to anyone reading the directory listing.

---

### 🟡 ARCH-5 — Tool role → user role conversion loses structured context

**File:** `src/grok_client_ext.rs` L123–131

```rust
"tool" => {
    // Fallback: report tool result as user message...
    Some(ChatMessage::user(format!(
        "Tool result (ID: {}): {}",
        tool_call_id, content.unwrap_or("")
    )))
}
```

Tool results are being downcast to user messages because the `grok_api` crate does not
support the `tool` role. This flattens structured tool output into free text. If the
API ever supports the tool role properly, update this immediately. In the meantime, add a
`tracing::warn!` here so the behavior is visible in logs.

---

### 🟡 ARCH-6 — `session_dna.json` in project root may be committed to VCS

**File:** `src/session/dna.rs` L39

Covered in SEC-5, but worth repeating as an arch concern: behavioral session data in the
project root couples the tool's runtime state to the source tree. Use `.grok/` exclusively.

---

## 5. READABILITY & STYLE

### 🟠 READ-1 — Typo in version string

**File:** `Cargo.toml` L3

```toml
version = "0.2.5-PreRelease"
```

"PreRelese" should be "PreRelease". This propagates into `--version` output, `cargo publish`
metadata, and any release artifacts.

---

### 🟡 READ-2 — `ExternalAccessResult::Denied` accepts a `String` but `ExternalAccessResult` is not an enum variant

The security module has `ExternalAccessResult::Denied(String)` that carries a human-readable
reason, but some branches that call it pass empty strings or lose the reason on the way
to the caller. Verify the denial reason is always propagated to the audit log.

---

### 🟡 READ-3 — Magic numbers throughout

Examples:
- `content.len() > 200_000` in `pre_write_hook.rs` — extract to a named constant
- `non_printable > content.len() / 10` — what does 10% binary mean? Name it.
- `10_000` char truncation in `web_fetch` — name it `WEB_FETCH_MAX_CHARS`
- `8` max tool preferences in `session_dna.rs` — name it `MAX_TOOL_PREFERENCES`

---

### 🟡 READ-4 — `detect_starlink_connection` is dead weight in the public API

**File:** `src/utils/network.rs` L147`

This function is `pub async` but never called from outside the module (only the test
harness would call it). Either use it properly or remove it. Having broken heuristics
in a pub API is worse than not having them.

---

### 🔵 READ-5 — `web_search_is_always_configured` is a vacuous test

**File:** `src/tools/web_tools.rs` L244–247

```rust
fn web_search_is_always_configured() {
    assert!(is_web_search_configured());
}
```

`is_web_search_configured()` is a function that always returns `true`. The test for it
always passes. Delete both the function and the test, or make `is_web_search_configured`
actually check something.

---

### 🔵 READ-6 — Import style inconsistency

Some modules use `use tracing::{warn};` inside function bodies (e.g., `file_tools.rs`
L110, L129) when the same module already imports `use tracing::info` at the top. Put all
tracing imports at the module level for consistency.

---

### 🔵 READ-7 — `format_grok_logo` / `get_logo_for_width` vs `print_grok_logo`

The `#[allow(deprecated)]` on the `print_grok_logo` re-export in `lib.rs` signals ongoing
technical debt. The deprecated functions should have been removed in the same PR that
introduced their replacements, not just `#[deprecated]`-tagged. Set a removal milestone.

---

## 6. TESTING

### 🟠 TEST-1 — Audit log tests write to production paths

See SEC-6 above. The tests are contaminating real user data. This is unacceptable.

---

### 🟡 TEST-2 — No integration test for tool dispatch round-trip

`execute_tool` → `tool_arbitration` → actual tool function is a critical path with no
end-to-end test. Add at least one test that calls `execute_tool("write_file", ...)` and
verifies both the arbitration and the file write happened correctly.

---

### 🟡 TEST-3 — `test_grok_client_creation` requires network

**File:** `src/grok_client_ext.rs` L233–239

```rust
#[tokio::test]
async fn test_grok_client_creation() {
    let client = GrokClient::with_settings("test-key", 30, 3);
    assert!(client.is_ok());
    let empty_key_client = GrokClient::with_settings("", 30, 3);
    assert!(empty_key_client.is_err());
}
```

Whether this test passes depends on whether `grok_api::GrokClient::builder().build()`
makes a network call during construction. If it does (e.g., to validate the key), this
is a flaky test in offline CI. Tag with `#[ignore]` if it requires network access, or use
a mock via the `GROK_API_BASE_URL` env var and `mockito`.

---

### 🔵 TEST-4 — `serial_test` for audit tests masks test isolation problems

Using `#[serial]` forces audit tests to run sequentially because they share global state
(the real audit file). The fix is proper test isolation (TempDir), not serialization.

---

## 7. POSITIVES (Yes, I have some)

✅ **Starlink-aware retry logic** — `web_tools.rs` and `utils/network.rs` have thoughtful
retry-with-backoff and jitter. The intent is solid; tighten the error patterns (COR-8).

✅ **`SecurityPolicy` path resolution** — Handling Windows drive-absolute paths on Linux CI
(`is_windows_drive_absolute`) and resolving symlinks before trust checks shows real
security engineering thinking.

✅ **Structured logging** — Consistent use of `tracing` with field names (`error = %e`,
`path = %path.display()`) makes log parsing pleasant. The dual sink (stderr + JSON file)
in `main.rs` is well done.

✅ **`AgentManager` is clean** — Good use of `Arc<RwLock<HashMap>>`, correct async patterns,
no `unwrap()` on lock poison, good tests with proper assertion messages.

✅ **`SafetyDecision` enum** — The pre-write hook design with `Allow/AllowWithWarning/
RequireConfirmation/Block` is the right shape. It just needs to be enforced (SEC-2).

✅ **`GrokClient::with_settings` environment variable override** — Using
`GROK_API_BASE_URL` for test mocking without changing production code is clean.

✅ **Binary detection heuristic in `pre_write_hook`** — Checking for >10% non-printable
bytes before writing is a sensible guard that prevents the model from accidentally writing
binary garbage to text files.

---

## 8. PRIORITY ACTION LIST

Fix these first, in this order:

1. 🔴 **SEC-2** — Wire up real `RequireConfirmation` logic or admit the safety hook is unimplemented
2. 🔴 **SEC-1** — Fix directory creation before path validation in `write_file`
3. 🔴 **SEC-3** — Fix or remove `TrustAlways` in `read_file`
4. 🔴 **ARCH-2** — Add a compile-time-enforced registry for tool names
5. 🔴 **COR-2** — Commit `Cargo.lock` to version control
6. 🟠 **SEC-4** — Change `require_api_key` to return `Result<String>`
7. 🟠 **PERF-1** — Shared `Arc<reqwest::Client>` for web tools
8. 🟠 **COR-3** — Fix `&&` → `;` PowerShell translation
9. 🟠 **COR-1** — Remove or fix `detect_starlink_connection`
10. 🟠 **ARCH-1** — Begin splitting `config/mod.rs`, `acp/mod.rs`, `registry.rs`
11. 🟠 **TEST-1** — Fix audit tests to use `TempDir`
12. 🟡 **COR-4/PERF-5** — Lazy static regex in `file_tools.rs`
13. 🟡 **READ-1** — Fix version typo in `Cargo.toml`
14. 🟠 **ARCH-4** — Delete `src/acp/mod .rs_bak`

---

*Review complete. The bones of this project are good. The security model has the right
concepts. The async patterns are generally correct. But there are real bugs (SEC-1, SEC-2,
SEC-3) and the codebase is beginning to show the weight of rapid feature addition without
consistent cleanup. Address the critical items before a public release.*

---

## 9. ADDITIONAL FINDINGS FROM FULL DEEP DIVE (Post-Original Review)

These items were identified during a comprehensive walk of the entire source tree, Cargo.toml, main entry points, large modules, and cross-cutting concerns (after the original review above was written).

### 🔴 BUILD-1 / COR-9 — `edition = "2024"` in Cargo.toml

**File:** `Cargo.toml` line 3

```toml
edition = "2024"
```

Rust 2024 edition is **not yet stable** (as of mid-2026 the stable edition is still 2021). This will cause `cargo build` / `cargo test` to fail on any machine using a stable Rust toolchain. This is a release blocker for anyone who clones the repo.

**Impact:** Breaks reproducible builds, CI, and contributor onboarding.

**Recommendation:** Change to `edition = "2021"` immediately. If any 2024-specific syntax is being used, backport it.

---

### 🟠 COR-10 — `run_shell_command` returns `Ok` even on non-zero exit

**Files:** `src/tools/shell_tools.rs`, `src/tools/registry.rs`

The shell tool does this:

```rust
if output.status.success() {
    Ok(format!("Stdout: ..."))
} else {
    Ok(format!("Command failed with code {}: ...", ...))   // Still Ok!
}
```

This is inconsistent with the rest of the tool system (most errors return `Err`). Callers (including the model and arbitration layer) cannot easily distinguish "tool ran and produced output" from "tool failed".

**Recommendation:** Return `Err` for non-zero exits (or at minimum wrap in a structured error type) so the upper layers can react correctly.

---

### 🟠 SEC-9 — `replace()` tool has weaker security than `write_file`

**File:** `src/tools/file_tools.rs` (the `replace` function)

- `write_file` goes through the full external-access approval + audit logging flow.
- `replace` does its own `is_path_trusted` check and **bypasses** the `ExternalRequiresApproval` path and the audit logger entirely.

An attacker (or confused model) can use `replace` to modify files that would have triggered an approval prompt.

**Recommendation:** Make `replace` go through the same `read_file` / `write_file` style approval + audit path, or explicitly document that `replace` is a "trusted-paths only" operation.

---

### 🟠 ARCH-7 — `handle_chat_completion` in `GrokAcpAgent` is ~1000 lines

**File:** `src/acp/mod.rs`

This single method contains:
- Multiple layers of history trimming + token budgeting
- Auto-compression + archiving logic
- Permission bridge interaction
- DNA + Bayesian injection
- Status bar / context usage emission
- Tool loop + retry logic
- Thinking trace handling

This is the heart of every ACP session and is extremely hard to reason about or test.

**Recommendation:** Extract at minimum:
- `trim_context_for_model()`
- `maybe_compress_history()`
- `emit_status_updates()`
- The core tool-execution sub-loop

---

### 🟡 PERF-6 — Eager directory creation in logging setup

**File:** `src/main.rs` (setup_logging)

```rust
if let Some(parent) = log_file_path.parent() {
    let _ = std::fs::create_dir_all(parent);
}
```

This runs unconditionally at startup, even if file logging later fails or is disabled. Minor, but contributes to the general pattern of "create things on disk before we know we need them".

---

### 🟡 READ-8 — Duplicated default values and magic thresholds

Seen across `config/mod.rs`, `acp/mod.rs`, `file_tools.rs`, `session/dna.rs`, etc.

Examples:
- Multiple places define `16_384` or `300` as token/timeout defaults.
- `8` as max tool preferences in DNA.
- `0.75` compression threshold, `0.40` chunk ratio, etc. with no named constants.

**Recommendation:** Centralize magic numbers into `const` items (ideally in a `constants.rs` or per-module).

---

### 🟡 ARCH-8 — Tool definitions are duplicated in three large blocks

Already covered as ARCH-2, but worth reinforcing after seeing the full registry:

- `get_tool_definitions()` (Vec<&str>)
- `get_full_tool_definitions()` (massive `vec![ json!{...}, ... ]`)
- The giant `match name` inside `execute_tool`

Any new tool (e.g. future vision or OKF tools) requires touching all three with no compiler help. The dynamic registration path (`register_dynamic_tool`) exists but is barely used.

---

### ✅ Positive notes from the deeper walk

- Lazy initialization of `router`, `security`, and `hook_manager` in `GrokAcpAgent` is excellent for fast `grok acp stdio` startup.
- The final-answer guard system message (Task 231) is a pragmatic and effective mitigation for tool-loop explosions.
- WorkflowTrace + TUI viewer (Tasks 232–233) is a genuinely nice observability addition.
- Hierarchical config loading + project-local `.grok/` is well executed.
- Use of raw `serde_json::Value` for message history (instead of round-tripping through typed structs) preserves `tool_call_id` fidelity.

---

## 10. UPDATED PRIORITY ACTION LIST (Augmented)

Add these to the existing list:

15. 🔴 **BUILD-1** — Change `edition = "2024"` → `"2021"` in Cargo.toml
16. 🟠 **COR-10** — Make shell tool return proper `Err` on non-zero exit
17. 🟠 **SEC-9** — Align `replace()` security/audit path with `write_file`
18. 🟠 **ARCH-7** — Refactor `handle_chat_completion` (extract sub-functions)
19. 🟡 **PERF-6** — Make log directory creation lazy
20. 🟡 **ARCH-8** — Strengthen the tool registry (macro or static table) — see task 244

*These new items were discovered during a full source-tree + dependency + data-flow review.*
