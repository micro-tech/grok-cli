//! Skill Registry and Plugin System
//!
//! Provides manifest-based skill metadata (`skill.json`), auto-discovery,
//! arbitration-score-ranked context generation, and dynamic enable/disable
//! persistence.
//!
//! # Overview
//!
//! Every skill lives in a sub-directory of the skills folder.  A skill
//! directory *must* contain a `SKILL.md` file (loaded by the existing
//! [`crate::skills::manager`] machinery) and *may* optionally contain a
//! `skill.json` manifest that enriches it with version, author, tags, and an
//! arbitration score.
//!
//! When multiple skills are active at the same time their instructions are
//! injected into the system prompt in **descending arbitration-score order**,
//! so higher-priority skills always appear first.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::skills::config::Skill;
use crate::skills::manager::list_skills;

// ─── Manifest Schema ──────────────────────────────────────────────────────────

/// Contents of the optional `skill.json` manifest file.
///
/// This file sits alongside `SKILL.md` in the skill directory and provides
/// richer metadata than the YAML frontmatter alone.  Every field except
/// `name` is optional or has a sensible default.
///
/// ## Minimal example
/// ```json
/// { "name": "rust-expert" }
/// ```
///
/// ## Full example
/// ```json
/// {
///   "name": "rust-expert",
///   "version": "1.2.0",
///   "author": "Alice",
///   "description": "Expert Rust guidance",
///   "tags": ["rust", "systems", "performance"],
///   "arbitration_score": 80,
///   "enabled": true,
///   "dependencies": ["cli-design"],
///   "min_grok_version": "0.1.8"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Must match the `name` field in the companion `SKILL.md` frontmatter.
    pub name: String,

    /// Semantic version string (e.g. `"1.0.0"`).  Defaults to `"1.0.0"`.
    #[serde(default = "default_version")]
    pub version: String,

    /// Skill author or maintainer (optional).
    #[serde(default)]
    pub author: Option<String>,

    /// Human-readable description.  When present it is shown in the skills
    /// list alongside (not instead of) the SKILL.md description.
    #[serde(default)]
    pub description: Option<String>,

    /// Categorisation tags, e.g. `["rust", "systems", "debugging"]`.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Arbitration score in the range 0–100.  Higher scores cause the skill's
    /// instructions to be injected earlier in the system prompt when multiple
    /// skills are active, giving it more influence over the model.
    /// Defaults to `50`.
    #[serde(default = "default_arbitration_score")]
    pub arbitration_score: u8,

    /// Global enabled flag.  When `false` the skill cannot be activated in
    /// any session until re-enabled.  Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Names of other skills this skill depends on.  Informational only for
    /// now; no automatic activation is performed.
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Minimum grok-cli version required (optional, informational).
    #[serde(default)]
    pub min_grok_version: Option<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_arbitration_score() -> u8 {
    50
}

fn default_true() -> bool {
    true
}

impl Default for SkillManifest {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: default_version(),
            author: None,
            description: None,
            tags: Vec::new(),
            arbitration_score: default_arbitration_score(),
            enabled: default_true(),
            dependencies: Vec::new(),
            min_grok_version: None,
        }
    }
}

// ─── SkillEntry ───────────────────────────────────────────────────────────────

/// A fully-loaded registry entry: the `SKILL.md` skill plus any optional
/// `skill.json` manifest metadata.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// Skill content loaded from `SKILL.md`.
    pub skill: Skill,
    /// Manifest data loaded from `skill.json`, or `None` if the file is absent.
    pub manifest: Option<SkillManifest>,
}

impl SkillEntry {
    // ── Convenience accessors ────────────────────────────────────────────────

    /// Skill name (sourced from `SKILL.md` frontmatter).
    pub fn name(&self) -> &str {
        &self.skill.config.name
    }

    /// Skill description (sourced from `SKILL.md` frontmatter).
    pub fn description(&self) -> &str {
        &self.skill.config.description
    }

    /// Effective arbitration score: from manifest when available, else `50`.
    pub fn arbitration_score(&self) -> u8 {
        self.manifest
            .as_ref()
            .map(|m| m.arbitration_score)
            .unwrap_or(50)
    }

