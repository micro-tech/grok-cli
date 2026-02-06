# SKILL.md Specification

## Overview

Agent Skills are modular capabilities for AI agents, defined in SKILL.md files. These files allow developers to create reusable prompts and behaviors for AI assistants.

## File Structure

SKILL.md files use Markdown with YAML frontmatter.

### Frontmatter Fields

- **name**: (required) The skill's name
- **description**: (required) Brief description of the skill
- **prompt**: (required) The prompt text to integrate when the skill is active
- **activation**: (optional) How to activate the skill (e.g., keyword or command)
- **authors**: (optional) List of authors
- **version**: (optional) Skill version
- **dependencies**: (optional) List of required skills

### Markdown Content

The body contains documentation, examples, and additional information about the skill.

## Example SKILL.md

```markdown
---
name: Rust Expert
description: Provides expertise in Rust programming
prompt: "You are a Rust programming expert. Provide accurate, idiomatic Rust code and explanations."
activation: "rust mode"
authors:
  - AI Assistant
version: 1.0
---

# Rust Expert Skill

This skill enhances the AI's ability to assist with Rust development.

## Capabilities

- Writing idiomatic Rust code
- Debugging Rust programs
- Explaining Rust concepts

## Usage

Activate this skill when working on Rust projects.
```

## Directory Structure

Skills are stored in the `skills/` directory, with each skill in its own SKILL.md file.

## Integration

The AI system reads SKILL.md files from the skills directory, parses the frontmatter, and integrates the prompt when the skill is activated.