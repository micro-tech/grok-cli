//! /init command — Initialize a new Grok project
//!
//! Creates a recommended project structure with:
//! - .grok/ directory
//! - config.toml
//| - context.md (project context)
//! - .gitignore updates
//! - README with Grok usage tips

use anyhow::Result;
use std::fs;
use std::path::Path;

/// Run the /init command in the current directory
pub fn run_init() -> Result<String> {
    let cwd = std::env::current_dir()?;

    // Create .grok directory
    let grok_dir = cwd.join(".grok");
    fs::create_dir_all(&grok_dir)?;

    // Create basic config
    let config_path = grok_dir.join("config.toml");
    if !config_path.exists() {
        fs::write(
            &config_path,
            r#"# Grok CLI project configuration
# See https://github.com/grok-cli/grok-cli for docs

[project]
name = "my-project"
description = "A new Grok-powered project"
"#,
        )?;
    }

    // Create context.md if it doesn't exist
    let context_path = cwd.join("context.md");
    if !context_path.exists() {
        fs::write(
            &context_path,
            r#"# Project Context

This file is automatically loaded by Grok CLI.
Add any project-specific instructions, architecture notes, or coding guidelines here.

## Key Files
- `src/` — main source code
- `tests/` — test suite

## Notes
- Use Rust 2021 edition
- Prefer `anyhow` for error handling
"#,
        )?;
    }

    // Add .grok to .gitignore if needed
    let gitignore = cwd.join(".gitignore");
    if gitignore.exists() {
        let content = fs::read_to_string(&gitignore)?;
        if !content.contains(".grok/") {
            fs::write(&gitignore, format!("{}\n.grok/\n", content.trim_end()))?;
        }
    } else {
        fs::write(&gitignore, ".grok/\n")?;
    }

    Ok(format!(
        "✅ Grok project initialized in {}\n\n\
         Created:\n\
         • .grok/config.toml\n\
         • context.md\n\
         • Updated .gitignore\n\n\
         You can now use Grok CLI in this project!",
        cwd.display()
    ))
}
