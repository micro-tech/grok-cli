# Self-Updating Skills Catalog

**Task 273** introduced a major reliability improvement: Grok CLI now maintains a **live, self-updating catalog** that the model reads on every single turn.

The catalog lives at:

```
.grok/SKILLS_HOOKS_OPTIMIZATION.md
```

(or `~/.grok-cli/SKILLS_HOOKS_OPTIMIZATION.md` as fallback)

This file is **injected directly into the system prompt** in interactive mode, so the model always has up-to-date knowledge of:

- Every available skill
- How and when to use them
- The hooks system
- Performance best practices

---

## Why This Matters

Previously, when you created a new skill (even using the powerful `skill-builder`), the model often had no idea it existed until you manually described it or restarted.

Now:

- Create a skill → catalog is automatically refreshed
- The model immediately sees the new skill with proper ranking, triggers, and usage instructions
- No manual prompting, no context dumping, no restarts required

This is a foundational step toward **truly dynamic, user-extensible AI behavior**.

---

## File Structure

The catalog uses clear HTML comment markers so both humans and the model can reliably parse it:

```markdown
<!-- SKILLS CATALOG START -->
## SKILLS CATALOG

... ranked list of skills ...
<!-- SKILLS CATALOG END -->

<!-- HOOKS START -->
## HOOKS

... documentation ...
<!-- HOOKS END -->

<!-- OPTIMIZATION HEURISTICS START -->
## OPTIMIZATION HEURISTICS

... performance guidance ...
<!-- OPTIMIZATION HEURISTICS END -->
```

### 1. SKILLS CATALOG (Ranked)

Skills are listed in **descending arbitration score order** (highest priority first).

Each entry includes:

- Name and description
- Version, author, tags
- Arbitration score (`[score:85]`)
- Dependencies
- **Auto-activation hints** (keywords, regex patterns, file extensions, minimum confidence)
- **"When to prefer / activate"** guidance with exact syntax:
  - `execute_skill "skill-name"`
  - `/activate skill-name`

### 2. HOOKS

Authoritative documentation of the hooks system:

- `before_tool` and `after_tool` behavior
- Common use cases (logging, validation, security, transformation)
- Strong guidance: *"Rely on hooks instead of re-implementing the same logic in your responses."*

### 3. OPTIMIZATION HEURISTICS

Concrete performance patterns the model is instructed to follow:

- Prefer `execute_skill` over long primitive tool loops
- Use cheap `Arc<str>` clones for conversation history
- Prompt construction best practices (`String::with_capacity`, static caching)
- Awareness of `GROK_PERF=1`
- Lock minimization patterns
- Context efficiency tips

---

## How the Catalog Is Kept Up to Date

### Automatic Updates

| Action                              | Does it refresh the catalog? | Notes |
|-------------------------------------|------------------------------|-------|
| `grok skills new <name>`            | Yes                          | Automatic |
| Using the `skill-builder` skill     | Yes                          | The meta-skill explicitly runs `grok skills generate-catalog` via `run_shell_command` |
| Creating skills programmatically    | Recommended                  | Call `grok skills generate-catalog` afterward |
| Manual edit of `skill.json`         | No (run generate manually)   | — |

### Manual Refresh

```bash
grok skills generate-catalog
```

This command regenerates the catalog from the current `SkillRegistry`.

---

## Viewing the Catalog

The catalog is a regular Markdown file. You can read it anytime:

```bash
# Project-local (preferred)
cat .grok/SKILLS_HOOKS_OPTIMIZATION.md

# Or global fallback
cat ~/.grok-cli/SKILLS_HOOKS_OPTIMIZATION.md
```

Because it is human-readable, you can:

- Review what the model currently "knows"
- Debug why a skill isn't being activated
- Understand the arbitration ranking
- See the exact guidance the model is receiving

---

## How It Affects the Model

Every time you send a message in interactive mode, Grok CLI does this:

1. Builds the normal system prompt
2. Appends active skills context (via `SkillRegistry::ranked_context`)
3. **Appends the full contents of `SKILLS_HOOKS_OPTIMIZATION.md`** (if present)
4. Sends the combined prompt

This means the model has **fresh, ranked, authoritative knowledge** of your entire skills ecosystem on every turn.

---

## Best Practices for Skill Authors

When writing skills (especially via `skill-builder`), keep these things in mind because they directly affect the catalog:

- Write a clear, concise `description` — it appears in the catalog.
- Use meaningful **tags**.
- Set a sensible `arbitration_score` (0–100). Higher = more influence when multiple skills are active.
- Include good **auto-activation hints** in `SKILL.md` frontmatter (keywords, patterns, extensions).
- After creation, the catalog will automatically highlight "When to use this skill".

The `skill-builder` skill is specifically instructed to:
- Create good metadata
- Run `grok skills generate-catalog` immediately after writing the files
- Tell the user the catalog was refreshed

---

## Commands Reference

| Command                              | Purpose |
|--------------------------------------|--------|
| `grok skills generate-catalog`       | Manually regenerate the catalog |
| `grok skills new <name>`             | Create skill + auto-refresh catalog |
| `grok skills list`                   | See skills (now consistent with what the model sees) |
| `/skills` (in interactive)           | View skills + activation status |
| `/activate <name>`                   | Activate a skill for the current session |

---

## Technical Details

- **Generator**: `src/skills/catalog.rs`
- **CLI handler**: `src/cli/commands/skills.rs`
- **Injection point**: `src/display/interactive.rs` (in `send_to_grok`)
- Uses the modern `SkillRegistry` (manifest-aware) so `arbitration_score`, `enabled`, tags, and dependencies are respected.
- The catalog is **optional** — if the file doesn't exist, nothing is injected.

---

## Future Directions

This system is designed to grow. Possible future enhancements:

- Per-project catalog variants
- Catalog diffing / change notifications
- Richer auto-activation metadata (confidence tuning, project-type detectors)
- Model-driven suggestions for catalog improvements

---

## Summary

The Self-Updating Skills Catalog turns skill creation from a "hope the model notices" experience into a **reliable, automatic, and transparent** process.

Create a skill → catalog refreshes → model knows about it on the next message.

**Human readable. Model authoritative. Automatically maintained.**

This is one of the most important usability improvements for the skills system to date.
