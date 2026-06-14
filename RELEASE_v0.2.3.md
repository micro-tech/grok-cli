# Grok CLI v0.2.3 — Commit Message Generator

**Release Date:** 2025-01-15  
**Previous:** v0.2.2

---

## ✨ Highlights

### `/commit` Slash Command + `generate_commit_message` Tool (Task 161)

Grok CLI now helps you write high-quality commit messages directly from the terminal or inside Zed.

**New slash command:**
```bash
/commit                    # Generate Conventional Commits message from current git diff
/commit fix auth edge case  # Add extra instructions
```

**New tool for the agent:**
- `generate_commit_message` — the AI can call this when it needs to propose a commit message during a workflow.

**Key features:**
- Uses `git diff --cached` first, falls back to `git diff` if nothing is staged
- Follows **Conventional Commits** by default (`<type>(<scope>): <description>`)
- Respects the new `acp.commit_message_instructions` config field
- Works with **Session DNA** and active goals for context-aware messages
- Subject line kept under 72 characters

**Example output:**
```
feat(auth): add JWT refresh token rotation

- Implement secure refresh token rotation with HttpOnly cookies
- Add rate limiting on refresh endpoint
- Update tests for new rotation logic

Closes #123
```

---

## 📦 Other Changes

- Updated version to **0.2.3** across `Cargo.toml`, `package.json`, and documentation
- Full documentation added to:
  - README.md
  - CHANGELOG.md
  - Doc/CONFIGURATION.md
  - Doc/QUICK_REFERENCE.md

---

## 🔧 Configuration

```toml
[acp]
commit_message_instructions = "Use Conventional Commits with scope and breaking-change footer when appropriate."
```

---

## 📥 Installation / Upgrade

```bash
# From source
git clone https://github.com/microtech/grok-cli
cd grok-cli
cargo build --release

# Or via cargo (once published)
cargo install grok-cli --version 0.2.3
```

---

## 🙏 Thanks

Special thanks to everyone who contributed to making commit message generation first-class in Grok CLI!

---

**Full Changelog:** https://github.com/microtech/grok-cli/blob/main/CHANGELOG.md
