use crate::skills::{
    SkillRegistry, default_manifest_template, get_default_skills_dir, list_skills,
};
use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::fs;

#[derive(Subcommand, Debug, Clone)]
pub enum SkillsCommand {
    /// List available skills with manifest metadata and arbitration scores
    List,
    /// Show full details of a specific skill
    Show {
        /// Name of the skill to show
        name: String,
    },
    /// Create a new skill template (SKILL.md + skill.json)
    New {
        /// Name of the new skill (use kebab-case, e.g. rust-expert)
        name: String,
    },
    /// Validate a skill for security issues
    Validate {
        /// Name of the skill to validate
        name: String,
    },
    /// Globally enable a skill (sets enabled=true in skill.json)
    Enable {
        /// Name of the skill to enable
        name: String,
    },
    /// Globally disable a skill (sets enabled=false in skill.json)
    Disable {
        /// Name of the skill to disable
        name: String,
    },
}

const SKILL_MD_TEMPLATE: &str = r#"---
name: {name}
description: Description for {name}
license: MIT
auto-activate:
  enabled: true
  keywords: []
  patterns: []
  file_extensions: []
  min_confidence: 50
---

# Instructions for {name}

Write your skill instructions here.

## What This Skill Does

Describe the purpose of this skill.

## Guidelines

- Guideline 1
- Guideline 2
"#;

