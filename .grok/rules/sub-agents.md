# Sub-Agents & Multi-Agent Work

Grok CLI has built-in support for spawning focused **sub-agents** with different personas/roles. When developing or using the system, deliberately choose the right role — this is a core architectural capability.

## Standard Roles (defined in `SubAgentConfig`)

- **Planner**
  - For: Task decomposition, creating structured plans, high-level exploration, deciding *what* to do.
  - Often used together with `enter_plan_mode` / `exit_plan_mode`.
  - Good at producing step-by-step plans that can later be executed by coders or researchers.

- **Coder**
  - For: Actual code writing, file edits, implementation, refactoring.
  - In this repo: Frequently given restricted tool sets (write + read tools, limited shell).
  - Should follow the "Language Idiomatic Layouts" rules strictly.

- **Researcher**
  - For: Investigation, reading code/docs, web searches, exploring the codebase, gathering facts.
  - Default for discovery work. Safer because it usually has fewer write privileges.

- **Verifier**
  - For: Code review, testing plans/implementations, finding bugs/edge cases, security & correctness checks.
  - Excellent as a second pass (e.g. after a coder finishes).

## How to Use in Practice (inside this project)

- Prefer `spawn_agent` with full `SubAgentConfig` (model, persona, `allowed_tools`, `trusted_dirs`, `max_tool_iterations`).
- Use the convenience presets when appropriate:
  - `SubAgentConfig::planner()`
  - `SubAgentConfig::coder()`
  - `SubAgentConfig::researcher()`
  - `SubAgentConfig::verifier()`
- Use `fork_agent` + `join_agents` for parallel work (e.g. multiple researchers or a researcher + verifier).
- Use plan mode (`enter_plan_mode` / `exit_plan_mode`) when the main agent should focus purely on planning.
- Always monitor with `list_agents`, `get_agent_status`, and `cancel_agent`.
- Give sub-agents **scoped** tasks + relevant context. Be explicit about constraints and success criteria.

## Rules for Working with Agents Here

- Match the persona/role to the actual work being delegated.
- Aggressively restrict tools for safety (especially coder and verifier agents).
- Review sub-agent output before treating it as authoritative.
- For complex features: researcher → planner → coder(s) → verifier is a common effective flow.
- Sub-agents are **not** just fancy tool calls — they are full focused agent sessions. Design delegation with care.
- When editing agent-related code, keep ACP events, status tracking, and the message bus in sync.

This multi-agent pattern is one of the more powerful features of the system. Use it deliberately.

## Documentation

- High-level docs live in `Doc/`.
- Keep `README.md` and `Doc/QUICK_REFERENCE.md` reasonably up to date.
- Major architectural decisions should be noted (see `RELEASE_*.md` files for examples).