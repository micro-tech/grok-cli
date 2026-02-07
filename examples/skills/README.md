# Example Skills for Grok CLI

This directory contains example skills that demonstrate the Agent Skills format and can be used with Grok CLI.

## What are Skills?

Skills are modular instruction sets that enhance the AI agent's capabilities in specific domains. Each skill is a directory containing a `SKILL.md` file with YAML frontmatter and markdown instructions.

## Using These Skills

### Option 1: Copy to Your Skills Directory

Copy the skill directories to your global skills directory:

```bash
# On Windows
xcopy examples\skills\* %USERPROFILE%\.grok\skills\ /E /I

# On macOS/Linux
cp -r examples/skills/* ~/.grok/skills/
```

### Option 2: Use the `grok skills` Command

Navigate to a skill directory and use the CLI:

```bash
cd examples/skills
grok skills list
```

### Option 3: Activate in Interactive Mode

In interactive mode, you can activate skills on-demand:

```
grok interactive

> /skills
Available Skills:
  [○ inactive] rust-expert - Expert guidance for Rust development...
  [○ inactive] cli-design - Expert guidance for designing intuitive CLIs...

> /activate rust-expert
✓ Skill 'rust-expert' activated

> /skills
Available Skills:
  [✓ ACTIVE] rust-expert - Expert guidance for Rust development...
  [○ inactive] cli-design - Expert guidance for designing intuitive CLIs...
```

## Available Example Skills

### 1. rust-expert

Expert guidance for Rust development including:
- Best practices and idiomatic patterns
- Error handling with Result and Option
- Ownership and borrowing patterns
- Common design patterns (Builder, Newtype, RAII)
- Memory management strategies
- Concurrency and async/await
- Testing approaches
- Performance optimization tips

**When to use**: Activate when working on Rust projects or debugging Rust-specific issues.

### 2. cli-design

Expert guidance for designing command-line interfaces:
- Command structure and naming conventions
- Argument parsing best practices
- Output formatting (colors, tables, progress bars)
- Error message design
- Interactive features (prompts, confirmations)
- Configuration management
- Platform-specific considerations
- Performance and security

**When to use**: Activate when building or improving CLI tools.

## Creating Your Own Skills

Use the CLI to create new skills:

```bash
grok skills new my-skill
```

This creates a template skill directory with a `SKILL.md` file. Edit it to add your instructions.

### Skill Format

Each skill must have a `SKILL.md` file with this structure:

```markdown
---
name: skill-name
description: What the skill does and when to use it
license: MIT
metadata:
  author: your-name
  version: "1.0"
---

# Skill Instructions

Write your skill instructions here in Markdown format.
```

### Required Fields

- `name`: Lowercase, hyphen-separated (must match directory name)
- `description`: Clear description including when to use the skill (max 1024 chars)

### Optional Fields

- `license`: License information
- `compatibility`: System requirements or compatibility notes
- `metadata`: Additional key-value metadata
- `allowed-tools`: Space-delimited list of pre-approved tools

## Progressive Disclosure

Skills use progressive disclosure to manage context efficiently:

1. **Metadata** (~100 tokens): Name and description loaded at startup for all skills
2. **Instructions** (~500-5000 tokens): Full `SKILL.md` loaded only when skill is activated
3. **Resources** (as needed): Additional files in `scripts/`, `references/`, or `assets/` loaded on demand

This keeps context size manageable while providing deep expertise when needed.

## Best Practices

1. **Be Specific**: Include concrete examples and code snippets
2. **Stay Focused**: One skill per domain or task type
3. **Include Keywords**: Help the AI identify when to suggest the skill
4. **Keep It Concise**: Aim for under 500 lines in SKILL.md
5. **Organize with Sections**: Use clear headers and structure
6. **Test Thoroughly**: Verify the skill provides helpful guidance

## Additional Directories

Skills can include optional directories:

- `scripts/`: Executable scripts the AI can reference or run
- `references/`: Additional documentation loaded on demand
- `assets/`: Templates, diagrams, or data files

## Contributing Skills

If you create useful skills, consider sharing them! You can:

1. Add them to this examples directory via pull request
2. Share them in the community
3. Publish them to the Agent Skills repository (agentskills.io)

## Resources

- [Agent Skills Specification](https://agentskills.io/specification)
- [Grok CLI Skills Documentation](../../Doc/SKILL_SPECIFICATION.md)
- [Agent Skills Home](https://agentskills.io)

## License

These example skills are provided under the MIT License. Feel free to use, modify, and share them.