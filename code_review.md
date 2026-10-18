# Grok-CLI Code Review — Fresh Baseline (2026)

**Reviewer:** Systematic Code Review  
**Date:** Current HEAD  
**Scope:** Full project (src/, tests/, config/, .grok/, Cargo.toml, key binaries)  
**Focus:** Security, Correctness, Architecture (especially ARCH-2 registry), Agent System, ACP, Tooling, Maintainability

> "Starting from a clean slate. Old review content discarded. Current state evaluated on its own merits."

---

## Legend

| Symbol | Severity |
|--------|----------|
| 🔴 | Critical — fix before release |
| 🟠 | High — address soon |
| 🟡 | Medium |
| 🔵 | Low / polish |
| ✅ | Good / strong area |

---

## 1. SECURITY

### ✅ SEC-2 / SEC-9 — Write & Replace paths aligned
- Both `write_file` and `replace` go through `validate_path_access` + audit logging + `SafetyDecision`.
- `RequireConfirmation` now correctly returns `Err(...)` in both (consistent with previous fixes).
- `TrustAlways` (`add_session_trusted_path`) is wired.

### ✅ Audit & Session correlation
- `ToolContext.session_id` is properly propagated.
- Audit logger uses lazy directory creation + in-memory cache for stats.

### ✅ CoT Guard (Radioactive Isotope Policy)
- `utils/cot_guard.rs` is exported from lib.
- Strict cleaning + debug asserts in `chat_turn.rs`.
- `thinking_mode: Off` + `stream_thinking` controls are respected.

### 🟠 SEC-4 — `process::exit`
- Still present in:
  - `src/bin/installer.rs` (binary — acceptable per comments)
  - `src/cli/commands/acp.rs` (stdio mode exit)
- Library code (`utils/auth.rs`) correctly returns `Err` instead of exiting.
- **Recommendation:** Keep comments, consider a `graceful_exit` helper for binaries only.

### ✅ SEC-5 — `session_dna.json` location
- Project-local now lives under `.grok/session_dna.json` (VCS-safe).
- `SessionDna::load()` prefers `.grok/` then `~/.grok-cli/`.
- Root-level file is ignored for new DNA (good).

### ✅ External access & sandboxing
- SecurityPolicy + trusted directories model is solid.
- Sub-agent sandbox support exists.

**Overall Security:** Strong. The model has matured significantly.

---

## 2. CORRECTNESS

### ✅ COR-10 — Shell exit codes
- `run_shell_command` now returns `Err` on non-zero exit (rich error containing output).
- Updated tests reflect this.

### ✅ PowerShell `&&` translation
- Proper conditional translation implemented (not naive replace).

### ✅ Tool dispatch & arbitration
- `tool_arbitration` + registry is consistent.
- Round-trip tests (`execute_tool_round_trip_write_read_unknown_missing`) cover happy + error paths.

### ✅ Static regexes & helpers
- JSONC trailing comma, code defs, etc. are `Lazy` / `OnceLock`.

### 🟡 Rate limiting
- Config exists but is largely advisory (no hard client-side enforcement in all paths).

### 🟡 Some platform-specific tests
- Several file/shell tests are `#[cfg(target_os = "windows")]` or ignored on non-Windows.

---

## 3. ARCHITECTURE & DESIGN

### ✅ ARCH-2 / Task 260 — Unified Tool Registry (Major Win)
This is now one of the strongest parts of the codebase:

- **Single source of truth**: `get_full_tool_definitions()` (JSON schemas).
- **Thin handlers**: Every tool has a dedicated `handle_*` function using `require_str` / `require_*` helpers.
- **Pure dispatch table**: `execute_tool` match is now ~1 line per tool.
- **Static caching**: `OnceLock` for definitions, names, required params map, known tools set.
- **O(1) hot paths**: `is_known_tool_fast`, `get_required_parameters_fast`.
- **Drift guards**:
  - `every_tool_schema_has_corresponding_handler` test
  - `unknown` arm that gives a clear "add handler" error
  - Symmetry tests
- **How to add a tool**: Clearly documented (schema + one handler + one match line).

This is excellent engineering. The registry is now maintainable and testable.

### 🟠 Library / Binary separation (documented debt)
- `src/lib.rs` has an honest "Current State" section listing violations.
- Many `cli/commands/*`, `display`, and terminal I/O still do direct printing.
- Progress: `terminal/` module created, some deprecation markers.
- **Recommendation**: Continue the migration path outlined in lib.rs. Avoid adding new I/O to lib.

### 🟠 Monolithic files
- `src/acp/mod.rs` is still very large (~1800+ lines).
- `src/tools/registry.rs` is now intentionally large but **well-structured** (handlers + tests + docs).
- `handle_chat_completion` has been partially extracted into `chat_turn.rs`.

