# Grok CLI Documentation

This folder contains the **full detailed documentation** for Grok CLI.

The root [`README.md`](../README.md) and [`CHANGELOG.md`](../CHANGELOG.md) are kept as concise summaries.

## Quick Links

| Document                        | Description                                      |
|--------------------------------|--------------------------------------------------|
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | Command cheat sheet & common usage              |
| [CONFIGURATION.md](CONFIGURATION.md)     | Full configuration reference                    |
| [SETUP.md](SETUP.md)                     | Installation & initial setup                    |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Common issues and solutions                     |
| [FIXES.md](FIXES.md)                     | Recent bug fixes and workarounds                |
| [CONTRIBUTING.md](CONTRIBUTING.md)       | Development & contribution guidelines           |
| [SECURITY.md](SECURITY.md)               | Security model & external access controls       |
| [HOOKS_AND_EXTENSIONS.md](HOOKS_AND_EXTENSIONS.md) | Extension & hook system                |
| [MAX_TOOL_LOOP_ITERATIONS.md](MAX_TOOL_LOOP_ITERATIONS.md) | Tool loop limit configuration     |
| [TASK_GRAPH_DATA_FLOW.md](TASK_GRAPH_DATA_FLOW.md) | Task graph engine details             |
| [EXTERNAL_ACCESS_QUICK_START.md](EXTERNAL_ACCESS_QUICK_START.md) | External file access guide     |
| [SKILLS_QUICK_START.md](SKILLS_QUICK_START.md) | Skills system quick start               |
| [SKILLS_CATALOG.md](SKILLS_CATALOG.md)         | Self-updating skills catalog (Task 273) |
| [CONFIG_QUICK_START.md](CONFIG_QUICK_START.md) | Configuration quick start               |
| [acp-migration-map.md](acp-migration-map.md) | ACP migration status & plan             |
| [CHANGELOG_FULL.md](CHANGELOG_FULL.md)   | Complete detailed changelog history             |

## Subfolders

- `commands/` — Individual command documentation
- `ai-generated-summaries/` — AI-generated analysis and summaries

## Contributing to Docs

When updating documentation:
- Keep the root `README.md` and `CHANGELOG.md` as **summaries only**.
- Put detailed explanations, examples, and long-form content here in `Doc/`.
- Use relative links (`[File](OtherFile.md)`) when linking between docs.

Last updated: 2026-08 (Self-Updating Skills Catalog + Skill Builder improvements)

### Recent Changes
- **Self-Updating Skills Catalog (Task 273)**: New dedicated system that maintains `.grok/SKILLS_HOOKS_OPTIMIZATION.md`. The model receives a ranked SKILLS CATALOG, HOOKS documentation, and OPTIMIZATION HEURISTICS on **every turn**. See [SKILLS_CATALOG.md](SKILLS_CATALOG.md).
- **Skill Builder now auto-refreshes the catalog**: After creating a skill, `skill-builder` automatically runs `grok skills generate-catalog`. New skills immediately appear in the live catalog.
- **New command**: `grok skills generate-catalog` for manual refreshes.
- **Slash command behavior documented**: Most commands (`/think`, `/clear`, `/help`, `/okf`, etc.) are handled locally as builtins and are **never forwarded to the LLM**.
- **Tool error log clarification**: `grok-tool-error-log.log` contains both `TOOL-OK` (success) and `TOOL-ERROR` entries.
