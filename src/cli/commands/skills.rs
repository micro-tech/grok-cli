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
    }
    Ok(())
}
