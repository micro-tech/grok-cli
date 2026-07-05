//! Sandbox command — safe isolated playground for grok-cli.
//!
//! `grok sandbox` creates a temporary workspace pre-populated with sample
//! Rust source files and starts an interactive session scoped **only** to that
//! directory.  Because `start_interactive_mode` anchors the `SecurityPolicy`
//! to `env::current_dir()`, simply changing the CWD to the sandbox is enough
//! to restrict all file-tool operations to the playground.
//!
//! Flags
//! -----
//! - `--keep`       — do not delete the sandbox dir on exit (prints path)
//! - `--dir <path>` — use an existing directory instead of creating a new one
//! - `--dry-run`    — display a note in the banner (full wiring: task 219.2)

#![allow(deprecated)]

use anyhow::{Context, Result, anyhow};
use colored::Colorize;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::config::Config;
use crate::display::interactive::{InteractiveConfig, PromptStyle, start_interactive_mode};

// ── Sample workspace files ────────────────────────────────────────────────────

struct SandboxFile {
    path: &'static str,
    content: &'static str,
}

const SANDBOX_FILES: &[SandboxFile] = &[
    SandboxFile {
        path: "Cargo.toml",
        content: r#"[package]
name = "grok-sandbox"
version = "0.1.0"
edition = "2021"
description = "A safe playground workspace for grok-cli experiments."

[dependencies]

[dev-dependencies]
"#,
    },
    SandboxFile {
        path: ".gitignore",
        content: "/target\nCargo.lock\n",
    },
    SandboxFile {
        path: "README.md",
        content: r#"# grok-sandbox

This is a **safe playground** created by `grok sandbox`.

Grok-CLI is scoped to this directory only — it cannot read or write files
outside this folder.  Experiment freely!

## Files

| File | Purpose |
|---|---|
| `src/main.rs` | Entry point with a small demo program |
| `src/lib.rs` | Library with a few functions to explore |
| `src/utils.rs` | Utility helpers (try asking Grok to improve them) |
| `tests/integration_test.rs` | Integration tests (try asking Grok to add more) |
| `data/sample.json` | Sample JSON data |
| `data/notes.txt` | Plain-text scratch pad |

## Ideas to try

- "Explain the code in src/lib.rs"
- "Add a function to utils.rs that reverses a string"
- "Write tests for the greet function"
- "Find any bugs in this project"
- "Refactor main.rs to use the library functions"
"#,
    },
    SandboxFile {
        path: "src/main.rs",
        content: r#"use grok_sandbox::{greet, add, fibonacci};

fn main() {
    println!("{}", greet("world"));
    println!("3 + 4 = {}", add(3, 4));
    println!("fib(10) = {}", fibonacci(10));
}
"#,
    },
    SandboxFile {
        path: "src/lib.rs",
        content: r#"//! grok-sandbox library
//!
//! A small collection of functions for the grok-cli sandbox playground.
//! Ask Grok to explain, improve, test, or refactor anything here.

pub mod utils;

/// Return a greeting string.
///
/// # Examples
/// ```
/// assert_eq!(grok_sandbox::greet("Alice"), "Hello, Alice!");
/// ```
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Add two integers.
pub fn add(a: i64, b: i64) -> i64 {
    a + b
}

/// Compute the nth Fibonacci number (recursive, intentionally naive).
///
/// Try asking Grok to optimise this with memoisation!
pub fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

/// Count words in a string (splits on whitespace).
pub fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Reverse the characters of a string.
// TODO: handle multi-byte Unicode properly
pub fn reverse(s: &str) -> String {
    s.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("world"), "Hello, world!");
    }

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
    }

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(0), 0);
        assert_eq!(fibonacci(1), 1);
        assert_eq!(fibonacci(10), 55);
    }

    #[test]
    fn test_word_count() {
        assert_eq!(word_count("hello world"), 2);
        assert_eq!(word_count("  spaces  "), 1);
        assert_eq!(word_count(""), 0);
    }
}
"#,
    },
    SandboxFile {
        path: "src/utils.rs",
        content: r#"//! Utility helpers for the grok-sandbox.
//!
//! These are intentionally simple so Grok has something to explore and improve.

/// Clamp a value between min and max (inclusive).
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Check whether a string is a palindrome (ASCII, case-insensitive).
pub fn is_palindrome(s: &str) -> bool {
    let clean: String = s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    clean == clean.chars().rev().collect::<String>()
}

/// Flatten a nested list of integers one level deep.
pub fn flatten(nested: Vec<Vec<i32>>) -> Vec<i32> {
    nested.into_iter().flatten().collect()
}

// TODO: add more utilities — ask Grok for suggestions!
"#,
    },
    SandboxFile {
        path: "tests/integration_test.rs",
        content: r#"//! Integration tests for the grok-sandbox library.

use grok_sandbox::{greet, add, fibonacci, word_count};
use grok_sandbox::utils::{clamp, is_palindrome, flatten};

#[test]
fn greet_returns_expected_string() {
    assert_eq!(greet("Grok"), "Hello, Grok!");
}

#[test]
fn add_handles_negatives() {
    assert_eq!(add(-5, 3), -2);
}

#[test]
fn fibonacci_sequence_spot_check() {
    let expected = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
    for (i, &val) in expected.iter().enumerate() {
        assert_eq!(fibonacci(i as u32), val, "fib({}) should be {}", i, val);
    }
}

#[test]
fn word_count_multi_line() {
    assert_eq!(word_count("one\ntwo\nthree"), 3);
}

#[test]
fn clamp_within_bounds() {
    assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
    assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
    assert_eq!(clamp(11.0, 0.0, 10.0), 10.0);
}

#[test]
fn palindrome_detection() {
    assert!(is_palindrome("racecar"));
    assert!(is_palindrome("A man a plan a canal Panama"));
    assert!(!is_palindrome("hello"));
}

#[test]
fn flatten_nested() {
    assert_eq!(flatten(vec![vec![1, 2], vec![3], vec![4, 5]]), vec![1, 2, 3, 4, 5]);
}
"#,
    },
    SandboxFile {
        path: "data/sample.json",
        content: r#"{
  "project": "grok-sandbox",
  "description": "Sample JSON data for the grok-cli playground.",
  "items": [
    { "id": 1, "name": "Alpha",   "value": 42,   "active": true  },
    { "id": 2, "name": "Beta",    "value": 17,   "active": false },
    { "id": 3, "name": "Gamma",   "value": 99,   "active": true  },
    { "id": 4, "name": "Delta",   "value": 3,    "active": true  },
    { "id": 5, "name": "Epsilon", "value": 256,  "active": false }
  ],
  "metadata": {
    "created": "2026-07-04",
    "version": "1.0",
    "tags": ["sandbox", "demo", "grok"]
  }
}
"#,
    },
    SandboxFile {
        path: "data/notes.txt",
        content: r#"Grok-CLI Sandbox Notes
======================

This is a plain-text scratch pad. Use it however you like.

Try asking Grok:
  - "Summarise the Rust files in this project"
  - "What does the fibonacci function do and how could it be improved?"
  - "Parse data/sample.json and tell me the total value of active items"
  - "Add error handling to src/main.rs"

Happy experimenting!
"#,
    },
];

