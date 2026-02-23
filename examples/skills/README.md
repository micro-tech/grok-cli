# Example Skills for Grok CLI

This directory contains example skills that demonstrate the Agent Skills format and can be used with Grok CLI.

## What are Skills?

Skills are modular instruction sets that enhance the AI agent's capabilities in specific domains. Each skill is a directory containing a `SKILL.md` file with YAML frontmatter and markdown instructions.

**New in v0.1.41**: Skills can now be created dynamically using the **Skill Builder** and activated immediately without restarting your session!

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

### 1. skill-builder (NEW!)

**The Skill Builder** is a meta-skill that creates new skills dynamically from specifications or natural language descriptions.

**Capabilities:**
- Create skills from natural language descriptions
- Generate skills from YAML/JSON specifications
- Interactive step-by-step skill building
- Clone and extend existing skills
- Immediate activation without restart
- Security validation and tool permission management

**How to use:**
```
> /activate skill-builder
> Create a skill for Python debugging

[Skill Builder creates and activates the skill immediately]

> /skills
Available Skills:
  [✓ ACTIVE] skill-builder - Dynamic skill creation system
  [✓ ACTIVE] python-debugger - Python debugging expert (just created!)
```

**See also:**
- [Skill Builder Documentation](./skill-builder/SKILL.md)
- [Skill Builder Examples](./skill-builder-examples.md)
- [Skill Specification Format](./SKILL_SPEC.md)

### 2. rust-expert

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

### 3. cli-design

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

### Method 1: Dynamic Creation with Skill Builder (Recommended)

The fastest way to create skills is using the Skill Builder in interactive mode:

```
grok interactive

> /activate skill-builder
> Create a skill for [your domain/task]

[Skill is created, validated, and activated immediately!]
```

**Examples:**
```
"Create a skill for Docker troubleshooting"
"Create a skill for TypeScript React debugging with read-only tools"
"Help me build a skill for SQL query optimization"
"Create a skill like rust-expert but for Go"
```

The Skill Builder will:
1. Parse your requirements
2. Generate a complete SKILL.md file
3. Validate for security issues
4. Save to `~/.grok/skills/<skill-name>/`
5. Activate immediately in your session

### Method 2: CLI Command

Use the CLI to create a template:

```bash
grok skills new my-skill
```

This creates a template skill directory with a `SKILL.md` file. Edit it to add your instructions.

### Method 3: Manual Creation

Create a directory structure manually following the skill specification format.

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

## Dynamic Skill Creation Features

### On-the-Fly Creation
Skills created via Skill Builder are immediately available in your current session without restart.

### Security Validation
All skills are automatically validated for:
- Suspicious command patterns
- Excessive tool permissions
- Potential security risks
- Best practice compliance

### Tool Permissions
Skills can specify which tools they're allowed to use:
```yaml
allowed-tools:
  - read_file
  - grep
  - list_directory
```

### Skill Specifications
See [SKILL_SPEC.md](./SKILL_SPEC.md) for the complete specification format including:
- Required and optional fields
- Security configurations
- Dependency management
- Auto-activation triggers
- Configuration options

## Best Practices

1. **Be Specific**: Include concrete examples and code snippets
2. **Stay Focused**: One skill per domain or task type
3. **Include Keywords**: Help the AI identify when to suggest the skill
4. **Keep It Concise**: Aim for under 500 lines in SKILL.md
5. **Organize with Sections**: Use clear headers and structure
6. **Test Thoroughly**: Verify the skill provides helpful guidance
7. **Use Skill Builder**: Let it handle formatting and validation
8. **Start with Read-Only Tools**: Only add write permissions when necessary

## Additional Directories

Skills can include optional directories:

- `scripts/`: Executable scripts the AI can reference or run
- `references/`: Additional documentation loaded on demand
- `assets/`: Templates, diagrams, or data files

## Quick Start Guide

### Creating Your First Dynamic Skill

1. **Start interactive mode:**
   ```bash
   grok interactive
   ```

2. **Activate Skill Builder:**
   ```
   > /activate skill-builder
   ```

3. **Create a skill:**
   ```
   > Create a skill for JavaScript testing with Jest that can read files
   ```

4. **Use your new skill immediately:**
   ```
   > How do I test async functions in Jest?
   ```

That's it! No restart needed, no manual file editing required.

## Contributing Skills

If you create useful skills, consider sharing them! You can:

1. Add them to this examples directory via pull request
2. Share them in the community
3. Publish them to the Agent Skills repository (agentskills.io)

## Resources

- [Skill Builder Guide](./skill-builder/SKILL.md) - Dynamic skill creation
- [Skill Builder Examples](./skill-builder-examples.md) - Real-world examples
- [Skill Specification](./SKILL_SPEC.md) - Complete format specification
- [Agent Skills Specification](https://agentskills.io/specification)
- [Grok CLI Skills Documentation](../../Doc/SKILL_SPECIFICATION.md)
- [Agent Skills Home](https://agentskills.io)

## Troubleshooting

### Skill Not Activating
- Verify the skill exists: `/skills`
- Check skill name spelling
- Review security validation results

### Skill Builder Not Working
- Ensure you're in interactive mode
- Activate skill-builder first: `/activate skill-builder`
- Check you have write permissions to `~/.grok/skills/`

### Tool Permissions Issues
- Skills are restricted to specified tools for security
- Use read-only tools by default (read_file, grep, list_directory)
- Request write permissions only when necessary

## License

These example skills are provided under the MIT License. Feel free to use, modify, and share them.