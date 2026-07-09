//! `grok init` — Initialize a Grok project in the current directory.
//!
//! Creates a `.grok/` folder and populates it by copying safe configuration
//! files from the user's global Grok directories.
//!
//! # Windows source directories
//!
//! | What          | Location                                          |
//! |---------------|---------------------------------------------------|
//! | `config.toml` | `%APPDATA%\grok-cli`  (via `grok_config_dir()`)   |
//! | `agents/`     | `~\.grok-cli`         (via `grok_data_dir()`)     |
//! | `skills/`     | `~\.grok-cli`                                     |
//! | `rules/`      | `~\.grok-cli`                                     |
//!
//! # NOT copied (system-only / runtime state)
//! `sessions/`, `logs/`, `audit/`, `traces/`, `memory.json`, `memory.md`,
//! `bayes_profile.json`, `bayes_profile.json_bak`, `chat_sessions/`,
//! `session_dna.json`, `icon.svg`, `context.md`
//!
//! # Force flag
//! * `force = false` — skip files/directories that already exist in `.grok/`
//! * `force = true`  — overwrite any conflicting files/directories

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Items copied from the global CONFIG directory (`%APPDATA%\grok-cli` on Windows).
/// `config.toml` lives here, not in the data directory.
const CONFIG_ITEMS: &[&str] = &["config.toml"];

/// Items copied from the global DATA directory (`~\.grok-cli` on Windows).
const DATA_ITEMS: &[&str] = &["agents", "skills", "rules"];

/// Names that must never be copied into a project `.grok/` — these are
/// system-only runtime files that belong only in the global directory.
const SKIP_ITEMS: &[&str] = &[
    "sessions",
    "logs",
    "audit",
    "traces",
    "memory.json",
    "memory.md",
    "bayes_profile.json",
    "bayes_profile.json_bak",
    "chat_sessions",
    "session_dna.json",
    "icon.svg",
    "context.md",
];

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Run `grok init [--force]` in the current working directory.
///
/// * `force = false` — per-item check; files that already exist are left alone.
/// * `force = true`  — overwrite any conflicting files or directories.
pub fn run_init(force: bool) -> Result<String> {
    let cwd = std::env::current_dir()?;
    let project_grok = cwd.join(".grok");

    // Always ensure the .grok dir exists (idempotent).
    fs::create_dir_all(&project_grok)?;

    // Two source roots on Windows:
    //   config_dir  → %APPDATA%\grok-cli         (holds config.toml)
    //   data_dir    → C:\Users\<user>\.grok-cli  (holds agents, skills, rules)
    let config_dir = crate::config::grok_config_dir();
    let data_dir = crate::config::grok_data_dir();

    let mut copied: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    // Copy config.toml from the CONFIG dir (AppData on Windows)
    for item in CONFIG_ITEMS {
        copy_item(
            item,
            &config_dir,
            &project_grok,
            force,
            &mut copied,
            &mut skipped,
            &mut missing,
        )?;
    }

    // Copy agents/, skills/, rules/ from the DATA dir (~/.grok-cli)
    for item in DATA_ITEMS {
        copy_item(
            item,
            &data_dir,
            &project_grok,
            force,
            &mut copied,
            &mut skipped,
            &mut missing,
        )?;
    }

    // Always make sure .grok/ is git-ignored.
    update_gitignore(&cwd)?;

    // ── Build result message ──────────────────────────────────────────────────
    let action = if force {
        "re-initialized (--force)"
    } else {
        "initialized"
    };
    let mut msg = format!("✅  Grok project {} in {}\n", action, cwd.display());

    if !copied.is_empty() {
        msg.push_str("\nCopied from global config:\n");
        for item in &copied {
            msg.push_str(&format!("  ✓  {}\n", item));
        }
    }

    if !skipped.is_empty() {
        msg.push_str("\nSkipped (already present — use --force to overwrite):\n");
        for item in &skipped {
            msg.push_str(&format!("  •  {}\n", item));
        }
    }

    if !missing.is_empty() {
        msg.push_str("\nNot found in global config (nothing to copy):\n");
        for item in &missing {
            msg.push_str(&format!("  ⚠  {}\n", item));
        }
    }

    msg.push_str("\n`.grok/` is ready — you can customize settings per-project.\n");
    msg.push_str("Tip: `.grok/` is already listed in .gitignore.\n");

    Ok(msg)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Copy one file or directory from `src_base/item` to `dst_base/item`.
///
/// Records the outcome in one of the three output vectors.
fn copy_item(
    item: &str,
    src_base: &PathBuf,
    dst_base: &PathBuf,
    force: bool,
    copied: &mut Vec<String>,
    skipped: &mut Vec<String>,
    missing: &mut Vec<String>,
) -> Result<()> {
    let src = src_base.join(item);
    let dst = dst_base.join(item);

    if !src.exists() {
        missing.push(item.to_string());
        return Ok(());
    }

    // Per-item skip when not forcing
    if dst.exists() && !force {
        skipped.push(item.to_string());
        return Ok(());
    }

    if src.is_dir() {
        // In force mode, wipe the destination first so we get a clean copy.
        if dst.exists() && force {
            fs::remove_dir_all(&dst)?;
        }
        copy_dir_recursive(&src, &dst)?;
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&src, &dst)?;
    }

    copied.push(item.to_string());
    Ok(())
}

/// Recursively copy `src` into `dst`, skipping system-only filenames.
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // Never copy system-only items regardless of nesting depth.
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

/// Ensure `.grok/` appears in the project's `.gitignore`.
fn update_gitignore(cwd: &PathBuf) -> Result<()> {
    let gitignore = cwd.join(".gitignore");
    let entry = ".grok/\n";

    if gitignore.exists() {
        let content = fs::read_to_string(&gitignore)?;
        if !content.contains(".grok/") {
            let trimmed = content.trim_end().to_string();
            fs::write(&gitignore, format!("{}\n{}", trimmed, entry))?;
        }
    } else {
        fs::write(&gitignore, entry)?;
    }

    Ok(())
}
