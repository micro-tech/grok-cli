//! Project context file loading utilities
//!
//! This module provides functionality to detect and load project-specific
//! context files (like GEMINI.md, .grok/context.md, etc.) that help ground
//! the AI agent in project conventions and guidelines.

use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

/// Standard context file names to search for, in order of preference
const CONTEXT_FILE_NAMES: &[&str] = &[
    "GEMINI.md",
    ".gemini.md",
    ".claude.md",
    ".zed/rules",
    ".grok/context.md",
    ".ai/context.md",
    "CONTEXT.md",
    ".gemini/context.md",
    ".cursor/rules",
    "AI_RULES.md",
];

/// Maximum context file size to load (5 MB)
const MAX_CONTEXT_SIZE: u64 = 5 * 1024 * 1024;

/// Standard context file names to search for in the global configuration directory
const GLOBAL_CONTEXT_FILE_NAMES: &[&str] = &["context.md", "CONTEXT.md"];

/// Get the global context directory (e.g., ~/.grok)
fn get_global_context_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".grok"))
}

/// Load project context from standard context files
///
/// Searches for context files in the project root directory in the following order:
/// 1. GEMINI.md
/// 2. .gemini.md
/// 3. .claude.md
/// 4. .zed/rules
/// 5. .grok/context.md
/// 6. .ai/context.md
/// 7. CONTEXT.md
/// 8. .gemini/context.md
/// 9. .cursor/rules
/// 10. AI_RULES.md
///
/// Returns the content of the first file found, or None if no context file exists.
pub fn load_project_context<P: AsRef<Path>>(project_root: P) -> Result<Option<String>> {
    let project_root = project_root.as_ref();

    // 1. Check project directory
    if project_root.exists() && project_root.is_dir() {
        for file_name in CONTEXT_FILE_NAMES {
            let file_path = project_root.join(file_name);

            if file_path.exists() && file_path.is_file() {
                // Check file size before reading
                let metadata = fs::metadata(&file_path)?;
                if metadata.len() > MAX_CONTEXT_SIZE {
                    eprintln!(
                        "Warning: Context file {} is too large ({} bytes), skipping",
                        file_path.display(),
                        metadata.len()
                    );
                    continue;
                }

                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        if content.trim().is_empty() {
                            // Skip empty files and continue searching
                            continue;
                        }
                        return Ok(Some(content));
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to read context file {}: {}",
                            file_path.display(),
                            e
                        );
                        continue;
                    }
                }
            }
        }
    }

    // 2. Check global directory
    if let Some(global_dir) = get_global_context_dir() {
        if global_dir.exists() && global_dir.is_dir() {
            for file_name in GLOBAL_CONTEXT_FILE_NAMES {
                let file_path = global_dir.join(file_name);

                if file_path.exists() && file_path.is_file() {
                    let metadata = fs::metadata(&file_path)?;
                    if metadata.len() > MAX_CONTEXT_SIZE {
                        continue;
                    }

                    match fs::read_to_string(&file_path) {
                        Ok(content) => {
                            if !content.trim().is_empty() {
                                return Ok(Some(content));
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
    }

    // No context file found
    Ok(None)
}

/// Load and merge multiple project context files
///
/// Unlike load_project_context which returns the first file found,
/// this function loads and merges all available context files,
/// allowing projects to use multiple context sources (e.g., both
/// .zed/rules and .gemini.md).
///
/// Returns a merged context string, or None if no files are found.
pub fn load_and_merge_project_context<P: AsRef<Path>>(project_root: P) -> Result<Option<String>> {
    let project_root = project_root.as_ref();
    let mut merged_content = Vec::new();

    // 1. Load from project directory
    if project_root.exists() && project_root.is_dir() {
        for file_name in CONTEXT_FILE_NAMES {
            let file_path = project_root.join(file_name);

            if file_path.exists() && file_path.is_file() {
                // Check file size before reading
                let metadata = fs::metadata(&file_path)?;
                if metadata.len() > MAX_CONTEXT_SIZE {
                    eprintln!(
                        "Warning: Context file {} is too large ({} bytes), skipping",
                        file_path.display(),
                        metadata.len()
                    );
                    continue;
                }

                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        if !content.trim().is_empty() {
                            // Add source annotation
                            let annotated =
                                format!("## From: {}\n\n{}\n", file_name, content.trim());
                            merged_content.push(annotated);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to read context file {}: {}",
                            file_path.display(),
                            e
                        );
                        continue;
                    }
                }
            }
        }
    }

    // 2. Load from global directory
    if let Some(global_dir) = get_global_context_dir() {
        if global_dir.exists() && global_dir.is_dir() {
            for file_name in GLOBAL_CONTEXT_FILE_NAMES {
                let file_path = global_dir.join(file_name);

                if file_path.exists() && file_path.is_file() {
                    let metadata = fs::metadata(&file_path)?;
                    if metadata.len() > MAX_CONTEXT_SIZE {
                        continue;
                    }

                    match fs::read_to_string(&file_path) {
                        Ok(content) => {
                            if !content.trim().is_empty() {
                                let annotated = format!(
                                    "## From: Global {}\n\n{}\n",
                                    file_name,
                                    content.trim()
                                );
                                merged_content.push(annotated);
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
    }

    if merged_content.is_empty() {
        Ok(None)
    } else {
        Ok(Some(merged_content.join("\n---\n\n")))
    }
}

/// Get all available context file paths in the project
///
/// Returns a vector of paths to all existing context files (project and global).
pub fn get_all_context_file_paths<P: AsRef<Path>>(project_root: P) -> Vec<PathBuf> {
    let project_root = project_root.as_ref();
    let mut paths = Vec::new();

    // 1. Check project directory
    if project_root.exists() && project_root.is_dir() {
        for file_name in CONTEXT_FILE_NAMES {
            let file_path = project_root.join(file_name);
            if file_path.exists() && file_path.is_file() {
                paths.push(file_path);
            }
        }
    }

    // 2. Check global directory
    if let Some(global_dir) = get_global_context_dir() {
        if global_dir.exists() && global_dir.is_dir() {
            for file_name in GLOBAL_CONTEXT_FILE_NAMES {
                let file_path = global_dir.join(file_name);
                if file_path.exists() && file_path.is_file() {
                    paths.push(file_path);
                }
            }
        }
    }

    paths
}

/// Get the path of the context file if it exists
///
/// Returns the path to the first available context file, or None if no file exists.
pub fn get_context_file_path<P: AsRef<Path>>(project_root: P) -> Option<PathBuf> {
    let project_root = project_root.as_ref();

    // 1. Check project directory
    if project_root.exists() && project_root.is_dir() {
        for file_name in CONTEXT_FILE_NAMES {
            let file_path = project_root.join(file_name);
            if file_path.exists() && file_path.is_file() {
                return Some(file_path);
            }
        }
    }

    // 2. Check global directory
    if let Some(global_dir) = get_global_context_dir() {
        if global_dir.exists() && global_dir.is_dir() {
            for file_name in GLOBAL_CONTEXT_FILE_NAMES {
                let file_path = global_dir.join(file_name);
                if file_path.exists() && file_path.is_file() {
                    return Some(file_path);
                }
            }
        }
    }

    None
}

/// Format context content for injection into system prompt
///
/// Wraps the context content with appropriate markers to distinguish it
/// from the base system prompt.
pub fn format_context_for_prompt(context: &str) -> String {
    format!(
        "\n\n## Project Context\n\nThe following context has been loaded from the project:\n\n{}\n\n---\n",
        context.trim()
    )
}

/// Validate that context content is reasonable for injection
///
/// Checks for potential issues like excessive length or problematic content.
pub fn validate_context(context: &str) -> Result<()> {
    let trimmed = context.trim();

    if trimmed.is_empty() {
        return Err(anyhow!("Context content is empty"));
    }

    // Check token estimate (rough estimate: 1 token ≈ 4 characters)
    let estimated_tokens = trimmed.len() / 4;
    if estimated_tokens > 100_000 {
        return Err(anyhow!(
            "Context is too large (estimated {} tokens). Consider reducing size.",
            estimated_tokens
        ));
    }

    Ok(())
}

/// Create a default context file template
pub fn create_default_context_template() -> String {
    r#"# Project Context

This file provides context for AI assistants working on this project.

## Project Overview
<!-- Briefly describe what this project does -->

## Architecture
<!-- Describe the high-level architecture and key components -->

## Development Guidelines
<!-- Any coding standards, conventions, or best practices -->

## Key Technologies
<!-- List main frameworks, libraries, and tools used -->

## Common Tasks
<!-- Frequently performed development tasks and how to do them -->

## Important Notes
<!-- Any gotchas, quirks, or important considerations -->
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_load_project_context_gemini_md() {
        let temp_dir = tempdir().unwrap();
        let gemini_file = temp_dir.path().join("GEMINI.md");
        fs::write(&gemini_file, "# Test Project\nThis is a test context.").unwrap();

        let result = load_project_context(temp_dir.path()).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Test Project"));
    }

    #[test]
    fn test_load_project_context_grok_dir() {
        let temp_dir = tempdir().unwrap();
        let grok_dir = temp_dir.path().join(".grok");
        fs::create_dir(&grok_dir).unwrap();
        let context_file = grok_dir.join("context.md");
        fs::write(&context_file, "# Grok Context\nGrok-specific context.").unwrap();

        let result = load_project_context(temp_dir.path()).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("Grok Context"));
    }

    #[test]
    fn test_load_project_context_priority() {
        let temp_dir = tempdir().unwrap();

        // Create multiple context files
        fs::write(temp_dir.path().join("GEMINI.md"), "GEMINI content").unwrap();
        fs::write(temp_dir.path().join("CONTEXT.md"), "CONTEXT content").unwrap();

        let result = load_project_context(temp_dir.path()).unwrap();
        assert!(result.is_some());
        // Should prefer GEMINI.md
        assert_eq!(result.unwrap(), "GEMINI content");
    }

    #[test]
    fn test_load_project_context_no_file() {
        let temp_dir = tempdir().unwrap();
        let result = load_project_context(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_project_context_empty_file() {
        let temp_dir = tempdir().unwrap();
        let gemini_file = temp_dir.path().join("GEMINI.md");
        fs::write(&gemini_file, "   \n\n  ").unwrap();

        let result = load_project_context(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_context_file_path() {
        let temp_dir = tempdir().unwrap();
        let gemini_file = temp_dir.path().join("GEMINI.md");
        fs::write(&gemini_file, "test").unwrap();

        let result = get_context_file_path(temp_dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), gemini_file);
    }

    #[test]
    fn test_format_context_for_prompt() {
        let context = "Test context content";
        let formatted = format_context_for_prompt(context);
        assert!(formatted.contains("## Project Context"));
        assert!(formatted.contains("Test context content"));
    }

    #[test]
    fn test_validate_context() {
        assert!(validate_context("Valid content").is_ok());
        assert!(validate_context("").is_err());
        assert!(validate_context("   ").is_err());
    }

    #[test]
    fn test_create_default_template() {
        let template = create_default_context_template();
        assert!(template.contains("# Project Context"));
        assert!(template.contains("## Project Overview"));
    }

    #[test]
    fn test_load_and_merge_multiple_contexts() {
        let temp_dir = tempdir().unwrap();

        // Create multiple context files
        fs::write(
            temp_dir.path().join("GEMINI.md"),
            "# Gemini Context\nGemini rules.",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join(".claude.md"),
            "# Claude Context\nClaude rules.",
        )
        .unwrap();

        let result = load_and_merge_project_context(temp_dir.path()).unwrap();
        assert!(result.is_some());

        let merged = result.unwrap();
        assert!(merged.contains("GEMINI.md"));
        assert!(merged.contains("Gemini rules"));
        assert!(merged.contains(".claude.md"));
        assert!(merged.contains("Claude rules"));
    }

    #[test]
    fn test_get_all_context_file_paths() {
        let temp_dir = tempdir().unwrap();

        // Create multiple context files
        fs::write(temp_dir.path().join("GEMINI.md"), "test1").unwrap();

        let zed_dir = temp_dir.path().join(".zed");
        fs::create_dir(&zed_dir).unwrap();
        fs::write(zed_dir.join("rules"), "test2").unwrap();

        let paths = get_all_context_file_paths(temp_dir.path());
        assert_eq!(paths.len(), 2);
    }
}
