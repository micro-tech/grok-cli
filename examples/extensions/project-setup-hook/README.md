# Project Setup Hook Extension

## Overview

The **Project Setup Hook** is a powerful extension that automates the creation of new Rust projects with complete Grok CLI integration. With one command, you get a fully configured Rust project with Cargo, Git, Zed editor settings, and Grok CLI configuration - everything you need to start coding immediately with AI assistance.

## Features

- ✅ **Automatic Rust Project Creation** - Uses `cargo new` to create proper Rust projects
- ✅ **Git Initialization** - Automatically initializes git and creates initial commits
- ✅ **Zed Editor Configuration** - Includes `.zed/` directory with settings, tasks, and AI rules
- ✅ **Grok CLI Integration** - Adds `.grok/` directory with project-specific configuration
- ✅ **Smart Defaults** - Includes common dependencies and project structure
- ✅ **Flexible Options** - Create binary or library projects with customizable settings
- ✅ **Cross-Platform** - Works on Windows (Git Bash/WSL), macOS, and Linux
- ✅ **Template System** - Use custom templates or built-in defaults

## Quick Start

### Installation

```bash
# Copy the extension to your Grok extensions directory
cp -r examples/extensions/project-setup-hook ~/.grok/extensions/

# Make scripts executable (Unix-like systems)
chmod +x ~/.grok/extensions/project-setup-hook/scripts/*.sh
```

### Basic Usage

```bash
# Navigate to where you want to create the project
cd ~/projects

# Create a new project
~/.grok/extensions/project-setup-hook/scripts/setup_project.sh my-awesome-app

# The project is now ready!
cd my-awesome-app
cargo build
grok interactive
```

## Usage Examples

### Create a Simple Binary Project

```bash
./setup_project.sh my-cli-tool
```

**Creates:**
```
my-cli-tool/
├── .git/
├── .gitignore
├── .grok/
│   ├── config.toml
│   └── .env
├── .zed/
│   ├── settings.json
│   ├── tasks.json
│   └── rules
├── Cargo.toml
├── src/
│   └── main.rs
└── docs/
    └── README.md
```

### Create a Library Project

```bash
./setup_project.sh my-awesome-lib --lib
```

Includes `examples/` directory for example code.

### Create in Specific Directory

```bash
./setup_project.sh web-server --path ~/projects/rust
```

### Create Without Git

```bash
./setup_project.sh quick-tool --no-git
```

### Create Minimal Project

```bash
./setup_project.sh minimal --no-zed --no-grok
```

Just Cargo project without Grok CLI extras.

## Command Reference

### Syntax

```bash
setup_project.sh <project-name> [options]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `project-name` | Yes | Name of the project (lowercase, hyphens/underscores allowed) |

### Options

| Option | Description |
|--------|-------------|
| `--path PATH` | Parent directory for the project (default: current directory) |
| `--bin` | Create a binary project (default) |
| `--lib` | Create a library project |
| `--no-git` | Skip git initialization |
| `--no-zed` | Skip .zed directory setup |
| `--no-grok` | Skip .grok directory setup |
| `--help`, `-h` | Show help message |

### Examples

```bash
# Binary project (default)
./setup_project.sh my-app

# Library project
./setup_project.sh my-lib --lib

# Custom location
./setup_project.sh my-tool --path ~/workspace

