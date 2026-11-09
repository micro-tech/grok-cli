# Code Review — grok-cli (Cobble jr)

- **Reviewer:** Cobble jr
- **Date:** 2026-11-09
- **Base:** `origin/master` fa0f047 (commits 16db425 + fa0f047, 2026-11-09)
- **Scope:** whole grok-cli crate — `src/` (369 files, ~99,861 LOC), `tests/`, `Cargo.toml`, build/clippy
- **Weighting:** speed/performance first, security a slim second (per John's stated weighting for his local projects)

## Severity legend

- **Critical** — breaks the build, corrupts data, or opens a live hole. Fix before anything else.
- **High** — a real bug or a live footgun; will bite in normal use or on the next wiring change.
- **Medium** — a genuine defect or cost (perf, correctness, portability) that should be fixed but is not urgent.
- **Low** — cosmetic, hygiene, or minor.

## Executive summary

**Verdict: 🔧 Fix these and I'll sign off.**

John's 2026-11-09 fixes are real and they hold. The multi-file patch bug is actually fixed (per-file content resolution, multi-file patches now skip instead of fanning out), the `ExecuteWithApproval` gating is actually enforced (only `FullAuto` performs real writes), and `cargo clippy --lib` is genuinely **0 warnings** — I verified it, the "69 → 0" claim holds for the lib target.

But the Nov 6 must-fixes are still open, all five of them. The two that explain John's own live errors are still live:

- **M2 (per-model context windows)** — `get_context_window` still returns 1,048,576 for *every* `grok-4*` model, but base grok-4 is 256,000 and grok-4.5/4.6/4.7 are 500,000. The code over-promises the window by 2–4×, then overflows it. And `LEGACY_CONTEXT_BUDGET` (220,000) is *larger* than `LEGACY_CONTEXT_WINDOW` (131,072), so trimming to the budget cannot prevent overflow on legacy models.
- **M3 (TPM limiter double-count)** — the limiter estimates the full history (chars/3 + 800) and then *adds* recorded prior usage that already includes that same history. It over-counts by roughly a third and fails hard with "Tokens per minute" before `route()` runs, so no retry ever happens. This is the most likely cause of John's "too many tokens per minute" error.
- **M1 (history trimming)** — the CLI `conversation_history` still grows without bound. Only the ACP path trims.
- **M4 (classify_error)** — still no 400 / `context_length_exceeded` handling and no `Retry-After` honoring.
- **M5 (grok_api 429 body)** — still pinned to grok_api 0.2.4, which discards the 429 body.

New findings from this review: the `content_for` diff-summary fallback is a live footgun (currently unreachable only because its producers are dead code), the MCP config files commit a machine-specific temp path, the perf bench (`benches/perf_bench.rs`) does not compile, and `usage_stats.json` is still read + written on every API call.

**Top items to fix, in order:** H2 (per-model windows), H3 (TPM double-count), H6 (CLI history trimming), H4 (classify_error 400), H1 (diff-summary fallback), H5 (grok_api 429 body).

## Verification performed

| Check | Result |
|---|---|
| `cargo clippy --lib` | **0 warnings**, exit 0. John's "69 → 0" claim holds for the lib. |
| `cargo clippy --lib --all-targets` | **FAILS to compile** — `benches/perf_bench.rs` has 4 errors (see M3). The lib itself is clean; the bench target is broken. |
| `cargo check` | **Exit 0** (compiles). But the `installer` bin has **4 warnings** (see L3) — unused imports + dead functions in `src/bin/installer.rs`. |
| Targeted HOH patch-safety tests (`cargo test --lib hoh::patch_applier`) | **3 passed, 0 failed** — `content_for_resolves_only_single_file_patches`, `multi_file_patch_does_not_fan_out_shared_blob`, `single_file_patch_writes_intended_content` (0.00s; 1,124 other lib tests filtered out). The multi-file fix is proven by test. |
| Full test suite (1,331 tests) | **Not run** — deliberately skipped to avoid a long run; targeted tests only. |

## Findings

### High

#### H1 — `content_for` falls back to `diff_summary` (prose) as file content — latent file corruption
- **Where:** `src/hoh/state.rs:157-166`
- **What:** `PatchSet::content_for(file_path)` returns `intended_content` when present, but when it is `None` and `diff_summary` is non-empty, it returns the `diff_summary` as the file content. A diff summary is prose ("HOH auto-fix for {bug.id}: {fix}"), not file content. Writing it into a source file corrupts that file.
- **Why it is latent, not live:** the two producers that create single-file `PatchSet`s with `intended_content: None` and a prose `diff_summary` are both currently dead code:
  - `src/hoh/autonomous_debugger.rs:84` — `generate_fix_patch` builds a single-file PatchSet with `intended_content: None` and a prose `diff_summary`; `run_debug_session` pushes it into `state.patches` (line 102). `run_debug_session` has **no callers anywhere in `src/`**.
  - `src/hoh/api_evolution.rs:83` — `to_patch` does the same (single file from `proposal.target.split("::").next()`, prose `diff_summary`, `intended_content: None`). `to_patch` also has **no callers**.
- **Why it still matters:** the corruption path is one wiring change away. If anyone connects `run_debug_session` into the loop, the next FullAuto run with passing tests and a meta score > 0.70 would write prose into a source file. The apply path's last line of defense should not be "the producer happens to be dead code".
- **Fix:** never return `diff_summary` as content. Return `None` when `intended_content` is `None` (and let `apply_patch` skip with an error, as it already does for multi-file patches). Consider making `intended_content` a required field on `PatchSet` or adding a `content: HashMap<PathBuf, String>` per-file field so multi-file patches can be applied correctly instead of being skipped (see M5).

#### H2 — Per-model context windows are wrong for grok-4* (M2, still open)
- **Where:** `src/constants.rs:111` (`get_context_window`), `src/constants.rs` (`get_context_budget`, `LEGACY_CONTEXT_BUDGET`)
- **What:** `get_context_window` returns **1,048,576 for all `grok-4*` models**. But base grok-4 is **256,000** and grok-4.5 / 4.6 / 4.7 are **500,000** (only grok-4.3 / 4.20 are 1M). `get_context_budget` returns 950,000 for all grok-4*. The code over-promises the window by 2–4× for the models John actually uses, then overflows it. This is a direct contributor to the "too many tokens" error John hit.
- **Also:** `LEGACY_CONTEXT_BUDGET` is **220,000** while `LEGACY_CONTEXT_WINDOW` is **131,072**. The soft budget *exceeds* the actual window by ~69%, so trimming to the budget cannot prevent overflow on legacy models. The budget must be smaller than the window, not larger.
- **What 16db425 changed here:** only doc-comment delimiters (`/// ---` → `// ---`) in `constants.rs` (+56 lines). No constant values or helper logic changed. The window fix was not in the commit.
- **Fix:** give each grok-4 variant its real window (grok-4 = 256,000; grok-4.5/4.6/4.7 = 500,000; grok-4.3/4.20 = 1,048,576) and make `LEGACY_CONTEXT_BUDGET` smaller than `LEGACY_CONTEXT_WINDOW`. Verify the alias resolution first — the `grok-4` alias may resolve to a newer checkpoint; check xAI's `/v1/models` or docs before hard-coding.

#### H3 — TPM limiter double-counts history and fails hard before routing (M3, still open)
- **Where:** `src/router/app_router.rs:190-192` (estimate + check), `src/utils/rate_limiter.rs:49-73` (`check_limit`)
- **What:** `app_router.rs` estimates the full history at `chars/3 + 800` and passes it to `check_limit`. `check_limit` then adds the recorded prior usage (`current_tokens + estimated_tokens > max_tokens_per_minute`). But each prior call's recorded `prompt_tokens` already includes the history — so the history is counted **twice**. The limiter over-counts by roughly a third and rejects calls that would have fit.
- **Worse:** it fails hard with `"Tokens per minute"` *before* `route()` runs, so no retry or wait-for-window ever happens. The call just dies. This is the most likely cause of John's "too many tokens per minute" error — the message string `"Tokens per minute"` appears only in this limiter, not in the xAI 429 path.
- **Partial mitigation (already present):** the first call of a minute is always allowed (`current_requests > 0` guard in `check_limit`), so a large-but-valid first request is not blocked.
- **Fix:** count the history once — either record only the *new* tokens per call (excluding history) in `request_history`, or estimate only the incremental tokens in `app_router.rs`. And instead of failing hard, wait for the window to roll over (sleep until the oldest entry expires) or return a retryable error that the caller can honor.

#### H4 — `classify_error` has no 400 / `context_length_exceeded` path and no `Retry-After` (M4, still open)
- **Where:** `src/router/backends/grok.rs:82` (`classify_error`)
- **What:** `classify_error` handles 401, 429, network drops, and timeouts — but **not** HTTP 400 / `context_length_exceeded`. A context-overflow response falls through to `RouterError::BackendError`, which is fatal: no trim-and-retry, no graceful degradation. The user just gets an error.
- **Also:** there is no `Retry-After` honoring on 429. When xAI says "retry after N seconds", the code does not read it.
- **Fix:** map 400 / `context_length_exceeded` to a distinct, recoverable error kind that triggers history trimming and a single retry. Parse and honor `Retry-After` on 429 (sleep that long, then retry once).

#### H5 — grok_api 0.2.4 discards the 429 body (M5, still open)
- **Where:** `Cargo.toml` (pins `grok_api = "0.2.4"`); `Cargo.toml:43` (commented-out path dep for grok_api 0.3.1)
- **What:** grok_api 0.2.4 discards the 429 response body, so the retry guidance xAI sends (including `Retry-After` and the specific limit hit) is lost before grok-cli ever sees it. The 0.3.1 path dep is commented out (`# grok_api = { path = "../grok_crate/grok_api", version = "0.3.1" }`), so the fix that exists upstream is not wired in.
- **Fix:** upgrade to grok_api 0.3.1 (or whatever version preserves the 429 body) and plumb the body through to the retry logic in H4.

#### H6 — CLI `conversation_history` grows without bound (M1, still open)
- **Where:** `src/cli/commands/chat.rs:245` (`conversation_history: Vec<serde_json::Value>`), pushes at lines 253, 379, 493, 508
- **What:** the CLI chat loop pushes every turn onto `conversation_history` and never trims it. There is no `drain`, `truncate`, `remove(0)`, or `MAX_HISTORY_MESSAGES` check anywhere in `chat.rs` — the only trimming-related matches in the file are string `.trim()` calls for input parsing. `MAX_HISTORY_MESSAGES` (40) is referenced only in `src/config/acp.rs` (the ACP path). So a long CLI session accumulates the entire conversation and eventually overflows the context window — the second half of John's "too many tokens" error.
- **Fix:** apply the same trimming in the CLI loop that the ACP path already has (cap at `MAX_HISTORY_MESSAGES`, or trim to the model's context budget from H2). Trim before sending, not after failing.

### Medium

#### M1 — `usage_stats.json` is read + written on every API call (S5, still open)
- **Where:** `src/router/app_router.rs:190` (load before the call), `src/router/app_router.rs:225` (load again after the call), `src/utils/rate_limiter.rs:77-95` (`record_usage` auto-saves)
- **What:** every API call does **two disk reads** (`UsageStats::load()` — JSON parse from disk) and **one disk write** (`record_usage` → `save()` — `fs::create_dir_all` + `serde_json::to_string_pretty` + `fs::write`). That is three disk I/Os plus JSON (de)serialization on the hot path, for every single request. For a CLI that makes many calls in a session, this is a steady latency and disk-wear cost, and it is exactly the kind of per-call overhead John's speed-first weighting says to eliminate.
- **Fix:** keep the stats in memory for the process lifetime (load once at startup, or hold a single `UsageStats` in the router) and persist on a schedule (e.g. every N calls, or on clean shutdown) instead of on every call. At minimum, do not load twice per call.

#### M2 — MCP config files commit a machine-specific temp path
- **Where:** `.mcp.json`, `.cursor/mcp.json` (also `.gemini/settings.json`, `opencode.json` added in 16db425)
- **What:** both files point the `blast-radius` MCP server at a hardcoded Windows temp path:
  `C:\Users\johnm\AppData\Local\Temp\bfdeb1cc-f025-4acc-8ace-5406c3360731_Blast Radius 0.1.1 for Windows (x86_64).zip.731\blast_radius.exe`
  This is a temp-extracted binary under `AppData\Local\Temp` with a GUID-like hash in the path. It works only on John's ASUS (username `johnm`), and it breaks the moment Windows cleans the temp dir or the extraction hash changes. It also leaks the Windows username into the repo. The commit message itself flags these as "problematic hardcoded local paths".
- **Fix:** do not commit machine-specific MCP server paths. Either install `blast_radius` to a stable location (e.g. `%LOCALAPPDATA%\blast-radius\blast_radius.exe`) and reference that, or move these configs to a git-ignored local file (`.mcp.local.json` / user settings) and commit only a `.mcp.example.json` with a placeholder path.

#### M3 — `benches/perf_bench.rs` does not compile (the perf bench is dead)
- **Where:** `benches/perf_bench.rs:95,141,145,146`
- **What:** `cargo clippy --lib --all-targets` fails because the bench does not compile — 4 errors:
  - `E0116` (orphan rule): `impl Router` for a type defined outside the crate (`benches/perf_bench.rs:141`).
  - `E0599`: no method named `route_sync_for_bench` on `grok_cli::agent::router::Router` (`:95`).
  - `E0616` (×2): field `bayes` of `Router` is private (`:145`, `:146`).
  - Plus 18 warnings in the bench target.
- **Why it matters:** John weights speed first, and the perf bench is the tool that would let him *measure* speed. A dead bench means perf regressions are caught by feel, not by numbers. It also means `cargo clippy --all-targets` and `cargo test --benches` are broken, so CI (if it runs them) is red for a reason unrelated to the lib.
- **Fix:** either repair the bench (add a `route_sync_for_bench` test helper or a public bench API, and a public accessor for the Bayes state instead of touching a private field) or delete the bench if it is superseded. Do not leave a broken bench in the tree.

#### M4 — Shell denylist: mostly fixed, but two mechanisms can drift (G7, mostly fixed)
- **Where:** `src/acp/security.rs:566` (`SecurityPolicy::validate_shell_command`, the check `shell_tools.rs` actually uses); `src/utils/shell_permissions.rs:171` (`is_blocked`, a second mechanism)
- **What changed (good):** the ACP denylist now has a proper **argv[0] layer (Task 469)** — it tokenizes the command first (collapsing quoting tricks like `r''m` → `rm`, `"rm"` → `rm`, and `${IFS}` games into plain whitespace) and checks the real program name. There are regression tests for the bypasses (`security.rs:975-999`, including `rm -rf /`, `rm -rf $HOME`, `rm -rf ~`, `r''m -rf /`, `rm${IFS}-rf${IFS}/`, `sudo rm -rf /`, `/bin/rm -rf /`, `rm -rfv /`, `dd if=/dev/zero of=/dev/sda`). The false-positive concern from the Nov 6 review (substring-matching single command names) is addressed for the argv[0] layer, which uses exact program-name matching.
- **Residual (this finding):** there are **two** denylist mechanisms. `acp/security.rs` (`validate_shell_command`) is the one `shell_tools.rs` calls, but `utils/shell_permissions.rs` (`is_blocked`, exact root-command matching + its own `"rm -rf /"` substring check) still exists as a separate policy. Two policies that must agree will eventually disagree. And the **substring layer** in `validate_shell_command` (line ~560, "Substring denylist — catches known-dangerous patterns anywhere") can still false-positive on a command that merely *mentions* a dangerous pattern in a benign context (e.g. `echo "do not run rm -rf /"` contains the substring and would be blocked). That is over-cautious, not dangerous, but it is the same class of issue.
- **Fix:** keep one denylist (the ACP `SecurityPolicy` one) and remove or delegate the `shell_permissions.rs` one. For the substring layer, consider exempting quoted/echoed contexts, or accept the over-blocking and document it.

#### M5 — `PatchSet` has no per-file content, so multi-file patches can never be applied (capability gap)
- **Where:** `src/hoh/state.rs` (`PatchSet`, `content_for`), `src/hoh/patch_applier.rs` (`apply_patch`)
- **What:** the fix for the multi-file corruption bug works by *refusing* multi-file patches (because `content_for` returns `None` for them and `apply_patch` skips them with errors). That is the safe choice, but it means the HOH can plan multi-file changes and can never apply them. The capability is gone, not fixed.
- **Fix:** add a per-file content field to `PatchSet` (e.g. `content: HashMap<PathBuf, String>` or `Vec<(PathBuf, String)>`), populate it in the producers, and have `content_for` look it up. Until then, multi-file apply is a known limitation — document it in the HOH doc.

### Low

#### L1 — CHANGELOG formatting drift in 16db425
- **Where:** `CHANGELOG.md` (the 16db425 diff)
- **What:** the diff adds stray blank lines before several pre-existing entries (e.g. before the "SSRF redirect bypass closed" and "Logging" items). Cosmetic, but it makes the changelog harder to scan and suggests the file was edited by hand without a formatter pass.
- **Fix:** remove the stray blank lines; consider a changelog lint or a consistent "one blank line between sections, none between bullets" rule.

#### L2 — The 16db425 commit message is a worklog, not a summary
- **Where:** commit 16db425 ("Fix HOH patch safety and clippy warnings")
- **What:** the commit message body contains the author's internal deliberation about the subject line ("Mention perhaps ... subject enough", "Fix(3)+spaces etc ~47"). It reads like a draft that was committed as-is. The subject itself is fine; the body is not a useful record.
- **Fix:** nothing to do retroactively (do not rewrite a pushed commit); just note it as a hygiene point for future commits.

## What is good / verified to hold

- **The multi-file patch fix holds.** `apply_patch` now resolves content per file via `PatchSet::content_for`; a single blob is no longer written to every file in `files_changed`. Multi-file patches without per-file content are skipped with errors rather than corrupting files. The new tests prove it (single-file write writes the intended content; multi-file with a shared blob writes nothing and records 2 errors). `create_backup_dir` no longer panics on a pre-epoch clock (`.unwrap()` → `.unwrap_or_default()`), and `ApplyResult` now derives `Default` instead of a manual impl.
- **The ExecuteWithApproval gating holds.** `build_apply_config` (`src/hoh/outer_loop.rs:401-418`) sets `dry_run = !matches!(autonomy_level, FullAuto)` — only `FullAuto` performs real writes. `Observe`, `Propose`, and `ExecuteWithApproval` are dry-run. The comment explicitly notes the approval mechanism is "not yet wired into this non-interactive loop", which is honest. The default autonomy level is `ExecuteWithApproval` (`src/hoh/mod.rs`), so the default is safe. The Phase 6 apply gate (`outer_loop.rs:350-354`) requires tests passed *and* patches non-empty *and* meta score > 0.70.
- **Clippy is genuinely clean for the lib.** `cargo clippy --lib` → 0 warnings, exit 0. The "69 → 0" claim in the CHANGELOG holds for the lib target. (The `--all-targets` failure is the broken bench, not the lib — see M3.)
- **The ACP shell denylist is well-engineered.** Task 469's argv[0] analysis with quoting/`${IFS}` normalization is the right design, and the regression tests cover the bypasses. This is a real improvement over a naive substring denylist.
- **Phase 4 testing runs real `cargo test`.** `src/hoh/testing.rs` runs `cargo test --quiet` from the real project root (a previous bug ran it from `.grok/hoh/` instead). The HOH does not simulate test results.
- **The HOH has real safety scaffolding.** Backup directories, `dry_run` / `simulation_mode`, a snapshot manager with rollback (`grok-cli hoh rollback`), and a governance layer (`src/hoh/governance.rs`, `safety.rs`) are all present. The scaffolding is genuine; the weak point is the evaluation, not the plumbing (see the harness review).
- **The 16db425 non-HOH diffs are clean clippy fixes.** `.map_or(false, ...)` → `.is_some_and(...)`, redundant `.clone()` removals, `if let Some ... else return None` → `?`, `.split(|c| ...)` → `.split([...])`, `format!("mem.X")` → `"mem.X".to_string()`, unnecessary `as u32` casts. No behavioral changes, as the CHANGELOG claims.

#### L3 — The `installer` bin has 4 warnings (unused imports + dead code)
- **Where:** `src/bin/installer.rs:4,6,13,26`
- **What:** `cargo check` exits 0, but the `installer` bin generates 4 warnings:
  - Unused imports `Write` and `self` (`use std::io::{self, Write};`, line 4).
  - Unused import `std::process::Command` (line 6).
  - Dead function `find_project_root` never used (line 13).
  - Dead function `get_version` never used (line 26).
- **Why it matters:** the "clippy is clean" claim holds for the lib (`cargo clippy --lib` → 0 warnings), but the bin is not clean. These are trivial to fix (`cargo fix --bin installer` would apply 2 of them). Leaving dead helper functions in the installer suggests the installer was refactored and the leftovers were not cleaned up.
- **Fix:** remove the unused imports and the two dead functions (or wire them up if they are meant to be used).

## Verification detail

- `cargo clippy --lib`: **0 warnings**, exit 0. (Run 2026-11-09, sandbox, toolchain 1.97.1.)
- `cargo clippy --lib --all-targets`: **fails** — `benches/perf_bench.rs` does not compile (4 errors, 18 warnings). See M3.
- `cargo check`: **exit 0** (compiles). The `installer` bin has 4 warnings (see L3).
- `cargo test --lib hoh::patch_applier`: **3 passed, 0 failed** — `content_for_resolves_only_single_file_patches`, `multi_file_patch_does_not_fan_out_shared_blob`, `single_file_patch_writes_intended_content`. (Test binary compile took ~12m 24s on first build; the tests themselves ran in 0.00s.)
- Full test suite: **not run** (deliberately skipped).

## Open items from the Nov 6 review — status

| Item | Status at fa0f047 |
|---|---|
| M1 history trimming | **Still open** (H6) — CLI history unbounded; only ACP trims |
| M2 per-model windows | **Still open** (H2) — all grok-4* = 1,048,576; LEGACY budget > window |
| M3 TPM limiter double-count | **Still open** (H3) — history counted twice; fails hard before `route()` |
| M4 classify_error 400 / Retry-After | **Still open** (H4) — no 400 path, no `Retry-After` |
| M5 grok_api 429 body | **Still open** (H5) — still pinned to 0.2.4 |
| G7 shell denylist substring | **Mostly fixed** (M4) — argv[0] layer is exact; two mechanisms + substring residual remain |

---

*Reviewer: Cobble jr — 2026-11-09. Independent review; the existing review files were not read before or during this review.*
