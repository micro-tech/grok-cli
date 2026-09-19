# Harness-of-Harnesses (HOH) — Autonomous Outer Loop

HOH is the **multi-day autonomous development system** for Grok CLI.

It acts as an outer loop that:
- Reads your `.zed/task_list.json`
- Plans work using real dependency graphs and prioritization (327)
- Evolves the task list itself
- Proposes architecture changes
- Performs autonomous refactoring
- Routes work to specialized agents
- Captures patches, runs evaluations, and feeds improvements back into the next cycle

**Core Philosophy (2026 evolution):** HOH is a **Harness of Harnesses** — an outer orchestration layer that coordinates many inner "agent harnesses" (specialized profiles, simulators, collaborators, and lifecycle-managed agents).

## Core Ideas (361 Batch + Extensions)

| Task / Layer | Component                              | Purpose |
|--------------|----------------------------------------|-------|
| 327          | Task Intelligence (Dependency Graph, Prioritization, Selection, Evolution) | Single source of truth for task scheduling and mutation |
| 361.1        | Architecture Evolution Engine          | Proposes structural changes to the codebase and to HOH itself |
| 361.2        | Self-Refinement Loop                   | Improves HOH's own planner, scoring, and heuristics |
| 361.3        | Autonomous Refactoring Engine          | Turns proposals into concrete, executable refactoring actions + tiny safe patches (A/B/C/D loop) |
| 361.4 / 361.5| Specialized Agent Profiles + Continual Improvement | Routes high-confidence work to role-specific sub-agents; closed meta-loop that turns outcomes into suggestions |
| 401          | Creativity Engine                      | Generates novel ideas and seeds new tasks |
| 402          | Generative Architecture Designer       | Builds on creativity to propose new modules and designs |
| 403–405      | Agent Lifecycle / Birth / Evolution    | Register, birth, evolve, retire, and hibernate agents |
| 406          | Multi-Domain Reasoner                  | Cross-domain reasoning across goals |
| 407          | Governance Engine                      | Constitutional rules and high-bar proposal blocking |
| 408          | Ethics Engine                          | Ethics constraint checking |
| 409          | Meta-Planning Engine                   | Plans for HOH's own improvement |
| 410          | Meta-Evaluation Engine                 | Evaluates overall HOH health and improvement rate |
| 361.7 / 361.8| MultiAgentOrchestrator + Simulations   | **The Inner Harness**: coordinates specialized agents, runs sim-first predictions, delegation, skill evolution |
| 361.9        | Long-Term Strategy Engine              | Maintains strategic goals that span many iterations |
| 361.11       | Cross-Project Knowledge Transfer       | Extracts and applies patterns across projects |
| 361.0101 / 370 | Multi-Project Orchestrator           | Coordinates work across multiple related projects |

## The Harness-of-Harnesses Concept

HOH is not just one agent. It is:

- **Outer HOH Loop** (`outer_loop.rs` + `planner.rs`): The meta-orchestrator that runs planning → execution → evaluation → improvement cycles.
- **Inner Agent Harnesses** (via `MultiAgentOrchestrator`): Many specialized "harnesses" (profiles like Architect, Debugger, Researcher + simulation + collaboration protocols).
- **Governance / Safety Layer**: Cross-cutting rules (407/408) that sit above both.
- **Evolution Layers**: Creativity, agent birth/evolution, and long-term strategy that allow the whole system to grow.

This creates a true **harness around harnesses** — the outer loop treats inner agent coordination as just another capability it can orchestrate, simulate, and improve.

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