### ✅ Agent roles & personas
- Clear separation between `reviewer` (sarcastic senior dev, read+limited shell) and `verifier` (strict QA, test runner).
- `SubAgentConfig::reviewer()` fixed.
- Role inference in `chat_turn.rs` and status bar icons updated (`👀` for reviewer).
- Presets in `config/agents/` and `.grok/agents/`.

### ✅ Thinking / CoT controls
- `ThinkingMode` (Off/Low/High) + `stream_thinking` fully wired through ACP, status bar, chat_turn, and slash commands.
- `/cot` command support.

---

## 4. AGENT SYSTEM & SUB-AGENTS

### Strong points
- Rich `SubAgentConfig` builder (model, system_prompt, allowed_tools, trusted_dirs, max_tool_iterations).
- Memory bus, team, fork/join, send/receive messages.
- `spawn_agent_configured` + role inference.
- DNA integration (`SessionDna` influences bayes, skill weights, planning, etc.).

### Observations
- Agent system is quite sophisticated (hoh/ evolution layer, bayes, etc.).
- Good isolation via per-agent trusted dirs and tool whitelists.

---

## 5. ACP, STATUS BAR, SLASH COMMANDS

- Status bar correctly reflects thinking mode and agent role.
- Slash commands for thinking mode, agents, etc.
- MCP bridge and tool discovery present.
- Elicitation and cancellation support.

---

## 6. TOOLING, FILE & SHELL

- File tools have consistent `&ToolContext` signatures.
- Good use of `SecurityPolicy`.
- Shell tool now fails correctly on error.
- Notebook, LSP, MCP, vision, image tools present.
- OKF (knowledge) tools added.

---

## 7. TESTING & QUALITY

### ✅ Excellent registry tests
- `tool_definitions_are_statically_cached`
- `every_tool_schema_has_corresponding_handler`
- Round-trip write/read + error cases
- Symmetry and required-params accuracy

### 🟡 Platform & network tests
- Some tests still skip on non-Windows or require network.
- Vacuous tests have been mostly cleaned up.

### ✅ Integration tests exist
- `tests/file_tools_tests.rs`, `tests/tool_loop_integration.rs`, etc.

---

## 8. BUILD, CONFIG & RELEASE

- `Cargo.toml` has `edition = "2024"`, `rust-version = "1.97.1"`.
- `Cargo.lock` is committed.
- Good use of `OnceLock` for zero-cost statics.
- Directory unification (`~/.grok-cli` for global data) is complete.
- `session_dna.json` properly handled for VCS safety.

---

## 9. POSITIVES (Current State)

- **ARCH-2 registry** is a model of how to do tool dispatch cleanly in Rust.
- Security model (validate early + audit + safety decisions) is now consistently applied.
- CoT policy is strictly enforced.
- Agent role distinction (reviewer vs verifier) is clear.
- Static caching + O(1) lookups in hot paths.
- Honest debt documentation in `lib.rs`.
- Thin main + real binary entry point.
- DNA + bayesian + sub-agent system is advanced.
- Many previous critical items (SEC-2, COR-10, directory paths, etc.) have been resolved.

---

## 10. PRIORITY ACTION LIST (Fresh Baseline)

### Must address

1. 🟠 Continue library/binary separation (per the TODOs in `lib.rs`).
2. 🟡 Rate limiting — either implement hard enforcement or document as advisory only.
3. 🟡 Reduce size of `acp/mod.rs` (more extraction into focused modules like `chat_turn.rs`).

### Nice to have / polish

4. 🔵 Centralize remaining magic numbers (timeouts, token budgets, etc.).
5. 🔵 Improve test coverage for cross-platform + offline scenarios.
6. 🔵 Consider a small `ToolError` enum instead of mixing `anyhow!` + structured JSON strings in some paths.
7. 🔵 Add a declarative table/macro for the registry in a future major version (current hand-written version is already very good).

### Not blockers

- `process::exit` in installer and acp stdio mode (documented + binary-only).
- Installer being Windows-focused (current delivery model).
- Some large files that are now well-organized.

---

## 11. RECOMMENDATIONS

1. **Keep the registry discipline.** The pattern (schema + thin handler + 1-line dispatch + guard test) is excellent. Apply similar thinking to other registries if they appear.
2. **Document the "why" for reviewer vs verifier** in user-facing docs (they are intentionally different roles).
3. **When touching ACP**, keep extracting pieces out of `mod.rs`.
4. **For new features**, prefer adding thin handlers + schema over direct `match` arms.

---

**End of Fresh Review**

The project has made substantial architectural progress, especially around the tool system and security enforcement. The bones are good and the recent work on consistency and guards is visible.

*Review baseline established.*