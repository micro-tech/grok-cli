use crate::skills::{get_default_skills_dir, list_skills};
use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::fs;

#[derive(Subcommand, Debug, Clone)]
pub enum SkillsCommand {
    /// List available skills
    List,
    /// Show details of a specific skill
    Show {
        /// Name of the skill to show
        name: String,
    },
    /// Create a new skill template
    New {
        /// Name of the new skill
        name: String,
    },
    /// Validate a skill for security issues
    Validate {
        /// Name of the skill to validate
        name: String,
    },
}

pub async fn handle_skills_command(command: SkillsCommand) -> Result<()> {
    let skills_dir = get_default_skills_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap().join(".grok/skills"));

    match command {
        SkillsCommand::List => {
            if !skills_dir.exists() {
                println!(
                    "No skills directory found at {}. Use 'grok skills new <name>' to create one.",
                    skills_dir.display()
                );
                return Ok(());
            }
            let skills = list_skills(&skills_dir)?;
            if skills.is_empty() {
                println!("No skills found in {}", skills_dir.display());
            } else {
                println!("{}", "Available Skills:".bright_cyan().bold());
                for skill in skills {
                    println!(
                        "  • {} - {}",
                        skill.config.name.green().bold(),
                        skill.config.description.dimmed()
                    );
                }
            }
        }
        SkillsCommand::Show { name } => {
            let skills = list_skills(&skills_dir)?;
            if let Some(skill) = skills.into_iter().find(|s| s.config.name == name) {
                println!("{}", format!("Skill: {}", skill.config.name).green().bold());
                println!("Description: {}", skill.config.description);
                if let Some(license) = &skill.config.license {
                    println!("License: {}", license);
                }
                println!();
                println!("{}", "Instructions:".bright_yellow());
                println!("{}", skill.instructions);
            } else {
                println!("Skill '{}' not found.", name.red());
            }
        }
        SkillsCommand::New { name } => {
            let skill_path = skills_dir.join(&name);
            if skill_path.exists() {
                println!(
                    "Skill '{}' already exists at {}",
                    name,
                    skill_path.display()
                );
                return Ok(());
            }
            fs::create_dir_all(&skill_path)?;
            let skill_md = format!(
                r#"---
name: {}
description: Description for {}
license: MIT
---

# Instructions for {}

Write your skill instructions here.
"#,
                name, name, name
            );
            fs::write(skill_path.join("SKILL.md"), skill_md)?;
            println!(
                "Created new skill '{}' at {}",
                name.green(),
                skill_path.display()
            );
        }
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
                        println!("No security issues detected.");
                    }
                    crate::skills::ValidationLevel::Warning(warnings) => {
                        println!(
                            "{} {}",
                            "⚠".bright_yellow(),
                            "WARNING".bright_yellow().bold()
                        );
                        println!("Minor issues detected:");
                        for warning in warnings {
                            println!("  • {}", warning.yellow());
                        }
                    }
                    crate::skills::ValidationLevel::Suspicious(issues) => {
                        println!(
                            "{} {}",
                            "🔶".bright_yellow(),
                            "SUSPICIOUS".bright_yellow().bold()
                        );
                        println!("Potentially dangerous patterns detected:");
                        for issue in issues {
                            println!("  • {}", issue.yellow());
                        }
                        println!();
                        println!("{}", "Review carefully before activating.".bright_yellow());
                    }
                    crate::skills::ValidationLevel::Dangerous(issues) => {
                        println!("{} {}", "🛑".bright_red(), "DANGEROUS".bright_red().bold());
                        println!("BLOCKED - Malicious patterns detected:");
                        for issue in issues {
                            println!("  • {}", issue.red());
                        }
                        println!();
                        println!("{}", "DO NOT USE THIS SKILL.".bright_red().bold());
                    }
                }
            } else {
                println!("{} Skill '{}' not found.", "✗".bright_red(), name);
            }
        }
    }
    Ok(())
}
