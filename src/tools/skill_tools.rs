//! Skill execution tool — look up a named skill and return its instructions
//! formatted as a ready-to-use context block.
//!
//! Skills live in `~/.grok/skills/<skill-name>/SKILL.md`.  See
//! [`crate::skills::manager`] for the loader.

use crate::skills::manager::{find_skill, get_default_skills_dir, list_skills};
use anyhow::{Result, anyhow};

/// Maximum input size (32 KB) to avoid overflowing the model context window.
const MAX_INPUT_BYTES: usize = 32_768;

/// Execute a named skill by formatting its instructions with the user input.
///
/// This does **not** make an API call; it returns the skill's instruction
/// block together with the provided input so the LLM can follow the
/// skill's guidance in its next response.
///
/// # Arguments
/// * `skill_name` — exact name as declared in the skill's `SKILL.md` frontmatter.
/// * `input`      — the user's request or context to pass into the skill.
///
/// # Errors
/// Returns an error when the skills directory cannot be determined or the
/// skill is not found.  A formatted list of available skills is included in
/// the error so the model can suggest alternatives.
pub fn execute_skill(skill_name: &str, input: &str) -> Result<String> {
    if skill_name.trim().is_empty() {
        tracing::warn!("skill_tools: execute_skill called with empty skill_name");
        return Err(anyhow!("skill_name cannot be empty"));
    }

    if input.len() > MAX_INPUT_BYTES {
        tracing::warn!(
            input_bytes = input.len(),
            max_bytes = MAX_INPUT_BYTES,
            "skill_tools: input exceeds 32 KB — this may overflow the model context window"
        );
    }

    let skills_dir = match get_default_skills_dir() {
        Some(d) => d,
        None => {
            tracing::warn!(
                "skill_tools::execute_skill: cannot determine skills directory (HOME not set?)"
            );
            return Err(anyhow!("Cannot determine skills directory (HOME not set?)"));
        }
    };

    let skill = match find_skill(skill_name, &skills_dir) {
        Some(s) => s,
        None => {
            tracing::warn!(
                skill_name = skill_name,
                skills_dir = %skills_dir.display(),
                "skill_tools::execute_skill: skill not found"
            );
            // Build a helpful error listing available skills
            let available = list_skills(&skills_dir)
                .map(|skills| {
                    if skills.is_empty() {
                        "No skills installed.".to_string()
                    } else {
                        skills
                            .iter()
                            .map(|s| format!("  - {} ({})", s.config.name, s.config.description))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                })
                .unwrap_or_else(|_| "Could not list skills.".to_string());

            return Err(anyhow!(
                "Skill '{}' not found in {}.\n\nAvailable skills:\n{}",
                skill_name,
                skills_dir.display(),
                available
            ));
        }
    };

    // Format the skill context for consumption by the LLM
    let allowed_tools = skill
        .config
        .allowed_tools
        .as_deref()
        .map(|tools| tools.join(", "))
        .unwrap_or_else(|| "all".to_string());

    let output = format!(
        "## Skill: {name}\n\
         **Description:** {desc}\n\
         **Allowed tools:** {tools}\n\
         \n\
         ### Instructions\n\
         {instructions}\n\
         \n\
         ---\n\
         ### User Input\n\
         {input}\n",
        name = skill.config.name,
        desc = skill.config.description,
        tools = allowed_tools,
        instructions = skill.instructions.trim(),
        input = if input.is_empty() {
            "(none provided)"
        } else {
            input
        },
    );

    Ok(output)
}

/// List all available skills and their descriptions.
pub fn list_available_skills() -> Result<String> {
    let skills_dir = match get_default_skills_dir() {
        Some(d) => d,
        None => {
            tracing::warn!(
                "skill_tools::list_available_skills: cannot determine skills directory (HOME not set?)"
            );
            return Err(anyhow!("Cannot determine skills directory"));
        }
    };

    let skills = list_skills(&skills_dir).map_err(|e| {
        tracing::warn!(
            error = %e,
            "skill_tools::list_available_skills: failed to read skills directory"
        );
        anyhow!("Failed to list skills: {}", e)
    })?;

    if skills.is_empty() {
        Ok(format!(
            "No skills found in {}.\n\
             Install skills by placing a directory with a SKILL.md file there.",
            skills_dir.display()
        ))
    } else {
        let lines: Vec<String> = skills
            .iter()
            .map(|s| format!("  {:.<30} {}", s.config.name, s.config.description))
            .collect();
        Ok(format!(
            "Available skills ({} total):\n{}",
            skills.len(),
            lines.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_skill_name_returns_error() {
        let r = execute_skill("", "some input");
        assert!(r.is_err());
    }

    #[test]
    fn unknown_skill_returns_helpful_error() {
        let r = execute_skill("no_such_skill_xyz_abc", "test input");
        // Must fail — but should include the skills directory or "not found" in the message
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("not found") || msg.contains("No skills") || msg.contains("directory"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn list_skills_returns_ok() {
        let result = list_available_skills();
        assert!(
            result.is_ok(),
            "list_available_skills must not return Err, got: {:?}",
            result
        );
    }
}
