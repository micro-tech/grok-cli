# Project Setup Hook - Quick Start Guide

## 🚀 Create a New Rust Project in Seconds!

This extension automates creating fully-configured Rust projects with Grok CLI integration.

## Installation (One-Time Setup)

```bash
# 1. Copy extension to your Grok directory
cp -r examples/extensions/project-setup-hook ~/.grok/extensions/

# 2. Make scripts executable
chmod +x ~/.grok/extensions/project-setup-hook/scripts/*.sh

# 3. Create a convenient alias (optional but recommended)
echo 'alias grok-new="~/.grok/extensions/project-setup-hook/scripts/setup_project.sh"' >> ~/.bashrc
source ~/.bashrc
```

## Usage

### Basic: Create a Binary Project

```bash
grok-new my-awesome-app
cd my-awesome-app
cargo run
```

That's it! You now have a complete Rust project with:
- ✅ Cargo configuration
- ✅ Git repository (with initial commit)
- ✅ Zed editor settings
- ✅ Grok CLI configuration
- ✅ Common dependencies pre-added
- ✅ Documentation structure

### Create a Library

```bash
grok-new my-library --lib
```

### Create in Specific Location

```bash
grok-new web-server --path ~/projects/rust
```

### Minimal Setup (No Git or Editor Config)

```bash
grok-new quick-tool --no-git --no-zed --no-grok
```

## What You Get

Every project includes:

```
my-project/
├── .git/              # Git repository
├── .grok/             # Grok CLI config + .env for API keys
├── .zed/              # Editor settings + AI rules
├── Cargo.toml         # With common dependencies
├── src/main.rs        # Your code starts here
└── docs/              # Documentation
```

## Next Steps After Creation

```bash
cd my-project

# 1. Add your API key
echo 'X_API_KEY=your_key_here' >> .grok/.env

# 2. Build and run
cargo build
cargo run

# 3. Start using AI assistance!
grok interactive
> Add a CLI parser using clap
> Create tests for the main function
> Set up error handling with anyhow
```

## Common Workflows

### Web Application

```bash
grok-new my-web-app
cd my-web-app
cargo add axum tower
grok interactive
> Create a basic web server with health check endpoint
```

### CLI Tool

```bash
grok-new my-cli
cd my-cli
cargo add clap colored
grok interactive
> Create a CLI that accepts --input and --output flags
```

### Library with Examples

```bash
grok-new my-utils --lib
cd my-utils
grok interactive
> Create a string utility module with tests and examples
```

## Using with AI

Once your project is created, the AI understands your project structure:

```bash
grok interactive

> What's the project structure?
> Add proper error handling
> Create a test for the main function
> Optimize the Cargo.toml dependencies
> Add GitHub Actions CI/CD
```

The AI has access to:
- `.zed/rules` - Your code style guidelines
- `.grok/config.toml` - Project settings
- All project files

## Tips & Tricks

### 1. Create Multiple Projects Quickly

```bash
for app in auth-service api-gateway frontend-cli; do
    grok-new $app --path ~/workspace
done
```

### 2. Template Customization

Edit the templates to match your preferences:
```bash
vim ~/.grok/extensions/project-setup-hook/templates/.zed/rules
vim ~/.grok/extensions/project-setup-hook/templates/.grok/config.toml
```

All future projects will use your custom templates!

### 3. Quick Project + GitHub

```bash
# Create project
grok-new my-app
cd my-app

# Push to GitHub (after creating repo on GitHub)
git remote add origin https://github.com/username/my-app.git
git push -u origin main
```

### 4. Add to PATH

For system-wide access:
```bash
sudo ln -s ~/.grok/extensions/project-setup-hook/scripts/setup_project.sh /usr/local/bin/grok-new
```

Now use `grok-new` from anywhere!

## Troubleshooting

### Script Not Found
```bash
# Check if installed
ls ~/.grok/extensions/project-setup-hook/

# If not, install it:
cp -r examples/extensions/project-setup-hook ~/.grok/extensions/
```

### Permission Denied
```bash
chmod +x ~/.grok/extensions/project-setup-hook/scripts/*.sh
```

### Cargo Not Found
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Project Name Invalid
Project names must:
- Start with lowercase letter
- Use only: lowercase, numbers, hyphens, underscores

✅ Good: `my-app`, `web_server`, `tool-2024`  
❌ Bad: `MyApp`, `my app`, `123-tool`, `my@app`

## Command Reference

```bash
# Basic usage
grok-new <project-name>

# Options
--path PATH      # Custom parent directory
--bin            # Binary project (default)
--lib            # Library project
--no-git         # Skip git init
--no-zed         # Skip .zed directory
--no-grok        # Skip .grok directory
--help           # Show help

# Examples
grok-new my-app
grok-new my-lib --lib
grok-new tool --path ~/workspace
grok-new test --no-git --no-zed
```

## Full Example Session

```bash
# Create a web API project
$ grok-new weather-api --path ~/projects
[INFO] Setting up Rust project: weather-api
[✓] Cargo project created
[✓] Git repository initialized
[✓] .zed directory created
[✓] .grok directory created
[✓] Project setup complete!

# Navigate and start coding
$ cd ~/projects/weather-api
$ ls -la
.git/  .gitignore  .grok/  .zed/  Cargo.toml  src/  docs/

# Add dependencies
$ cargo add axum tokio serde reqwest

# Start AI-assisted development
$ grok interactive
> Create a REST API with endpoints for weather data
> Add error handling using anyhow
> Create tests for all endpoints
> Add OpenAPI documentation

# Build and run
$ cargo build --release
$ ./target/release/weather-api
```

## Resources

- **Full Documentation**: See `README.md` in the extension directory
- **Templates**: `~/.grok/extensions/project-setup-hook/templates/`
- **Script Source**: `~/.grok/extensions/project-setup-hook/scripts/setup_project.sh`

## What's Next?

1. ✅ Install the extension
2. ✅ Create your first project: `grok-new my-first-app`
3. ✅ Add API key to `.grok/.env`
4. ✅ Start coding with AI: `grok interactive`

**You're ready to build amazing things! 🦀🚀**

---

Need help? Check the full README or ask in the Grok CLI community!