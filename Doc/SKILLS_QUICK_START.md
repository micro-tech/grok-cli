# Skills Quick Start Guide

## What are Skills?

Skills are modular instruction sets that give Grok expertise in specific domains. Think of them as specialized knowledge modules you can activate on-demand during your conversations.

## Quick Start

### 1. List Available Skills

```bash
# In interactive mode
grok interactive

> /skills
```

You'll see something like:

```
Available Skills:

  [○ inactive] rust-expert - Expert guidance for Rust development...
  [○ inactive] cli-design - Expert guidance for designing intuitive CLIs...

  Use /activate <skill-name> to enable a skill
```

### 2. Activate a Skill

```bash
> /activate rust-expert
✓ Skill 'rust-expert' activated
  The skill's instructions will be included in the next message
```

### 3. Use the Skill

Now when you ask questions, Grok will use the skill's expertise:

```bash
> How should I handle errors in my Rust application?
```

Grok will provide detailed, expert-level guidance based on the rust-expert skill instructions.

### 4. Deactivate When Done

```bash
> /deactivate rust-expert
✓ Skill 'rust-expert' deactivated
```

## Available Commands

| Command | Description |
|---------|-------------|
| `/skills` | List all available skills with their status |
| `/activate <name>` | Activate a skill for this session |
| `/deactivate <name>` | Deactivate an active skill |

## Why Use Skills?

### Token Efficiency

- **Without Skills**: All instructions loaded = more tokens used
- **With Progressive Skills**: Only active skills loaded = fewer tokens

This means:
- Faster responses
- Lower API costs
- More focused conversations

### Better Context Control

You decide what expertise is active. No more irrelevant information cluttering the context.

### Session Persistence

Active skills are saved with your session:

```bash
> /activate rust-expert
> /save my-project
✓ Session saved to ...

# Later...
> /load my-project
✓ Session 'my-project' loaded
  Model: grok-3
  
> /skills
Available Skills:
  [✓ ACTIVE] rust-expert - Expert guidance for Rust development...
```

## Creating Your Own Skills

### 1. Use the CLI

```bash
grok skills new my-skill
```

This creates: `~/.grok/skills/my-skill/SKILL.md`

### 2. Edit the Skill File

```yaml
---
name: my-skill
description: What your skill does and when to use it
license: MIT
metadata:
  author: Your Name
  version: "1.0"
---

# My Skill Instructions

Write your instructions here in Markdown format.

## When to Use

Explain when this skill should be activated.

## Guidelines

- Provide clear, actionable guidance
- Include examples
- Be specific
```

### 3. Test It

```bash
grok interactive

> /skills
Available Skills:
  [○ inactive] my-skill - What your skill does...

> /activate my-skill
✓ Skill 'my-skill' activated
```

## Best Practices

### ✅ DO

- **Activate skills when you need them**: `I'm working on a Rust project` → `/activate rust-expert`
- **Deactivate when switching topics**: Changing from Rust to Python → `/deactivate rust-expert`
- **Check active skills**: Use `/skills` to see what's currently active
- **Save sessions with skills**: Your skill state is preserved

### ❌ DON'T

- **Don't activate all skills at once**: More skills = more tokens = slower/costlier
- **Don't forget to deactivate**: Clean up your context when you're done
- **Don't create overly broad skills**: Keep skills focused on specific domains

## Example Workflows

### Rust Development Session

```bash
grok interactive

> /activate rust-expert
> I'm getting a borrow checker error...
[Get expert Rust guidance]

> How do I properly structure error handling?
[Get detailed error handling patterns]

> /deactivate rust-expert
```

### CLI Tool Design

```bash
> /activate cli-design
> I'm building a CLI tool. What's the best way to structure commands?
[Get CLI design best practices]

> How should I format error messages?
[Get error message guidelines]

> /deactivate cli-design
```

### Multiple Skills

```bash
# Working on a Rust CLI tool
> /activate rust-expert
> /activate cli-design

> How do I build a CLI tool in Rust with good UX?
[Get combined expertise from both skills]

> /deactivate rust-expert
> /deactivate cli-design
```

## Example Skills Included

### rust-expert

