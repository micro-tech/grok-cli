# Skill Builder Quick Start Guide

Get up and running with dynamic skill creation in 5 minutes!

## What is the Skill Builder?

The Skill Builder is a meta-skill that creates new custom skills dynamically and activates them immediately in your current session - no restart required!

## Prerequisites

- Grok CLI v0.1.41 or higher
- Interactive mode access
- Write permissions to `~/.grok/skills/` directory

## Installation

The Skill Builder comes pre-installed in `examples/skills/skill-builder/`. Copy it to your skills directory:

**Windows:**
```powershell
xcopy examples\skills\skill-builder %USERPROFILE%\.grok\skills\skill-builder\ /E /I
```

**macOS/Linux:**
```bash
cp -r examples/skills/skill-builder ~/.grok/skills/
```

Or let Grok copy it for you:
```
grok interactive
> /activate skill-builder
[First time: Grok will ask if you want to install it]
```

## Your First Skill (30 seconds)

### 1. Start Interactive Mode
```bash
grok interactive
```

### 2. Activate Skill Builder
```
> /activate skill-builder
✓ Skill 'skill-builder' activated
```

### 3. Create a Skill
```
> Create a skill for Python debugging
```

### 4. Use Your New Skill
```
> How do I debug a Python TypeError?
```

That's it! Your skill is created, activated, and ready to use.

## Common Creation Patterns

### Pattern 1: Simple Request
```
> Create a skill for Docker troubleshooting
```
Skill Builder will ask clarifying questions and create the skill.

### Pattern 2: Detailed Request
```
> Create a skill for TypeScript React debugging that:
> - Helps with type errors
> - Understands React hooks
> - Can read TypeScript files
> - Uses only read_file and grep tools
```
Skill Builder creates exactly what you specify.

### Pattern 3: Interactive Building
```
> Help me build a custom skill

[Skill Builder guides you step-by-step]
```

### Pattern 4: Clone Existing
```
> Create a skill like rust-expert but for Go
```
Skill Builder clones and adapts the structure.

### Pattern 5: From Specification
```yaml
> Create skill from this spec:
> ---
> name: sql-optimizer
> description: SQL query optimization expert
> allowed-tools:
>   - read_file
>   - grep
> ---
```

## Essential Commands

### Activation
```
/activate skill-builder    # Start creating skills
```

### List Skills
```
/skills                    # See all available skills
```

### Check Status
```
> Show me the <skill-name> skill
> List active skills
```

### Deactivate
```
/deactivate skill-builder  # When done creating
/deactivate <skill-name>   # Deactivate specific skill
```

## Quick Examples

### Example 1: JavaScript Testing
```
> /activate skill-builder
> Create a skill for Jest testing that can read test files

✓ Skill 'jest-tester' created!
✓ Security: SAFE (read-only tools)
✓ Activated!

> How do I test async functions in Jest?
[Skill provides Jest-specific guidance]
```

### Example 2: API Design
```
> Create a skill for REST API design review with tools: read_file, grep

✓ Skill 'api-design-reviewer' created!
✓ Now active!

> Review this API endpoint structure
[Skill analyzes and provides feedback]
```

### Example 3: Database Expert
```
> I need help with PostgreSQL query optimization - create a skill

[Skill Builder asks clarifying questions]
- Focus on: query performance? schema design? both?
> Both

✓ Skill 'postgres-optimizer' created and activated!

> Why is my JOIN query slow?
[Skill provides PostgreSQL-specific optimization advice]
```

## Tips for Success

### 1. Be Specific
✅ Good: "Create a skill for Python async/await debugging"
❌ Bad: "Create a Python skill"

### 2. Specify Tools
```
"...that can read and analyze files"  → read_file, grep
"...read-only access"                 → read_file, list_directory, grep
"...with write access"                → read_file, write_file (review security!)
```

### 3. Start Simple
Create a basic skill first, then update it:
```
> Add GraphQL expertise to the api-design-reviewer skill
```

### 4. Use Read-Only Tools
For security, start with:
- `read_file` - Read files
- `list_directory` - List contents
- `grep` - Search patterns
- `find_path` - Find files

Only add write tools (`write_file`, `terminal`) when absolutely necessary.

### 5. Test Immediately
After creation:
```
> Test the [skill-name] skill with: [example question]
```

## Common Workflows

