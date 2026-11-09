# Harness Review — HOH (Harness-of-Harnesses), grok-cli (Cobble jr)

- **Reviewer:** Cobble jr
- **Date:** 2026-11-09
- **Base:** `origin/master` fa0f047 (commits 16db425 + fa0f047, 2026-11-09)
- **Scope:** the HOH autonomous outer loop in `src/hoh/` (~98 files, ~22,484 LOC) — plan → execute → test → evaluate → apply
- **Weighting:** speed/performance first, security a slim second (per John's stated weighting for his local projects)

## Severity legend

- **Critical** — breaks the build, corrupts data, or opens a live hole. Fix before anything else.
- **High** — a real bug or a live footgun; will bite in normal use or on the next wiring change.
- **Medium** — a genuine defect or cost (perf, correctness, maintainability) that should be fixed but is not urgent.
- **Low** — cosmetic, hygiene, or minor.

## Executive summary

**Verdict: 🔧 Fix the evaluation before trusting FullAuto.**

The HOH is a large, ambitious, and mostly well-scaffolded autonomous loop. The plumbing is real: it plans, it executes, it runs the actual `cargo test` suite (not a simulation), it gates the apply step on tests passing, it takes backups, it has a snapshot/rollback command, and by default it runs in `ExecuteWithApproval` mode, which is a dry run — so the default is safe.

The problem is the **evaluation**, which is the thing the apply gate depends on. The evaluation is synthetic, and the gate is effectively a rubber stamp:

- The `meta_improvement_score` — the number that must exceed 0.70 before patches are applied — **starts at 0.68** and gets +0.04 to +0.08 for *any* activity (tasks materialized, stubs produced, routes made). Any single thing the HOH does pushes it over the threshold. It counts activity, not improvement. It does not measure whether the code got better.
- The "independent Helix evaluation" **does not call Helix**. `evaluate_with_helix` is a local heuristic that rewards patch *volume* (more patches = higher score). The name and the log message say "independent cross-check"; the code says "self-graded".
- There are (at least) two different meta-score computations, and it is not obvious which one governs.

So: the HOH is **safe by default** (dry run), but the moment John flips it to `FullAuto`, the evaluation does not meaningfully discriminate a good iteration from a bad one. Patches will be applied whenever tests pass and the HOH did *anything*. And `architecture_evolution.rs` actively suggests moving to `FullAuto` when the score is consistently above 0.7 — the same threshold that gates the apply — so the system can talk itself into unguarded auto-apply.

**Top items to fix, in order:** H1 (meta score is a participation trophy), H2 (`evaluate_with_helix` is not Helix), H5 (the FullAuto nudge uses the same threshold as the apply gate), H4 (the diff-summary fallback in the apply path), H3 (two meta-score computations).

## What the HOH actually does (vs what the docs claim)

| Doc claim (`Doc/HOH.md`, `README.md`) | Reality |
|---|---|
| "Multi-day autonomous development system" | Plausible in structure (plan → execute → test → evaluate → apply, with a task list, agent lifecycle, governance, meta engines). But the loop's judgment is heuristic, so "autonomous" currently means "runs without a human in the loop", not "makes good autonomous judgments". |
| "Performs autonomous refactoring" | It generates refactoring actions and patch stubs (the A/B/C/D loop in `refactor_materialize_phase`). Whether those patches are *good* is not measured. |
| "Captures patches, runs evaluations, and feeds improvements back into the next cycle" | True for the mechanics (patches are captured, tests run, `continual_improvement.rs` feeds suggestions into the next cycle's goals). But the "evaluation" is a heuristic score, not a measurement of improvement. |
| "Run tests + Helix evaluation" (`Doc/HOH.md:102`) | **Tests: true** — real `cargo test` from the project root. **Helix evaluation: false** — `evaluate_with_helix` never contacts Helix; it is a local patch-volume heuristic (see H2). |
| "Meta-evaluation of overall health, improvement rate, and patch/agent metrics" (`Doc/HOH.md:153`) | The meta-evaluation exists but is the additive heuristic in `evaluate_phase` (see H1). It measures activity, not health or improvement rate. |
| "Governance / Safety Layer" | Real — `governance.rs`, `safety.rs`, backups, snapshot/rollback, dry-run gating. This is the strongest part of the system. |
| "Full iteration lifecycle (Planning → Executing → Testing → Evaluating → Applying)" | True as a pipeline. The weak link is Evaluating, which is the only step that is supposed to exercise judgment. |

## The loop, phase by phase

### Phase 1–2: Plan → Execute
- `planner.rs` (1,313 lines) builds an `HOHPlan` from the task list, refactoring signals, and prior-cycle suggestions. It materializes tasks, generates patch stubs, and routes work to specialist agents (the A/B/C/D loop).
- Execution runs the plan's actions. The plan is real (it reads the actual task list), and the materialized tasks are real tasks.
- **Concern:** the planner is the largest single file in the subsystem (1,313 lines) and the planning logic is heavily heuristic (see `detect_large_modules` in `architecture_evolution.rs`, which boosts scores for tasks whose titles *mention* certain module names, and adds a flat +800 for "planner" and +600 for "evolution_engine" — hardcoded hotspots, not measured ones).

### Phase 3 (actually Phase 4 in the code): Test — REAL
- `src/hoh/testing.rs:18-25` runs `cargo test --quiet` from the real project root (a previous bug ran it from `.grok/hoh/` instead; that is fixed). The result is a real pass/fail plus raw output and a list of failing items. This is genuine — the HOH does not simulate test results.
- **Cost:** it runs the *full* suite (1,331 tests) every iteration. That is minutes per iteration plus the LLM calls for planning and execution. See M1.

### Phase 5: Evaluate — SYNTHETIC (the core problem)
- `evaluate_phase` (`src/hoh/outer_loop.rs:708-774`) computes the `meta_improvement_score` by starting at **0.68** and adding bonuses for activity (see H1). It also merges in the "Helix" score from `evaluate_with_helix`, which is itself a heuristic (see H2).
- The evaluation does **not** measure whether the code improved: it does not compare test results before/after (beyond a single pass/fail), it does not run a linter or clippy on the patched code, it does not check whether a real bug was fixed, and it does not ask any model to judge the patch. It counts how much stuff the HOH did.
- The evaluation notes are littered with task-number codes ("327 tasklist + 361 closed loop active") that are opaque without the task list (see L1).

### Phase 6: Apply — gated, but the gate is weak
- The gate (`src/hoh/outer_loop.rs:350-354`) requires `test_result.passed && !state.patches.is_empty() && meta_improvement_score > 0.70`. Tests passing is a real signal. Patches non-empty is trivial. The meta score > 0.70 is, as shown in H1, nearly always true whenever the HOH did anything.
- `build_apply_config` (`outer_loop.rs:401-418`) sets `dry_run = !matches!(autonomy_level, FullAuto)`, so only `FullAuto` performs real writes. The default autonomy level is `ExecuteWithApproval` (`src/hoh/mod.rs`), so the default is a dry run. This is the correct default and it holds.
- `apply_patch` itself is now safe for the multi-file case (see the code review — per-file content resolution, multi-file patches skipped). But the `content_for` diff-summary fallback (H4) means the apply path can still write prose into a file if the dead-code producers are ever wired up.

## Findings

### High

#### H1 — The meta score is a participation trophy, not a measurement of improvement
- **Where:** `src/hoh/outer_loop.rs:708-774` (`evaluate_phase`)
- **What:** the `meta_improvement_score` — the number that gates whether patches are applied (must be > 0.70) — is computed by starting at **0.68** and adding bonuses for activity:
  - `+0.08` if any tasks were materialized
  - `+0.06` if any patch stubs were produced
  - `+0.05` if any actions were routed to specialists
  - `+0.04` if any refactoring actions were generated
  - `+0.03` if the average diff length is "focused" (0–400 chars); `-0.02` if it is "overly large" (> 1200)
  - `+0.05` if tests passed; `-0.04` if they failed
  - Capped at 0.95.
- **Why it is a rubber stamp:** the base (0.68) is only 0.02 below the threshold (0.70). *Any single activity bonus* pushes it over. Materializing one task → 0.76. Producing one stub → 0.74. Generating one refactoring action → 0.72. Tests passing → 0.73. In practice, any iteration in which the HOH did *anything at all* clears the gate. The score does not discriminate a good iteration from a bad one, because it measures activity (did we do stuff?) rather than outcome (did the code get better? did a real bug get fixed? did a previously-failing test start passing?).
- **Fix:** make the score measure outcomes. At minimum: start at a neutral base well below the threshold (e.g. 0.50), require *evidence* of improvement to cross it (e.g. a previously-failing test now passes, a clippy warning count went down, a real bug from the task list was closed), and do not award points for merely producing artifacts. Better: replace the hand-tuned additive formula with a small set of explicit, auditable criteria, each with a pass/fail, and require all (or a stated subset) to pass before applying.

#### H2 — `evaluate_with_helix` does not call Helix; there is no independent cross-check
- **Where:** `src/hoh/helix.rs:10-40` (`evaluate_with_helix`); `src/hoh/outer_loop.rs:377-383` (where it is merged in)
- **What:** despite the name, the doc claim ("Run tests + Helix evaluation", `Doc/HOH.md:102`), and the log message ("Helix independent evaluation"), `evaluate_with_helix` never contacts Helix (the agent on the R630), never calls an LLM, and never independently verifies anything. It computes a score from patch metrics with hardcoded heuristics:
  - Base 0.65.
  - `+0.05` if any patches; `+0.05` more if ≥ 3 patches.
  - `+0.08` if the average diff is small (0–350 chars); `-0.06` if it is large (> 1000).
  - `+0.04` if files_changed is between patch_count and patch_count × 4.
  - Clamped to [0.4, 0.92].
- **Why it matters:** the score *rewards patch volume* — more patches means a higher score, regardless of whether the patches are good. And it is the HOH grading its own homework: the same system that produced the patches is the one scoring them, using a formula that favours producing more of them. The "independent cross-check" the docs promise does not exist.
- **Fix:** either (a) actually call Helix (or a configured reviewer model) to judge the patches and use that judgment, or (b) rename the function and the doc to say what it is — a local patch-shape heuristic — and stop presenting it as an independent evaluation. Do not leave a function named `evaluate_with_helix` that does not evaluate with Helix.

#### H3 — Two different meta-score computations; unclear which governs
- **Where:** `src/hoh/outer_loop.rs:708-774` (inline additive score in `evaluate_phase`); `src/hoh/architecture_evolution.rs:308` (`compute_meta_improvement_score`)
- **What:** `evaluate_phase` computes the meta score inline with hand-tuned bonuses. Separately, `architecture_evolution.rs:308` has a `compute_meta_improvement_score(recent_proposal_count, recent_success_rate)` that computes a different score (`proposal_factor * 0.4 + success_factor * 0.6`). It is not obvious from reading the code which one feeds the apply gate, or whether both are used in different paths. Two formulas for the same concept, in two places, is a maintenance hazard — a future fix to one will silently leave the other stale.
- **Fix:** have one canonical meta-score function, called from one place, with its formula and its inputs documented. Delete or delegate the duplicate.

#### H4 — The `content_for` diff-summary fallback can write prose into source files (apply-path hazard)
- **Where:** `src/hoh/state.rs:157-166` (`PatchSet::content_for`); producers at `src/hoh/autonomous_debugger.rs:84` and `src/hoh/api_evolution.rs:83`
- **What:** `content_for` returns the `diff_summary` (prose) as file content when `intended_content` is `None`. The two producers that create single-file PatchSets with `intended_content: None` and a prose `diff_summary` are currently dead code (no callers), so the path cannot fire today. But the apply path's last line of defense is "the producer happens to be dead", not a validation. If anyone wires `run_debug_session` into the loop, the next FullAuto run with passing tests and a meta score > 0.70 writes prose into a source file.
- **Fix:** never return `diff_summary` as content — return `None` and skip (as `apply_patch` already does for multi-file patches). Add a per-file content field to `PatchSet` so multi-file patches can be applied correctly (see code review M5). And add a content-sanity check in `apply_patch` (e.g. refuse to write content that looks like prose/a summary rather than source code) as defence in depth.

#### H5 — The FullAuto nudge uses the same threshold as the apply gate, so the system can talk itself into unguarded auto-apply
- **Where:** `src/hoh/architecture_evolution.rs:288` (suggestion to move toward FullAuto when the score is consistently > 0.7); `src/hoh/outer_loop.rs:355` (apply gate: meta score > 0.70)
- **What:** `architecture_evolution.rs:288` suggests moving from `ExecuteWithApproval` toward `FullAuto` when the meta score is consistently above 0.7. That is the same number that gates the apply step. Combined with H1 (the score is nearly always above 0.70 whenever the HOH does anything), the system will consistently "earn" the suggestion to go FullAuto — and once it is in FullAuto, the same rubber-stamp score gates the apply. The suggestion and the gate are not independent checks; they are the same number used twice.
- **Fix:** decouple them. The FullAuto suggestion should require a *different, stronger* signal than the apply gate (e.g. N consecutive iterations in which a real, independently-verified improvement landed, with no rollbacks), and it should require an explicit human approval — it should never be an automatic consequence of the same score that gates the apply.

### Medium

#### M1 — Every iteration runs the full `cargo test` suite (1,331 tests)
- **Where:** `src/hoh/testing.rs:23-25` (`Command::new("cargo").args(["test", "--quiet"])`)
- **What:** Phase 4 runs the entire test suite every iteration. With 1,331 tests, that is minutes per iteration, plus the LLM calls for planning and execution. The HOH has no incremental or targeted test selection — it does not run only the tests affected by the patches it just applied.
- **Fix:** after applying (or before applying, as a dry-run check), run only the tests for the modules the patches touch (e.g. `cargo test --lib <module>`), and run the full suite only periodically (e.g. once per day, or before a commit). This is the single biggest speed win available to the harness, and it matters because John's weighting is speed first.

#### M2 — Dead code in the apply path: `run_debug_session` and `to_patch` have no callers
- **Where:** `src/hoh/autonomous_debugger.rs` (`run_debug_session`), `src/hoh/api_evolution.rs:83` (`to_patch`)
- **What:** both functions produce `PatchSet`s and push them into the loop's patch list, but neither has any caller in `src/`. They look like live parts of the apply path but are not. Dead code that *creates patches* is worse than ordinary dead code: it will confuse anyone who wires it up (they will assume it is a tested, live path), and it is the source of the dangerous PatchSets in H4.
- **Fix:** either wire them up properly (with the `content_for` fix from H4 and tests) or delete them. Do not leave patch-producing dead code in the tree.

#### M3 — ~98 files / ~22.5k LOC: the harness is a second codebase inside grok-cli
- **Where:** `src/hoh/` (98 files, 22,484 LOC); largest files: `planner.rs` (1,313), `task_prioritization.rs` (1,013), `evolution_engine.rs` (923), `outer_loop.rs` (916), `tasklist_adapter.rs` (881), `okf_tasklist_sync.rs` (855)
- **What:** the HOH subsystem is larger than many whole CLIs. It is organized by phase and module (which is good), but its size means it has its own maintenance burden, its own dead code (M2), its own duplicated logic (H3), and its own heuristic formulas that need tuning. The largest files are over 1,000 lines each, which is where the complexity concentrates.
- **Fix:** no immediate action — the size is a consequence of the ambition. But treat the HOH as a first-class subsystem with its own review bar: do not add new heuristic scores or new modules without removing or consolidating an old one, and keep the per-file size down (split `planner.rs` and `task_prioritization.rs` if they keep growing).

#### M4 — `PatchSet` has no per-file content, so multi-file patches can never be applied
- **Where:** `src/hoh/state.rs` (`PatchSet`), `src/hoh/patch_applier.rs` (`apply_patch`)
- **What:** the fix for the multi-file corruption bug works by refusing multi-file patches. The HOH can plan multi-file changes but can only apply single-file ones. This is a capability gap, not a safety bug, but it limits what the autonomous loop can actually do — and it means the "autonomous refactoring" the docs promise is, in practice, single-file-only.
- **Fix:** add a per-file content field to `PatchSet` and populate it in the producers. Until then, document the limitation in `Doc/HOH.md` so the docs do not over-promise.

#### M5 — The planner's "large module" detection is keyword matching on task titles, with hardcoded boosts
- **Where:** `src/hoh/architecture_evolution.rs:318-340` (`detect_large_modules`)
- **What:** the detector decides which modules are "large" by checking whether task titles/descriptions *mention* certain words (`planner`, `evolution_engine`, `outer_loop`, `multi_agent`, `agent`), then adds a flat +800 for "planner" and +600 for "evolution_engine". These are hardcoded hotspots, not measured ones — the function never looks at actual file sizes or line counts. A task that happens to mention "planner" in passing inflates the signal.
- **Fix:** measure actual module sizes (line counts, or the file list the HOH already has) instead of keyword-matching task text. Remove the hardcoded +800/+600 boosts, or derive them from measured data.

### Low

#### L1 — Evaluation notes use opaque task-number codes
- **Where:** `src/hoh/outer_loop.rs:711` (`"327 tasklist + 361 closed loop active"`), and similar notes throughout `evaluate_phase`
- **What:** the evaluation notes are written in task-number shorthand that is meaningless without the task list open. When John reads an evaluation report, he has to look up what 327 and 361 are.
- **Fix:** write the notes in plain language ("task-list integration active; closed feedback loop active").

#### L2 — `max_iterations_per_day` defaults to 0, and it is ambiguous what 0 means
- **Where:** `src/hoh/mod.rs` (`HOHConfig`, `#[derive(Default)]` — `max_iterations_per_day: u32` defaults to 0)
- **What:** it is not documented whether 0 means "unlimited" or "no iterations allowed". An ambiguous default on a safety-relevant limit (how many autonomous iterations run per day) is a footgun.
- **Fix:** document it, or give it an explicit default (e.g. a named constant) so the meaning is clear from the code.

## Safety assessment

| Question | Answer |
|---|---|
| Is running the HOH safe by default? | **Yes.** Default autonomy is `ExecuteWithApproval`, and `build_apply_config` treats that as a dry run — no writes happen. Simulation mode is available and the CLI forces it safe in some paths. |
| Is the apply path itself safe? | **Mostly.** The multi-file fan-out bug is fixed; backups are taken; tests must pass. But the `content_for` diff-summary fallback (H4) is a latent corruption path, and the apply gate (H1) is weak. |
| Is the evaluation real? | **No, not really.** Tests are real. The meta score and the "Helix" score are heuristics that reward activity and patch volume. There is no independent judgment of patch quality. |
| Can the HOH talk itself into FullAuto? | **Yes, in principle.** The FullAuto suggestion uses the same 0.7 threshold as the apply gate, and the score is nearly always above it (H5). Whether the suggestion is acted on automatically depends on the caller; the suggestion itself is the hazard. |
| Can it corrupt the tree? | **Not today, by default.** It would need FullAuto *and* a wired-up dead-code producer (H4) *and* passing tests. That is three conditions, and the first two are not met today. But the margin is thinner than it looks, because the evaluation does not push back. |
| Is it maintainable? | **Borderline.** 98 files / 22.5k LOC is a lot for a harness, and it already has dead code, duplicated scoring logic, and hardcoded heuristics. It needs the same review discipline as the rest of the crate. |

## What is good / strong

- **The default is safe.** `ExecuteWithApproval` is the default autonomy level, and it is enforced as a dry run in `build_apply_config`. The HOH does not write to the tree unless someone deliberately chooses `FullAuto`. That is the single most important safety property, and it holds.
- **Tests are real.** Phase 4 runs the actual `cargo test` suite from the real project root. The HOH does not fake test results.
- **The multi-file patch fix is real.** `apply_patch` resolves content per file; multi-file patches without per-file content are skipped with errors. The new tests prove it.
- **Backups and rollback exist.** `create_backup_dir` takes a backup before applying, and `grok-cli hoh rollback` can restore a previous snapshot. The safety scaffolding is genuine.
- **The governance layer exists.** `governance.rs` and `safety.rs` are real modules, not stubs. The subsystem was built with safety as a design concern, even if the evaluation is the weak point.
- **The loop closes.** `continual_improvement.rs` feeds test-failure hints and suggestions into the next cycle's goals, so the loop does learn from its own test failures. That is a real closed loop, even if the scoring is heuristic.

## Recommended order of fixes

1. **H1** — make the meta score measure outcomes, not activity (start well below the threshold; require evidence of real improvement).
2. **H2** — either actually call Helix for the independent evaluation, or rename the function and fix the doc so it stops claiming an independent cross-check.
3. **H5** — decouple the FullAuto suggestion from the apply-gate threshold, and require human approval for FullAuto.
4. **H4** — remove the diff-summary fallback from `content_for`; add a per-file content field.
5. **H3** — consolidate to one meta-score function.
6. **M1** — targeted tests per iteration instead of the full suite every time.
7. **M2** — wire up or delete the dead patch producers.
8. **M4 / M5** — per-file content for multi-file patches; measure module sizes instead of keyword-matching.

---

*Reviewer: Cobble jr — 2026-11-09. Independent review; the existing review files were not read before or during this review.*