# Task list versioning (327.13)
grok-cli hoh versions                    # List historical snapshots
grok-cli hoh save-version "before-refactor"
grok-cli hoh rollback v1728123456        # Restore a previous snapshot
```

All commands work **without an API key** because they are local orchestration + planning.

## Data Locations

- Task list (input): `.zed/task_list.json`
- HOH state & iterations: `.grok/hoh/iterations/`
- Persistence, patches, and logs live under `.grok/hoh/`

## How It Works (High Level)

1. **Plan Phase**
   - Task list evolution (327.34)
   - Architecture proposals (361.1)
   - Self-refinement proposals (361.2)
   - Autonomous refactoring actions (361.3)
   - Creativity + generative design (401/402)
   - Agent birth/lifecycle signals (403–405)
   - Governance + ethics checks (407/408)
   - Meta-planning (409)
   - Long-term strategy influence (361.9)
   - Cross-project pattern extraction (361.11)
   - Multi-project orchestration (361.0101)
   - Dependency-aware selection + scoring via TaskSelectionEngine (327.17)
   - Inner harness simulation predictions (361.8)

2. **Execute / Materialize**
   - Materialize new tasks from high-confidence refactorings (A)
   - Generate tiny safe patch stubs (C)
   - Route to specialized agents via MultiAgentOrchestrator (D + 361.7)
   - Execute scheduled work (real or simulated)

3. **Evaluate + Improve**
   - Run tests + Helix evaluation
   - Meta-evaluation (410)
   - Continual improvement suggestions (361.5)
   - Feed signals (including rich test failures) back into the next cycle
   - Update agent metrics, strategies, and cross-project knowledge

## Features by Layer (Harness-of-Harnesses)

### 1. Outer Orchestration Layer
- Full iteration lifecycle (Planning → Executing → Testing → Evaluating → Applying)
- Feedback from previous iterations into planning goals (361.5)
- Rich test failure signal injection
- Patch quality metrics in evaluations
- Persistence of full iteration history

### 2. Task Intelligence Layer (327)
- Real dependency graph + topological ordering
- Multi-signal prioritization (completion history, helix scores, OKF terms, refactoring signals)
- Live scheduling (`TaskSelectionEngine::schedule()` / `select_with_history()`)
- Autonomous task list evolution + mutation (327.34)
- Consistency checking and conflict detection (327.33 / 327.18)

### 3. Meta-Evolution & Refactoring Layer (361.1–361.3 + 401/402)
- Architecture proposals that target both the codebase and HOH itself
- Self-refinement of planner scoring and heuristics
- Autonomous Refactoring Engine that produces concrete actions + tiny safe patches
- Creativity Engine that generates novel ideas and turns them into real `AddTask` mutations
- Generative Architecture Designer

### 4. Inner Agent Harness Layer (361.4 / 361.7 / 361.8)
- Specialized Agent Profiles with differentiated risk tolerance and success thresholds
- `MultiAgentOrchestrator`: simulation-first delegation, collaboration protocols, skill evolution
- Multi-agent simulation (delegation predictions, monte-carlo collaboration, ecosystem pressure)
- Routing of high-confidence work to the right profile (D phase)
- Orchestration results fed back into improvement suggestions

### 5. Agent Lifecycle & Evolution Layer (403–405)
- Agent birth system (spawn new specialists when pressure is detected)
- Performance tracking + retirement/hibernation decisions
- Agent evolution events that improve capabilities over time
- Re-use of retired agent slots

### 6. Governance, Ethics & Safety Layer (407 / 408 + safety.rs)
- Constitutional rules with hard thresholds
- Proposal blocking (e.g., self-modify without simulation)
- Ethics constraint checks
- Cross-cutting governance decisions logged in every plan

### 7. Strategic & Meta Layers (361.9 / 409 / 410)
- Long-term strategy engine that influences short-term goals across many iterations
- Meta-planning for HOH's own improvement
- Meta-evaluation of overall health, improvement rate, and patch/agent metrics

### 8. Cross-Project & Multi-Project Layer (361.11 / 361.0101)
- Extraction of transferable patterns
- Incoming pattern application
- Registration and orchestration across multiple projects (grok-cli + helix + okf, etc.)
- Resource allocation and shared goal planning

### 9. Evaluation & Continual Improvement Layer
- Helix independent scoring
- Rich test output + failure hint capture
- Continual improvement suggestions injected into next cycle
- Patch metrics (count, files changed, avg diff length) influencing scores

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
- Patch application is gated (requires test pass + high meta score).
- Tiny safe patches are preferred over large rewrites.
- Governance + Ethics engines run on every high-confidence proposal.
- Full `FullAuto` mode is opt-in and should be used with care.

## Related Code

- `src/hoh/` — all the real implementation (40+ modules)
- `src/hoh/outer_loop.rs` — main iteration driver
- `src/hoh/planner.rs` — planning + 361.x + 327 + inner harness integration
- `src/hoh/multi_agent_orchestrator.rs` — the inner harness
- `src/hoh/governance.rs`, `ethics.rs`, `safety.rs`
- `src/hoh/autonomous_refactoring.rs`, `architecture_evolution.rs`, `creativity.rs`
- `src/hoh/agent_*` family (lifecycle, birth, evolution)
- `src/hoh/long_term_strategy.rs`, `cross_project_knowledge.rs`, `multi_project_orchestrator.rs`
- `src/hoh/cli.rs` — command handling
- `src/cli/app.rs` — wires `grok-cli hoh ...`

## Status

As of late 2026, the full 327 + 361.x + 401–410 + 361.7–361.11 + 361.0101 stack is implemented and wired into the planner and outer loop.

The system now embodies a true **Harness of Harnesses**:
- Outer loop plans and improves the whole system.
- Inner multi-agent orchestrator runs specialized harnesses with simulation.
- Governance, lifecycle, strategy, and cross-project layers provide long-term coherence.

See also:
- Task list in `.zed/task_list.json` (the source of truth HOH reads)
- `grok-cli hoh status` for live history
- `.grok/hoh/` for iteration artifacts

---

*Last major update: full Harness-of-Harnesses layering + 361.7–361.11 + 401–410 features documented.*