    /// Effective version string: from manifest when available, else `"—"`.
    pub fn version(&self) -> &str {
        self.manifest
            .as_ref()
            .map(|m| m.version.as_str())
            .unwrap_or("—")
    }

    /// Author string from the manifest, if present.
    pub fn author(&self) -> Option<&str> {
        self.manifest.as_ref().and_then(|m| m.author.as_deref())
    }

    /// Tags from the manifest (empty slice when no manifest).
    pub fn tags(&self) -> &[String] {
        self.manifest
            .as_ref()
            .map(|m| m.tags.as_slice())
            .unwrap_or(&[])
    }

    /// Whether the skill is globally enabled.
    ///
    /// A skill without a manifest is always considered enabled.  Set to
    /// `false` via `SkillRegistry::set_enabled` to block activation.
    pub fn is_enabled(&self) -> bool {
        self.manifest.as_ref().map(|m| m.enabled).unwrap_or(true)
    }

    /// Dependency names declared in the manifest.
    pub fn dependencies(&self) -> &[String] {
        self.manifest
            .as_ref()
            .map(|m| m.dependencies.as_slice())
            .unwrap_or(&[])
    }
}

// ─── SkillRegistry ────────────────────────────────────────────────────────────

/// Manages the full skill collection: discovery, ranking, and context
/// generation.
///
/// # Usage
///
/// ```rust,ignore
/// let registry = SkillRegistry::load(&skills_dir)?;
///
/// // Show all skills sorted by arbitration score
/// for entry in registry.entries() {
///     println!("{} (score={})", entry.name(), entry.arbitration_score());
/// }
///
/// // Build ranked context for active skills
/// if let Some(ctx) = registry.ranked_context(&session.active_skills) {
///     // inject ctx into system prompt
/// }
/// ```
pub struct SkillRegistry {
    base_dir: PathBuf,
    /// Entries sorted by descending arbitration score.
    entries: Vec<SkillEntry>,
}

impl SkillRegistry {
    /// Create a new registry by scanning `base_dir` for skills.
    ///
    /// Returns an empty (valid) registry if the directory does not exist.
    pub fn load(base_dir: &Path) -> Result<Self> {
        let entries = discover_skills(base_dir)?;
        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            entries,
        })
    }

    /// Re-scan the skills directory and refresh the in-memory list.
    pub fn refresh(&mut self) -> Result<()> {
        self.entries = discover_skills(&self.base_dir)?;
        Ok(())
    }

    /// All discovered entries, sorted by **descending arbitration score**.
    pub fn entries(&self) -> &[SkillEntry] {
        &self.entries
    }

    /// Total number of discovered skills (enabled and disabled).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when no skills were found.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find a skill entry by exact name match.
    pub fn find(&self, name: &str) -> Option<&SkillEntry> {
        self.entries.iter().find(|e| e.name() == name)
    }

    /// Build a ranked context string for the supplied active skill names.
    ///
    /// Skills are ordered by **descending arbitration score** so the
    /// highest-priority skill's instructions appear first in the system
    /// prompt.  Returns `None` when `active_names` is empty or none of the
    /// names match known skills.
    pub fn ranked_context(&self, active_names: &[String]) -> Option<String> {
        if active_names.is_empty() {
            return None;
        }

        // Collect matching entries (preserve registry order = desc score)
        let mut active: Vec<&SkillEntry> = self
            .entries
            .iter()
            .filter(|e| active_names.contains(&e.name().to_string()))
            .collect();

        if active.is_empty() {
            return None;
        }

        // Secondary sort by the order the caller provided (tie-break within
        // equal scores keeps user intent intact).
        active.sort_by_key(|s| std::cmp::Reverse(s.arbitration_score()));

        let mut ctx = String::from(
            "\n\n## Active Skills\n\nThe following skills are currently active \
             (ordered by arbitration score — highest priority first):\n\n",
        );

        for entry in active {
            let score = entry.arbitration_score();
            ctx.push_str(&format!("### {} (priority: {})\n", entry.name(), score));
            ctx.push_str(&format!("Description: {}\n", entry.description()));

            let tags = entry.tags();
            if !tags.is_empty() {
                ctx.push_str(&format!("Tags: {}\n", tags.join(", ")));
            }

            let deps = entry.dependencies();
            if !deps.is_empty() {
                ctx.push_str(&format!("Dependencies: {}\n", deps.join(", ")));
            }

            ctx.push_str("\nInstructions:\n");
            ctx.push_str(&entry.skill.instructions);
            ctx.push_str("\n\n---\n\n");
        }

        Some(ctx)
    }

    /// Toggle the `enabled` flag on a skill and persist the change to
    /// `skill.json`.
    ///
    /// If no manifest exists for that skill a new one is created with
    /// defaults.  Other manifest fields are left unchanged.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.name() == name)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found in registry", name))?;

        let manifest = entry.manifest.get_or_insert_with(|| SkillManifest {
            name: name.to_string(),
            ..Default::default()
        });
        manifest.enabled = enabled;

        let manifest_path = entry.skill.path.join("skill.json");
        let json =
            serde_json::to_string_pretty(manifest).context("Failed to serialise skill manifest")?;
        fs::write(&manifest_path, json)
            .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

        Ok(())
    }

    /// Create or replace the `skill.json` manifest for a named skill.
    ///
    /// The in-memory entry is updated to reflect the new manifest.
    pub fn save_manifest(&mut self, name: &str, new_manifest: SkillManifest) -> Result<()> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.name() == name)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found in registry", name))?;

        let manifest_path = entry.skill.path.join("skill.json");
        let json = serde_json::to_string_pretty(&new_manifest)
            .context("Failed to serialise skill manifest")?;
        fs::write(&manifest_path, json)
            .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

        entry.manifest = Some(new_manifest);
        Ok(())
    }
}

