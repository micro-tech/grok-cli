#!/bin/bash
# Project Setup Script for Grok CLI
# Creates a new Rust project with git, .zed, and .grok configurations

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to show usage
usage() {
    cat << EOF
Usage: $0 <project-name> [options]

Creates a new Rust project with complete Grok CLI setup.

Arguments:
    project-name    Name of the project (required)

Options:
    --path PATH     Parent directory for the project (default: current directory)
    --bin           Create a binary project (default)
    --lib           Create a library project
    --no-git        Skip git initialization
    --no-zed        Skip .zed directory setup
    --no-grok       Skip .grok directory setup
    --help          Show this help message

Examples:
    $0 my-awesome-project
    $0 my-lib --lib --path ~/projects
    $0 quick-tool --bin --no-git

EOF
}

# Parse arguments
PROJECT_NAME=""
PROJECT_PATH="."
PROJECT_TYPE="--bin"
INIT_GIT=true
SETUP_ZED=true
SETUP_GROK=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            usage
            exit 0
            ;;
        --path)
            PROJECT_PATH="$2"
            shift 2
            ;;
        --bin)
            PROJECT_TYPE="--bin"
            shift
            ;;
        --lib)
            PROJECT_TYPE="--lib"
            shift
            ;;
        --no-git)
            INIT_GIT=false
            shift
            ;;
        --no-zed)
            SETUP_ZED=false
            shift
            ;;
        --no-grok)
            SETUP_GROK=false
            shift
            ;;
        -*)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
        *)
            if [ -z "$PROJECT_NAME" ]; then
                PROJECT_NAME="$1"
            else
                print_error "Multiple project names specified"
                usage
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate project name
if [ -z "$PROJECT_NAME" ]; then
    print_error "Project name is required"
    usage
    exit 1
fi

# Validate project name format (Rust package naming rules)
if ! echo "$PROJECT_NAME" | grep -qE '^[a-z][a-z0-9_-]*$'; then
    print_error "Invalid project name: '$PROJECT_NAME'"
    echo "Project name must:"
    echo "  - Start with a lowercase letter"
    echo "  - Contain only lowercase letters, numbers, hyphens, and underscores"
    exit 1
fi

# Get extension directory (where this script is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXTENSION_DIR="$(dirname "$SCRIPT_DIR")"
TEMPLATES_DIR="$EXTENSION_DIR/templates"

print_info "Setting up Rust project: $PROJECT_NAME"
echo ""

# Create project path if it doesn't exist
if [ "$PROJECT_PATH" != "." ]; then
    mkdir -p "$PROJECT_PATH"
    PROJECT_PATH="$(cd "$PROJECT_PATH" && pwd)"
fi

# Full project directory path
FULL_PROJECT_PATH="$PROJECT_PATH/$PROJECT_NAME"

# Check if project already exists
if [ -d "$FULL_PROJECT_PATH" ]; then
    print_error "Project directory already exists: $FULL_PROJECT_PATH"
    exit 1
fi

# Step 1: Create Rust project with Cargo
print_info "Creating Rust project with cargo..."
cd "$PROJECT_PATH"

if cargo new "$PROJECT_NAME" $PROJECT_TYPE; then
    print_success "Cargo project created"
else
    print_error "Failed to create cargo project"
    exit 1
fi

cd "$PROJECT_NAME"

# Step 2: Initialize Git
if [ "$INIT_GIT" = true ]; then
    print_info "Initializing git repository..."

    if [ -d ".git" ]; then
        print_warning "Git repository already initialized by cargo"
    else
        if git init; then
            print_success "Git repository initialized"
        else
            print_warning "Failed to initialize git repository"
        fi
    fi

    # Create initial commit
    git add .
    git commit -m "Initial commit: Rust project setup with Grok CLI" > /dev/null 2>&1 || true
    print_success "Initial commit created"
fi

# Step 3: Set up .zed directory
if [ "$SETUP_ZED" = true ]; then
    print_info "Setting up .zed directory..."

    if [ -d "$TEMPLATES_DIR/.zed" ]; then
        cp -r "$TEMPLATES_DIR/.zed" .
        print_success ".zed directory created from template"
    else
        mkdir -p .zed

        # Create default settings.json
        cat > .zed/settings.json << 'ZEDEOF'
{
  "languages": {
    "Rust": {
      "format_on_save": "on",
      "formatter": "language_server"
    }
  },
  "lsp": {
    "rust-analyzer": {
      "initialization_options": {
        "check": {
          "command": "clippy"
        }
      }
    }
  }
}
ZEDEOF

        # Create default tasks.json
        cat > .zed/tasks.json << 'ZEDEOF'
