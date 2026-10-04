# Architectural Guardrails (Non-Negotiable)

These are the hard rules that protect the integrity of this codebase.

- **Symmetry is sacred** — Schemas, handlers, dispatch logic, tests, and documentation must change together. Never update one without the others in the same change.
- **Contracts are the source of truth** — The schema definitions (`get_full_tool_definitions()`) and public structs are authoritative. Implementation must match them exactly. Behavior that is not declared does not exist.
- **No partial states** — A change is not complete until the entire relevant surface (code + tests + contracts) is consistent.
- **Guard tests must pass** — The `every_tool_schema_has_corresponding_handler` test (and similar drift detectors) are not optional. They must stay green.
- **Unknown arms are sacred** — The `unknown` match arm in tool dispatch must remain a hard failure. Do not weaken it.

Violating these is a direct violation of the Production-Grade Mandate.

## Specific Things to Watch

- **Starlink resilience**: Network code must use the configured retry/backoff logic.
- **No unwraps** in hot or user-facing paths.
- **Schema ↔ Handler symmetry** is the #1 source of drift we actively fight.
- Task list format must stay consistent (task tools handle some legacy layouts gracefully).
- When editing the registry, the `every_tool_schema_has_corresponding_handler` test is your friend — keep it happy.

## Relationship to Global Rules

All rules from `~/.grok-cli/context.md` apply here, especially:

- Production-Grade Mandate
- Never use partial implementations
- Always write full code
- Finish tasks
- Key Guardrails (Symmetry, Contracts as source of truth)

This file adds the **Grok CLI project-specific** details and stricter processes on top.

**Last major cleanup**: Restructured after significant architecture work (ARCH-2, config split, task tools, sub-agents, MCP changes).

Update this file when project-specific patterns or mandatory practices evolve.