// ─── Discovery ────────────────────────────────────────────────────────────────

/// Scan `base_dir` for skill directories and load each one.
///
/// A directory qualifies as a skill if it contains `SKILL.md` (handled by
/// the existing [`list_skills`] function).  The optional `skill.json`
/// manifest is loaded alongside it.
///
/// The returned list is sorted by **descending arbitration score**.
fn discover_skills(base_dir: &Path) -> Result<Vec<SkillEntry>> {
    let skills = list_skills(base_dir)?;

    let mut entries: Vec<SkillEntry> = skills
        .into_iter()
        .map(|skill| {
            // Attempt to load the manifest; ignore absence, surface real errors
            // as warnings (we don't want a corrupt manifest to block all skills).
            let manifest = match load_manifest(&skill.path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "⚠  Failed to load skill.json for '{}': {}",
                        skill.config.name, e
                    );
                    None
                }
            };
            SkillEntry { skill, manifest }
        })
        .collect();

    // Stable sort: descending arbitration score, alphabetical name as tie-break
    entries.sort_by(|a, b| {
        b.arbitration_score()
            .cmp(&a.arbitration_score())
            .then_with(|| a.name().cmp(b.name()))
    });

    Ok(entries)
}

// ─── Public helpers ───────────────────────────────────────────────────────────

/// Attempt to load a `skill.json` manifest from a skill directory.
///
/// Returns `Ok(None)` when the file is absent — this is not an error.
/// Returns `Err` only if the file exists but cannot be read or parsed.
pub fn load_manifest(skill_dir: &Path) -> Result<Option<SkillManifest>> {
    let path = skill_dir.join("skill.json");
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let manifest: SkillManifest = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON in {}", path.display()))?;
    Ok(Some(manifest))
}

