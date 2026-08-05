# Session DNA

Session DNA is a lightweight **persistent personality and behavior configuration** system.

It allows you to define how Grok should behave in a specific project by storing preferences in a `session_dna.json` file. These preferences are loaded at the start of every session and injected into the system prompt.

## Why Use Session DNA?

Instead of repeating instructions like "be concise", "use a friendly tone", or "prefer functional programming" every time, you can define them once and have them automatically applied.

This is especially useful for:
- Long-running projects with a consistent style
- Team projects where everyone wants the same AI behavior
- Switching between different modes (e.g., "research mode" vs "production mode")

## The `session_dna.json` File (SEC-5: VCS-safe location)

Create the file **inside** the `.grok/` directory (VCS-safe location):

```json
{
  "tone": "friendly",
  "verbosity": "medium",
  "risk_tolerance": "medium",
  "coding_style": "functional",
  "tool_preferences": ["prefer_rust", "avoid_shell"]
}
```

**Path:** `.grok/session_dna.json` (inside your project)

> **Note:** A bare `session_dna.json` directly in the project root is **deliberately ignored** to prevent accidental commits of personal preferences to version control. Always use the `.grok/` subdirectory.

### Available Fields

| Field                | Type     | Description                                      | Example Values                     |
|----------------------|----------|--------------------------------------------------|------------------------------------|
| `tone`               | string   | Overall personality                              | `neutral`, `friendly`, `direct`, `professional` |
| `verbosity`          | string   | How much detail to provide                       | `low`, `medium`, `high`            |
| `risk_tolerance`     | string   | Willingness to take initiative                   | `low`, `medium`, `high`            |
| `coding_style`       | string   | Preferred coding approach                        | `standard`, `functional`, `object-oriented`, `minimal` |
| `tool_preferences`   | array    | Hints about tool usage                           | `["prefer_rust", "avoid_shell"]`   |

## How It Works

1. When a session starts, Grok CLI looks for `.grok/session_dna.json` (project-local under `.grok/`).
2. Falls back to `~/.grok-cli/session_dna.json` (global user defaults).
3. If found, it is parsed and stored.
4. The values are injected into the system prompt at the beginning of the conversation.
5. If the file is missing or invalid, sensible defaults are used.

**Search order (SEC-5):**
- `./.grok/session_dna.json`
- `~/.grok-cli/session_dna.json`
- Built-in defaults

Example of what gets injected:

```
Tone: friendly
Verbosity: medium
Risk tolerance: medium
Coding style: functional
```

## Loading Behavior

- The file is loaded **once per session** at startup.
- Changes to `.grok/session_dna.json` require restarting the session to take effect.
- You can have different `.grok/session_dna.json` files per project (VCS-safe).
- Bare `session_dna.json` in the project root is ignored.

## Related Features

- **Hierarchical Configuration** — Complements `.grok/config.toml`
- **Knowledge Injection** — Works alongside `knowledge/` files
- **Context Archiving** — Long-term memory is still preserved

## See Also

- [Configuration Guide](CONFIGURATION.md)
- [Data Flow Map](dataflow_map.md)
- [Quick Reference](QUICK_REFERENCE.md)
