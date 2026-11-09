# Grok-CLI — Harness Review: HOH (Harness-of-Harnesses) (claude-5.1)

**Reviewer:** Claude (Opus 4.8) — independent pass
**Date:** 2026-11-09
**Scope:** `src/hoh/` (~95 modules) — the autonomous "Harness-of-Harnesses" outer loop, its orchestration, patch application, evaluation, safety gating, and the claims made about it.
**Companion:** `code_review-claude-5.1.md` (whole-crate review).

> "Harness" here = the HOH autonomous outer loop (plan → execute → test → evaluate → apply), the system the README calls a _"Harness around Harnesses."_ This review assesses what it **actually does** versus what it **claims to do**, and whether running it is safe.

> **Update 2026-11-09 — fixes applied:** HARNESS-1 (`ExecuteWithApproval` now a dry run; only `FullAuto` writes) and HARNESS-3 (multi-file patch fan-out) are **fixed**, with new `patch_applier` tests and a clean `cargo clippy` (0 warnings). Still open: HARNESS-2 (sandbox HOH writes under `SecurityPolicy`/`.grok/hoh`), HARNESS-4 (real evaluation), HARNESS-5 (lossy backups), module-tree consolidation, and the README maturity claims.

---

## Legend

| Symbol | Severity     |
| ------ | ------------ |
| 🔴     | Critical     |
| 🟠     | High         |
| 🟡     | Medium       |
| 🔵     | Low / polish |
| ✅     | Good         |

---

## 0. Headline Finding

**The HOH loop is a well-structured _scaffold / simulation_, not a working autonomous developer — yet the README markets it as "production-ready," "fully operational," performing "full autonomous multi-day development."**

Concretely, the two phases that are supposed to produce real code changes — `execute_phase` and `refactor_materialize_phase` in `outer_loop.rs` — only ever write **diagnostic sidecar files** into `.grok/hoh/scratch/`. There is no call to an LLM to write real code, and no patch that targets `src/`. The "evaluation" that gates application is mostly hardcoded arithmetic (`meta = 0.68` base + small fixed bumps; `helix_score: Some(0.78)` literal).

So: the loop runs, persists iterations, writes scratch artifacts, and reports success — but it does not develop the project. That is a defensible _engineering scaffold_, but the documentation materially misrepresents it.

---

## 1. What the harness actually does (traced)

`HOHManager::run_iteration()` (`outer_loop.rs`) executes six phases:

1. **Plan** (`plan_phase` → `HOHPlanner::create_plan`) — real-ish: reads the task list, runs task evolution/prioritization. But when the planner path isn't taken, there is a **giant hardcoded fallback `HOHPlan`** (lines ~472–513) with literal task IDs, experiments, and "refactoring actions" as strings.
2. **Refactor/Materialize** (`refactor_materialize_phase`) — ✍️ writes `*.txt`/`*.rs` **diagnostic stubs to `.grok/hoh/scratch/` only**. Each stub's own content says _"This is a simulation / visibility stub … does NOT represent a real patch."_
3. **Execute** (`execute_phase`) — ✍️ for each scheduled task, writes `task_<id>_progress.rs` to `.grok/hoh/scratch/`. The content is a canned `TaskProgress` struct + a trivial passing test, built by `build_task_progress_content`. No LLM, no real edit.
4. **Test** (`testing::run_testing`) — real: runs `cargo test` at `project_root` and parses output into pass/fail + failure hints. This part is genuine and the **failure-feedback loop into next-cycle goals is a nice design**.
5. **Evaluate** (`evaluate_phase`) — mostly synthetic: `meta` starts at `0.68`, gets `+0.08/+0.06/+0.05/...` for having materialized tasks / stubs / routes, `helix_score` is the literal `Some(0.78)`.
6. **Apply** (`apply_patches`) — real `fs::write`, but since phases 2–3 only emit scratch paths, it writes scratch files. Gated by `test_passed && patches>0 && meta_improvement_score > 0.70`.

