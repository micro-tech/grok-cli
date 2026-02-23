# Skill Specification Format (SKILL_SPEC)

## Overview

This document defines the standard format for creating Grok CLI skills. Skills can be created from specifications either manually or programmatically by the Skill Builder.

## Specification Format

A skill specification is a structured document that defines all aspects of a skill. It can be provided as YAML, JSON, or a structured natural language description.

## YAML Format (Recommended)

```yaml
# Required Fields
name: skill-name
description: Brief description of what this skill does
version: 1.0.0

# Optional Metadata
author: Your Name
license: MIT
created: 2025-02-15
tags:
  - category1
  - category2
compatibility:
  - grok-cli >= 0.1.41

# Skill Behavior
instructions: |
  Detailed instructions for Grok on how to behave when this skill is active.
  
  ## Context
  Provide context about the skill's domain expertise.
  
  ## Capabilities
  - List specific capabilities
  - Be explicit about what the skill can help with
  
  ## Response Patterns
  When responding, follow these patterns:
  - Pattern 1
  - Pattern 2
  
  ## Examples
  Example 1: ...
  Example 2: ...

# Security & Permissions
allowed_tools:
  - read_file
  - list_directory
  - grep
restricted_tools:
  - write_file
  - terminal
  - delete_path

# Activation
auto_activate: false
activation_triggers:
  - keyword: python
    context: code
  - keyword: debug
    context: any

# Dependencies
requires:
  - skill: rust-expert
    optional: true
conflicts_with:
  - skill: java-expert

# Configuration
config:
  max_context_length: 2000
  include_examples: true
  verbosity: normal
```

## JSON Format

```json
{
  "name": "skill-name",
  "description": "Brief description",
  "version": "1.0.0",
  "author": "Your Name",
  "license": "MIT",
  "instructions": "Detailed instructions...",
  "allowed_tools": ["read_file", "list_directory"],
  "tags": ["category1", "category2"]
}
```

## Natural Language Format

Skills can also be created from natural language descriptions:

```
Create a skill for Python debugging that:
- Helps identify and fix Python errors
- Suggests best practices for debugging
- Provides step-by-step debugging strategies
- Can read Python files and analyze stack traces
- Uses tools: read_file, grep, list_directory
- Tags: python, debugging, troubleshooting
```

## Field Definitions

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique identifier (lowercase, hyphens allowed) |
| `description` | string | Brief 1-2 sentence description |
| `instructions` | string | Detailed instructions for Grok's behavior |

### Optional Metadata Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Semantic version (e.g., "1.0.0") |
| `author` | string | Skill creator's name |
| `license` | string | License identifier (MIT, Apache-2.0, etc.) |
| `created` | date | Creation date (YYYY-MM-DD) |
| `updated` | date | Last update date |
| `tags` | array | Categorization tags |
| `compatibility` | array | Required grok-cli versions |

### Security Fields

| Field | Type | Description |
|-------|------|-------------|
| `allowed_tools` | array | Whitelist of tools this skill can use |
| `restricted_tools` | array | Tools this skill should not use |
| `security_level` | string | "safe", "trusted", "review" |

### Activation Fields

| Field | Type | Description |
|-------|------|-------------|
| `auto_activate` | boolean | Auto-activate in certain contexts |
| `activation_triggers` | array | Conditions for auto-activation |
| `deactivation_triggers` | array | Conditions for auto-deactivation |

### Dependency Fields

| Field | Type | Description |
|-------|------|-------------|
| `requires` | array | Required skills or dependencies |
| `conflicts_with` | array | Skills that conflict with this one |
| `extends` | string | Base skill to extend |

### Configuration Fields

| Field | Type | Description |
|-------|------|-------------|
| `config` | object | Custom configuration options |
| `max_context_length` | number | Max context to include |
| `include_examples` | boolean | Include examples in context |
| `verbosity` | string | "minimal", "normal", "verbose" |

## Instructions Format

The `instructions` field should follow this structure:

```markdown
# Overview
Brief overview of the skill's purpose and expertise.

## Context
Provide domain-specific context and knowledge.

## Capabilities
- Capability 1: Description
- Capability 2: Description
- Capability 3: Description

## Response Patterns
When responding with this skill active:
1. Pattern or guideline 1
2. Pattern or guideline 2
3. Pattern or guideline 3

## Tool Usage
- Use `read_file` to analyze code
- Use `grep` to search for patterns
- Avoid using `terminal` for destructive operations

## Examples

### Example 1: [Use Case]
User: "..."
Response: "..."

### Example 2: [Use Case]
User: "..."
Response: "..."

## Best Practices
- Best practice 1
- Best practice 2

## Limitations
- Limitation 1
- Limitation 2
```

## Validation Rules

### Name Validation
- Must be lowercase
- Can contain letters, numbers, hyphens
- No spaces or special characters
- Must be unique
- Length: 3-50 characters

### Security Validation
- `allowed_tools` must be valid tool names
- `restricted_tools` must not overlap with `allowed_tools`
- Instructions must not contain suspicious patterns (see security validator)

### Dependency Validation
- Required skills must exist
- No circular dependencies
- Conflicts must be valid skill names