# Multiple options
./setup_project.sh test-app --lib --no-git --path /tmp
```

## What Gets Created

### Directory Structure

```
project-name/
├── .git/                      # Git repository (if --no-git not used)
├── .gitignore                 # Ignores target/, .grok/.env, etc.
├── .grok/                     # Grok CLI configuration
│   ├── config.toml            # Project-specific Grok settings
│   └── .env                   # API keys and secrets (git-ignored)
├── .zed/                      # Zed editor configuration
│   ├── settings.json          # Editor settings (Rust LSP, formatting)
│   ├── tasks.json             # Quick tasks (build, run, test, clippy)
│   └── rules                  # AI assistant guidelines
├── Cargo.toml                 # Rust project manifest with dependencies
├── Cargo.lock                 # Dependency lock file
├── src/
│   └── main.rs or lib.rs      # Entry point
├── docs/
│   └── README.md              # Documentation template
└── examples/                  # Example code (libraries only)
```

### Files Created

#### `.grok/config.toml`
Project-specific Grok CLI configuration including:
- ACP server settings
- Model preferences (grok-3, temperature, max tokens)
- Tool permissions
- Security settings
- Extension configuration
- Context files to load

#### `.grok/.env`
Template for sensitive configuration:
- X API credentials placeholders
- Network settings for Starlink
- Logging configuration
- Project-specific environment variables

#### `.zed/settings.json`
Zed editor settings:
- Rust-analyzer LSP configuration
- Format on save enabled
- Clippy integration
- Inlay hints enabled
- Terminal and theme settings

#### `.zed/tasks.json`
Quick access tasks:
- `cargo build`
- `cargo run`
- `cargo test`
- `cargo clippy`

#### `.zed/rules`
AI assistant guidelines:
- Rust code style conventions
- Error handling patterns
- Testing requirements
- Documentation standards
- Security considerations
- Common commands reference

#### `Cargo.toml`
Enhanced with common dependencies:
- `anyhow` - Error handling
- `clap` - Command-line parsing
- `serde` - Serialization
- `serde_json` - JSON support
- `tokio` - Async runtime
- Release profile optimizations

## Using With AI

Once your project is created, you can immediately start using Grok CLI with full context:

```bash
cd my-project

# Start interactive mode
grok interactive

# The AI already knows about your project!
> Add a new function to parse command-line arguments

> Create a test for the main function

> Set up CI/CD with GitHub Actions
```

The AI will have access to:
- Project structure and layout
- Code style guidelines from `.zed/rules`
- Available tools and dependencies
- Testing requirements
- Documentation standards

## Customizing Templates

### Using Custom Templates

You can customize the default templates by editing files in:
```
~/.grok/extensions/project-setup-hook/templates/
```

#### Template Structure

```
templates/
├── .zed/
│   ├── settings.json
│   ├── tasks.json
│   └── rules
└── .grok/
    ├── config.toml
    └── .env
```

Edit these files to change what gets created in new projects.

### Creating Project-Type Templates

Create specialized templates for different project types:

```bash
# In the extension directory
mkdir -p templates/web-app/.zed
mkdir -p templates/cli-tool/.zed
mkdir -p templates/library/.zed

# Customize each template
# Modify the script to use different templates based on flags
```

## Advanced Features

### Post-Setup Customization

After the script creates your project, you can:

1. **Add More Dependencies**
   ```bash
   cd my-project
   cargo add serde tokio thiserror
   ```

2. **Configure Git Remote**
   ```bash
   git remote add origin https://github.com/user/repo.git
   git push -u origin main
   ```

3. **Add API Credentials**
   ```bash
   # Edit .grok/.env
   vim .grok/.env
   # Add: X_API_KEY=your_key_here
   ```

4. **Customize Settings**
   ```bash
   # Edit project-specific Grok config
   vim .grok/config.toml
   ```

### Integration with CI/CD

The project structure is ready for CI/CD:

**GitHub Actions** (`.github/workflows/ci.yml`):
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --verbose
      - run: cargo test --verbose
      - run: cargo clippy -- -D warnings
```

### Multiple Projects

Create multiple projects quickly:

```bash
#!/bin/bash
# create_projects.sh

PROJECTS=(
    "backend-api"
    "frontend-cli"
    "shared-lib --lib"
    "data-processor"
)

for project in "${PROJECTS[@]}"; do
    ./setup_project.sh $project --path ~/workspace
done
```

## Troubleshooting

### Cargo Not Found

**Problem:** `cargo: command not found`

**Solution:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart shell or run:
source $HOME/.cargo/env
```

### Git Not Found

**Problem:** `git: command not found`

**Solution:**
```bash
# Windows: Install Git from https://git-scm.com/
# macOS: xcode-select --install
# Linux: sudo apt-get install git
```

### Permission Denied

**Problem:** `Permission denied` when running script

**Solution:**
```bash
chmod +x ~/.grok/extensions/project-setup-hook/scripts/setup_project.sh
```

### Project Already Exists

**Problem:** `Project directory already exists`

**Solution:**
```bash
# Remove the existing directory
rm -rf my-project