Because the only patches produced target `.grok/hoh/scratch/`, the apply step is effectively harmless in the default flow — **but that's incidental, not enforced** (see §3).

---

## 2. Scope & maintainability 🟠

- `src/hoh/` is **~95 files**: 29 `task_*`, 9 `patch_*`, 7 `agent_*`, 6 `multi_*`, plus `governance`, `ethics`, `creativity`, `generative_designer`, `meta_planning`, `meta_evaluation`, `long_term_strategy`, `cross_project_knowledge`, `self_improvement`, etc.
- `mod.rs` is ~160 lines of `pub mod` + `pub use` re-exports alone.
- Modules I sampled (`creativity.rs`, `evaluate_phase`, fallback plan) are **heuristic/string-templating engines** — e.g. `CreativityEngine` scores "novelty" by set-distance from a hardcoded seed list of concept names. That's fine as a placeholder, but 95 such files create a large surface that _looks_ like deep capability while being mostly scaffolding.
- 🟠 **Recommendation:** collapse the HOH tree behind a small number of real, tested engines. Delete or clearly mark the aspirational modules (`generative_designer`, `ethics`, `meta_*`, `cross_project_*`) as `experimental` behind a feature flag so the build and the mental model shrink. A 95-file subsystem that writes scratch files is a net maintenance liability.

---

## 3. Safety / correctness of the apply path

### 🟠 HARNESS-1 — `ExecuteWithApproval` (the default) does not ask for approval

`AutonomyLevel` defaults to `ExecuteWithApproval` (`hoh/mod.rs`). But `build_apply_config` only marks `Observe | Propose` as `dry_run`:

```rust
dry_run: matches!(self.config.autonomy_level,
    AutonomyLevel::Observe | AutonomyLevel::Propose),
```

So `ExecuteWithApproval` **and** `FullAuto` both perform real `fs::write` with no human gate. The name promises a confirmation step that does not exist. And `grok-cli hoh start` dispatches with `simulation = false` (`cli/app.rs`), so the default live run is `simulation_mode=false` + `ExecuteWithApproval` → writes to disk if the (synthetic) criteria pass.

- **Fix:** either implement an actual approval prompt for `ExecuteWithApproval`, or treat it as dry-run until approval exists, or rename it to `Execute`. Right now the safest-sounding default is one of the least safe.

### 🟠 HARNESS-2 — No `SecurityPolicy` sandbox on HOH writes

`patch_applier::apply_patch` writes via raw `fs::write(Path::new(file_path), …)`. It never calls `SecurityPolicy::validate_path_access`. The only real guard is `max_files_per_patch = 20`. Today paths are always under `.grok/hoh/scratch/` because the phases hardcode that — but if a planner/LLM-driven phase ever populates `files_changed` with real or absolute paths, the autonomous loop can write anywhere the process can.

- **Fix:** jail every HOH write under `<project_root>/.grok/hoh/` (or route through `SecurityPolicy`). Defense in depth matters most precisely in the autonomous path.

### 🔴 HARNESS-3 — Multi-file patch content bug (shared with core)

`PatchSet { files_changed: Vec<String>, intended_content: Option<String> }` + `content_to_apply() -> String`. `apply_patch` loops over N files writing the **same** single blob to each. Every multi-file patch corrupts all-but-one file into identical content. Latent today (one-file scratch patches) but structurally wrong. (Detailed in the code review.)

### 🟡 HARNESS-4 — Synthetic evaluation gates real writes

The apply gate uses `meta_improvement_score > 0.70`, but `meta` is a deterministic sum of fixed bumps and `helix_score` is the literal `0.78`. So the "quality gate" is effectively "did we produce any artifacts + did tests pass." If/when phases 2–3 start writing real code, this gate will wave through changes it never actually evaluated. Make evaluation real before making execution real.

### 🟡 HARNESS-5 — Backups are lossy/collision-prone

