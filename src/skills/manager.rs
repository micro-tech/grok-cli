use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::skills::config::{Skill, SkillConfig};

/// Default location for global skills
pub fn get_default_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".grok").join("skills"))
}

/// Load a skill from a directory (must contain SKILL.md)
pub fn load_skill(skill_dir: &Path) -> Result<Skill> {
    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.exists() {
        return Err(anyhow!("No SKILL.md found in {}", skill_dir.display()));
    }

    let content = fs::read_to_string(&skill_file)
        .with_context(|| format!("Failed to read {}", skill_file.display()))?;

    // Parse frontmatter and content manually to be robust
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(anyhow!("SKILL.md must start with YAML frontmatter (---)"));
    }

    // Find the end of the frontmatter
    // We look for the second "---" after the first one
    let end_fm_idx = content[3..]
        .find("\n---")
        .map(|i| i + 3) // Add back the offset
        .ok_or_else(|| anyhow!("Could not find end of frontmatter in SKILL.md"))?;

    let frontmatter = &content[3..end_fm_idx].trim();

    // The instructions start after the second "---"
    // end_fm_idx points to the start of the second "---"
    // The second "---" has length 3, plus usually a newline
    let instructions_start = end_fm_idx + 4; // +4 for "\n---" length if we found via "\n---"

    // Check if we have enough content
    let instructions = if instructions_start < content.len() {
        content[instructions_start..].trim().to_string()
    } else {
        String::new()
    };

    let config: SkillConfig = serde_yaml::from_str(frontmatter).with_context(|| {
        format!(
            "Failed to parse YAML frontmatter in {}",
            skill_file.display()
        )
    })?;

    Ok(Skill {
        config,
        instructions,
        path: skill_dir.to_path_buf(),
    })
}

/// Discover all skills in a given directory
pub fn list_skills(base_dir: &Path) -> Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !base_dir.exists() {
        return Ok(skills);
    }

    // Look for directories containing SKILL.md
    // We only look at immediate subdirectories of the skills folder
    for entry in WalkDir::new(base_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() && entry.path().join("SKILL.md").exists() {
            match load_skill(entry.path()) {
                Ok(skill) => skills.push(skill),
                Err(_e) => {
                    // Silently ignore or log warning if possible
                    // eprintln!("Warning: Failed to load skill at {}: {}", entry.path().display(), e);
                }
            }
        }
    }

    Ok(skills)
}

/// Find a specific skill by name in the available skills
pub fn find_skill(name: &str, base_dir: &Path) -> Option<Skill> {
    if let Ok(skills) = list_skills(base_dir) {
        skills.into_iter().find(|s| s.config.name == name)
    } else {
        None
    }
}

/// Get formatted context string for all available skills
pub fn get_skills_context(base_dir: &Path) -> Result<String> {
    let skills = list_skills(base_dir)?;
    if skills.is_empty() {
        return Ok(String::new());
    }

    let mut context =
        String::from("\n\n## Available Skills\n\nThe following skills are available for use:\n\n");
    for skill in skills {
        context.push_str(&format!("### Skill: {}\n", skill.config.name));
        context.push_str(&format!("Description: {}\n", skill.config.description));
        context.push_str("\nInstructions:\n");
        context.push_str(&skill.instructions);
        context.push_str("\n\n---\n\n");
    }
    Ok(context)
}
