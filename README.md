# Grok CLI

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/microtech/grok-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

A powerful command-line interface for interacting with Grok AI via X API, featuring a beautiful interactive experience inspired by Gemini CLI.

> **Latest (v0.2.2)**: ACP startup performance overhaul — `AppRouter`, `SecurityManager`, and `HookManager` are now fully lazy (`OnceLock`). `grok acp stdio` starts in milliseconds and can immediately answer Zed’s `initialize` request. Also includes per-iteration context trimming, slash-command lock fixes, `text`→`content` schema fix, and TGS-RAG engine.

## ✨ Features

<<<<<<< HEAD
- **Interactive Terminal UI** — Gemini-style rich prompts, adaptive ASCII art, progress indicators, and color output
- **Session Persistence** — Save, load, list, and fork conversations (`/save`, `/load`, `/list`)
- **Session DNA** — Persistent personality & behavior config (`session_dna.json`) injected into system prompts ([Doc/SESSION_DNA.md](Doc/SESSION_DNA.md))
- **Skill Auto-Activation** — Skills activate automatically based on project context and keywords
- **Task Graph Engine** — Run complex multi-step workflows with dependency resolution
- **Hierarchical Configuration** — Project-local overrides via `.grok/config.toml`
- **Context Discovery** — Merges `.zed/rules`, `.claude.md`, `GEMINI.md`, and more
- **External Access Controls** — Securely read files outside the project with approval + audit logging
- **Chat Logging & Replay** — Automatic logging with search and history replay
- **Context Compression** — AI-powered summarization + archiving when context fills up
- **Zed Editor Integration** — Full Agent Client Protocol (ACP) support with **instant stdio startup** (lazy router, security & hook managers) and session resume/fork
- **Thinking Modes** — `/think off|low|high` for controllable reasoning effort
- **TGS-RAG Engine** — Text-Graph Synergy Retrieval: hybrid BM25 + embeddings + graph-aware code context (tree-sitter + syn)
- **Code Intelligence** — Explain, review, generate, and refactor across any language
- **Starlink Optimizations** — Smart retries and timeout handling for satellite connections
=======
### 🧠 Reasoning Systems (v0.1.9-pre)
- **Reasoning Protocol Layer (RPL)** — Structured observability for every AI decision: goal analysis, tool selection, uncertainty scoring, and suppression-safe trace logging. All traces are privacy-guarded by default. See `docs/rpl_architecture.md`.
- **Reasoning Engine** — Active decision-making FSM that decomposes goals into multi-step plans, integrates Bayesian belief updates, queries long-term memory, and applies bounded self-correction when steps fail. See `docs/engine_architecture.md`.
- **Privacy Controls** — `SuppressionLayer` and `RedactionConfig` ensure reasoning traces and engine state never leak sensitive data (API keys, secrets, passwords) to user output.
- **227 new tests** — Full unit and integration test coverage for all reasoning components.

### 🆕 New Features (v0.1.8-pre)
- **🔒 External Directory Access** - Securely read files outside project boundaries with interactive approval prompts, comprehensive audit logging, and pattern-based security protections. Perfect for shared configs and cross-project references! [Quick Start Guide](Doc/EXTERNAL_ACCESS_QUICK_START.md)
- **🤖 Skill Auto-Activation** - Skills now activate automatically based on keywords, regex patterns, and file types in your project. Add an `auto-activate` block to any `SKILL.md` to declare triggers. Toggle with `/auto-skills on|off`.
- **🔧 ACP Workspace Access Fix** - The project root where Grok is opened is always trusted from the very first tool call. Handles `file://` URIs, Windows forward-slash paths, and Git-bash style paths correctly.
- **Session Persistence** - Save and resume conversations with `/save`, `/load`, and `/list` commands
- **Hierarchical Configuration** - Project-local settings override system defaults (`.grok/config.toml`)
- **Enhanced Context Discovery** - Multi-editor support: `.zed/rules`, `.claude.md`, `.cursor/rules`, and more
- **Context File Merging** - Automatically merges all available context files with source annotations
- **Extension System** - Extend functionality with custom hooks and plugins
- **Project-Aware AI** - Agent automatically understands your project conventions
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a

See [Doc/QUICK_REFERENCE.md](Doc/QUICK_REFERENCE.md) for the full command list and [Doc/FEATURES.md](Doc/FEATURES.md) (coming soon) for details.

<<<<<<< HEAD
## 🚀 Quick Start
=======
### 💬 Advanced Chat Capabilities
- **Interactive Sessions** - Persistent conversations with context tracking
- **Automatic Tool Execution** - Grok can now create files and directories automatically!
- **Chat Logging** - Automatic conversation logging with full history
- **Session Search** - Search through all past conversations
- **History Replay** - Review and analyze previous sessions
- **System Prompts** - Customize AI behavior for specialized tasks
- **Temperature Control** - Adjust creativity levels (0.0-2.0)
- **Token Management** - Real-time context usage monitoring

### 💻 Code Intelligence
- **Code Explanation** - Understand complex codebases instantly
- **Code Review** - Get detailed feedback with security focus
- **Code Generation** - Create code from natural language descriptions
- **Multi-language Support** - Works with any programming language

