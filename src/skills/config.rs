use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Auto-activation configuration for a skill.
///
/// When any trigger condition is matched against the user's input or working
/// directory, the skill is automatically suggested (or activated, depending on
/// the session's `auto_skills_enabled` setting).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoActivateConfig {
    /// Whether auto-activation is enabled for this skill (default: true if the
    /// section is present, false if the section is absent).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Case-insensitive keywords.  A single match in the user message is
    /// enough to trigger the skill.
    ///
    /// Example (SKILL.md frontmatter):
    /// ```yaml
    /// auto-activate:
    ///   keywords: ["rust", "cargo", "borrow checker", "lifetime"]
    /// ```
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Rust `regex` patterns matched against the full user message.
    /// Any match triggers the skill.
    ///
    /// Example:
    /// ```yaml
    /// auto-activate:
    ///   patterns: ["fn\\s+\\w+", "\\.rs\\b"]
    /// ```
    #[serde(default)]
    pub patterns: Vec<String>,

    /// File extensions (without the leading dot, e.g. `"rs"`, `"py"`).
    /// If the current working directory contains *any* file with one of these
    /// extensions the skill will be suggested.  Checked lazily only when the
    /// other triggers produce no match.
    ///
    /// Example:
    /// ```yaml
    /// auto-activate:
    ///   file_extensions: ["rs", "toml"]
    /// ```
    #[serde(default)]
    pub file_extensions: Vec<String>,

    /// Minimum confidence score (0–100) required before the skill is
    /// auto-activated.  Defaults to 50.  Raise this value for noisier skills.
    #[serde(default = "default_min_confidence")]
    pub min_confidence: u8,
}

fn default_true() -> bool {
    true
}

fn default_min_confidence() -> u8 {
    50
}

/// Skill configuration parsed from the YAML frontmatter of `SKILL.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    /// Unique name of the skill (lower-kebab-case recommended).
    pub name: String,

    /// Human-readable description of what the skill does.
    pub description: String,

    /// SPDX license identifier (optional).
    pub license: Option<String>,

    /// Compatibility notes – e.g. `["rust >= 1.70", "internet required"]`
    /// (optional).
    pub compatibility: Option<Vec<String>>,

    /// Arbitrary key-value metadata (optional).
    pub metadata: Option<HashMap<String, String>>,

    /// Restrict which ACP tools this skill may invoke.  When set, only the
    /// listed tool names are permitted while the skill is active.  `None`
    /// means "no restriction" (optional).
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<Vec<String>>,

    /// Auto-activation triggers.  When `None` the skill is never
    /// automatically suggested; the user must activate it manually with
    /// `/activate <skill-name>`.
    #[serde(rename = "auto-activate")]
    pub auto_activate: Option<AutoActivateConfig>,
}

/// A fully-loaded skill: its parsed configuration plus the raw instruction
/// text and the path on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Configuration from the YAML frontmatter.
    pub config: SkillConfig,

    /// Instruction text from the Markdown body of `SKILL.md`.
    pub instructions: String,

    /// Absolute path to the skill directory (the directory that contains
    /// `SKILL.md`).
    pub path: std::path::PathBuf,
}
