use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Skill configuration (YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    /// Unique name of the skill
    pub name: String,

    /// Description of what the skill does
    pub description: String,

    /// License for the skill (optional)
    pub license: Option<String>,

    /// Compatibility information (optional)
    pub compatibility: Option<Vec<String>>,

    /// Additional metadata (optional)
    pub metadata: Option<HashMap<String, String>>,

    /// Allowed tools for this skill (optional)
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<Vec<String>>,
}

/// Skill structure combining configuration and instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Configuration from frontmatter
    pub config: SkillConfig,

    /// Instructions from markdown content
    pub instructions: String,

    /// Path to the skill directory
    pub path: std::path::PathBuf,
}
