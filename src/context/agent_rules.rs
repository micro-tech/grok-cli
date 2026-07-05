//! Agent Rules Loader (AGENTS.md / CLAUDE.md style)
//!
//! Supports the popular agent instruction file format used by many tools.
//!
//! Loading order (later overrides earlier):
//! 1. Global rules (~/.grok/AGENTS.md + ~/.grok/rules/*.md)
//! 2. Repo root → CWD walk (AGENTS.md, CLAUDE.md, etc. in each directory)
//! 3. .agents/rules/*.md and .grok/rules/*.md (legacy compat)

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Supported agent rule filenames (in priority order within a directory)
const AGENT_FILENAMES: &[&str] = &[
    "AGENTS.md",
    "Agents.md",
    "AGENT.md",
    "CLAUDE.md",
    "Claude.md",
    "CLAUDE.local.md",
];

/// A single rule file loaded from disk
#[derive(Debug, Clone)]
pub struct AgentRule {
    pub filename: String,
    pub path: PathBuf,
    pub content: String,
    pub source: RuleSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleSource {
    Global,
    RepoRoot,
    Directory(u8), // depth from root (higher = deeper = higher priority)
    LegacyProject,
}

/// Find the git repository root (or fall back to current dir)
fn find_repo_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return current;
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            return start.to_path_buf(); // fallback
        }
    }
}

/// Load global rules from ~/.grok/
fn load_global_rules() -> Vec<AgentRule> {
    let mut rules = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let grok_dir = home.join(".grok");

        // ~/.grok/AGENTS.md (and variants)
        for name in AGENT_FILENAMES {
            let path = grok_dir.join(name);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    rules.push(AgentRule {
                        filename: name.to_string(),
                        path,
                        content,
                        source: RuleSource::Global,
                    });
                }
            }
        }

        // ~/.grok/rules/*.md
        let rules_dir = grok_dir.join("rules");
        if rules_dir.exists() {
            if let Ok(entries) = fs::read_dir(&rules_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "md") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                                rules.push(AgentRule {
                                    filename: filename.to_string(),
                                    path,
                                    content,
                                    source: RuleSource::Global,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    rules
}

/// Load rules by walking from repo root to current directory
fn load_directory_rules(start_dir: &Path) -> Vec<AgentRule> {
    let repo_root = find_repo_root(start_dir);
    let mut rules = Vec::new();
    let mut current = repo_root.clone();
    let mut depth: u8 = 0;

    // Collect all directories from root to start_dir
    let mut dirs_to_check = Vec::new();
    let mut cursor = start_dir.to_path_buf();

    while cursor.starts_with(&repo_root) {
        dirs_to_check.push(cursor.clone());
        if let Some(parent) = cursor.parent() {
            if parent == cursor {
                break;
            }
            cursor = parent.to_path_buf();
        } else {
            break;
        }
    }
    dirs_to_check.reverse(); // root first

    for dir in dirs_to_check {
        depth += 1;
        for name in AGENT_FILENAMES {
            let path = dir.join(name);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    rules.push(AgentRule {
                        filename: name.to_string(),
                        path,
                        content,
                        source: RuleSource::Directory(depth),
                    });
                }
            }
        }

        // Also check .grok/rules/ and .agents/rules/ inside each dir
        for rules_dir_name in &[".grok/rules", ".agents/rules"] {
            let rules_dir = dir.join(rules_dir_name);
            if rules_dir.exists() {
                if let Ok(entries) = fs::read_dir(&rules_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map_or(false, |e| e == "md") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                                    rules.push(AgentRule {
                                        filename: filename.to_string(),
                                        path,
                                        content,
                                        source: RuleSource::Directory(depth),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    rules
}

/// Main entry point: load all agent rules for the current project
pub fn load_all_agent_rules(project_root: &Path) -> Result<Vec<AgentRule>> {
    let mut all_rules: HashMap<String, AgentRule> = HashMap::new();

    // 1. Global rules (lowest priority)
    for rule in load_global_rules() {
        all_rules.insert(rule.filename.clone(), rule);
    }

    // 2. Directory walking rules (higher priority wins)
    for rule in load_directory_rules(project_root) {
        // Deeper directories override
        if let Some(existing) = all_rules.get(&rule.filename) {
            if rule.source > existing.source {
                all_rules.insert(rule.filename.clone(), rule);
            }
        } else {
            all_rules.insert(rule.filename.clone(), rule);
        }
    }

    // 3. Legacy .agents/rules + .grok-cli/agents/rules (still supported)
    // (we can call the old loader here if needed)

    let mut result: Vec<AgentRule> = all_rules.into_values().collect();
    result.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.filename.cmp(&b.filename)));

    Ok(result)
}

/// Format rules into a compact prompt section
pub fn format_agent_rules_for_prompt(rules: &[AgentRule]) -> String {
    if rules.is_empty() {
        return String::new();
    }

    let mut output = String::from("\n\n## Agent Rules\n\n");

    let mut current_source: Option<RuleSource> = None;

    for rule in rules {
        if Some(rule.source) != current_source {
            current_source = Some(rule.source);
            match rule.source {
                RuleSource::Global => output.push_str("### Global Rules\n\n"),
                RuleSource::RepoRoot | RuleSource::Directory(_) => {
                    output.push_str("### Project Rules\n\n")
                }
                RuleSource::LegacyProject => output.push_str("### Legacy Rules\n\n"),
            }
        }
        output.push_str(&format!("**{}**\n{}\n\n", rule.filename, rule.content.trim()));
    }

    output
}