/// Generate a default `skill.json` template for a newly created skill.
///
/// The caller should serialise and write this to `<skill_dir>/skill.json`
/// after the skill directory and `SKILL.md` have been created.
pub fn default_manifest_template(name: &str) -> SkillManifest {
    SkillManifest {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        author: None,
        description: None,
        tags: Vec::new(),
        arbitration_score: 50,
        enabled: true,
        dependencies: Vec::new(),
        min_grok_version: None,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn write_skill_md(dir: &Path, name: &str, desc: &str) {
        let md = format!(
            "---\nname: {}\ndescription: {}\n---\n\n# Instructions\n\nDo stuff for {}.\n",
            name, desc, name
        );
        fs::write(dir.join("SKILL.md"), md).unwrap();
    }

    fn write_manifest(dir: &Path, manifest: &SkillManifest) {
        let json = serde_json::to_string_pretty(manifest).unwrap();
        fs::write(dir.join("skill.json"), json).unwrap();
    }

    fn make_skill_dir(base: &Path, name: &str) -> PathBuf {
        let dir = base.join(name);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── load_manifest ─────────────────────────────────────────────────────────

    #[test]
    fn test_load_manifest_absent_returns_none() {
        let tmp = tempdir().unwrap();
        let result = load_manifest(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_manifest_present_parses_correctly() {
        let tmp = tempdir().unwrap();
        let m = SkillManifest {
            name: "test-skill".to_string(),
            version: "2.1.0".to_string(),
            author: Some("Alice".to_string()),
            arbitration_score: 85,
            tags: vec!["rust".to_string(), "systems".to_string()],
            enabled: true,
            ..Default::default()
        };
        write_manifest(tmp.path(), &m);

        let loaded = load_manifest(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.name, "test-skill");
        assert_eq!(loaded.version, "2.1.0");
        assert_eq!(loaded.author.as_deref(), Some("Alice"));
        assert_eq!(loaded.arbitration_score, 85);
        assert_eq!(loaded.tags, vec!["rust", "systems"]);
    }

    #[test]
    fn test_load_manifest_invalid_json_returns_error() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("skill.json"), "{ this is not valid json }").unwrap();
        assert!(load_manifest(tmp.path()).is_err());
    }

    // ── SkillEntry accessors ──────────────────────────────────────────────────

    #[test]
    fn test_entry_defaults_without_manifest() {
        let tmp = tempdir().unwrap();
        let dir = make_skill_dir(tmp.path(), "bare");
        write_skill_md(&dir, "bare", "A bare skill");

        let reg = SkillRegistry::load(tmp.path()).unwrap();
        let entry = reg.find("bare").expect("bare should be discovered");

        assert_eq!(entry.arbitration_score(), 50);
        assert_eq!(entry.version(), "—");
        assert!(entry.is_enabled());
        assert!(entry.author().is_none());
        assert!(entry.tags().is_empty());
        assert!(entry.dependencies().is_empty());
    }

    #[test]
    fn test_entry_values_from_manifest() {
        let tmp = tempdir().unwrap();
        let dir = make_skill_dir(tmp.path(), "rich");
        write_skill_md(&dir, "rich", "Rich skill");
        write_manifest(
            &dir,
            &SkillManifest {
                name: "rich".to_string(),
                version: "3.0.0".to_string(),
                author: Some("Bob".to_string()),
                arbitration_score: 70,
                tags: vec!["cli".to_string()],
                enabled: false,
                dependencies: vec!["other".to_string()],
                ..Default::default()
            },
        );

        let reg = SkillRegistry::load(tmp.path()).unwrap();
        let entry = reg.find("rich").unwrap();

        assert_eq!(entry.arbitration_score(), 70);
        assert_eq!(entry.version(), "3.0.0");
        assert_eq!(entry.author(), Some("Bob"));
        assert_eq!(entry.tags(), &["cli".to_string()]);
        assert!(!entry.is_enabled());
        assert_eq!(entry.dependencies(), &["other".to_string()]);
    }

    // ── SkillRegistry discovery & sorting ────────────────────────────────────

    #[test]
    fn test_registry_sorts_by_descending_score() {
        let tmp = tempdir().unwrap();

        for (name, score) in &[("alpha", 30u8), ("beta", 90u8), ("gamma", 60u8)] {
            let dir = make_skill_dir(tmp.path(), name);
            write_skill_md(&dir, name, &format!("{} desc", name));
            write_manifest(
                &dir,
                &SkillManifest {
                    name: name.to_string(),
                    arbitration_score: *score,
                    ..Default::default()
                },
            );
        }

        let reg = SkillRegistry::load(tmp.path()).unwrap();
        let names: Vec<&str> = reg.entries().iter().map(|e| e.name()).collect();
        // Expected order: beta(90) > gamma(60) > alpha(30)
        assert_eq!(names, vec!["beta", "gamma", "alpha"]);
    }

    #[test]
    fn test_registry_alphabetical_tiebreak() {
        let tmp = tempdir().unwrap();

        for name in &["zebra", "apple", "mango"] {
            let dir = make_skill_dir(tmp.path(), name);
            write_skill_md(&dir, name, "same score");
            write_manifest(
                &dir,
                &SkillManifest {
                    name: name.to_string(),
                    arbitration_score: 50,
                    ..Default::default()
                },
            );
        }

        let reg = SkillRegistry::load(tmp.path()).unwrap();
        let names: Vec<&str> = reg.entries().iter().map(|e| e.name()).collect();
        assert_eq!(names, vec!["apple", "mango", "zebra"]);
    }

    // ── ranked_context ────────────────────────────────────────────────────────

    #[test]
    fn test_ranked_context_order_matches_scores() {
        let tmp = tempdir().unwrap();

        for (name, score) in &[("low", 20u8), ("high", 95u8), ("mid", 60u8)] {
            let dir = make_skill_dir(tmp.path(), name);
            write_skill_md(&dir, name, &format!("{} skill", name));
            write_manifest(
                &dir,
                &SkillManifest {
                    name: name.to_string(),
                    arbitration_score: *score,
                    ..Default::default()
                },
            );
        }

        let reg = SkillRegistry::load(tmp.path()).unwrap();
        let active = vec!["low".to_string(), "high".to_string(), "mid".to_string()];
        let ctx = reg.ranked_context(&active).unwrap();

        let high_pos = ctx.find("### high").unwrap();
        let mid_pos = ctx.find("### mid").unwrap();
        let low_pos = ctx.find("### low").unwrap();
        assert!(high_pos < mid_pos, "high should come before mid");
        assert!(mid_pos < low_pos, "mid should come before low");
    }

    #[test]
    fn test_ranked_context_none_when_empty_active() {
        let tmp = tempdir().unwrap();
        let reg = SkillRegistry::load(tmp.path()).unwrap();
        assert!(reg.ranked_context(&[]).is_none());
    }

    #[test]
    fn test_ranked_context_none_when_no_match() {
        let tmp = tempdir().unwrap();
        let dir = make_skill_dir(tmp.path(), "real");
        write_skill_md(&dir, "real", "A real skill");

        let reg = SkillRegistry::load(tmp.path()).unwrap();
        let result = reg.ranked_context(&["nonexistent".to_string()]);
        assert!(result.is_none());
    }

    // ── set_enabled / save_manifest ───────────────────────────────────────────

    #[test]
    fn test_set_enabled_false_creates_manifest_on_disk() {
        let tmp = tempdir().unwrap();
        let skill_dir = make_skill_dir(tmp.path(), "my-skill");
        write_skill_md(&skill_dir, "my-skill", "Test skill");

        let mut reg = SkillRegistry::load(tmp.path()).unwrap();
        assert!(reg.find("my-skill").unwrap().is_enabled()); // default = true

        reg.set_enabled("my-skill", false).unwrap();

        // Persisted on disk
        let on_disk = load_manifest(&skill_dir).unwrap().unwrap();
        assert!(!on_disk.enabled);
        // In-memory also updated
        assert!(!reg.find("my-skill").unwrap().is_enabled());
    }

    #[test]
    fn test_set_enabled_preserves_existing_fields() {
        let tmp = tempdir().unwrap();
        let skill_dir = make_skill_dir(tmp.path(), "keep");
        write_skill_md(&skill_dir, "keep", "Keep fields");
        write_manifest(
            &skill_dir,
            &SkillManifest {
                name: "keep".to_string(),
                arbitration_score: 77,
                author: Some("Carol".to_string()),
                enabled: true,
                ..Default::default()
            },
        );

        let mut reg = SkillRegistry::load(tmp.path()).unwrap();
        reg.set_enabled("keep", false).unwrap();

        let on_disk = load_manifest(&skill_dir).unwrap().unwrap();
        assert!(!on_disk.enabled);
        assert_eq!(on_disk.arbitration_score, 77);
        assert_eq!(on_disk.author.as_deref(), Some("Carol"));
    }

    // ── default_manifest_template ─────────────────────────────────────────────

    #[test]
    fn test_default_manifest_template_fields() {
        let m = default_manifest_template("new-skill");
        assert_eq!(m.name, "new-skill");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.arbitration_score, 50);
        assert!(m.enabled);
        assert!(m.author.is_none());
        assert!(m.tags.is_empty());
        assert!(m.dependencies.is_empty());
    }
}