### Workflow 1: Development Project
```
1. Start interactive mode
2. /activate skill-builder
3. Create framework-specific skill (e.g., "Create a Next.js expert skill")
4. /deactivate skill-builder
5. Use your new skill for the project
```

### Workflow 2: Learning New Technology
```
1. /activate skill-builder
2. "Create a skill for [technology] with tutorials and examples"
3. Use skill as learning companion
```

### Workflow 3: Troubleshooting
```
1. /activate skill-builder
2. "Create a troubleshooting skill for [tool/framework]"
3. Ask diagnostic questions
```

## Security Best Practices

### Safe Tools (Recommended)
- `read_file` - Read files ✓
- `list_directory` - List directories ✓
- `grep` - Search patterns ✓
- `find_path` - Find files ✓

### Use with Caution
- `write_file` - Modifies files ⚠️
- `terminal` - Executes commands ⚠️
- `delete_path` - Deletes files 🛑

### Skill Builder Will Warn You
```
⚠ Security validation found potential issues:
- Requesting terminal access (high risk)

Recommended: Create with read-only tools
Proceed? (yes/no)
```

## Troubleshooting

### Problem: Skill Builder not found
**Solution:**
```bash
# Copy from examples
cp -r examples/skills/skill-builder ~/.grok/skills/
```

### Problem: Skill created but not active
**Solution:**
```
> /skills
> /activate <skill-name>
```

### Problem: "Permission denied" when creating skill
**Solution:**
```bash
# Check permissions
ls -la ~/.grok/skills/

# Create directory if missing
mkdir -p ~/.grok/skills/
```

### Problem: Skill doesn't behave as expected
**Solution:**
```
> Show me the <skill-name> skill
[Review the instructions]

> Update <skill-name> to [add clarification]
```

## Next Steps

### Learn More
- [Skill Builder Documentation](./skill-builder/SKILL.md) - Complete guide
- [Skill Builder Examples](./skill-builder-examples.md) - 20+ real examples
- [Skill Specification](./SKILL_SPEC.md) - Format reference

### Create Advanced Skills
- Skills with dependencies
- Skills with auto-activation triggers
- Skills that extend other skills
- Skills with custom configurations

### Share Your Skills
- Contribute to examples/skills/
- Share with the community
- Publish to Agent Skills repository

## Cheat Sheet

```bash
# Activate Skill Builder
/activate skill-builder

# Create skills
"Create a skill for [domain]"
"Create skill from spec: [yaml]"
"Help me build a skill"
"Clone [existing] for [new domain]"

# Manage skills
/skills                    # List all
/activate <name>           # Activate
/deactivate <name>         # Deactivate
"Show me the <name> skill" # View details

# Update skills
"Add [feature] to <name>"
"Update <name> to include [capability]"
```

## Success Checklist

- ✓ Skill Builder installed and activated
- ✓ Created first skill successfully
- ✓ Skill activated immediately (no restart)
- ✓ Tested skill with real question
- ✓ Understand tool permissions
- ✓ Know how to update skills

## Examples by Use Case

### Software Development
- `"Create a skill for React hooks"`
- `"Create a skill for Git workflow"`
- `"Create a skill for debugging Node.js"`

### DevOps
- `"Create a skill for Kubernetes troubleshooting"`
- `"Create a skill for CI/CD pipeline design"`
- `"Create a skill for AWS Lambda optimization"`

### Data & Databases
- `"Create a skill for SQL query tuning"`
- `"Create a skill for MongoDB schema design"`
- `"Create a skill for data pipeline debugging"`

### Testing
- `"Create a skill for API testing strategies"`
- `"Create a skill for unit test best practices"`
- `"Create a skill for E2E test automation"`

## Getting Help

1. **In-session help:**
   ```
   > /activate skill-builder
   > How do I create a skill for [your use case]?
   ```

2. **Documentation:**
   - Full guide: `examples/skills/skill-builder/SKILL.md`
   - Examples: `examples/skills/skill-builder-examples.md`
   - Spec: `examples/skills/SKILL_SPEC.md`

3. **Community:**
   - GitHub Issues: https://github.com/microtech/grok-cli/issues
   - Discussions: https://github.com/microtech/grok-cli/discussions

## Ready to Build?

```bash
grok interactive
```

```
> /activate skill-builder
> Create a skill for [your domain]
```

Start creating powerful, custom skills in seconds! 🚀