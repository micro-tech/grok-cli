# Grok CLI — Project Context & Rules

This is the **project-local** context for developing Grok CLI itself.

These rules **build on** the global rules in `~/.grok-cli/context.md`. The global Production-Grade Mandate still applies 100% — no exceptions.

---

## Core Philosophy (Reinforced)

- **Full code. Finish the task.** No partials, no TODOs, no "we'll come back".
- Every change must be production-grade and complete.
- Use the task system religiously for non-trivial work.

## Task Management (This Project)

- Task list lives at **`.zed/task_list.json`** (JSON with `{ "tasks": [...] }`).
- When acting as an assistant for a project (including this one), prefer the application tools `task_create`, `task_update`, `task_get` (these are registered in the Grok CLI tool system and operate directly on the task list).
- In this meta/tool-calling environment the task tools may not be directly exposed — fall back to reading `.zed/task_list.json` with available file tools and editing it carefully with `replace` when needed.
- When starting work, read the current task list first (use `task_get` if available, otherwise read the file).
- Create proper tasks with `details`, `testStrategy`, and subtasks where useful.
- Mark tasks `done` **only** when code + tests + docs are complete and clean.

**When you need to create or execute tasks, follow the authoritative rules in:**
- `.grok/skills/task-list/SKILL.md` (full **Task Builder** section for creating tasks + **Task Runner** section for executing them)
- Always prefer the `task_create` / `task_update` / `task_get` tools when they are available in the environment.
- Legacy notes (old "Task Builder Guide" / "Task Runner Guide" using `task_manager.json`) still exist in some chat logs and temp files — they are historical only. The skill above is the current source of truth.
- Current major areas of work often include:
  - Tool registry & ARCH-2 improvements
  - ACP / sub-agents
  - MCP protocol evolution
  - Task graph / workflow engine
  - Configuration modularization

## How to Work on Grok CLI Code

Detailed rules live in the `rules/` subdirectory:

- **[Architectural Guardrails](rules/architectural-guardrails.md)** — Non-negotiable symmetry, contracts, guard tests.
- **[Tool Development Process](rules/tool-development.md)** — Strict schema/handler process, language layouts, testing requirements.
- **[Sub-Agents & Multi-Agent Work](rules/sub-agents.md)** — Personas, delegation patterns, and best practices.

## Workflow When Starting Work

1. Read current `.zed/task_list.json` (or use `task_get`).
2. Create or pick a task.
3. Set it to `in_progress`.
4. Do the work **completely** (full code + tests + contract updates + docs).
5. Update the task to `done`.
6. Consider whether a changelog entry or doc update is needed.

## Additional Notes

- See `.grok/rules/` for more focused guidelines.
- The old monolithic context has been split to reduce token usage while keeping critical rules accessible.
- SKILLS & optimization info lives in `SKILLS_HOOKS_OPTIMIZATION.md` (auto-generated).

---

**Last major cleanup**: Restructured after significant architecture work (ARCH-2, config split, task tools, sub-agents, MCP changes).

Update this file when project-specific patterns or mandatory practices evolve.