// ── SandboxWorkspace ──────────────────────────────────────────────────────────

/// An isolated temporary workspace for safe grok-cli experimentation.
///
/// Deletes itself on `Drop` unless `keep` is `true`.
pub struct SandboxWorkspace {
    pub path: PathBuf,
    keep: bool,
}

impl SandboxWorkspace {
    /// Create a new sandbox in `std::env::temp_dir()` and populate it with
    /// sample files.  Returns an error if any file cannot be created.
    pub fn create(keep: bool) -> Result<Self> {
        let id = uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("tmp")
            .to_string();
        let dir = std::env::temp_dir().join(format!("grok-sandbox-{}", id));
        Self::populate(&dir, keep)
    }

    /// Use an existing directory as the sandbox (no sample files written).
    pub fn from_existing(path: PathBuf, keep: bool) -> Result<Self> {
        if !path.exists() {
            return Err(anyhow!(
                "Sandbox directory does not exist: {}",
                path.display()
            ));
        }
        info!(path = %path.display(), "Using existing directory as sandbox");
        Ok(Self { path, keep })
    }

    /// Create a sandbox at a specific path with sample files.
    pub fn create_at(path: PathBuf, keep: bool) -> Result<Self> {
        Self::populate(&path, keep)
    }

    fn populate(dir: &Path, keep: bool) -> Result<Self> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create sandbox directory: {}", dir.display()))?;

        for file in SANDBOX_FILES {
            let target = dir.join(file.path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory for {}", file.path))?;
            }
            std::fs::write(&target, file.content)
                .with_context(|| format!("Failed to write sandbox file: {}", file.path))?;
        }

        info!(
            path = %dir.display(),
            files = SANDBOX_FILES.len(),
            "Sandbox workspace created"
        );

        Ok(Self {
            path: dir.to_path_buf(),
            keep,
        })
    }

    /// Return the list of files that would be written (for testing).
    pub fn file_paths() -> Vec<&'static str> {
        SANDBOX_FILES.iter().map(|f| f.path).collect()
    }
}

impl Drop for SandboxWorkspace {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        if let Err(e) = std::fs::remove_dir_all(&self.path) {
            // Best-effort cleanup — warn but don't panic.
            warn!(path = %self.path.display(), error = %e, "Sandbox cleanup failed");
        } else {
            info!(path = %self.path.display(), "Sandbox cleaned up");
        }
    }
}

// ── handle_sandbox ────────────────────────────────────────────────────────────

