# Grok-CLI Harness Review — Harness-of-Harnesses (HOH) (Grok 4.2)

**Reviewer:** Grok 4.2 (independent pass after Claude review + fixes)
**Date:** 2026-11-09
**Scope:** `src/hoh/` (~95 modules), outer loop, patch system, evaluation, autonomy gating, simulation vs real execution
**Companion:** `code_review-grok-4.2.md`

> This is a fresh review focused exclusively on the "Harness-of-Harnesses" autonomous outer loop. It reflects the state **after** the multi-file patch bug fix, `ExecuteWithApproval` gating improvement, and full clippy cleanup.

---

## Legend

| Symbol | Severity |
|--------|----------|
| 🔴 | Critical |
| 🟠 | High |
| 🟡 | Medium |
| 🔵 | Low / polish |
| ✅ | Good |

---

## Executive Summary

The HOH subsystem is a **sophisticated, well-structured simulation scaffold** with excellent phase separation, persistence, test-failure feedback, and hygiene (never pollutes `src/` or `.zed/`). It is **not yet** a production autonomous developer as the README claims.

**Major improvements since last review:**
- 🔴 Multi-file patch bug fixed (`PatchSet::content_for` + new tests).
- 🟠 `ExecuteWithApproval` (default) now treated as dry-run. Only `FullAuto` writes.
- All clippy warnings cleared.

**Remaining gaps:**
- Execution phases still only write diagnostic sidecars to `.grok/hoh/scratch/`.
- Evaluation is mostly synthetic (hardcoded base scores + fixed bonuses).
- HOH writes still bypass `SecurityPolicy`.
- 95-module tree is excessive for a scaffold.

The test-failure → next-cycle feedback loop is genuinely valuable and production-grade. The bones are excellent — the gap is between marketing and current implementation.

---

## What HOH Actually Does Today

`HOHManager::run_iteration()` follows a clean 6-phase pipeline:

1. **Plan** — Reads task list, runs prioritization/evolution. Falls back to a large hardcoded `HOHPlan` when planner path isn't taken.
2. **Refactor/Materialize** — Writes diagnostic stubs **only to `.grok/hoh/scratch/`**.
3. **Execute** — For each scheduled task, writes `task_<id>_progress.rs` (canned `TaskProgress` struct + test) to scratch directory. No LLM, no real code changes.
4. **Test** — **Real**: runs `cargo test` against project root, parses output, injects failure hints as next-cycle goals. This is the strongest real piece.
5. **Evaluate** — Mostly synthetic: base meta score 0.68 + fixed bonuses; `helix_score` is literally `Some(0.78)`.
6. **Apply** — Now correctly gated: only `FullAuto` writes. Uses the fixed per-file content resolution.

**Key insight:** The default `hoh start` flow is a **simulation that produces visible artifacts**. It does not yet perform real autonomous development of the project. The README's claims ("production-ready", "full autonomous multi-day development") are significantly ahead of the current implementation.

---

## Architecture & Maintainability

**✅ Strengths**
- Clean phase structure with explicit `IterationStatus`.
- Strong persistence and multi-cycle memory.
- Excellent hygiene: all diagnostic output confined to `.grok/hoh/scratch/`, real task list mutations go through guarded `TaskListAdapter`.
- Real test feedback loop that turns failures into concrete improvement suggestions.

**🟠 Concerns**
- **Module explosion**: ~95 files (29 task_*, 9 patch_*, 7 agent_*, 6 multi_*, plus governance/ethics/creativity/meta/long-term/etc.). This is disproportionate for a simulation scaffold.
- Many "engines" are heuristic/string-templating over seed lists rather than real LLM-driven reasoning.
- `mod.rs` is a giant re-export list (~160 lines).
- Hardcoded fallback `HOHPlan` is still large and should be replaced with a minimal stub.

**Recommendation:** Collapse HOH behind a smaller public API. Mark aspirational modules (`generative_designer`, `ethics`, `meta_planning`, `cross_project_knowledge`, etc.) as experimental behind a feature flag until they do real work.

---

## Safety & Correctness (Post-Fix)

**✅ Fixed**
- Multi-file patch bug: `content_for(path)` prevents fanning one blob across multiple files. New tests verify single-file writes and multi-file skip behavior.
- Autonomy gating: `build_apply_config` now treats everything except `FullAuto` as dry-run. Default behavior no longer silently writes.
- Backup creation uses `unwrap_or_default()`.

**🟠 Still Open**
- HOH writes bypass `SecurityPolicy` entirely. While current paths are safe (scratch only), this is a latent risk if any phase starts targeting real files.
- Evaluation is not data-driven. The apply gate (`meta_improvement_score > 0.70`) is mostly deterministic arithmetic.
- Backups remain lossy (skip binary files, path-collision flattening).

**Recommendation:** Add a strict HOH jail (`<project>/.grok/hoh/`) enforced at the type level or via `SecurityPolicy`. Make evaluation call real LLM scoring before claiming "meta-evaluation".

---

## Documentation vs Reality

The README describes HOH as:
> "Harness-of-Harnesses (HOH) autonomous outer loop … production-ready … full autonomous multi-day development system"

**Reality (as of this review):**
- Produces diagnostic artifacts and runs real tests.
- Does **not** yet generate or apply real code changes in the default flow.
- Evaluation is synthetic.
- The valuable part (test-failure feedback) is real and should be highlighted.

**🟠 Action:** Update the README and `Doc/HOH.md` to clearly label HOH as an **experimental autonomous scaffold** with a roadmap to real code generation. This prevents users from being surprised when `grok-cli hoh start` doesn't rewrite their codebase.

---

## Priority Actions for HOH

**Must fix (High)**
1. **Sandbox HOH writes** — route all autonomous writes through `SecurityPolicy` or a strict `.grok/hoh/` jail.
2. **Make evaluation real** — replace hardcoded scores with actual LLM or heuristic patch-quality analysis.
3. **Consolidate module tree** — reduce 95 files to a maintainable core + experimental modules.

**Should do**
4. Update documentation to match current capabilities (README, `Doc/HOH.md`, changelog).
5. Improve backup robustness (handle binaries, avoid path collisions).
6. Replace large hardcoded fallback `HOHPlan` with a minimal stub.

**Nice to have**
7. Add type-level guarantee that execution phases only target scratch directory.
8. Expose simulation vs real mode more clearly in CLI output.
9. Add integration test that runs a full HOH cycle in a temp project and validates scratch-only behavior.

---

## Bottom Line

The HOH harness is **impressively engineered** with clean phases, real test integration, persistence, and strong safety intent. The recent fixes (multi-file patch bug, proper dry-run gating, clippy cleanup) have eliminated the previous critical issues.

It is currently a **high-quality simulation and feedback scaffold** that produces visible artifacts and learns from test failures. Turning it into a true autonomous developer requires real LLM-driven refactoring in the execute phase, data-driven evaluation, and continued safety hardening.

The foundation is excellent. With the documentation updated to match reality and the remaining safety/evaluation gaps closed, HOH has the potential to become one of the most interesting autonomous coding systems in the Rust ecosystem.

---

**End of Harness Review (Grok 4.2)**  
*Reviewed after the patch safety fixes and full clippy cleanup. Critical items resolved; scaffold is solid but not yet the production autonomous system described in the README.*