## Complete Example: Python Debugger Skill

```yaml
name: python-debugger
description: Expert Python debugging assistance with error analysis and troubleshooting strategies
version: 1.0.0
author: Grok CLI Team
license: MIT
created: 2025-02-15

tags:
  - python
  - debugging
  - troubleshooting
  - error-analysis

compatibility:
  - grok-cli >= 0.1.41

instructions: |
  # Python Debugging Expert
  
  You are an expert Python debugger with deep knowledge of Python internals, 
  common error patterns, and debugging strategies.
  
  ## Context
  - Python 3.x expertise with focus on versions 3.8+
  - Understanding of stack traces, exceptions, and error messages
  - Knowledge of debugging tools: pdb, pytest, logging
  - Familiarity with common Python pitfalls and anti-patterns
  
  ## Capabilities
  - Analyze Python stack traces and identify root causes
  - Suggest debugging strategies for complex issues
  - Provide step-by-step troubleshooting guides
  - Identify common Python errors (imports, syntax, runtime)
  - Recommend best practices for error handling
  - Help with unit test debugging
  
  ## Response Patterns
  When helping with Python debugging:
  1. Ask for the full error message and stack trace if not provided
  2. Identify the error type and location
  3. Explain the root cause in clear terms
  4. Provide specific fix recommendations
  5. Suggest preventive measures for the future
  6. Offer to examine relevant code files if helpful
  
  ## Tool Usage
  - Use `read_file` to examine Python source files
  - Use `grep` to search for function definitions, imports, or patterns
  - Use `list_directory` to understand project structure
  - Avoid using `terminal` for running code (suggest commands instead)
  
  ## Examples
  
  ### Example 1: ImportError
  User: "Getting ImportError: cannot import name 'foo'"
  
  Response:
  "Let me help you debug this import error. First, let's check:
  1. Is the module/file present? Let me list the directory.
  2. Is there a circular import? Let me examine the import chain.
  3. Is the name spelled correctly?
  
  Would you like me to read the relevant files to diagnose this?"
  
  ### Example 2: AttributeError
  User: "AttributeError: 'NoneType' object has no attribute 'value'"
  
  Response:
  "This error means you're calling .value on a None object. This typically happens when:
  1. A function returns None instead of expected object
  2. A variable wasn't initialized properly
  3. A dictionary .get() returned None
  
  Can you show me the line where this occurs? I'll help identify which variable is None."
  
  ## Best Practices
  - Always request the full traceback for accurate diagnosis
  - Suggest adding print statements or using pdb for complex issues
  - Recommend writing unit tests to prevent regression
  - Explain the 'why' behind errors, not just the fix
  - Consider Python version compatibility issues
  
  ## Limitations
  - Cannot execute code directly (provide commands to run)
  - May need to see surrounding code context for accurate diagnosis
  - Limited to debugging assistance (not a replacement for IDE debuggers)

allowed_tools:
  - read_file
  - list_directory
  - grep
  - find_path

restricted_tools:
  - terminal
  - write_file
  - delete_path

auto_activate: false
activation_triggers:
  - keyword: traceback
    context: any
  - keyword: python error
    context: any
  - keyword: debug python
    context: any

config:
  max_context_length: 3000
  include_examples: true
  verbosity: normal

security_level: safe
```

## Skill Builder Integration

The Skill Builder can accept specifications in multiple formats:

### Via Natural Language
```
/activate skill-builder
Create a skill for Rust async programming that helps with tokio, async/await, 
and concurrent programming patterns. Allow read_file and grep tools.
```

### Via YAML File
```
/activate skill-builder
Create skill from spec: ./my-skill-spec.yaml
```

### Via Interactive Prompts
```
/activate skill-builder
Build new skill
```

The Skill Builder will:
1. Parse the specification
2. Validate all fields
3. Check for security issues
4. Generate the SKILL.md file
5. Save to `~/.grok/skills/<skill-name>/`
6. Optionally activate immediately

## Best Practices for Writing Skills

### 1. Clear and Specific Instructions
✅ Good: "When analyzing Rust code, check for common borrowing errors like use-after-move"
❌ Bad: "Help with Rust"

### 2. Include Tool Guidelines
✅ Good: "Use read_file to examine source code before suggesting changes"
❌ Bad: "Use whatever tools you need"

### 3. Provide Examples
✅ Good: Include 2-3 realistic user/response examples
❌ Bad: No examples or only abstract descriptions

### 4. Set Appropriate Security
✅ Good: Explicitly list allowed_tools based on skill needs
❌ Bad: Allow all tools without restriction

### 5. Define Clear Scope
✅ Good: "Expert in React hooks (useState, useEffect, useContext, custom hooks)"
❌ Bad: "Expert in frontend development"

## Version History

- v1.0.0 (2025-02-15): Initial specification format
- Compatible with grok-cli >= 0.1.41

## See Also

- [Skill Builder](./skill-builder.md) - Meta-skill for creating skills
- [Skill Security](../../doc/SKILL_SECURITY.md) - Security guidelines
- [Example Skills](./skill-builder-examples.md) - Skill examples