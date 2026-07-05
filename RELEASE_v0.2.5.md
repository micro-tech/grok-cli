# Grok CLI v0.2.5 — Session Rules + Slash Command Registry Fix

**Release Date:** 2026-07-04
**Previous:** v0.2.3

---

## ✨ Highlights

### Session-Only Rules — `/rule` Command

Grok CLI now lets you set temporary per-session rules that are silently injected into every message you send, so you never have to repeat yourself.

**Commands:**
```
/rule add Always use anyhow::Result for error handling
/rule add Never suggest .unwrap() in production code
/rule list          ← show all active rules with IDs
/rule remove 2      ← remove rule #2
/rule clear         ← wipe all rules
/rules              ← alias for /rule list
```

**How it works:** each rule is appended to your message before it reaches the model, meaning the model always sees them — even if you typed them once at the start of the session.

Rules are session-scoped (not persisted to disk). For permanent rules, use a context file (`.grok/context.md`, `.zed/rules`, etc.).

---

### `/rule` Now Visible in Command Picker

The `/rule` command was implemented but missing from `get_available_commands()` — the function that advertises commands to ACP clients like Zed. This meant it never appeared in the `/` autocomplete menu. Now fixed.

---

## 📦 Changes

| Area | Change |
|------|--------|
| `src/acp/slash_commands.rs` | Added `RuleAdd`, `RuleRemove`, `RuleList`, `RuleClear` variants to `SlashCommand` match arms; new `BuiltinResult` variants; `/rule` registered in `get_available_commands()` |
| `src/acp/mod.rs` | `SessionData` gains `session_rules: SessionRules` field; rules injected in `refine_prompt`; four new agent methods: `add_session_rule`, `remove_session_rule`, `list_session_rules`, `clear_session_rules` |
| `src/cli/commands/acp.rs` | `handle_builtin_result` handles the four new `BuiltinResult` variants |
| `src/cli/commands/chat.rs` | `handle_interactive_command` handles the four new `BuiltinResult` variants |
| `Doc/QUICK_REFERENCE.md` | Added **Session Rules** section under Interactive Mode Commands |
| `Cargo.toml` | Version bumped to `0.2.5` (typo in pre-release suffix also fixed) |
| `package.json` | Version bumped to `0.2.5` |

---

## 🔧 Storage & Layout

No changes to storage layout. Session rules are in-memory only and reset on session close.

---

## 📥 Installation / Upgrade

```bash
# From source (Windows)
git clone https://github.com/microtech/grok-cli
cd grok-cli
cargo build --release
cargo run --bin installer

# Or just build
cargo build --release
```

---

## 🙏 Thanks

Buy me a coffee: https://buymeacoffee.com/micro.tech

---

**Full Changelog:** https://github.com/microtech/grok-cli/blob/main/CHANGELOG.md
