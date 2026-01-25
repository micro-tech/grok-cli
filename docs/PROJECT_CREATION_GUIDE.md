# Project Creation Quick Start Guide

This guide shows you how to use Grok CLI's automatic tool execution to create new projects effortlessly.

## Table of Contents
- [Quick Start](#quick-start)
- [Step-by-Step Tutorial](#step-by-step-tutorial)
- [Project Templates](#project-templates)
- [Best Practices](#best-practices)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

## Quick Start

### 1. Create a New Project Directory
```bash
mkdir my-project
cd my-project
```

### 2. Start Grok CLI
```bash
grok
```

### 3. Ask Grok to Create Your Project
```
You: Create a new Rust CLI application with:
- Cargo.toml for a binary called my-app
- src/main.rs with clap for argument parsing
- README.md with installation and usage instructions
- .gitignore for Rust projects
```

### 4. Watch the Magic Happen
```
Grok is executing operations...
  ✓ Successfully wrote to Cargo.toml
  ✓ Successfully wrote to src/main.rs
  ✓ Successfully wrote to README.md
  ✓ Successfully wrote to .gitignore
All operations completed!
```

That's it! Your project is ready to use.

## Step-by-Step Tutorial

### Example: Creating a REST API Server

#### Step 1: Set Up Directory
```bash
mkdir todo-api
cd todo-api
```

#### Step 2: Start Interactive Session
```bash
grok
```

#### Step 3: Create Project Structure
```
You: Create a Rust REST API project structure using Axum with the following:

1. Cargo.toml with dependencies:
   - axum 0.7
   - tokio with full features
   - serde and serde_json
   - tower-http for CORS
   - sqlx with postgres features

2. Directory structure:
   - src/main.rs - Server entry point
   - src/routes/ - API route handlers
   - src/models/ - Data models
   - src/db/ - Database connection

3. .env.example with:
   - DATABASE_URL
   - PORT
   - HOST

4. README.md with API documentation
```

#### Step 4: Add Route Handlers
```
You: Create src/routes/mod.rs that exports all route modules

You: Create src/routes/todos.rs with CRUD endpoints:
- GET /todos - List all todos
- POST /todos - Create new todo
- GET /todos/:id - Get single todo
- PUT /todos/:id - Update todo
- DELETE /todos/:id - Delete todo
```

#### Step 5: Add Data Models
```
You: Create src/models/todo.rs with:
- Todo struct with id, title, description, completed, created_at
- Derive Serialize, Deserialize, and sqlx::FromRow
```

#### Step 6: Set Up Database
```
You: Create src/db/mod.rs with:
- Database connection pool setup
- Migration runner function
- Error handling
```

#### Step 7: Complete the Application
```
You: Update src/main.rs to:
- Initialize database connection
- Set up Axum router with all routes
- Configure CORS middleware
- Start server on configured port
```

#### Step 8: Verify Structure
```
You: List all files we created
```

Output:
```
  ✓ Directory contents of .:
    Cargo.toml
    README.md
    .env.example
    src/
    src/main.rs
    src/routes/
    src/routes/mod.rs
    src/routes/todos.rs
    src/models/
    src/models/todo.rs
    src/db/
    src/db/mod.rs
```

## Project Templates

### Rust CLI Application
```
You: Create a Rust CLI application with:
- Cargo.toml for binary with clap, colored, and anyhow
- src/main.rs with command-line argument parsing
- src/cli/ directory for command modules
- src/utils/ for helper functions
- tests/ directory with integration tests
- README.md with usage examples
- .gitignore for Rust
Then run cargo init and git init to set it up
```

### Web Application (Full Stack)
```
You: Create a full-stack web application with:

Backend (Rust + Axum):
- backend/Cargo.toml with axum, tokio, sqlx
- backend/src/main.rs - API server
- backend/src/routes/ - API endpoints
- backend/src/models/ - Database models
- backend/src/middleware/ - Auth, CORS, logging

Frontend (HTML/CSS/JS):
- frontend/index.html - Main page
- frontend/css/styles.css - Styling
- frontend/js/app.js - API interactions
- frontend/js/components/ - Reusable components

Configuration:
- .env.example with all required variables
- docker-compose.yml for PostgreSQL
- README.md with setup instructions
```

### Microservice
```
You: Create a microservice project with:
- Service code in src/
- Health check endpoint
- Metrics endpoint (Prometheus format)
- Docker configuration
- Kubernetes manifests in k8s/
- CI/CD pipeline (.github/workflows/)
- Comprehensive README
```

### Library/Package
```
You: Create a Rust library with:
- Cargo.toml configured as a library
- src/lib.rs with module exports
- src/ directory with core modules
- examples/ with usage examples
- tests/ with unit and integration tests
- benches/ with benchmarks
- docs/ with architecture documentation
- README.md with API overview
- CONTRIBUTING.md
- LICENSE (MIT)
```

### Data Processing Pipeline
```
You: Create a data processing pipeline with:
- Input data parsers in src/parsers/
- Transformation logic in src/transforms/
- Output writers in src/writers/
- Configuration system in src/config/
- Error handling and logging
- Example data files in data/examples/
- Pipeline documentation
```

## Best Practices

### 1. Start with Structure
Always create the directory structure first:
```
You: Create the following directory structure:
- src/
- tests/
- docs/
- examples/
```

### 2. One Step at a Time
Don't ask for everything at once. Break it down:
```
✅ Good:
You: First, create the Cargo.toml
You: Now create the main.rs with basic setup
You: Add the configuration module

❌ Too Much:
You: Create everything for a complete application
```

### 3. Be Specific About Dependencies
```
✅ Good:
You: Add dependencies: axum 0.7, tokio 1.35 with full features, serde 1.0

❌ Vague:
You: Add some web framework dependencies
```

### 4. Verify as You Go
```
You: List all files in src/
You: Show me the contents of main.rs
You: What's in the Cargo.toml?
```

### 5. Use Version Control
```bash
# Initialize git before starting
git init

# Commit after major steps
You: Create the project structure
# Verify it looks good
git add .
git commit -m "Initial project structure"
```

## Common Patterns

### Pattern 1: Iterative Development
```
You: Create a basic main.rs with hello world
# Review output
You: Add command-line argument parsing
# Test it
You: Add configuration file support
# Build incrementally
```

### Pattern 2: Template + Customize
```
You: Create a standard Rust binary project structure
You: Now customize it for a web scraper with reqwest and scraper crates
You: Add concurrent processing with tokio
```

### Pattern 3: Copy from Reference
```
You: Create a project structure similar to a typical Rust web API
You: Follow the axum examples for the router setup
You: Use the same error handling pattern as the anyhow documentation
```

### Pattern 4: Modular Creation
```
You: Create src/database/mod.rs with connection pooling
You: Create src/database/models.rs with user model
You: Create src/database/queries.rs with CRUD operations
You: Now create src/database/migrations.rs
```

## Advanced Usage

### Multi-Language Projects
```
You: Create a polyglot project with:

Rust backend:
- backend/Cargo.toml
- backend/src/main.rs
- backend/src/api/

Python ML service:
- ml-service/requirements.txt
- ml-service/main.py
- ml-service/models/

Configuration:
- docker-compose.yml connecting both services
- README with setup for both
```

### Monorepo Structure
```
You: Create a monorepo with:

Workspace Cargo.toml at root
Packages:
- packages/core/ - Shared library
- packages/cli/ - CLI application
- packages/server/ - Web server
- packages/client/ - API client

Documentation:
- docs/ARCHITECTURE.md
- Each package has its own README
```

### Generated Code Projects
```
You: Create a code generation project:
- templates/ directory with template files
- src/generator/ with code generation logic
- src/parser/ to parse input specifications
- examples/output/ showing generated results
- README explaining the generation process
```

## Troubleshooting

### Problem: Files Not Created
**Symptom**: Grok describes what to do but doesn't create files.

**Solution**:
1. Check version: `grok --version` (need v0.1.2+)
2. Be more explicit: "Create the file..." not "You should create..."
3. Make sure you're in interactive mode

### Problem: Wrong Directory
**Symptom**: Files created in unexpected location.

**Solution**:
1. Check current directory: `pwd` or `cd`
2. Change to correct directory before starting grok
3. Use relative paths: `src/main.rs` not `/path/to/src/main.rs`

### Problem: Files Overwritten
**Symptom**: Existing files were replaced.

**Solution**:
1. Always use git: `git init` before starting
2. Ask Grok to read files first: "Show me main.rs"
3. Use specific modifications: "Add function X to main.rs" instead of "Recreate main.rs"

### Problem: Incomplete Structure
**Symptom**: Some files/directories missing.

**Solution**:
1. Ask explicitly for each part
2. Verify with: "List all files and directories we've created"
3. Fill gaps: "We're missing tests/, please create it"

### Problem: Syntax Errors in Generated Code
**Symptom**: Created files have syntax issues.

**Solution**:
1. Run cargo check or equivalent: `cargo check`
2. Show errors to Grok: "Fix these compiler errors: [paste errors]"
3. Ask for specific fixes: "The import path is wrong in main.rs"

## Examples Gallery

### Example 1: Blog Engine
```bash
mkdir blog-engine && cd blog-engine
grok
```
```
You: Create a blog engine with:
- Post creation and editing
- Markdown support
- SQLite database
- Web interface with templates
- RSS feed generation
- Tag system
- Search functionality
```

### Example 2: Discord Bot
```bash
mkdir discord-bot && cd discord-bot
grok
```
```
You: Create a Discord bot with:
- serenity crate setup
- Command framework
- src/commands/ with modular commands
- Configuration from .env
- Database for persistent data
- README with bot setup instructions
```

### Example 3: File Converter
```bash
mkdir file-converter && cd file-converter
grok
```
```
You: Create a file format converter CLI that:
- Supports JSON, YAML, TOML, CSV
- Uses clap for arguments
- Parallel processing with rayon
- Progress bar with indicatif
- Comprehensive error handling
- Example files in examples/
Then initialize it with cargo init --bin and git init
```

### Example 4: Monitoring Dashboard
```bash
mkdir monitor-dash && cd monitor-dash
grok
```
```
You: Create a system monitoring dashboard:
- Backend API collecting system metrics
- WebSocket for real-time updates
- Simple HTML/JS frontend
- Charts using Chart.js
- Docker deployment
- Prometheus integration
```

## Tips for Success

### 1. Learn from Examples
```
You: Show me a typical Rust project structure for a web API
You: What files are essential for a CLI application?
You: What's the standard layout for tests in Rust?
```

### 2. Ask for Explanations
```
You: Why did you structure it this way?
You: Explain the dependencies in Cargo.toml
You: What does this configuration do?
```

### 3. Iterate and Improve
```
You: Create basic version
You: Add error handling
You: Add logging
You: Add configuration
You: Add tests
You: Add documentation
```

### 4. Use Templates Consistently
Create your own patterns:
```
You: Remember that I prefer this project structure: [describe]
# Grok will save to memory
You: Create a new project following my preferred structure
```

### 5. Combine with Git Workflow
```bash
git init
grok
# Create initial structure
git add .
git commit -m "Initial commit"

# Continue development
# Each major step -> commit
```

## Next Steps

After creating your project:

1. **Build and Test**
   ```bash
   cargo build
   cargo test
   cargo run
   ```
   
   Or ask Grok to do it:
   ```
   You: Run cargo build to compile the project
   You: Execute cargo test to run the tests
   ```

2. **Review Generated Code**
   - Check for TODO comments
   - Verify error handling
   - Review dependencies

3. **Customize**
   ```
   You: Add custom error types to src/error.rs
   You: Implement authentication middleware
   You: Add database migrations
   ```

4. **Document**
   ```
   You: Add doc comments to all public functions
   You: Create ARCHITECTURE.md explaining the design
   You: Add examples to README
   ```

5. **Set Up CI/CD**
   ```
   You: Create GitHub Actions workflow for testing
   You: Add clippy and rustfmt checks
   You: Set up deployment configuration
   You: Run git add . and git commit -m "Initial commit"
   ```

## Resources

- [FILE_OPERATIONS.md](FILE_OPERATIONS.md) - Detailed tool documentation
- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - Command reference
- [CHAT_LOGGING.md](CHAT_LOGGING.md) - Session management
- Main [README.md](../README.md) - Getting started

## Feedback

Share your project creation experiences:
- GitHub Issues: Report problems or request features
- GitHub Discussions: Share your project templates
- Pull Requests: Contribute example templates

---

**Happy Creating! 🚀**

Version: 0.1.2  
Last Updated: 2026-01-13