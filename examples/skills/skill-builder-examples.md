# Skill Builder - Real-World Examples

This document provides comprehensive examples of using the Skill Builder to create custom skills dynamically.

## Table of Contents

1. [Quick Creation Examples](#quick-creation-examples)
2. [Natural Language Creation](#natural-language-creation)
3. [Specification-Based Creation](#specification-based-creation)
4. [Interactive Building](#interactive-building)
5. [Cloning and Extending](#cloning-and-extending)
6. [Complete Skill Examples](#complete-skill-examples)
7. [Advanced Patterns](#advanced-patterns)
8. [Common Mistakes](#common-mistakes)

---

## Quick Creation Examples

### Example 1: Simple Programming Language Skill

```
User: Create a skill for Go programming

Grok: I'll create a Go programming expert skill for you!

Let me clarify a few things:
1. Should it cover Go modules and dependency management?
2. Include concurrency patterns (goroutines, channels)?
3. Focus on any specific use case? (web servers, CLI tools, etc.)

Meanwhile, I'll create the basic structure with common Go expertise...

[Creates skill]

✓ Skill 'go-expert' created at ~/.grok/skills/go-expert/SKILL.md
✓ Security validation: SAFE
✓ Activated and ready to use!

Try asking: "How do I handle errors in Go?" or "Explain goroutines"
```

### Example 2: Framework Specialist

```
User: I need help with Next.js - create a skill for it

Grok: Creating a Next.js specialist skill!

Focusing on:
- App Router and Pages Router
- Server Components vs Client Components
- Data fetching patterns
- Routing and navigation
- Performance optimization

[Creates skill with write_file]

✓ Skill 'nextjs-specialist' created
✓ Security validation: SAFE
✓ Now active in this session!

The skill can help with Next.js 13+ features. Try: "Explain Server Components"
```

### Example 3: Domain Expert

```
User: Create a skill that knows about AWS Lambda best practices

Grok: Building an AWS Lambda expert skill!

This will cover:
- Lambda function design patterns
- Performance optimization
- Cold start mitigation
- IAM and security
- Integration with other AWS services

Tools: read_file, list_directory, grep (to analyze Lambda code)

[Creates skill]

✓ Skill 'aws-lambda-expert' created and activated!

Test it: "How can I reduce Lambda cold starts?"
```

---

## Natural Language Creation

### Example 4: Detailed Natural Language Request

```
User: Create a skill for TypeScript debugging that:
- Helps identify and fix TypeScript compilation errors
- Understands tsconfig.json settings
- Can read TypeScript files and suggest fixes
- Knows about common type issues
- Focuses on React projects
- Uses read_file and grep tools only

Grok: Perfect! I'll create a comprehensive TypeScript debugging skill.

Parsing your requirements:
✓ Domain: TypeScript debugging
✓ Special focus: React projects
✓ Configuration: tsconfig.json knowledge
✓ Tools: read_file, grep (read-only, safe)
✓ Scope: Compilation errors and type issues

Creating skill specification...

---
name: typescript-debugger
description: TypeScript debugging expert specializing in React projects with focus on compilation errors and type issues
version: 1.0.0
tags:
  - typescript
  - debugging
  - react
  - type-system
allowed-tools:
  - read_file
  - grep
  - list_directory
---

[Full skill content generated]

Writing to ~/.grok/skills/typescript-debugger/SKILL.md...

✓ Skill created successfully!
✓ Security validation: SAFE (read-only tools, no suspicious patterns)
✓ Activated in current session!

You can now ask TypeScript debugging questions like:
- "Why am I getting 'Type X is not assignable to type Y'?"
- "Help me understand this tsconfig.json setting"
- "How do I type this React component prop?"
```

### Example 5: Conversational Style

```
User: I'm working on Docker a lot and need help with troubleshooting

Grok: I can create a Docker troubleshooting skill for you!

What specific areas do you need most help with?
- Container issues (won't start, exits immediately)?
- Networking problems?
- Volume/storage issues?
- Dockerfile optimization?
- Docker Compose configurations?
- All of the above?

User: All of them, especially networking and why containers fail

Grok: Great! Creating a comprehensive Docker troubleshooting expert...

Focusing on:
✓ Container lifecycle issues (startup, crashes, exits)
✓ Network troubleshooting (connectivity, DNS, ports)
✓ Volume and storage problems
✓ Dockerfile best practices
✓ Docker Compose debugging
✓ Log analysis strategies

Tools I'll allow:
- read_file (to examine Dockerfiles, docker-compose.yml)
- list_directory (to check project structure)
- grep (to search logs and configs)

Creating skill...

✓ Skill 'docker-troubleshooter' created!
✓ All read-only tools - very safe
✓ Active now!

Try: "My container exits immediately with code 1, how do I debug this?"
```

---

## Specification-Based Creation

### Example 6: YAML Specification

```
User: Create skill from this spec:

---
name: sql-query-optimizer
description: SQL query optimization and performance tuning expert
version: 1.0.0
author: DevTeam
license: MIT
tags:
  - sql
  - database
  - performance
  - optimization
allowed-tools:
  - read_file
  - grep
instructions: |
  Expert in SQL query optimization for PostgreSQL, MySQL, and SQL Server.
  
  ## Capabilities
  - Analyze query plans and explain performance
  - Suggest index improvements
  - Identify N+1 query problems
  - Recommend query rewrites
  - Explain JOIN strategies
  
  ## Response Pattern
  When analyzing queries:
  1. Ask for the query and schema if not provided
  2. Analyze the query structure
  3. Identify potential bottlenecks
  4. Suggest specific optimizations
  5. Explain the reasoning
  
  ## Tool Usage
  - Use read_file to examine SQL files or migration scripts
  - Use grep to find query patterns across codebase
---

Grok: Excellent specification! Validating...

✓ Name: 'sql-query-optimizer' (valid format)
✓ Required fields: Present
✓ Tools: read_file, grep (safe, read-only)
✓ Instructions: Comprehensive and clear
✓ No security issues detected

Creating SKILL.md from specification...

[write_file: ~/.grok/skills/sql-query-optimizer/SKILL.md]

✓ Skill created from specification!
✓ Security: SAFE
✓ Activated!

The SQL query optimizer is ready. Try:
"How can I optimize this SELECT query with multiple JOINs?"
```

### Example 7: JSON Specification

```
User: Build this skill:
{
  "name": "api-design-reviewer",
  "description": "REST API design and review expert",
  "version": "1.0.0",
  "tags": ["api", "rest", "design", "architecture"],
  "allowed_tools": ["read_file", "grep", "list_directory"],
  "instructions": "Expert in REST API design...",
  "config": {
    "verbosity": "detailed",
    "include_examples": true
  }
}

Grok: Converting JSON specification to SKILL.md format...

Parsing JSON... ✓
Validating fields... ✓
Converting to YAML frontmatter... ✓
Expanding instructions section... ✓

Creating skill file...

✓ Skill 'api-design-reviewer' created!
✓ Configuration applied: detailed verbosity, examples included
✓ Activated!

Test with: "Review this REST API endpoint design"
```

---

## Interactive Building

### Example 8: Step-by-Step Guided Creation

```
User: /activate skill-builder
User: Help me build a custom skill

Grok: Let's build a skill together! I'll guide you through the process.

**Step 1: Domain Focus**
What should the skill focus on? Be as specific as possible.

Examples:
- "Python testing with pytest"
- "CSS Grid and Flexbox layouts"
- "Git workflow and branching strategies"
- "Kubernetes manifest debugging"

User: Terraform for AWS infrastructure

Grok: Great choice! Terraform + AWS.

**Step 2: Specific Capabilities**
What should the skill be able to help with? (Select 3-5)

Suggestions for Terraform/AWS:
a) Writing and structuring Terraform modules
b) State management and remote backends
c) AWS resource configurations (VPC, EC2, RDS, etc.)
d) Troubleshooting plan/apply errors
e) Best practices and security
f) Testing infrastructure code
g) Workspace management
h) Provider configuration

User: a, c, d, e

Grok: Perfect! Focusing on:
✓ Module structure and best practices
✓ AWS resource configurations
✓ Troubleshooting errors
✓ Security and best practices

**Step 3: Tool Permissions**
Which tools should the skill use?

Recommended for Terraform:
- read_file (examine .tf files) ✓ Safe
- list_directory (check project structure) ✓ Safe
- grep (search for patterns) ✓ Safe
- find_path (locate config files) ✓ Safe

Include all recommended? (yes/no)

User: yes

Grok: **Step 4: Security Level**
Any restricted operations?

For safety, I'll:
✓ Not allow write_file (prevent accidental modifications)
✓ Not allow terminal (prevent accidental terraform apply)
✓ Read-only access only

Sound good? (yes/no)

User: yes

Grok: Perfect! Creating your Terraform AWS expert skill...

Generated specification:
---
name: terraform-aws-expert
description: Terraform infrastructure-as-code expert for AWS with focus on modules, resources, troubleshooting, and security
version: 1.0.0
tags:
  - terraform
  - aws
  - infrastructure
  - iac
allowed-tools:
  - read_file
  - list_directory
  - grep
  - find_path
---

[Complete instructions generated]

Writing to ~/.grok/skills/terraform-aws-expert/SKILL.md...

✓ Skill created!
✓ Security: SAFE (read-only)
✓ Activated!

Your Terraform AWS expert is ready! Try:
- "Review my VPC module structure"
- "Why is my terraform plan failing?"
- "What's the best way to organize Terraform for multi-region AWS?"
```

---

## Cloning and Extending

### Example 9: Clone and Modify

```
User: Create a skill like rust-expert but for C++

Grok: I'll create a C++ expert skill based on the rust-expert template!

First, let me read the rust-expert skill to understand its structure...

[read_file: ~/.grok/skills/rust-expert/SKILL.md]

Analyzing structure:
✓ Comprehensive language expertise
✓ Focus on memory safety and ownership
✓ Common pitfalls and debugging
✓ Best practices and patterns
✓ Tool usage guidelines

Adapting for C++:
- Memory management → RAII, smart pointers, manual memory
- Ownership → Move semantics, copy elision
- Safety → Undefined behavior prevention
- Patterns → Modern C++ (C++17/20/23)
- Tools → Same (read_file, grep, etc.)

Creating cpp-expert skill...

✓ Skill 'cpp-expert' created!
✓ Based on rust-expert structure
✓ Adapted for C++ specifics
✓ Activated!

The C++ expert includes:
- Modern C++ features (auto, constexpr, concepts)
- Memory management best practices
- STL and standard library guidance
- Common C++ pitfalls and solutions

Try: "Explain smart pointers in C++"
```

### Example 10: Extend Existing Skill

```
User: Add GraphQL expertise to my nextjs-specialist skill

Grok: I'll enhance the nextjs-specialist skill with GraphQL capabilities!

Reading current skill...
[read_file: ~/.grok/skills/nextjs-specialist/SKILL.md]

Current capabilities:
- App Router and routing
- Server/Client Components
- Data fetching (REST)
- Performance optimization

Adding:
✓ GraphQL integration with Next.js
✓ Apollo Client setup
✓ GraphQL Code Generator
✓ Server-side GraphQL queries
✓ Caching strategies

Updating skill file...
[write_file: ~/.grok/skills/nextjs-specialist/SKILL.md]

✓ Skill updated!
✓ GraphQL expertise added
✓ Still active with new capabilities!

The enhanced skill now covers GraphQL in Next.js. Try:
"How do I set up Apollo Client in Next.js App Router?"
```

---

## Complete Skill Examples

### Example 11: Database Migration Expert

```yaml
---
name: database-migrations-expert
description: Database migration and schema evolution expert for multiple databases
version: 1.0.0
author: Skill Builder
license: MIT
tags:
  - database
  - migrations
  - schema
  - sql
compatibility:
  - grok-cli >= 0.1.41
allowed-tools:
  - read_file
  - list_directory
  - grep
  - find_path
---

# Database Migrations Expert

## Overview
Expert in database migrations, schema evolution, and version control for databases.
Supports multiple migration frameworks and databases.

## Context
- Migration frameworks: Alembic (Python), Flyway (Java), Liquibase, Rails migrations
- Databases: PostgreSQL, MySQL, SQLite, SQL Server
- Best practices for zero-downtime migrations
- Rollback strategies and data safety

## Capabilities
- Design safe migration strategies
- Review migration scripts for issues
- Suggest rollback procedures
- Identify breaking changes
- Recommend migration sequencing
- Handle data transformations

## Response Patterns
When helping with migrations:
1. Ask about database type and framework if not specified
2. Analyze the migration for safety (destructive operations, data loss)
3. Suggest ways to make it reversible
4. Consider production impact
5. Recommend testing strategies

## Tool Usage
- Use read_file to examine migration files
- Use list_directory to understand migration history
- Use grep to find related migrations
- Use find_path to locate migration directories

## Examples

### Example 1: Adding a Column
User: "I need to add a NOT NULL column to a large table"

Response:
"For a NOT NULL column on a large table, here's a safe approach:

Migration 1:
1. Add column as nullable
2. Backfill data with default value
3. Add NOT NULL constraint in separate migration

This prevents locking the entire table. Would you like me to review your migration file?"

### Example 2: Renaming a Column
User: "How do I rename a column without breaking the app?"

Response:
"Column renames need careful handling to avoid downtime:

Step 1: Add new column
Step 2: Dual-write to both columns (app change)
Step 3: Backfill data from old to new
Step 4: Switch reads to new column (app change)
Step 5: Stop writing to old column
Step 6: Drop old column

This zero-downtime approach ensures no data loss. Which step are you on?"

## Best Practices
- Always test migrations on production-like data
- Make migrations reversible when possible
- Avoid large data migrations in same transaction as DDL
- Use migration locks to prevent concurrent runs
- Keep migrations small and focused
- Document breaking changes clearly

## Limitations
- Cannot execute migrations (read-only)
- Cannot access actual database to verify
- Relies on reviewing code and providing guidance
```

### Example 12: API Testing Specialist

```yaml
---
name: api-testing-specialist
description: API testing expert covering REST, GraphQL, and test automation strategies
version: 1.0.0
tags:
  - testing
  - api
  - rest
  - graphql
  - automation
allowed-tools:
  - read_file
  - grep
  - list_directory
---

# API Testing Specialist

## Overview
Expert in API testing strategies, tools, and automation for REST and GraphQL APIs.

## Context
- Testing frameworks: Postman, REST Assured, Supertest, Pytest
- API types: REST, GraphQL, gRPC
- Test patterns: Contract testing, integration testing, E2E
- CI/CD integration

## Capabilities
- Design comprehensive API test suites
- Write test cases for REST endpoints
- Create GraphQL query tests
- Suggest mocking strategies
- Review existing test code
- Recommend test automation approaches
- Help with test data management

## Response Patterns
When helping with API testing:
1. Understand the API type and tech stack
2. Suggest appropriate testing levels (unit, integration, E2E)
3. Provide specific test examples in relevant framework
4. Consider edge cases and error scenarios
5. Recommend assertion strategies

## Tool Usage
- read_file: Examine API test files, specs, schemas
- grep: Find existing test patterns
- list_directory: Understand test structure

## Examples

### Example 1: REST Endpoint Testing
User: "How do I test a POST endpoint that creates users?"

Response:
"For a user creation endpoint, test these scenarios:

**Happy Path:**
- Valid user data → 201 Created
- Verify response includes user ID
- Confirm user in database

**Validation:**
- Missing required fields → 400 Bad Request
- Invalid email format → 400
- Duplicate email → 409 Conflict

**Security:**
- Authentication required → 401
- Insufficient permissions → 403

Would you like example test code for your framework?"

### Example 2: GraphQL Mutation Testing
User: "Test strategy for GraphQL mutations?"

Response:
"GraphQL mutation testing should cover:

1. **Valid Mutations:**
   - Correct input → Success response
   - Verify returned fields match schema
   - Check side effects (DB updates)

2. **Input Validation:**
   - Invalid input → Error with details
   - Missing required fields → Specific error
   - Type mismatches → Schema validation error

3. **Fragments & Variables:**
   - Test with variables
   - Test fragment spreading

Example test structure:
```javascript
test('createUser mutation with valid data', async () => {
  const mutation = `
    mutation CreateUser($input: UserInput!) {
      createUser(input: $input) {
        id
        email
        createdAt
      }
    }
  `
  // ... rest of test
})
```

Want to see more examples?"

## Best Practices
- Test happy paths AND error cases
- Use realistic test data
- Clean up after tests (idempotency)
- Mock external dependencies
- Test authentication/authorization
- Validate response schemas
- Test rate limiting
- Check error message clarity

## Limitations
- Cannot execute tests directly
- Cannot access actual APIs
- Provides guidance and code examples
```

---

## Advanced Patterns

### Example 13: Skill with Dependencies

```
User: Create a skill for testing React hooks that requires the React expert skill

Grok: Creating a React hooks testing skill with dependency on react-expert!

This skill will:
✓ Assume react-expert provides basic React knowledge
✓ Focus specifically on testing hooks
✓ Cover React Testing Library, Jest, and testing patterns
✓ Require react-expert skill to be activated together

Adding dependency declaration...

---
name: react-hooks-tester
description: React hooks testing specialist using React Testing Library and Jest
requires:
  - skill: react-expert
    optional: false
allowed-tools:
  - read_file
  - grep
---

✓ Skill created with dependency!
⚠ Note: 'react-expert' skill should be activated for full context

Activated react-hooks-tester! 

Would you also like to activate react-expert now? (recommended)
```

### Example 14: Skill with Conflict Declaration

```
User: Create a Python 2 legacy code skill

Grok: Creating Python 2 legacy code expert...

⚠ Note: This will conflict with the standard python-expert skill if activated.
I'll add a conflict declaration to prevent confusion.

---
name: python2-legacy
description: Python 2.7 legacy code maintenance expert
conflicts_with:
  - skill: python-expert
tags:
  - python
  - python2
  - legacy
---

✓ Skill created with conflict marker!
⚠ If you activate this, I recommend deactivating 'python-expert' first

Would you like to:
1. Activate python2-legacy (will deactivate python-expert)
2. Keep python-expert active
3. Create but don't activate yet
```

### Example 15: Skill with Auto-Activation Triggers

```
User: Create a Docker skill that auto-suggests activation when I mention docker

Grok: Creating Docker expert with smart activation triggers!

I'll set up triggers for:
- Keyword: "docker"
- Keyword: "dockerfile"  
- Keyword: "container"
- File patterns: "Dockerfile", "docker-compose.yml"

---
name: docker-expert
description: Docker and containerization expert
activation_triggers:
  - keyword: docker
    context: any
  - keyword: dockerfile
    context: any
  - keyword: container
    context: any
  - file_pattern: "**/Dockerfile"
  - file_pattern: "**/docker-compose.yml"
auto_activate: false
---

✓ Skill created with smart triggers!
✓ When you mention Docker, I'll suggest activating this skill

The skill is now available. Try mentioning "docker" in a question and I'll suggest activation!
```

---

## Common Mistakes

### Mistake 1: Too Broad Scope

❌ **Wrong:**
```
User: Create a skill for web development

Grok: [Creates overly broad skill covering HTML, CSS, JavaScript, frameworks, backend, databases, etc.]
```

✅ **Right:**
```
User: Create a skill for web development

Grok: "Web development" is quite broad! Let's narrow it down:

- Frontend (React, Vue, Angular)?
- Backend (Node.js, Python, Ruby)?
- Full-stack specific framework?
- HTML/CSS fundamentals?
- Performance optimization?

What's your primary focus?
```

### Mistake 2: Unsafe Tool Permissions

❌ **Wrong:**
```
User: Create a code reviewer skill

Grok: [Grants write_file, terminal, delete_path permissions]
```

✅ **Right:**
```
User: Create a code reviewer skill

Grok: Creating a code review skill with read-only permissions:
✓ read_file (to examine code)
✓ grep (to search patterns)
✓ list_directory (to understand structure)

❌ Not granting write_file (reviewers shouldn't modify code)
❌ Not granting terminal (no execution needed)

This keeps the skill safe for code review only.
```

### Mistake 3: Missing Examples

❌ **Wrong:**
```yaml
instructions: |
  Expert in Rust programming. Helps with Rust code.
```

✅ **Right:**
```yaml
instructions: |
  Expert in Rust programming.
  
  ## Examples
  
  ### Example 1: Borrow Checker Error
  User: "error: cannot borrow as mutable"
  Response: "This is a borrow checker error. You're trying to..."
  
  ### Example 2: Lifetime Questions
  User: "What does 'lifetime may not live long enough' mean?"
  Response: "This lifetime error occurs when..."
```

### Mistake 4: Unclear Instructions

❌ **Wrong:**
```yaml
instructions: Help with JavaScript stuff
```

✅ **Right:**
```yaml
instructions: |
  JavaScript expert focusing on ES2015+ features.
  
  ## Response Pattern
  1. Identify the JavaScript version/environment
  2. Provide modern ES6+ solutions
  3. Explain browser compatibility if relevant
  4. Suggest polyfills when needed
```

---

## Tips for Success

### 1. Start Specific, Expand Later
Create focused skills and expand them as needed rather than starting with broad skills.

### 2. Use Read-Only Tools by Default
Only grant write permissions when absolutely necessary and justified.

### 3. Include Real Examples
2-3 concrete examples are worth more than pages of abstract instructions.

### 4. Test Your Skills
After creation, test the skill with realistic queries to verify behavior.

### 5. Iterate and Improve
Skills can be updated! If something isn't working well, refine it.

### 6. Leverage Existing Skills
Clone and modify existing skills rather than starting from scratch.

### 7. Document Limitations
Be clear about what the skill can't do to set proper expectations.

---

## Quick Reference

### Creation Commands

```bash
# Natural language
"Create a skill for [domain]"

# From specification
"Create skill from this spec: [YAML/JSON]"

# Interactive
"Help me build a skill"

# Clone existing
"Create a skill like [existing-skill] but for [new-domain]"

# Update existing
"Add [capability] to [skill-name]"
```

### Validation Commands

```bash
# After creation
"Show me the [skill-name] skill"
"/skills"  # List all skills
"Test the [skill-name] skill"
```

### Management Commands

```bash
"/activate [skill-name]"  # Manually activate
"/deactivate [skill-name]"  # Deactivate
"Update [skill-name] to include [feature]"  # Modify
```

---

## Conclusion

The Skill Builder makes it easy to create custom, focused skills that extend Grok's capabilities immediately. Start simple, test often, and iterate based on real usage.

**Remember**: Skills are activated immediately in your current session - no restart required!

Happy skill building! 🚀