# Harness-of-Harnesses (HOH) — Autonomous Outer Loop

HOH is the **multi-day autonomous development system** for Grok CLI.

It acts as an outer loop that:
- Reads your `.zed/task_list.json`
- Plans work using real dependency graphs and prioritization
- Evolves the task list itself
- Proposes architecture changes
- Performs autonomous refactoring
- Routes work to specialized agents
- Captures patches, runs evaluations, and feeds improvements back into the next cycle

## Core Ideas (361 Batch)

| Task | Component                        | Purpose |
|------|----------------------------------|-------|
| 361.1 | Architecture Evolution Engine   | Proposes structural changes to the codebase and to HOH itself |
| 361.2 | Self-Refinement Loop            | Improves HOH's own planner, scoring, and heuristics |
| 361.3 | Autonomous Refactoring Engine   | Turns proposals into concrete, executable refactoring actions + tiny safe patches |
| 361.4 | Specialized Agent Profiles      | Routes high-confidence work to role-specific sub-agents (MetaHookInstaller, HeuristicTuner, etc.) |
| 361.5 | Continual Improvement           | Closed meta-loop that turns outcomes into suggestions for the *next* planning cycle |

## Commands

```bash
# Start a full autonomous iteration
grok-cli hoh start

# Check status and history
grok-cli hoh status
grok-cli hoh last
grok-cli hoh history

# Dry-run / simulation modes
grok-cli hoh apply --dry-run
grok-cli hoh simulate
```

All commands work **without an API key** because they are local orchestration + planning.

## Data Locations

- Task list (input): `.zed/task_list.json`
- HOH state & iterations: `.grok/hoh/iterations/`
- Persistence, patches, and logs live under `.grok/hoh/`

## How It Works (High Level)

1. **Plan Phase**
   - Task list evolution (327)
   - Architecture proposals (361.1)
   - Self-refinement proposals (361.2)
   - Autonomous refactoring actions (361.3)
   - Dependency-aware selection + scoring

2. **Execute / Materialize**
   - Materialize new tasks from high-confidence refactorings (A)
   - Generate tiny safe patch stubs (C)
   - Route to specialized agents (D)

3. **Evaluate + Improve**
   - Run tests + Helix evaluation
   - Continual improvement suggestions (361.5)
   - Feed signals back into the next cycle

## Configuration

HOH currently has minimal external configuration. Future knobs will likely appear under:

```toml
[hoh]
# enabled = true
# simulation_mode = false
# autonomy_level = "ExecuteWithApproval"   # Observe | Propose | ExecuteWithApproval | FullAuto
# max_iterations_per_day = 5
```

For now, behavior is controlled via the `grok-cli hoh` subcommands and the internal `HOHConfig`.

## Safety

- Most runs start in simulation / propose mode.
- Patch application is gated.
- Tiny safe patches are preferred over large rewrites.
- Full `FullAuto` mode is opt-in and should be used with care.

## Related Code

- `src/hoh/` — all the real implementation
- `src/hoh/planner.rs` — planning + 361.x integration
- `src/hoh/autonomous_refactoring.rs`
- `src/hoh/architecture_evolution.rs`
- `src/hoh/continual_improvement.rs`
- `src/hoh/specialized_agents.rs`
- `src/hoh/cli.rs` — command handling
- `src/cli/app.rs` — wires `grok-cli hoh ...`

## Status

As of late 2026, the 361.1–361.5 core loop is implemented and wired into the planner and outer loop. The system is designed for long-running, self-directed improvement across many iterations.

See also:
- Task list in `.zed/task_list.json` (the source of truth HOH reads)
- `grok-cli hoh status` for live history