pub async fn handle_skills_command(command: SkillsCommand) -> Result<()> {
    let skills_dir = get_default_skills_dir().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".grok/skills")
    });

    match command {
        // ── List ─────────────────────────────────────────────────────────────
        SkillsCommand::List => {
            if !skills_dir.exists() {
                println!(
                    "{} No skills directory found at {}",
                    "ℹ".bright_blue(),
                    skills_dir.display()
                );
                println!(
                    "  Create your first skill with: {}",
                    "grok skills new <name>".bright_cyan()
                );
                return Ok(());
            }

            match SkillRegistry::load(&skills_dir) {
                Ok(registry) => {
                    if registry.is_empty() {
                        println!(
                            "{} No skills found in {}",
                            "ℹ".bright_blue(),
                            skills_dir.display()
                        );
                        println!(
                            "  Create your first skill with: {}",
                            "grok skills new <name>".bright_cyan()
                        );
                        return Ok(());
                    }

                    println!("{}", "Available Skills".bright_cyan().bold());
                    println!(
                        "  {} — sorted by arbitration score (highest first)",
                        format!("{} skill(s)", registry.len()).dimmed()
                    );
                    println!();

                    for entry in registry.entries() {
                        let enabled_badge = if entry.is_enabled() {
                            "enabled".bright_green().to_string()
                        } else {
                            "DISABLED".bright_red().to_string()
                        };

                        println!(
                            "  {} {}  [score: {}]  [{}]  v{}",
                            "•".bright_white(),
                            entry.name().bright_yellow().bold(),
                            entry.arbitration_score().to_string().bright_white(),
                            enabled_badge,
                            entry.version().dimmed()
                        );
                        println!("    {}", entry.description().dimmed());

                        let tags = entry.tags();
                        if !tags.is_empty() {
                            println!("    {} {}", "tags:".dimmed(), tags.join(", ").bright_blue());
                        }

                        if let Some(author) = entry.author() {
                            println!("    {} {}", "author:".dimmed(), author.dimmed());
                        }

                        let deps = entry.dependencies();
                        if !deps.is_empty() {
                            println!(
                                "    {} {}",
                                "depends on:".dimmed(),
                                deps.join(", ").dimmed()
                            );
                        }

                        println!();
                    }

                    println!(
                        "  Tip: use {} or {} to show/hide skill details",
                        "grok skills show <name>".bright_cyan(),
                        "grok skills validate <name>".bright_cyan()
                    );
                }
                Err(e) => {
                    eprintln!("{} Failed to load skill registry: {}", "✗".bright_red(), e);
                }
            }
        }

        // ── Show ─────────────────────────────────────────────────────────────
        SkillsCommand::Show { name } => {
            match SkillRegistry::load(&skills_dir) {
                Ok(registry) => {
                    if let Some(entry) = registry.find(&name) {
                        println!(
                            "{}",
                            format!("Skill: {}", entry.name()).bright_cyan().bold()
                        );
                        println!();

                        // Core info
                        println!(
                            "  {} {}",
                            "Name:".bright_white(),
                            entry.name().bright_yellow()
                        );
                        println!(
                            "  {} {}",
                            "Description:".bright_white(),
                            entry.description()
                        );
                        println!("  {} {}", "Version:".bright_white(), entry.version());

                        if let Some(author) = entry.author() {
                            println!("  {} {}", "Author:".bright_white(), author);
                        }

                        let enabled_str = if entry.is_enabled() {
                            "yes".bright_green().to_string()
                        } else {
                            "no (globally disabled)".bright_red().to_string()
                        };
                        println!("  {} {}", "Enabled:".bright_white(), enabled_str);

                        println!(
                            "  {} {}",
                            "Arbitration score:".bright_white(),
                            entry.arbitration_score().to_string().bright_white()
                        );

                        let tags = entry.tags();
                        if !tags.is_empty() {
                            println!(
                                "  {} {}",
                                "Tags:".bright_white(),
                                tags.join(", ").bright_blue()
                            );
                        }

                        let deps = entry.dependencies();
                        if !deps.is_empty() {
                            println!("  {} {}", "Dependencies:".bright_white(), deps.join(", "));
                        }

                        if let Some(manifest) = &entry.manifest
                            && let Some(min_ver) = &manifest.min_grok_version
                        {
                            println!("  {} {}", "Min grok version:".bright_white(), min_ver);
                        }

                        // SKILL.md config extras
                        if let Some(ref allowed) = entry.skill.config.allowed_tools {
                            println!(
                                "  {} {}",
                                "Allowed tools:".bright_white(),
                                allowed.join(", ")
                            );
                        }

                        if let Some(ref compat) = entry.skill.config.compatibility {
                            println!(
                                "  {} {}",
                                "Compatibility:".bright_white(),
                                compat.join(", ")
                            );
                        }

                        println!();
                        println!(
                            "  {} {}",
                            "Path:".bright_white(),
                            entry.skill.path.display()
                        );
                        println!();

                        // Auto-activate config
                        if let Some(ref aa) = entry.skill.config.auto_activate {
                            println!("{}", "Auto-Activation:".bright_white().bold());
                            println!(
                                "  {} {}",
                                "Enabled:".dimmed(),
                                if aa.enabled { "yes" } else { "no" }
                            );
                            if !aa.keywords.is_empty() {
                                println!("  {} {}", "Keywords:".dimmed(), aa.keywords.join(", "));
                            }
                            if !aa.patterns.is_empty() {
                                println!("  {} {}", "Patterns:".dimmed(), aa.patterns.join(", "));
                            }
                            if !aa.file_extensions.is_empty() {
                                println!(
                                    "  {} {}",
                                    "File extensions:".dimmed(),
                                    aa.file_extensions.join(", ")
                                );
                            }
                            println!("  {} {}", "Min confidence:".dimmed(), aa.min_confidence);
                            println!();
                        }

                        // Instructions
                        println!("{}", "Instructions:".bright_yellow().bold());
                        println!();
                        println!("{}", entry.skill.instructions);
                    } else {
                        println!("{} Skill '{}' not found.", "✗".bright_red(), name.red());
                        println!(
                            "  Run {} to see available skills.",
                            "grok skills list".bright_cyan()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to load skill registry: {}", "✗".bright_red(), e);
                }
            }
        }

        // ── New ──────────────────────────────────────────────────────────────
        SkillsCommand::New { name } => {
            let skill_path = skills_dir.join(&name);

            if skill_path.exists() {
                println!(
                    "{} Skill '{}' already exists at {}",
                    "⚠".bright_yellow(),
                    name.bright_yellow(),
                    skill_path.display()
                );
                return Ok(());
            }

            // Create skill directory
            fs::create_dir_all(&skill_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create skill directory at {}: {}",
                    skill_path.display(),
                    e
                )
            })?;

            // Write SKILL.md
            let skill_md = SKILL_MD_TEMPLATE.replace("{name}", &name);
            fs::write(skill_path.join("SKILL.md"), &skill_md)
                .map_err(|e| anyhow::anyhow!("Failed to write SKILL.md for '{}': {}", name, e))?;

            // Write skill.json manifest
            let manifest = default_manifest_template(&name);
            let manifest_json = serde_json::to_string_pretty(&manifest)
                .map_err(|e| anyhow::anyhow!("Failed to serialise skill.json: {}", e))?;
            fs::write(skill_path.join("skill.json"), &manifest_json)
                .map_err(|e| anyhow::anyhow!("Failed to write skill.json for '{}': {}", name, e))?;

            println!(
                "{} Created new skill '{}'",
                "✓".bright_green(),
                name.bright_yellow().bold()
            );
            println!("  {}", skill_path.display().to_string().dimmed());
            println!();
            println!("  Files created:");
            println!(
                "    {} — skill instructions (edit this)",
                "SKILL.md".bright_white()
            );
            println!(
                "    {} — manifest metadata (version, tags, arbitration score)",
                "skill.json".bright_white()
            );
            println!();
            println!(
                "  Next: edit {} then activate with {}",
                skill_path.join("SKILL.md").display().to_string().dimmed(),
                format!("/activate {}", name).bright_cyan()
            );
        }

        // ── Validate ─────────────────────────────────────────────────────────
        SkillsCommand::Validate { name } => {
            let skills = list_skills(&skills_dir)?;
            if let Some(skill) = skills.into_iter().find(|s| s.config.name == name) {
                println!(
                    "{}",
                    format!("Validating skill: {}", name).bright_cyan().bold()
                );
                println!();

                let validator = crate::skills::SkillSecurityValidator::new();
                let result = validator.validate_skill(&skill.path)?;

                match result {
                    crate::skills::ValidationLevel::Safe => {
                        println!("{} {}", "✅".bright_green(), "SAFE".bright_green().bold());
                        println!("  No security issues detected.");
                    }
                    crate::skills::ValidationLevel::Warning(warnings) => {
                        println!(
                            "{} {}",
                            "⚠".bright_yellow(),
                            "WARNING".bright_yellow().bold()
                        );
                        println!("  Minor issues detected:");
                        for warning in warnings {
                            println!("    • {}", warning.yellow());
                        }
                    }
                    crate::skills::ValidationLevel::Suspicious(issues) => {
                        println!(
                            "{} {}",
                            "🔶".bright_yellow(),
                            "SUSPICIOUS".bright_yellow().bold()
                        );
                        println!("  Potentially dangerous patterns detected:");
                        for issue in issues {
                            println!("    • {}", issue.yellow());
                        }
                        println!();
                        println!(
                            "{}",
                            "  Review carefully before activating.".bright_yellow()
                        );
                    }
                    crate::skills::ValidationLevel::Dangerous(issues) => {
                        println!("{} {}", "🛑".bright_red(), "DANGEROUS".bright_red().bold());
                        println!("  BLOCKED — Malicious patterns detected:");
                        for issue in issues {
                            println!("    • {}", issue.red());
                        }
                        println!();
                        println!("{}", "  DO NOT USE THIS SKILL.".bright_red().bold());
                    }
                }
            } else {
                println!("{} Skill '{}' not found.", "✗".bright_red(), name.red());
                println!(
                    "  Run {} to see available skills.",
                    "grok skills list".bright_cyan()
                );
            }
        }

        // ── Enable ────────────────────────────────────────────────────────────
        SkillsCommand::Enable { name } => {
            let mut registry = SkillRegistry::load(&skills_dir)
                .map_err(|e| anyhow::anyhow!("Failed to load skill registry: {}", e))?;

            if registry.find(&name).is_none() {
                println!("{} Skill '{}' not found.", "✗".bright_red(), name.red());
                println!(
                    "  Run {} to see available skills.",
                    "grok skills list".bright_cyan()
                );
                return Ok(());
            }

            // Already enabled?
            if registry
                .find(&name)
                .map(|e| e.is_enabled())
                .unwrap_or(false)
            {
                println!(
                    "{} Skill '{}' is already enabled.",
                    "ℹ".bright_blue(),
                    name.bright_yellow()
                );
                return Ok(());
            }

            registry.set_enabled(&name, true)?;
            println!(
                "{} Skill '{}' has been {}.",
                "✓".bright_green(),
                name.bright_yellow(),
                "enabled".bright_green()
            );
            println!(
                "  Activate it in a session with: {}",
                format!("/activate {}", name).bright_cyan()
            );
        }

        // ── Disable ───────────────────────────────────────────────────────────
        SkillsCommand::Disable { name } => {
            let mut registry = SkillRegistry::load(&skills_dir)
                .map_err(|e| anyhow::anyhow!("Failed to load skill registry: {}", e))?;

            if registry.find(&name).is_none() {
                println!("{} Skill '{}' not found.", "✗".bright_red(), name.red());
                println!(
                    "  Run {} to see available skills.",
                    "grok skills list".bright_cyan()
                );
                return Ok(());
            }

            // Already disabled?
            if !registry.find(&name).map(|e| e.is_enabled()).unwrap_or(true) {
                println!(
                    "{} Skill '{}' is already disabled.",
                    "ℹ".bright_blue(),
                    name.bright_yellow()
                );
                return Ok(());
            }

            registry.set_enabled(&name, false)?;
            println!(
                "{} Skill '{}' has been {}.",
                "✓".bright_green(),
                name.bright_yellow(),
                "disabled".bright_red()
            );
            println!(
                "  {}",
                "The skill cannot be activated in any session until re-enabled.".dimmed()
            );
            println!(
                "  Re-enable with: {}",
                format!("grok skills enable {}", name).bright_cyan()
            );
        }
    }

    Ok(())
}
