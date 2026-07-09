//! /init command — Initialize a new Grok project
//!
//! Creates a project-local `.grok/` directory and copies safe configuration
//! files from the user's global `~/.grok-cli/` directory.
//!
//! Copied (project-customizable):
//! - config.toml
//! - agents/
//! - skills/
//! - agents/rules/
//!
//! NOT copied (system-only):
//! - sessions/, logs/, audit/, memory.json, bayes_profile.json, chat_sessions/

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Files and folders that should be copied from the global config
const COPY_ITEMS: &[&str] = &[
    "config.toml",
    "agents",
    "skills",
    "agents/rules",
];

/// System-only folders that must never be copied into a project
const SKIP_ITEMS: &[&str] = &[
    "sessions",
    "logs",
    "audit",
    "memory.json",
    "bayes_profile.json",
    "chat_sessions",
];

/// Run the /init command in the current directory
pub fn run_init() -> Result<String> {
    let cwd = std::env::current_dir()?;
    let project_grok = cwd.join(".grok");

    // Check if .grok already exists
    if project_grok.exists() {
        return Ok(format!(
            "⚠️  `.grok/` already exists in {}\n\n\
             Use `grok init --force` (not yet implemented) to overwrite, \
             or manually edit the existing files.",
            cwd.display()
        ));
    }

    // Create .grok directory
    fs::create_dir_all(&project_grok)?;

    // Find global data directory (agents, skills, logs, etc.)
    // We use grok_data_dir() so we can copy from ~/.grok-cli on Windows
    // while still allowing config.toml to live in AppData\Roaming\grok-cli
    let global_dir = crate::config::grok_data_dir();

    let mut copied = Vec::new();
    let mut skipped = Vec::new();

    // Copy safe items from global config
    for item in COPY_ITEMS {
        let src = global_dir.join(item);
        let dst = project_grok.join(item);

        if src.exists() {
            // Skip if destination already exists (user may have created it manually)
            if dst.exists() {
                skipped.push(format!("{} (already exists)", item));
                continue;
            }

            if src.is_dir() {
                copy_dir_recursive(&src, &dst)?;
            } else {
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dst)?;
            }
            copied.push(item.to_string());
        } else {
            skipped.push(format!("{} (not found in global)", item));
        }
    }

    // Always add .grok to .gitignore
    update_gitignore(&cwd)?;

    let mut msg = format!(
        "✅ Grok project initialized in {}\n\n",
        cwd.display()
    );

    if !copied.is_empty() {
        msg.push_str("Copied from global config:\n");
        for item in &copied {
            msg.push_str(&format!("  • {}\n", item));
        }
    }

    if !skipped.is_empty() {
        msg.push_str("\nSkipped (system-only or already present):\n");
        for item in &skipped {
            msg.push_str(&format!("  • {}\n", item));
        }
    }

    msg.push_str("\nProject `.grok/` is ready. You can now customize config per-project.");

    Ok(msg)
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // Skip system-only items
        if let Some(name) = src_path.file_name().and_then(|n| n.to_str()) {
            if SKIP_ITEMS.contains(&name) {
                continue;
            }
        }

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Add `.grok/` to .gitignore if not already present
fn update_gitignore(cwd: &PathBuf) -> Result<()> {
    let gitignore = cwd.join(".gitignore");

    let line = ".grok/\n";

    if gitignore.exists() {
        let content = fs::read_to_string(&gitignore)?;
        if !content.contains(".grok/") {
            fs::write(&gitignore, format!("{}{}", content.trim_end(), line))?;
        }
    } else {
        fs::write(&gitignore, line)?;
    }

    Ok(())
}
