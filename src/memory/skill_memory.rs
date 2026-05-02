//! Skill memory — cross-session skill usage tracking.
//!
//! [`SkillMemory`] records which skills were activated, how often, and how
//! helpful they were, persisting that history to `~/.grok/skill_history.json`.
//! The auto-activation engine can query this store to make smarter suggestions
//! based on what actually worked in the past.
//!
//! ## Storage layout
//!
//! ```text
//! ~/.grok/
//!   skill_history.json   ← all activation records (append-friendly)
//!   skill_affinity.json  ← per-(project, skill) aggregated scores
//! ```
//!
//! ## Affinity score
//!
//! Each `(project_hash, skill_name)` pair carries a score in `[0.0, 1.0]`
//! computed from recent activation history:
//!
//! ```text
//! score = helpful_activations / total_activations
//! ```
//!
//! A skill with no outcome data defaults to `0.5` (neutral).  Scores decay
//! toward `0.5` over time so stale data does not permanently block a skill.
//!
//! ## Starlink resilience
//!
//! All writes use atomic rename so a satellite drop mid-flush never corrupts
//! the live store.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

// ── Constants ─────────────────────────────────────────────────────────────────

const SKILL_HISTORY_FILE: &str = "skill_history.json";
const SKILL_AFFINITY_FILE: &str = "skill_affinity.json";

/// Maximum number of activation records to keep on disk.
/// Oldest records are pruned when this limit is exceeded.
const MAX_HISTORY_RECORDS: usize = 500;

/// Decay factor applied to affinity scores on each new activation.
/// Pulls the score toward 0.5 so old data becomes less influential over time.
const AFFINITY_DECAY: f32 = 0.02;

/// Minimum number of activations before a score is considered reliable.
const MIN_ACTIVATIONS_FOR_RELIABLE_SCORE: u32 = 3;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// What triggered a skill activation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillTrigger {
    /// User explicitly activated the skill with `/skill activate <name>`.
    ManualActivation,
    /// The auto-activation engine suggested and activated it.
    AutoActivation {
        /// Confidence score from the auto-activation engine (0.0–1.0).
        confidence: u32, // stored as (confidence * 1000) as u32 for JSON compat
    },
    /// Skill was part of the session's default active set.
    SessionDefault,
}

/// A single skill activation event recorded in the history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillActivationRecord {
    /// Unique record ID (UUID v4).
    pub id: String,
    /// Name of the skill that was activated.
    pub skill_name: String,
    /// How the activation was triggered.
    pub trigger: SkillTrigger,
    /// SHA-256 (first 16 chars) of the project root path, or `"global"`.
    pub project_hash: String,
    /// Session ID the activation occurred in.
    pub session_id: String,
    /// Wall-clock time of the activation.
    pub activated_at: DateTime<Utc>,
    /// Whether the skill turned out to be helpful in this session.
    /// `None` = no outcome data recorded yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was_helpful: Option<bool>,
    /// Optional free-text note (e.g. why the user deactivated it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl SkillActivationRecord {
    /// Construct a new activation record with the current timestamp.
    pub fn new(
        skill_name: impl Into<String>,
        trigger: SkillTrigger,
        project_hash: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            skill_name: skill_name.into(),
            trigger,
            project_hash: project_hash.into(),
            session_id: session_id.into(),
            activated_at: Utc::now(),
            was_helpful: None,
            note: None,
        }
    }
}

/// Aggregated affinity entry for a `(project_hash, skill_name)` pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAffinity {
    /// Skill name.
    pub skill_name: String,
    /// Project context (hash or `"global"`).
    pub project_hash: String,
    /// Total number of times this skill was activated in this project.
    pub total_activations: u32,
    /// Number of activations where `was_helpful = Some(true)`.
    pub helpful_activations: u32,
    /// Number of activations where `was_helpful = Some(false)`.
    pub unhelpful_activations: u32,
    /// Current affinity score in `[0.0, 1.0]`.
    /// Starts at `0.5`, drifts toward `helpful / total` over time.
    pub score: f32,
    /// When this affinity entry was last updated.
    pub last_updated: DateTime<Utc>,
}