Backup only covers files readable as UTF-8 (`read_to_string`), silently skipping binaries; and it flattens paths with `replace(['/','\\'], "_")`, so `a/b.rs` and `a_b.rs` collide. Rollback can't be trusted for those cases. (Also in code review.)

---

## 4. What's genuinely good ✅

- **Phase structure is clean and legible.** The six-phase pipeline with explicit `IterationStatus` transitions is easy to follow.
- **Test phase is real** and runs against the actual project root (a prior bug where tests ran from `.grok/hoh/` is fixed and commented).
- **Failure → next-cycle feedback loop** (`361.5`): real `cargo test` failures are parsed into bounded hints and injected as goals/`improvement_suggestions` for the next iteration. This is the most valuable real mechanism in HOH.
- **Hygiene discipline:** phases deliberately write only to `.grok/hoh/scratch/` and route real task-list mutations through a guarded `TaskListAdapter`. The intent to never pollute `src/`/`.zed/` is clear and consistently commented.
- **Persistence & resumability:** iterations are saved/loaded (`persistence`), enabling multi-cycle memory; `ensure_hoh_structure` is idempotent and defensive.
- **Backups + `max_files_per_patch` cap + simulation mode** show the author is thinking about blast radius.
- **Helix cross-check** concept (independent evaluator merged with meta score) is a good design even if currently stubbed.

---

## 5. Claims vs. reality (for the README)

| README claim                                                                  | Reality in code                                                                                                  |
| ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| "production-ready," "fully operational"                                       | Scaffold; execute/refactor phases write scratch files only                                                       |
| "full autonomous multi-day development"                                       | No LLM-driven code generation in the loop; persistence enables multi-cycle, but cycles don't develop the project |
| "Task Intelligence, Governance, Meta engines, Creativity" (Tasks 327/401–410) | Present as ~95 heuristic/template modules; mostly string-templating & scoring over seed lists                    |
| "Meta-evaluation health / improvement_rate"                                   | Deterministic arithmetic; `helix_score` hardcoded                                                                |
| "plans → executes → evaluates → improves itself"                              | plan/test/persist are real; execute/evaluate/apply-of-real-code are not                                          |

🟠 **Recommendation:** rebrand HOH in docs as **experimental scaffold / research preview** and describe precisely what a cycle does (plans from task list, runs tests, records progress artifacts, feeds failures forward). Honesty here costs little and prevents users from trusting it with a real repo.

---

## 6. Priority actions (harness)

### Must

1. 🔴 Fix the multi-file `PatchSet`/`apply_patch` content model (HARNESS-3).
2. 🟠 Make `ExecuteWithApproval` actually gate, or stop treating non-approval levels as live writes (HARNESS-1).
3. 🟠 Sandbox all HOH writes under `.grok/hoh/` / `SecurityPolicy` (HARNESS-2).

### Should

4. 🟠 Correct the README maturity claims (§5).
5. 🟡 Make evaluation real before making execution real (HARNESS-4).
6. 🟡 Fix lossy/colliding backups (HARNESS-5).
7. 🟠 Consolidate / feature-gate the 95-module tree (§2).

### Nice

8. 🔵 Clear HOH clippy warnings (bulk of the crate's 69 live under `src/hoh/`: collapsible_if, map_or, useless format!/vec!, `Iterator::last`, `div_ceil`, derivable `impl`s, `io::Error::other`).
9. 🔵 Replace the giant hardcoded fallback `HOHPlan` with a minimal, clearly-labeled stub.

---

## 7. Bottom line

The HOH harness is **impressively organized and safely sandboxed by accident**, with one genuinely useful mechanism (test-failure feedback) and solid pipeline/persistence bones. But it is **not** the autonomous developer the README advertises, it has a **real correctness bug** in patch application, and its **default autonomy level writes to disk without the approval its name implies**. Treat it as an experimental scaffold: fix the apply-path safety/correctness, make evaluation real, trim the module sprawl, and — most importantly — align the documentation with what the code actually does.

---

_End of harness review (claude-5.1)._
