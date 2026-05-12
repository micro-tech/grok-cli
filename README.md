# Grok CLI

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/microtech/grok-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

A powerful command-line interface for interacting with Grok AI via X API, featuring a beautiful interactive experience inspired by Gemini CLI.

> **Latest (v0.2.1)**: ACP connection-layer rewrite (Agent::builder + ByteStreams), session persistence & fork, per-iteration context trimming, slash-command lock fixes, `text`→`content` schema fix for Zed, ThinkingMode "on" alias, TGS-RAG engine (Text-Graph Synergy Retrieval), and quieter knowledge loader.

## ✨ Features

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
- **Zed Editor Integration** — Full Agent Client Protocol (ACP) support with session resume/fork
- **Thinking Modes** — `/think off|low|high` for controllable reasoning effort
- **TGS-RAG Engine** — Text-Graph Synergy Retrieval: hybrid BM25 + embeddings + graph-aware code context (tree-sitter + syn)
- **Code Intelligence** — Explain, review, generate, and refactor across any language
- **Starlink Optimizations** — Smart retries and timeout handling for satellite connections

See [Doc/QUICK_REFERENCE.md](Doc/QUICK_REFERENCE.md) for the full command list and [Doc/FEATURES.md](Doc/FEATURES.md) (coming soon) for details.

## 🚀 Quick Start

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