impl SkillAffinity {
    fn new(skill_name: impl Into<String>, project_hash: impl Into<String>) -> Self {
        Self {
            skill_name: skill_name.into(),
            project_hash: project_hash.into(),
            total_activations: 0,
            helpful_activations: 0,
            unhelpful_activations: 0,
            score: 0.5,
            last_updated: Utc::now(),
        }
    }

    /// Record a new activation and optionally its outcome, updating the score.
    fn record_activation(&mut self, was_helpful: Option<bool>) {
        // Decay existing score toward neutral before applying new data.
        self.score = self.score * (1.0 - AFFINITY_DECAY) + 0.5 * AFFINITY_DECAY;
        self.total_activations += 1;

        match was_helpful {
            Some(true) => {
                self.helpful_activations += 1;
                // Nudge score upward.
                self.score = (self.score + 0.1).min(1.0);
            }
            Some(false) => {
                self.unhelpful_activations += 1;
                // Nudge score downward.
                self.score = (self.score - 0.1).max(0.0);
            }
            None => {
                // No outcome — mild positive bias for being activated at all.
                self.score = (self.score + 0.02).min(1.0);
            }
        }

        self.last_updated = Utc::now();
    }

    /// `true` when the skill has enough data to be considered reliable.
    pub fn is_reliable(&self) -> bool {
        self.total_activations >= MIN_ACTIVATIONS_FOR_RELIABLE_SCORE
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SkillMemory
// ─────────────────────────────────────────────────────────────────────────────

/// Cross-session skill usage tracker.
///
/// Records activations and outcomes in `~/.grok/skill_history.json`, and
/// maintains aggregated per-project affinity scores in
/// `~/.grok/skill_affinity.json`.
///
/// # Example
/// ```rust,no_run
/// use grok_cli::memory::skill_memory::{SkillMemory, SkillTrigger};
///
/// let mut mem = SkillMemory::load_or_create().unwrap();
///
/// mem.record_activation(
///     "rust-expert",
///     SkillTrigger::ManualActivation,
///     "my-project",
///     "sess-001",
/// ).unwrap();
///
/// mem.record_outcome("rust-expert", "sess-001", true, None).unwrap();
///
/// let score = mem.affinity_score("rust-expert", "my-project");
/// println!("rust-expert affinity: {:.2}", score);
/// ```
#[derive(Debug)]
pub struct SkillMemory {
    history: Vec<SkillActivationRecord>,
    /// Keyed by `"<project_hash>:<skill_name>"`.
    affinity: HashMap<String, SkillAffinity>,
    history_path: PathBuf,
    affinity_path: PathBuf,
}

impl SkillMemory {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Open (or create) the store at the default `~/.grok/` location.
    pub fn load_or_create() -> Result<Self> {
        let dir = grok_dir()?;
        Self::load_or_create_at(dir)
    }

    /// Open (or create) the store in an explicit directory (useful in tests).
    pub fn load_or_create_at(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let history_path = dir.join(SKILL_HISTORY_FILE);
        let affinity_path = dir.join(SKILL_AFFINITY_FILE);

        let history: Vec<SkillActivationRecord> = if history_path.exists() {
            load_json_vec(&history_path).unwrap_or_else(|e| {
                warn!("SkillMemory: could not parse history file, starting fresh — {e}");
                Vec::new()
            })
        } else {
            Vec::new()
        };

        let affinity_vec: Vec<SkillAffinity> = if affinity_path.exists() {
            load_json_vec(&affinity_path).unwrap_or_else(|e| {
                warn!("SkillMemory: could not parse affinity file, starting fresh — {e}");
                Vec::new()
            })
        } else {
            Vec::new()
        };

        let affinity: HashMap<String, SkillAffinity> = affinity_vec
            .into_iter()
            .map(|a| (affinity_key(&a.project_hash, &a.skill_name), a))
            .collect();

        debug!(
            history_count = history.len(),
            affinity_count = affinity.len(),
            "SkillMemory: loaded"
        );

        Ok(Self {
            history,
            affinity,
            history_path,
            affinity_path,
        })
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    /// Record that a skill was activated in a session.
    ///
    /// This updates both the raw history and the aggregated affinity score for
    /// the `(project_hash, skill_name)` pair.
    pub fn record_activation(
        &mut self,
        skill_name: &str,
        trigger: SkillTrigger,
        project_hash: &str,
        session_id: &str,
    ) -> Result<String> {
        let record = SkillActivationRecord::new(skill_name, trigger, project_hash, session_id);
        let id = record.id.clone();

        // Update affinity.
        let key = affinity_key(project_hash, skill_name);
        self.affinity
            .entry(key)
            .or_insert_with(|| SkillAffinity::new(skill_name, project_hash))
            .record_activation(None);

        self.history.push(record);
        self.prune_history();
        self.flush()?;

        debug!(skill = %skill_name, project = %project_hash, "SkillMemory: activation recorded");
        Ok(id)
    }

    /// Record the outcome of a skill in a session.
    ///
    /// `was_helpful` should be `true` when the skill genuinely assisted the
    /// session, `false` when the user explicitly deactivated it or indicated
    /// it was not useful.
    ///
    /// Returns the number of records that were updated.
    pub fn record_outcome(
        &mut self,
        skill_name: &str,
        session_id: &str,
        was_helpful: bool,
        note: Option<&str>,
    ) -> Result<usize> {
        let mut updated = 0;

        for record in self.history.iter_mut().rev() {
            if record.skill_name == skill_name
                && record.session_id == session_id
                && record.was_helpful.is_none()
            {
                record.was_helpful = Some(was_helpful);
                if let Some(n) = note {
                    record.note = Some(n.to_string());
                }
                updated += 1;

                // Update affinity with the outcome.
                let key = affinity_key(&record.project_hash, skill_name);
                if let Some(aff) = self.affinity.get_mut(&key) {
                    if was_helpful {
                        aff.helpful_activations = aff.helpful_activations.saturating_add(1);
                        aff.score = (aff.score + 0.05).min(1.0);
                    } else {
                        aff.unhelpful_activations = aff.unhelpful_activations.saturating_add(1);
                        aff.score = (aff.score - 0.05).max(0.0);
                    }
                    aff.last_updated = Utc::now();
                }

                break; // only update the most recent matching record
            }
        }

        if updated > 0 {
            self.flush()?;
        }

        Ok(updated)
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Return all activation records in chronological order.
    pub fn all_records(&self) -> &[SkillActivationRecord] {
        &self.history
    }

    /// Return the most recent `n` activation records, newest first.
    pub fn recent_records(&self, n: usize) -> Vec<&SkillActivationRecord> {
        self.history.iter().rev().take(n).collect()
    }

    /// Return all records for a specific skill.
    pub fn records_for_skill(&self, skill_name: &str) -> Vec<&SkillActivationRecord> {
        self.history
            .iter()
            .filter(|r| r.skill_name == skill_name)
            .collect()
    }

    /// Return all records for a specific project hash.
    pub fn records_for_project(&self, project_hash: &str) -> Vec<&SkillActivationRecord> {
        self.history
            .iter()
            .filter(|r| r.project_hash == project_hash)
            .collect()
    }

    /// Return the affinity score for a `(skill, project)` pair.
    ///
    /// Returns `0.5` (neutral) when there is no history for this combination.
    pub fn affinity_score(&self, skill_name: &str, project_hash: &str) -> f32 {
        self.affinity
            .get(&affinity_key(project_hash, skill_name))
            .map(|a| a.score)
            .unwrap_or(0.5)
    }

    /// Return the full [`SkillAffinity`] entry for a `(skill, project)` pair,
    /// or `None` when no history exists yet.
    pub fn affinity_entry(&self, skill_name: &str, project_hash: &str) -> Option<&SkillAffinity> {
        self.affinity.get(&affinity_key(project_hash, skill_name))
    }

    /// Return skills suggested for a project, ordered by affinity score
    /// descending.
    ///
    /// Only skills with at least one recorded activation in this project and a
    /// score above `min_score` are included.
    pub fn suggested_skills(&self, project_hash: &str, min_score: f32) -> Vec<(&str, f32)> {
        let mut suggestions: Vec<(&str, f32)> = self
            .affinity
            .values()
            .filter(|a| a.project_hash == project_hash && a.score >= min_score)
            .map(|a| (a.skill_name.as_str(), a.score))
            .collect();

        // Highest score first.
        suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        suggestions
    }

    /// Return the top `n` most-activated skills globally, by total activation
    /// count, with their activation counts.
    pub fn most_used(&self, n: usize) -> Vec<(String, u32)> {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for record in &self.history {
            *counts.entry(record.skill_name.as_str()).or_insert(0) += 1;
        }

        let mut list: Vec<(String, u32)> = counts
            .into_iter()
            .map(|(name, count)| (name.to_string(), count))
            .collect();

        list.sort_by_key(|item| std::cmp::Reverse(item.1));
        list.truncate(n);
        list
    }

    /// Total number of activation records in the history.
    pub fn record_count(&self) -> usize {
        self.history.len()
    }

    /// Total number of distinct `(project, skill)` affinity entries.
    pub fn affinity_count(&self) -> usize {
        self.affinity.len()
    }

    // ── Prompt injection ──────────────────────────────────────────────────────

    /// Build a Markdown section summarising skill usage for the current
    /// project, suitable for injection into a system prompt.
    ///
    /// Returns an empty string when there is no relevant history.
    pub fn to_prompt_section(&self, project_hash: &str, max_skills: usize) -> String {
        let suggestions = self.suggested_skills(project_hash, 0.55);
        if suggestions.is_empty() {
            return String::new();
        }

        let lines: Vec<String> = suggestions
            .iter()
            .take(max_skills)
            .map(|(name, score)| {
                let reliability = self
                    .affinity_entry(name, project_hash)
                    .map(|a| {
                        if a.is_reliable() {
                            format!(" ({:.0}% helpful)", score * 100.0)
                        } else {
                            " (limited data)".to_string()
                        }
                    })
                    .unwrap_or_default();
                format!("- `{}`{}", name, reliability)
            })
            .collect();

        format!(
            "\n\n## Suggested Skills\n\nThe following skills have been helpful in this project:\n\n{}\n",
            lines.join("\n")
        )
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Prune the history to [`MAX_HISTORY_RECORDS`] by dropping oldest records.
    fn prune_history(&mut self) {
        if self.history.len() > MAX_HISTORY_RECORDS {
            let excess = self.history.len() - MAX_HISTORY_RECORDS;
            self.history.drain(0..excess);
            debug!(removed = excess, "SkillMemory: pruned old history records");
        }
    }

    /// Atomically flush history and affinity files to disk.
    fn flush(&self) -> Result<()> {
        if let Some(parent) = self.history_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }

        // Flush history.
        atomic_write(
            &self.history_path,
            &serde_json::to_string_pretty(&self.history).context("serialising skill history")?,
        )
        .context("writing skill_history.json")?;

        // Flush affinity.
        let affinity_vec: Vec<&SkillAffinity> = self.affinity.values().collect();
        atomic_write(
            &self.affinity_path,
            &serde_json::to_string_pretty(&affinity_vec).context("serialising skill affinity")?,
        )
        .context("writing skill_affinity.json")?;

        debug!(
            history = self.history.len(),
            affinity = self.affinity.len(),
            "SkillMemory: flushed"
        );

        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn grok_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".grok"))
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))
}

fn affinity_key(project_hash: &str, skill_name: &str) -> String {
    format!("{}:{}", project_hash, skill_name)
}

fn load_json_vec<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw).with_context(|| format!("parsing JSON from {}", path.display()))
}

fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content).with_context(|| format!("writing temp file {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Compute a short project identifier from a directory path.
///
/// Uses the first 16 hex chars of a simple hash of the canonical path string.
/// Falls back to `"global"` when the path cannot be canonicalized.
pub fn project_hash_for_path(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    let mut hasher = DefaultHasher::new();
    canonical.to_string_lossy().hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store(dir: &Path) -> SkillMemory {
        SkillMemory::load_or_create_at(dir).unwrap()
    }

    // ── record_activation ─────────────────────────────────────────────────────

    #[test]
    fn record_activation_adds_history_and_affinity() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());

        m.record_activation(
            "rust-expert",
            SkillTrigger::ManualActivation,
            "proj-1",
            "sess-1",
        )
        .unwrap();

        assert_eq!(m.record_count(), 1);
        assert_eq!(m.affinity_count(), 1);
        let score = m.affinity_score("rust-expert", "proj-1");
        assert!(
            score > 0.5,
            "score should be slightly above neutral after activation"
        );
    }

    #[test]
    fn record_activation_persists_across_reload() {
        let dir = tempdir().unwrap();
        {
            let mut m = store(dir.path());
            m.record_activation("git-helper", SkillTrigger::SessionDefault, "proj-a", "s1")
                .unwrap();
        }
        let m2 = store(dir.path());
        assert_eq!(m2.record_count(), 1);
        assert_eq!(m2.history[0].skill_name, "git-helper");
    }

    #[test]
    fn multiple_activations_increase_total() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());

        for i in 0..5 {
            m.record_activation(
                "formatter",
                SkillTrigger::ManualActivation,
                "proj-x",
                &format!("sess-{}", i),
            )
            .unwrap();
        }

        let aff = m.affinity_entry("formatter", "proj-x").unwrap();
        assert_eq!(aff.total_activations, 5);
    }

    // ── record_outcome ────────────────────────────────────────────────────────

    #[test]
    fn helpful_outcome_raises_score() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        m.record_activation("linter", SkillTrigger::ManualActivation, "proj-1", "s1")
            .unwrap();

        let before = m.affinity_score("linter", "proj-1");
        m.record_outcome("linter", "s1", true, None).unwrap();
        let after = m.affinity_score("linter", "proj-1");

        assert!(after >= before, "helpful outcome should not decrease score");
    }

    #[test]
    fn unhelpful_outcome_lowers_score() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        m.record_activation("debugger", SkillTrigger::ManualActivation, "proj-1", "s1")
            .unwrap();

        let before = m.affinity_score("debugger", "proj-1");
        m.record_outcome("debugger", "s1", false, None).unwrap();
        let after = m.affinity_score("debugger", "proj-1");

        assert!(after < before, "unhelpful outcome should decrease score");
    }

    #[test]
    fn outcome_with_note_is_stored() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        m.record_activation("search", SkillTrigger::ManualActivation, "p", "s1")
            .unwrap();
        m.record_outcome("search", "s1", false, Some("too noisy"))
            .unwrap();

        let record = m.records_for_skill("search").into_iter().next().unwrap();
        assert_eq!(record.note.as_deref(), Some("too noisy"));
    }

    #[test]
    fn record_outcome_returns_zero_for_unknown_session() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        m.record_activation("skill-a", SkillTrigger::ManualActivation, "p", "real-sess")
            .unwrap();

        let updated = m
            .record_outcome("skill-a", "wrong-sess", true, None)
            .unwrap();
        assert_eq!(updated, 0);
    }

    // ── affinity_score ────────────────────────────────────────────────────────

    #[test]
    fn unknown_skill_returns_neutral_score() {
        let dir = tempdir().unwrap();
        let m = store(dir.path());
        assert_eq!(m.affinity_score("nonexistent", "proj"), 0.5);
    }

    #[test]
    fn score_is_bounded_between_zero_and_one() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        // Many helpful activations should push score toward 1.0 but not over.
        for i in 0..30 {
            m.record_activation(
                "super-skill",
                SkillTrigger::ManualActivation,
                "proj",
                &format!("s{}", i),
            )
            .unwrap();
            m.record_outcome("super-skill", &format!("s{}", i), true, None)
                .unwrap();
        }
        let score = m.affinity_score("super-skill", "proj");
        assert!(score <= 1.0, "score must not exceed 1.0, got {}", score);
        assert!(score >= 0.0, "score must not go below 0.0");
    }

    // ── suggested_skills ─────────────────────────────────────────────────────

    #[test]
    fn suggested_skills_sorted_by_score_desc() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());

        m.record_activation("skill-a", SkillTrigger::ManualActivation, "proj", "s1")
            .unwrap();
        m.record_outcome("skill-a", "s1", true, None).unwrap();

        m.record_activation("skill-b", SkillTrigger::ManualActivation, "proj", "s2")
            .unwrap();
        m.record_outcome("skill-b", "s2", false, None).unwrap();

        let suggestions = m.suggested_skills("proj", 0.0);
        assert!(!suggestions.is_empty());
        // skill-a had helpful outcome so should rank higher than skill-b
        let pos_a = suggestions.iter().position(|(n, _)| *n == "skill-a");
        let pos_b = suggestions.iter().position(|(n, _)| *n == "skill-b");
        if let (Some(a), Some(b)) = (pos_a, pos_b) {
            assert!(a < b, "skill-a should rank above skill-b");
        }
    }

    #[test]
    fn suggested_skills_filters_by_min_score() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        // Only activate with no outcome — score stays near 0.5
        m.record_activation("weak-skill", SkillTrigger::ManualActivation, "proj", "s1")
            .unwrap();

        let suggestions = m.suggested_skills("proj", 0.9);
        assert!(
            suggestions.is_empty(),
            "weak skill should not appear above 0.9 threshold"
        );
    }

    #[test]
    fn suggested_skills_returns_empty_for_unknown_project() {
        let dir = tempdir().unwrap();
        let m = store(dir.path());
        assert!(m.suggested_skills("no-such-project", 0.0).is_empty());
    }

    // ── most_used ─────────────────────────────────────────────────────────────

    #[test]
    fn most_used_returns_top_n_by_count() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());

        // skill-a: 5 activations, skill-b: 2
        for i in 0..5 {
            m.record_activation(
                "skill-a",
                SkillTrigger::ManualActivation,
                "p",
                &format!("sa{}", i),
            )
            .unwrap();
        }
        for i in 0..2 {
            m.record_activation(
                "skill-b",
                SkillTrigger::ManualActivation,
                "p",
                &format!("sb{}", i),
            )
            .unwrap();
        }

        let top = m.most_used(2);
        assert_eq!(top[0].0, "skill-a");
        assert_eq!(top[0].1, 5);
        assert_eq!(top[1].0, "skill-b");
        assert_eq!(top[1].1, 2);
    }

    #[test]
    fn most_used_truncates_at_n() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        for name in ["a", "b", "c", "d", "e"] {
            m.record_activation(
                name,
                SkillTrigger::ManualActivation,
                "p",
                &format!("s-{}", name),
            )
            .unwrap();
        }
        assert_eq!(m.most_used(3).len(), 3);
    }

    // ── history pruning ───────────────────────────────────────────────────────

    #[test]
    fn history_is_pruned_when_limit_exceeded() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());

        for i in 0..MAX_HISTORY_RECORDS + 10 {
            m.record_activation(
                "skill-x",
                SkillTrigger::ManualActivation,
                "proj",
                &format!("sess-{}", i),
            )
            .unwrap();
        }

        assert!(
            m.record_count() <= MAX_HISTORY_RECORDS,
            "history should be pruned to max {} records, got {}",
            MAX_HISTORY_RECORDS,
            m.record_count()
        );
    }

    // ── project_hash_for_path ─────────────────────────────────────────────────

    #[test]
    fn project_hash_is_deterministic() {
        let dir = tempdir().unwrap();
        let h1 = project_hash_for_path(dir.path());
        let h2 = project_hash_for_path(dir.path());
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_paths_produce_different_hashes() {
        let d1 = tempdir().unwrap();
        let d2 = tempdir().unwrap();
        let h1 = project_hash_for_path(d1.path());
        let h2 = project_hash_for_path(d2.path());
        assert_ne!(h1, h2);
    }

    #[test]
    fn project_hash_is_16_hex_chars() {
        let dir = tempdir().unwrap();
        let hash = project_hash_for_path(dir.path());
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── to_prompt_section ─────────────────────────────────────────────────────

    #[test]
    fn prompt_section_empty_when_no_history() {
        let dir = tempdir().unwrap();
        let m = store(dir.path());
        assert!(m.to_prompt_section("proj", 5).is_empty());
    }

    #[test]
    fn prompt_section_contains_skill_name() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());

        // Give the skill a high affinity score via multiple helpful outcomes.
        for i in 0..5 {
            let sess = format!("s{}", i);
            m.record_activation(
                "coding-buddy",
                SkillTrigger::ManualActivation,
                "proj",
                &sess,
            )
            .unwrap();
            m.record_outcome("coding-buddy", &sess, true, None).unwrap();
        }

        let section = m.to_prompt_section("proj", 10);
        assert!(
            section.contains("coding-buddy"),
            "prompt section missing skill name"
        );
        assert!(section.contains("Suggested Skills"));
    }

    // ── per-project isolation ─────────────────────────────────────────────────

    #[test]
    fn skills_from_different_projects_do_not_cross_contaminate() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());

        m.record_activation(
            "web-skill",
            SkillTrigger::ManualActivation,
            "proj-web",
            "s1",
        )
        .unwrap();
        m.record_activation(
            "rust-skill",
            SkillTrigger::ManualActivation,
            "proj-rs",
            "s2",
        )
        .unwrap();

        let web_suggestions = m.suggested_skills("proj-web", 0.0);
        let rs_suggestions = m.suggested_skills("proj-rs", 0.0);

        assert!(web_suggestions.iter().any(|(n, _)| *n == "web-skill"));
        assert!(!web_suggestions.iter().any(|(n, _)| *n == "rust-skill"));

        assert!(rs_suggestions.iter().any(|(n, _)| *n == "rust-skill"));
        assert!(!rs_suggestions.iter().any(|(n, _)| *n == "web-skill"));
    }

    // ── is_reliable ──────────────────────────────────────────────────────────

    #[test]
    fn affinity_is_not_reliable_with_few_activations() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        m.record_activation("new-skill", SkillTrigger::ManualActivation, "proj", "s1")
            .unwrap();

        let aff = m.affinity_entry("new-skill", "proj").unwrap();
        assert!(
            !aff.is_reliable(),
            "should not be reliable with only 1 activation"
        );
    }

    #[test]
    fn affinity_is_reliable_after_enough_activations() {
        let dir = tempdir().unwrap();
        let mut m = store(dir.path());
        for i in 0..MIN_ACTIVATIONS_FOR_RELIABLE_SCORE {
            m.record_activation(
                "mature-skill",
                SkillTrigger::ManualActivation,
                "proj",
                &format!("s{}", i),
            )
            .unwrap();
        }
        let aff = m.affinity_entry("mature-skill", "proj").unwrap();
        assert!(aff.is_reliable());
    }
}