# Or choose a different name
./setup_project.sh my-project-v2
```

### jq Not Found

**Problem:** `jq: command not found`

**Solution:**
```bash
# macOS
brew install jq

# Ubuntu/Debian
sudo apt-get install jq

# Windows (Git Bash)
# Download from https://stedolan.github.io/jq/
```

## Best Practices

### Project Naming

✅ **Good names:**
- `my-awesome-app`
- `web_server`
- `data-processor`
- `cli-tool`

❌ **Bad names:**
- `MyAwesomeApp` (uppercase)
- `my awesome app` (spaces)
- `123-app` (starts with number)
- `my@app` (special characters)

### Before Starting

1. **Plan your project structure**
2. **Choose binary vs library**
3. **Decide on dependencies**
4. **Set up git remote** (GitHub, GitLab, etc.)
5. **Configure API keys** in `.grok/.env`

### After Creation

1. **Review Cargo.toml** - Remove unused dependencies
2. **Update README** - Add project-specific information
3. **Configure .gitignore** - Add any project-specific patterns
4. **Set up CI/CD** - Add GitHub Actions or other CI
5. **Test Grok CLI** - Ensure AI assistance works

## Integration Examples

### With GitHub

```bash
# Create project
./setup_project.sh my-app

cd my-app

# Create repo on GitHub, then:
git remote add origin https://github.com/user/my-app.git
git branch -M main
git push -u origin main
```

### With Docker

Add `Dockerfile` to your project:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/my-app /usr/local/bin/
CMD ["my-app"]
```

### With VS Code

The `.zed/` configuration can inspire `.vscode/` settings:

```bash
mkdir .vscode
# Add settings.json, launch.json, etc.
```

## Roadmap

Future enhancements:

- [ ] **Interactive mode** - Prompt for options interactively
- [ ] **More templates** - Web apps, CLI tools, services, etc.
- [ ] **Dependency wizard** - Choose common dependencies during setup
- [ ] **License selection** - Choose and add LICENSE file
- [ ] **README generator** - Generate README from project description
- [ ] **CI/CD templates** - Add GitHub Actions, GitLab CI, etc.
- [ ] **Docker integration** - Auto-generate Dockerfile
- [ ] **Workspace support** - Create Cargo workspaces with multiple crates
- [ ] **AI-assisted setup** - Let AI help configure the project

## Contributing

Contributions welcome! To add features:

1. Fork the repository
2. Create a feature branch
3. Add your changes
4. Test thoroughly
5. Submit a pull request

### Ideas for Contributions

- Additional templates (web, CLI, embedded, etc.)
- Windows PowerShell version of script
- Python version for better cross-platform support
- Integration with other editors (VS Code, IntelliJ)
- More comprehensive project scaffolding
- Interactive project wizard

## FAQ

### Q: Can I use this for non-Rust projects?

**A:** The script is designed for Rust, but you can adapt it. Fork the script and replace `cargo new` with your language's equivalent.

### Q: Does this work on Windows?

**A:** Yes, with Git Bash or WSL. Native Windows support (PowerShell) is planned.

### Q: Can I customize the templates?

**A:** Yes! Edit files in `~/.grok/extensions/project-setup-hook/templates/`

### Q: What if I don't use Zed editor?

**A:** Use `--no-zed` to skip Zed configuration. The `.zed/rules` file is still useful for AI context.

### Q: Do I need Grok CLI installed?

**A:** No, the script works standalone. But you'll want Grok CLI to use the AI features!

### Q: Can I create workspaces?

**A:** Not yet, but it's on the roadmap. For now, create individual projects.

### Q: How do I update the extension?

**A:** Copy the latest version from `examples/extensions/project-setup-hook/` to `~/.grok/extensions/`

## License

MIT License - See main [LICENSE](../../../LICENSE) file

## Author

john mcconnell <john.microtech@gmail.com>

## Acknowledgments

- Rust community for excellent tooling
- Zed editor team for great LSP integration
- Grok CLI contributors

---

**Ready to create your first project?**

```bash
~/.grok/extensions/project-setup-hook/scripts/setup_project.sh my-first-project
cd my-first-project
grok interactive
```

**Happy coding! 🦀🚀**