[
  {
    "label": "cargo build",
    "command": "cargo",
    "args": ["build"]
  },
  {
    "label": "cargo run",
    "command": "cargo",
    "args": ["run"]
  },
  {
    "label": "cargo test",
    "command": "cargo",
    "args": ["test"]
  },
  {
    "label": "cargo clippy",
    "command": "cargo",
    "args": ["clippy"]
  }
]
ZEDEOF

        # Create rules file
        cat > .zed/rules << 'ZEDEOF'
# Zed Editor Rules for AI Assistant

## Project Type
This is a Rust project using Cargo for build management.

## Code Style
- Follow Rust 2021 edition best practices
- Use `rustfmt` for formatting
- Run `clippy` before committing
- Write doc comments for public APIs
- Include unit tests for new functionality

## Testing
- Place unit tests in the same file using `#[cfg(test)]`
- Place integration tests in `tests/` directory
- Run `cargo test` before committing

## Error Handling
- Use `Result<T, E>` for recoverable errors
- Use `panic!` only for unrecoverable errors
- Create custom error types using `thiserror` for libraries
- Use `anyhow` for applications

## Dependencies
- Keep dependencies minimal and well-maintained
- Update dependencies regularly
- Document why each dependency is needed
ZEDEOF

        print_success ".zed directory created with defaults"
    fi
fi

# Step 4: Set up .grok directory
if [ "$SETUP_GROK" = true ]; then
    print_info "Setting up .grok directory..."

    if [ -d "$TEMPLATES_DIR/.grok" ]; then
        cp -r "$TEMPLATES_DIR/.grok" .
        print_success ".grok directory created from template"
    else
        mkdir -p .grok

        # Create default config.toml
        cat > .grok/config.toml << 'GROKEOF'
# Grok CLI Project Configuration

[acp]
enabled = true
bind_host = "127.0.0.1"
protocol_version = "2024-11-05"
dev_mode = false
max_tool_loop_iterations = 25

[context]
# Add project-specific context files
files = [".zed/rules"]

[experimental.extensions]
enabled = true
GROKEOF

        # Create .env file
        cat > .grok/.env << 'GROKEOF'
# Grok CLI Environment Variables
# Add your API keys and secrets here (this file is git-ignored)

# X_API_KEY=your_api_key_here
# X_API_SECRET=your_api_secret_here
GROKEOF

        print_success ".grok directory created with defaults"
    fi

    # Ensure .grok/.env is in .gitignore
    if ! grep -q "^\.grok/\.env$" .gitignore 2>/dev/null; then
        echo "" >> .gitignore
        echo "# Grok CLI environment variables" >> .gitignore
        echo ".grok/.env" >> .gitignore
        print_success "Added .grok/.env to .gitignore"
    fi
fi

# Step 5: Create initial project structure (optional enhancements)
print_info "Enhancing project structure..."

# Update Cargo.toml with better metadata
if [ "$PROJECT_TYPE" = "--bin" ]; then
    # Add common dependencies for binary projects
    cat >> Cargo.toml << 'CARGOEOF'

# Common dependencies
[dependencies]
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }

[profile.release]
strip = true
lto = true
codegen-units = 1
CARGOEOF
    print_success "Added common dependencies for binary project"
fi

# Create docs directory
mkdir -p docs
cat > docs/README.md << 'DOCSEOF'
# Documentation

## Getting Started

Add your project documentation here.

## Development

```bash
# Build the project
cargo build

# Run the project
cargo run

# Run tests
cargo test

# Run clippy
cargo clippy
```

## Contributing

Add contribution guidelines here.
DOCSEOF

# Create examples directory for lib projects
if [ "$PROJECT_TYPE" = "--lib" ]; then
    mkdir -p examples
    print_success "Created examples directory"
fi

# Step 6: Final git commit (if git enabled)
if [ "$INIT_GIT" = true ]; then
    git add .
    git commit -m "Add Grok CLI configuration and project structure" > /dev/null 2>&1 || true
    print_success "Configuration committed to git"
fi

# Print success summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_success "Project setup complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Project: $PROJECT_NAME"
echo "Location: $FULL_PROJECT_PATH"
echo "Type: ${PROJECT_TYPE#--}"
echo ""
echo "What's included:"
[ "$INIT_GIT" = true ] && echo "  ✓ Git repository initialized"
[ "$SETUP_ZED" = true ] && echo "  ✓ .zed directory with editor settings"
[ "$SETUP_GROK" = true ] && echo "  ✓ .grok directory with Grok CLI config"
echo "  ✓ Cargo.toml with common dependencies"
echo "  ✓ Documentation structure"
echo ""
echo "Next steps:"
echo "  1. cd $PROJECT_NAME"
echo "  2. Add your X API credentials to .grok/.env"
echo "  3. cargo build"
echo "  4. grok interactive  # Use AI assistance in your project!"
echo ""
print_info "Happy coding! 🦀"
