# Changelog

All notable changes to the Grok CLI project are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

**Full detailed history** is available in [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md).

---

## [Unreleased] - 2026-05-10

### Highlights

- **ACP connection-layer rewrite** (Task 111.3) — Replaced manual JSON-RPC dispatch with `Agent.builder() + ByteStreams`. Full typed handlers for `initialize`, `session/new`, `session/prompt`, etc.
- **Session persistence & fork** (Tasks 111.5, 111.7) — Disk-based session save/restore + `session/fork` support with fresh Bayesian engine.
- **ACP schema migration** (Task 111.1–111.2) — 11 types replaced with `agent-client-protocol` crate re-exports; wire-format verified.
- Multiple bug fixes for Zed compatibility (slash commands, thinking mode, file URI handling).

**655/655** lib tests + integration tests pass. Clippy clean.

See [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md) for the complete unreleased notes and all prior versions.

---

## [0.1.10] — 2024-10-04 (Summary)

- Task Graph Engine, Skill Auto-Activation, Session DNA, Plugin Sandbox
- External directory access with approval + audit logging
- Chat logging, search, and replay
- ACP workspace access fixes

---

## [0.1.9] and earlier

See the full archive in [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md) for detailed entries from v0.1.9 back to the initial public release (v0.1.2).

---

**Links**

- Repository: https://github.com/microtech/grok-cli
- Issues: https://github.com/microtech/grok-cli/issues
- Buy Me a Coffee: https://buymeacoffee.com/micro.tech
