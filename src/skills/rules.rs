//! Global + Project Rules Loader
//!
//! Loads plain-text rule files from both the global directory
//! (`~/.grok-cli/agents/rules/`) and the project directory
//! (`<project>/.agents/rules/`).
//!
//! Merge rules:
//! - Global rules are loaded first.
//! - Project rules are loaded second.
//! - If a rule file has the same filename in both locations,
//!   the **project version wins** (override).
//! - All other rules are included.

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A single rule file (plain text / markdown)
#[derive(Debug, Clone)]
pub struct RuleFile {
    /// Original filename (e.g. "markmap-docs.md")
    pub filename: String,
    /// Full path it was loaded from
    pub path: PathBuf,
    /// Raw content of the rule
    pub content: String,
    /// Whether this came from the global or project directory
    pub source: RuleSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSource {
    Global,
    Project,
}

/// Load all rules from both global and project locations.
///
/// Project rules override global rules with the same filename.
pub fn load_all_rules(project_root: &Path) -> Result<Vec<RuleFile>> {
    let mut rules_by_name: HashMap<String, RuleFile> = HashMap::new();

    // 1. Load global rules first
    if let Some(global_dir) = crate::skills::manager::get_global_rules_dir() {
        if global_dir.exists() {
            for entry in fs::read_dir(&global_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            rules_by_name.insert(
                                filename.to_string(),
                                RuleFile {
                                    filename: filename.to_string(),
                                    path: entry.path(),
                                    content,
                                    source: RuleSource::Global,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    // 2. Load project rules (override globals with same filename)
    let project_dir = crate::skills::manager::get_project_rules_dir(project_root);
    if project_dir.exists() {
        for entry in fs::read_dir(&project_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(filename) = entry.file_name().to_str() {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        // Project always wins
                        rules_by_name.insert(
                            filename.to_string(),
                            RuleFile {
                                filename: filename.to_string(),
                                path: entry.path(),
                                content,
                                source: RuleSource::Project,
                            },
                        );
                    }
                }
            }
        }
    }

    // Return in a stable order (global first, then project overrides)
    let mut result: Vec<RuleFile> = rules_by_name.into_values().collect();
    result.sort_by(|a, b| {
        // Sort by source (Global before Project), then by filename
        match (a.source, b.source) {
            (RuleSource::Global, RuleSource::Project) => std::cmp::Ordering::Less,
            (RuleSource::Project, RuleSource::Global) => std::cmp::Ordering::Greater,
            _ => a.filename.cmp(&b.filename),
        }
    });

    Ok(result)
}

/// Format loaded rules into a compact prompt-friendly string.
/// Uses minimal headers to save tokens.
pub fn format_rules_for_prompt(rules: &[RuleFile]) -> String {
    if rules.is_empty() {
        return String::new();
    }

    let mut output = String::from("\n\n## Agent Rules\n\n");

    let mut has_global = false;
    let mut has_project = false;

    for rule in rules {
        match rule.source {
            RuleSource::Global => has_global = true,
            RuleSource::Project => has_project = true,
        }
    }

    if has_global {
        output.push_str("### Global Rules\n\n");
        for rule in rules.iter().filter(|r| r.source == RuleSource::Global) {
            output.push_str(&format!("**{}**\n{}\n\n", rule.filename, rule.content.trim()));
        }
    }

    if has_project {
        output.push_str("### Project Rules\n\n");
        for rule in rules.iter().filter(|r| r.source == RuleSource::Project) {
            output.push_str(&format!("**{}**\n{}\n\n", rule.filename, rule.content.trim()));
        }
    }

    output
}

/// Convenience helper: load rules for a project and return them formatted for the prompt.
/// Returns an empty string if no rules exist.
pub fn load_and_format_rules(project_root: &Path) -> String {
    match load_all_rules(project_root) {
        Ok(rules) => format_rules_for_prompt(&rules),
        Err(_) => String::new(),
    }
}