**Covers:**
- Best practices and idioms
- Error handling (Result, Option, ?)
- Ownership and borrowing
- Design patterns
- Concurrency and async/await
- Testing strategies
- Performance optimization

**Activate when:** Writing Rust code, debugging ownership issues, designing Rust APIs

### cli-design

**Covers:**
- Command structure and naming
- Argument parsing
- Output formatting (colors, tables, progress)
- Error messages that help users
- Interactive features (prompts, confirmations)
- Configuration management
- Platform considerations

**Activate when:** Building CLI tools, improving UX, designing command interfaces

## Skills Directory Structure

```
~/.grok/skills/          # Your global skills directory
├── rust-expert/
│   └── SKILL.md
├── cli-design/
│   └── SKILL.md
└── my-skill/
    ├── SKILL.md         # Required
    ├── scripts/         # Optional: executable scripts
    ├── references/      # Optional: additional docs
    └── assets/          # Optional: templates, data
```

## Skill File Format

```markdown
---
name: skill-name           # Required: lowercase, hyphens only
description: Clear desc    # Required: max 1024 chars
license: MIT              # Optional
compatibility: notes      # Optional
metadata:                 # Optional
  author: name
  version: "1.0"
allowed-tools: Read Write # Optional: pre-approved tools
---

# Skill Instructions

Your instructions in Markdown format.
```

## Tips & Tricks

### 1. Check Session Info

```bash
> /status
Session: grok-20250115-1430
Model: grok-3
Skills: 2 available, 1 active
  Active: rust-expert
```

### 2. Quick Skill Toggle

```bash
# Activate for one question, then deactivate
> /activate rust-expert
> How do I use lifetimes?
> /deactivate rust-expert
```

### 3. Skill Sets for Projects

```bash
# Save a session with your skill set
> /activate rust-expert
> /activate cli-design
> /save rust-cli-project

# Load it later with skills already active
> /load rust-cli-project
```

### 4. List Without Activating

```bash
# Just browse available skills
> /skills

# Read full skill details (from command line)
$ grok skills show rust-expert
```

## Troubleshooting

### "Skill not found"

```bash
> /activate my-skill
✗ Skill 'my-skill' not found
  Use /skills to see available skills
```

**Solution:** Check skill name spelling or create it with `grok skills new my-skill`

### "Skill already active"

```bash
> /activate rust-expert
ℹ Skill 'rust-expert' is already active
```

**Solution:** Skill is already loaded, continue using it

### Skills Not Showing

**Solution:** Check your skills directory exists:
- Windows: `%USERPROFILE%\.grok\skills\`
- macOS/Linux: `~/.grok/skills/`

## Next Steps

1. **Try the examples**: Copy from `examples/skills/` to `~/.grok/skills/`
2. **Create your first skill**: `grok skills new my-first-skill`
3. **Explore the spec**: See `Doc/SKILL_SPECIFICATION.md` for details
4. **Share skills**: Contribute useful skills back to the community

### Validating Skills for Security

Before activating a skill, especially one from an external source, it's important to check for potential security issues. Use the `validate` command to analyze a skill:

```bash
grok skills validate my-skill
```

This will scan the skill for dangerous patterns or instructions and provide a safety rating (SAFE, WARNING, SUSPICIOUS, or DANGEROUS) along with specific issues if any are found.

### Using Skills via CLI (Non-Interactive)

In addition to interactive mode, you can manage skills directly from the command line:

```bash
# List all available skills
grok skills list

# Show details of a specific skill
grok skills show rust-expert

# Create a new skill template
grok skills new my-custom-skill
```

## Resources

- [Full Skills Documentation](SKILL_SPECIFICATION.md)
- [Agent Skills Specification](https://agentskills.io/specification)
- [Example Skills](../examples/skills/)
- [Skills System Architecture](../CHANGELOG.md#skills-system-enhancements)

## Summary

```bash
# The complete workflow
grok interactive          # Start interactive mode
> /skills                # See what's available
> /activate skill-name   # Turn on expertise
> [ask questions]        # Get expert guidance
> /deactivate skill-name # Turn off when done
> /save my-session       # Save with skill state
```

**Remember:** Skills give you control over Grok's expertise. Activate what you need, when you need it!