/// Run the `grok sandbox` command.
///
/// 1. Creates (or opens) the sandbox workspace.
/// 2. Changes `env::current_dir` to the sandbox so `SecurityPolicy` auto-scopes.
/// 3. Prints a sandbox welcome banner.
/// 4. Starts the normal interactive session.
/// 5. On exit the workspace is deleted (unless `--keep` was passed).
pub async fn handle_sandbox(
    keep: bool,
    dir: Option<PathBuf>,
    dry_run: bool,
    api_key: &str,
    model: &str,
    config: &Config,
) -> Result<()> {
    // ── Build workspace ───────────────────────────────────────────────────────
    let workspace = match dir {
        Some(path) if path.exists() => SandboxWorkspace::from_existing(path, keep)?,
        Some(path) => SandboxWorkspace::create_at(path, keep)?,
        None => SandboxWorkspace::create(keep)?,
    };

    let sandbox_path = workspace.path.clone();

    // ── Change CWD so SecurityPolicy scopes to sandbox ────────────────────────
    std::env::set_current_dir(&sandbox_path).with_context(|| {
        format!(
            "Failed to change directory to sandbox: {}",
            sandbox_path.display()
        )
    })?;

    // ── Print sandbox banner ──────────────────────────────────────────────────
    print_sandbox_banner(&sandbox_path, keep, dry_run);

    // ── Start interactive session ─────────────────────────────────────────────
    let interactive_config = InteractiveConfig {
        show_banner: false, // We printed our own banner above
        show_tips: false,
        show_status: true,
        auto_save_session: false,
        prompt_style: PromptStyle::Rich,
        check_directory: false, // Already validated
    };

    start_interactive_mode(api_key, model, config, interactive_config).await?;

    // workspace is dropped here → cleanup happens automatically via Drop
    drop(workspace);

    Ok(())
}

fn print_sandbox_banner(path: &Path, keep: bool, dry_run: bool) {
    println!();
    println!(
        "{}",
        "╔══════════════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║          🧪  Grok-CLI Sandbox Mode               ║".bright_cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════╝".bright_cyan()
    );
    println!();
    println!(
        "  {} {}",
        "Workspace:".bright_white().bold(),
        path.display().to_string().bright_yellow()
    );
    println!(
        "  {} {}",
        "Scope:    ".bright_white().bold(),
        "All file operations restricted to this directory".green()
    );
    if dry_run {
        println!(
            "  {} {}",
            "Mode:     ".bright_white().bold(),
            "DRY-RUN (note: full wiring in task 219.2)".bright_magenta()
        );
    }
    if keep {
        println!(
            "  {} {}",
            "Cleanup:  ".bright_white().bold(),
            "Sandbox directory will be KEPT after exit".bright_yellow()
        );
    } else {
        println!(
            "  {} {}",
            "Cleanup:  ".bright_white().bold(),
            "Sandbox directory will be deleted on exit".dimmed()
        );
    }
    println!();
    println!(
        "  {}",
        "Sample files ready: src/main.rs, src/lib.rs, src/utils.rs,".dimmed()
    );
    println!(
        "  {}",
        "  tests/integration_test.rs, data/sample.json, data/notes.txt".dimmed()
    );
    println!();
    println!(
        "  {}",
        "Try: \"explain src/lib.rs\" or \"add tests for the fibonacci function\"".bright_white()
    );
    println!();
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_creates_all_expected_files() {
        let ws = SandboxWorkspace::create(false).expect("sandbox creation should succeed");
        let path = ws.path.clone();

        for file in SANDBOX_FILES {
            let target = path.join(file.path);
            assert!(
                target.exists(),
                "Expected sandbox file to exist: {}",
                file.path
            );
            let content =
                std::fs::read_to_string(&target).expect("Should be able to read sandbox file");
            assert!(
                !content.is_empty(),
                "Sandbox file should not be empty: {}",
                file.path
            );
        }

        // Drop cleans up
        drop(ws);
        assert!(
            !path.exists(),
            "Sandbox directory should be deleted after Drop"
        );
    }

    #[test]
    fn sandbox_keep_preserves_directory() {
        let ws = SandboxWorkspace::create(true).expect("sandbox creation should succeed");
        let path = ws.path.clone();
        drop(ws);
        // keep=true → directory must still exist
        assert!(path.exists(), "Sandbox should be kept with keep=true");
        // Manual cleanup for the test
        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn sandbox_file_list_is_non_empty() {
        let files = SandboxWorkspace::file_paths();
        assert!(!files.is_empty());
        assert!(files.contains(&"Cargo.toml"));
        assert!(files.contains(&"src/main.rs"));
        assert!(files.contains(&"src/lib.rs"));
        assert!(files.contains(&"tests/integration_test.rs"));
    }

    #[test]
    fn sandbox_from_existing_nonexistent_returns_error() {
        let result = SandboxWorkspace::from_existing(
            PathBuf::from("/nonexistent/path/that/cannot/exist/ever"),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn cargo_toml_is_valid_toml() {
        let toml_file = SANDBOX_FILES
            .iter()
            .find(|f| f.path == "Cargo.toml")
            .unwrap();
        // Just check it parses as TOML (requires the toml crate, which is a dep)
        let parsed: Result<toml::Value, _> = toml::from_str(toml_file.content);
        assert!(parsed.is_ok(), "Cargo.toml content should be valid TOML");
    }
}