### 🔧 Developer Tools
- **External Access Controls** - Securely reference files outside project with approval and audit
- **Health Diagnostics** - Comprehensive system and API monitoring
- **Configuration Management** - Flexible TOML-based settings with validation
- **Audit Logging** - Complete access tracking with CSV export and analytics
- **Zed Editor Integration** - Agent Client Protocol (ACP) support
- **Network Resilience** - Starlink-optimized with retry logic
- **Reasoning Protocol Layer** — Structured trace logging with correlation IDs, log-level controls, suppression and redaction. See `Doc/REASONING_QUICK_START.md`
- **Reasoning Engine** — Goal decomposition, Bayesian belief updates, multi-step planning, memory-aware execution, and self-correction loops

## 🎨 Visual Demo

```
  ░██████╗░██████╗░░█████╗░██╗░░██╗
  ██╔════╝░██╔══██╗██╔══██╗██║░██╔╝
  ██║░░██╗░██████╔╝██║░░██║█████═╝░
  ██║░░╚██╗██╔══██╗██║░░██║██╔═██╗░
  ╚██████╔╝██║░░██║╚█████╔╝██║░╚██╗
  ░╚═════╝░╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝

┌─────────────────────────────────────────────────────┐
│                 Welcome to Grok CLI                │
├─────────────────────────────────────────────────────┤
│ Tips for getting started:                          │
│ 1. Ask questions, edit files, or run commands.     │
│ 2. Be specific for the best results.               │
│ 3. /help for more information.                     │
│ 4. Try: "Create a new Rust project structure"      │
└─────────────────────────────────────────────────────┘

Grok (grok-4-1-fast-reasoning) [demo | 100% context left | 0 messages] >
```

## 🤖 Automatic File Operations

**NEW!** Grok CLI now supports automatic file and directory creation during chat! Simply ask naturally and Grok will execute the operations for you.

### Available Tools
- **write_file** - Create or overwrite files with content
- **read_file** - Read file contents
- **replace** - Find and replace text in files
- **list_directory** - List directory contents
- **glob_search** - Find files matching patterns
- **save_memory** - Save facts to long-term memory
- **run_shell_command** - Execute shell commands (cargo, git, etc.)

### Example Usage
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a

```bash
# Install / build
git clone https://github.com/microtech/grok-cli
cd grok-cli
cargo build --release

# Start interactive session
grok

# One-shot query
grok chat "Explain Rust ownership"

# Save and resume sessions
> /save my-session
> /load my-session
```

See [Doc/SETUP.md](Doc/SETUP.md) and [Doc/INSTALL.md](Doc/INSTALL.md) for full installation and configuration instructions.

## 📦 Configuration

Project-local settings live in `.grok/config.toml` (overrides system defaults).

```toml
[api]
default_model = "grok-4-1-fast-reasoning"
default_temperature = 0.7

[acp]
max_tool_loop_iterations = 50
auto_compress = true
```

Full options: [Doc/CONFIGURATION.md](Doc/CONFIGURATION.md)

## 🛠️ Common Commands

| Command              | Description                              |
|----------------------|------------------------------------------|
| `/help`              | Show available commands                  |
| `/model <name>`      | Switch AI model                          |
| `/save <id>`         | Save current session                     |
| `/load <id>`         | Resume a saved session                   |
| `/goal <text>`       | Set an active goal for the session       |
| `/think off\|low\|high` | Control reasoning effort              |
| `/visualize`         | Show pipeline diagram                    |
| `/bayes show`        | Inspect Bayesian priors                  |

See [Doc/QUICK_REFERENCE.md](Doc/QUICK_REFERENCE.md) for the complete list.

## 🔧 Troubleshooting & Fixes

- Common issues and solutions: [Doc/TROUBLESHOOTING.md](Doc/TROUBLESHOOTING.md)
- Recent bug fixes: [Doc/FIXES.md](Doc/FIXES.md)
- Max tool loop iterations: [Doc/MAX_TOOL_LOOP_ITERATIONS.md](Doc/MAX_TOOL_LOOP_ITERATIONS.md)

## 📚 Documentation

All detailed guides live in the `Doc/` folder:

- [Doc/SETUP.md](Doc/SETUP.md) — Detailed setup & installation
- [Doc/CONFIGURATION.md](Doc/CONFIGURATION.md) — Full configuration reference
- [Doc/CONTRIBUTING.md](Doc/CONTRIBUTING.md) — Development & contribution guide
- [Doc/TROUBLESHOOTING.md](Doc/TROUBLESHOOTING.md) — Comprehensive troubleshooting
- [Doc/TESTING_TOOLS.md](Doc/TESTING_TOOLS.md) — Testing tools & workflows
- [Doc/FIXES.md](Doc/FIXES.md) — Known issues & resolutions
- [Doc/QUICK_REFERENCE.md](Doc/QUICK_REFERENCE.md) — Command cheat sheet
- [Doc/SECURITY.md](Doc/SECURITY.md) — Security model & external access
- [Doc/HOOKS_AND_EXTENSIONS.md](Doc/HOOKS_AND_EXTENSIONS.md) — Extension system
- Full changelog history: [Doc/CHANGELOG_FULL.md](Doc/CHANGELOG_FULL.md)

## 🤝 Contributing

See [Doc/CONTRIBUTING.md](Doc/CONTRIBUTING.md).

## 📄 License

MIT — see [LICENSE](LICENSE).

---

**Made with ❤️ for the Rust and AI community**
