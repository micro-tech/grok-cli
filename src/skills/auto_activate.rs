//! Skill auto-activation engine.
//!
//! Analyses the user's message and current working directory against each
//! skill's declared trigger conditions (`auto-activate` frontmatter block) and
//! returns a ranked list of skills that should be suggested or automatically
//! activated for the session.
//!
//! # Scoring
//!
//! Each trigger type contributes a partial score:
//!
//! | Trigger              | Score added per match |
//! |----------------------|-----------------------|
//! | Keyword match        | 30 points             |
//! | Regex pattern match  | 40 points             |
//! | File-extension match | 25 points             |
//!
//! Scores are capped at 100.  A skill is considered "triggered" when its
//! accumulated score meets or exceeds its `min_confidence` threshold
//! (default: 50).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use regex::Regex;
use tracing::{debug, warn};
use walkdir::WalkDir;

use crate::skills::config::Skill;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A skill that matched the current context, together with its confidence
/// score and the reasons that triggered it.
#[derive(Debug, Clone)]
pub struct SkillMatch {
    /// The skill name (mirrors `Skill::config::name`).
    pub skill_name: String,

    /// Confidence score in the range `[0, 100]`.
    pub confidence: u8,

    /// Human-readable reasons explaining why the skill was matched.
    pub reasons: Vec<String>,
}

/// The auto-activation engine.
///
/// Construct once, then call [`AutoActivationEngine::check`] for every user
/// message.  The engine is stateless with respect to sessions; the caller is
/// responsible for keeping track of which skills have already been activated.
pub struct AutoActivationEngine {
    /// Score contributed by a single keyword hit.
    keyword_score: u8,
    /// Score contributed by a single regex pattern hit.
    pattern_score: u8,
    /// Score contributed by a file-extension hit.
    extension_score: u8,
}

impl Default for AutoActivationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoActivationEngine {
    /// Create a new engine with the default scoring weights.
    pub fn new() -> Self {
        Self {
            keyword_score: 30,
            pattern_score: 40,
            extension_score: 25,
        }
    }

    /// Evaluate all `available_skills` against the user `input` and the
    /// `working_dir`.  Returns only skills whose confidence meets their own
    /// `min_confidence` threshold, sorted by descending confidence.
    ///
    /// Skills that are listed in `already_active` are silently skipped so the
    /// caller never sees duplicate suggestions.
    ///
    /// # Arguments
    ///
    /// * `input`          – The raw text the user just typed.
    /// * `working_dir`    – Current working directory of the session.
    /// * `available_skills` – All skills loaded from the skills directory.
    /// * `already_active` – Names of skills that are already active in the
    ///   session (these are excluded from the result).
    pub fn check(
        &self,
        input: &str,
        working_dir: &Path,
        available_skills: &[Skill],
        already_active: &[String],
    ) -> Vec<SkillMatch> {
        let active_set: HashSet<&str> = already_active.iter().map(|s| s.as_str()).collect();

        // Lazily compute the set of file extensions present in the working
        // directory (only when at least one skill needs it).
        let needs_extension_check = available_skills.iter().any(|s| {
            s.config
                .auto_activate
                .as_ref()
                .map(|a| a.enabled && !a.file_extensions.is_empty())
                .unwrap_or(false)
        });

        let dir_extensions: HashSet<String> = if needs_extension_check {
            collect_extensions(working_dir)
        } else {
            HashSet::new()
        };

        let input_lower = input.to_lowercase();

        let mut matches: Vec<SkillMatch> = available_skills
            .iter()
            .filter_map(|skill| {
                // Skip already-active skills.
                if active_set.contains(skill.config.name.as_str()) {
                    return None;
                }

                // Skip skills with no auto-activate configuration.
                let auto_cfg = skill.config.auto_activate.as_ref()?;

                // Respect the per-skill opt-out flag.
                if !auto_cfg.enabled {
                    return None;
                }

                let mut score: u32 = 0;
                let mut reasons: Vec<String> = Vec::new();

                // ── Keyword matching ─────────────────────────────────────
                for keyword in &auto_cfg.keywords {
                    let kw_lower = keyword.to_lowercase();
                    if input_lower.contains(&kw_lower) {
                        let contribution = self.keyword_score as u32;
                        score += contribution;
                        reasons.push(format!(
                            "keyword match: \"{}\" (+{})",
                            keyword, contribution
                        ));
                        debug!(
                            skill = %skill.config.name,
                            keyword = %keyword,
                            score,
                            "Auto-activate keyword hit"
                        );
                        // One keyword match is sufficient to contribute its
                        // full weight; additional keywords add more.
                    }
                }

                // ── Regex pattern matching ───────────────────────────────
                for pattern_str in &auto_cfg.patterns {
                    match Regex::new(pattern_str) {
                        Ok(re) => {
                            if re.is_match(input) {
                                let contribution = self.pattern_score as u32;
                                score += contribution;
                                reasons.push(format!(
                                    "pattern match: /{pattern_str}/ (+{contribution})"
                                ));
                                debug!(
                                    skill = %skill.config.name,
                                    pattern = %pattern_str,
                                    score,
                                    "Auto-activate regex hit"
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                skill = %skill.config.name,
                                pattern = %pattern_str,
                                error = %e,
                                "Skill has invalid regex in auto-activate.patterns – skipping"
                            );
                        }
                    }
                }

                // ── File-extension matching ──────────────────────────────
                if !auto_cfg.file_extensions.is_empty() {
                    for ext in &auto_cfg.file_extensions {
                        let ext_lower = ext.trim_start_matches('.').to_lowercase();
                        if dir_extensions.contains(&ext_lower) {
                            let contribution = self.extension_score as u32;
                            score += contribution;
                            reasons.push(format!(
                                "file extension in project: .{ext_lower} (+{contribution})"
                            ));
                            debug!(
                                skill = %skill.config.name,
                                ext = %ext_lower,
                                score,
                                "Auto-activate extension hit"
                            );
                            // Only count the extension check once even if
                            // multiple listed extensions are present.
                            break;
                        }
                    }
                }

                // Cap score at 100.
                let confidence = score.min(100) as u8;

                // Apply per-skill minimum confidence threshold.
                if confidence < auto_cfg.min_confidence {
                    debug!(
                        skill = %skill.config.name,
                        confidence,
                        threshold = auto_cfg.min_confidence,
                        "Auto-activate below threshold – skipping"
                    );
                    return None;
                }

                if reasons.is_empty() {
                    return None;
                }

                Some(SkillMatch {
                    skill_name: skill.config.name.clone(),
                    confidence,
                    reasons,
                })
            })
            .collect();

        // Sort by descending confidence so the most relevant skill comes first.
        matches.sort_by_key(|m| std::cmp::Reverse(m.confidence));
        matches
    }

    /// Convenience wrapper: check a single skill against the given input and
    /// working directory.  Returns `Some(SkillMatch)` if the skill is
    /// triggered, `None` otherwise.
    ///
    /// Useful in tests and for one-off validations.
    pub fn check_single(
        &self,
        input: &str,
        working_dir: &Path,
        skill: &Skill,
        already_active: &[String],
    ) -> Option<SkillMatch> {
        self.check(
            input,
            working_dir,
            std::slice::from_ref(skill),
            already_active,
        )
        .into_iter()
        .next()
    }

    /// Like [`check`] but adjusts scores using an optional [`crate::rpl::ReasoningTrace`].
    ///
    /// # Reasoning adjustments applied
    ///
    /// - **Tool-name keyword boost**: if a skill's keyword list contains a tool
    ///   name that appears in `trace.tool_evaluations` with `selected = true` and
    ///   `relevance_score >= 0.7`, add `tool_boost` points (default 15) to that
    ///   skill's confidence.
    ///
    /// - **Uncertainty penalty**: if `trace.uncertainty >= 0.7`, reduce every
    ///   matched skill's confidence by `uncertainty_penalty` points (default 10)
    ///   to signal lower overall reliability.  Scores are clamped to `[0, 100]`.
    ///
    /// # Fallback
    ///
    /// When `reasoning` is `None`, this method behaves identically to [`check`].
    pub fn check_with_reasoning(
        &self,
        input: &str,
        working_dir: &std::path::Path,
        available_skills: &[crate::skills::config::Skill],
        already_active: &[String],
        reasoning: Option<&crate::rpl::ReasoningTrace>,
    ) -> Vec<SkillMatch> {
        let mut matches = self.check(input, working_dir, available_skills, already_active);

        let trace = match reasoning {
            Some(t) => t,
            None => return matches,
        };

        // Build a map of lowercased tool name → original tool name for all
        // tools that were selected with sufficient relevance.
        let selected_tools: HashMap<String, String> = trace
            .tool_evaluations
            .iter()
            .filter(|te| te.selected && te.relevance_score >= 0.7)
            .map(|te| (te.tool_name.to_lowercase(), te.tool_name.clone()))
            .collect();

        let apply_penalty = trace.uncertainty >= 0.7;

        for m in &mut matches {
            let mut confidence = m.confidence as u32;

            // ── Tool-name keyword boost ──────────────────────────────────
            if !selected_tools.is_empty()
                && let Some(skill) = available_skills
                    .iter()
                    .find(|s| s.config.name == m.skill_name)
                && let Some(auto_cfg) = skill.config.auto_activate.as_ref()
            {
                'kw: for keyword in &auto_cfg.keywords {
                    let kw_lower = keyword.to_lowercase();
                    if let Some(orig_name) = selected_tools.get(&kw_lower) {
                        confidence = confidence.saturating_add(15).min(100);
                        m.reasons.push(format!("RPL tool match: {orig_name}"));
                        // A single boost per skill is sufficient.
                        break 'kw;
                    }
                }
            }

            // ── Uncertainty penalty ──────────────────────────────────────
            if apply_penalty {
                confidence = confidence.saturating_sub(10);
                m.reasons
                    .push("RPL uncertainty penalty applied".to_string());
            }

            m.confidence = confidence as u8;
        }

        // Re-sort by descending confidence after adjustments.
        matches.sort_by_key(|m| std::cmp::Reverse(m.confidence));

        // Filter out matches that now fall below their skill's min_confidence.
        matches.retain(|m| {
            available_skills
                .iter()
                .find(|s| s.config.name == m.skill_name)
                .and_then(|s| s.config.auto_activate.as_ref())
                .map(|cfg| m.confidence >= cfg.min_confidence)
                .unwrap_or(true)
        });

        matches
    }

    /// Like [`check_with_reasoning`] but applies an explicit fallback when
    /// reasoning uncertainty is too high or when no skills are matched.
    ///
    /// # Fallback behaviour
    ///
    /// When `reasoning` is `Some(trace)` and `trace.uncertainty >= 0.9`,
    /// **all** matched skills are returned with their confidence halved, and
    /// an additional reason `"RPL high-uncertainty fallback"` is appended to
    /// each.  This signals to the caller that skill activation should be
    /// treated as tentative.
    ///
    /// When `reasoning` is `None` or uncertainty is below `0.9`, this method
    /// delegates to [`check_with_reasoning`] unchanged.
    pub fn check_with_fallback(
        &self,
        input: &str,
        working_dir: &std::path::Path,
        available_skills: &[crate::skills::config::Skill],
        already_active: &[String],
        reasoning: Option<&crate::rpl::ReasoningTrace>,
    ) -> Vec<SkillMatch> {
        let mut matches = self.check_with_reasoning(
            input,
            working_dir,
            available_skills,
            already_active,
            reasoning,
        );

        let high_uncertainty = reasoning.map(|t| t.uncertainty >= 0.9).unwrap_or(false);

        if high_uncertainty {
            for m in &mut matches {
                m.confidence /= 2;
                m.reasons.push("RPL high-uncertainty fallback".to_string());
            }
        }

        matches
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk `dir` (non-recursively up to 3 levels deep) and collect all unique
/// file extensions found, lower-cased and without a leading dot.
///
/// The walk is intentionally shallow to avoid scanning large `target/` or
/// `node_modules/` trees.
fn collect_extensions(dir: &Path) -> HashSet<String> {
    let mut exts = HashSet::new();

    if !dir.exists() {
        return exts;
    }

    for entry in WalkDir::new(dir)
        .min_depth(1)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|e| e.to_str())
        {
            exts.insert(ext.to_lowercase());
        }
    }

    exts
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::config::{AutoActivateConfig, Skill, SkillConfig};
    use std::path::PathBuf;

    fn make_skill(
        name: &str,
        keywords: Vec<&str>,
        patterns: Vec<&str>,
        file_extensions: Vec<&str>,
        min_confidence: u8,
    ) -> Skill {
        Skill {
            config: SkillConfig {
                name: name.to_string(),
                description: format!("Test skill: {name}"),
                license: None,
                compatibility: None,
                metadata: None,
                allowed_tools: None,
                auto_activate: Some(AutoActivateConfig {
                    enabled: true,
                    keywords: keywords.into_iter().map(str::to_string).collect(),
                    patterns: patterns.into_iter().map(str::to_string).collect(),
                    file_extensions: file_extensions.into_iter().map(str::to_string).collect(),
                    min_confidence,
                }),
            },
            instructions: String::from("# Test instructions"),
            path: PathBuf::from("/tmp/fake_skill"),
        }
    }

    fn make_skill_no_auto(name: &str) -> Skill {
        Skill {
            config: SkillConfig {
                name: name.to_string(),
                description: format!("Test skill no auto: {name}"),
                license: None,
                compatibility: None,
                metadata: None,
                allowed_tools: None,
                auto_activate: None,
            },
            instructions: String::new(),
            path: PathBuf::from("/tmp/fake_skill_no_auto"),
        }
    }

    #[test]
    fn keyword_match_triggers_skill() {
        let engine = AutoActivationEngine::new();
        let skill = make_skill("rust-expert", vec!["rust", "cargo"], vec![], vec![], 20);
        let tmp = std::env::temp_dir();

        let result = engine.check("Help me with my Rust code", &tmp, &[skill], &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].skill_name, "rust-expert");
        assert!(result[0].confidence >= 20);
        assert!(result[0].reasons.iter().any(|r| r.contains("rust")));
    }

    #[test]
    fn multiple_keyword_matches_accumulate_score() {
        let engine = AutoActivationEngine::new();
        let skill = make_skill("rust-expert", vec!["rust", "cargo"], vec![], vec![], 50);
        let tmp = std::env::temp_dir();

        // Both keywords appear → score = 30 + 30 = 60 ≥ 50
        let result = engine.check("cargo build my Rust project", &tmp, &[skill], &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].confidence >= 50);
    }

    #[test]
    fn below_min_confidence_excluded() {
        let engine = AutoActivationEngine::new();
        // min_confidence = 80 but a single keyword only gives 30
        let skill = make_skill("rust-expert", vec!["rust"], vec![], vec![], 80);
        let tmp = std::env::temp_dir();

        let result = engine.check("rust code please", &tmp, &[skill], &[]);
        assert!(result.is_empty(), "should be below threshold");
    }

    #[test]
    fn regex_pattern_match_triggers_skill() {
        let engine = AutoActivationEngine::new();
        // Matches a Rust-style function signature
        let skill = make_skill("rust-expert", vec![], vec![r"fn\s+\w+"], vec![], 30);
        let tmp = std::env::temp_dir();

        let result = engine.check("fn my_function(x: i32) -> i32 {}", &tmp, &[skill], &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].confidence >= 30);
    }

    #[test]
    fn invalid_regex_does_not_panic() {
        let engine = AutoActivationEngine::new();
        let skill = make_skill("rust-expert", vec![], vec!["[invalid(regex"], vec![], 10);
        let tmp = std::env::temp_dir();

        // Should not panic; invalid pattern is skipped with a warning.
        let result = engine.check("fn my_function()", &tmp, &[skill], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn already_active_skill_excluded() {
        let engine = AutoActivationEngine::new();
        let skill = make_skill("rust-expert", vec!["rust"], vec![], vec![], 20);
        let tmp = std::env::temp_dir();

        let result = engine.check("rust code", &tmp, &[skill], &["rust-expert".to_string()]);
        assert!(result.is_empty(), "already active skill must be excluded");
    }

    #[test]
    fn skill_without_auto_activate_ignored() {
        let engine = AutoActivationEngine::new();
        let skill = make_skill_no_auto("python-expert");
        let tmp = std::env::temp_dir();

        let result = engine.check("python code", &tmp, &[skill], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn disabled_auto_activate_skill_ignored() {
        let engine = AutoActivationEngine::new();
        let mut skill = make_skill("python-expert", vec!["python"], vec![], vec![], 20);
        if let Some(ref mut cfg) = skill.config.auto_activate {
            cfg.enabled = false;
        }
        let tmp = std::env::temp_dir();

        let result = engine.check("python code", &tmp, &[skill], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn results_sorted_by_descending_confidence() {
        let engine = AutoActivationEngine::new();
        // skill_a: one keyword hit → 30
        let skill_a = make_skill("skill-a", vec!["alpha"], vec![], vec![], 10);
        // skill_b: one keyword + one pattern hit → 30 + 40 = 70
        let skill_b = make_skill("skill-b", vec!["beta"], vec![r"beta"], vec![], 10);
        let tmp = std::env::temp_dir();

        let result = engine.check("alpha beta text", &tmp, &[skill_a, skill_b], &[]);
        assert_eq!(result.len(), 2);
        assert!(
            result[0].confidence >= result[1].confidence,
            "results should be sorted highest-first"
        );
        assert_eq!(result[0].skill_name, "skill-b");
    }

    #[test]
    fn score_capped_at_100() {
        let engine = AutoActivationEngine::new();
        // Many keywords + patterns → raw score would exceed 100
        let skill = make_skill(
            "heavy-skill",
            vec!["a", "b", "c", "d"],
            vec!["a", "b", "c"],
            vec![],
            10,
        );
        let tmp = std::env::temp_dir();

        let result = engine.check("a b c d", &tmp, &[skill], &[]);
        assert!(!result.is_empty());
        assert!(result[0].confidence <= 100);
    }

    #[test]
    fn case_insensitive_keyword_matching() {
        let engine = AutoActivationEngine::new();
        let skill = make_skill("rust-expert", vec!["RUST"], vec![], vec![], 20);
        let tmp = std::env::temp_dir();

        let result = engine.check("i love rust", &tmp, &[skill], &[]);
        assert_eq!(
            result.len(),
            1,
            "keyword matching should be case-insensitive"
        );
    }

    // ── RPL-aware scoring tests ──────────────────────────────────────────

    #[test]
    fn rpl_tool_match_boosts_confidence() {
        use crate::rpl::{ReasoningPhase, ReasoningTrace, ToolEvaluation};

        let engine = AutoActivationEngine::new();
        // "files" in the input triggers the base keyword score; "list_directory"
        // is a keyword that also appears as a selected RPL tool, earning the +15
        // boost without contributing to the base score itself.
        let skill = make_skill(
            "fs-expert",
            vec!["list_directory", "files"],
            vec![],
            vec![],
            10,
        );
        let tmp = std::env::temp_dir();

        // Establish baseline without RPL.
        let base = engine.check(
            "list some files in the directory",
            &tmp,
            &[skill.clone()],
            &[],
        );
        assert_eq!(base.len(), 1, "skill should match the base input");
        let base_confidence = base[0].confidence;

        // Trace selects "list_directory" with high relevance; uncertainty is 0.0
        // so no penalty is applied.
        let mut trace = ReasoningTrace::new(ReasoningPhase::Complete).with_uncertainty(0.0);
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "list_directory".to_string(),
            relevance_score: 0.9,
            reason: None,
            selected: true,
        });

        let result = engine.check_with_reasoning(
            "list some files in the directory",
            &tmp,
            &[skill],
            &[],
            Some(&trace),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].confidence,
            (base_confidence as u32).saturating_add(15).min(100) as u8,
            "tool-name keyword match should add 15 points"
        );
        assert!(
            result[0]
                .reasons
                .iter()
                .any(|r| r.contains("RPL tool match")),
            "reasons should mention the RPL tool match"
        );
    }

    #[test]
    fn rpl_uncertainty_penalty_reduces_confidence() {
        use crate::rpl::{ReasoningPhase, ReasoningTrace};

        let engine = AutoActivationEngine::new();
        let skill = make_skill("rust-expert", vec!["rust"], vec![], vec![], 10);
        let tmp = std::env::temp_dir();

        // Establish baseline (one keyword hit → 30 points).
        let base = engine.check("rust code", &tmp, &[skill.clone()], &[]);
        assert_eq!(base.len(), 1);
        let base_confidence = base[0].confidence;

        // uncertainty = 0.8 >= 0.7 → -10 penalty.
        let trace = ReasoningTrace::new(ReasoningPhase::Complete).with_uncertainty(0.8);

        let result = engine.check_with_reasoning("rust code", &tmp, &[skill], &[], Some(&trace));

        assert_eq!(result.len(), 1, "skill should remain above min_confidence");
        assert_eq!(
            result[0].confidence,
            base_confidence.saturating_sub(10),
            "uncertainty >= 0.7 should subtract 10 from confidence"
        );
        assert!(
            result[0]
                .reasons
                .iter()
                .any(|r| r.contains("RPL uncertainty penalty applied")),
            "reasons should document the penalty"
        );
    }

    #[test]
    fn rpl_high_uncertainty_fallback_halves_confidence() {
        use crate::rpl::{ReasoningPhase, ReasoningTrace};

        let engine = AutoActivationEngine::new();
        // min_confidence is very low so the skill survives both the -10 penalty
        // and the subsequent halving in check_with_fallback.
        let skill = make_skill("rust-expert", vec!["rust"], vec![], vec![], 5);
        let tmp = std::env::temp_dir();

        let trace = ReasoningTrace::new(ReasoningPhase::Complete).with_uncertainty(0.95);

        // check_with_reasoning applies the -10 penalty (0.95 >= 0.7).
        let reasoning_result =
            engine.check_with_reasoning("rust code", &tmp, &[skill.clone()], &[], Some(&trace));
        assert_eq!(reasoning_result.len(), 1);
        let reasoning_confidence = reasoning_result[0].confidence;

        // check_with_fallback should additionally halve the confidence.
        let fallback_result =
            engine.check_with_fallback("rust code", &tmp, &[skill], &[], Some(&trace));

        assert_eq!(fallback_result.len(), 1);
        assert_eq!(
            fallback_result[0].confidence,
            reasoning_confidence / 2,
            "high-uncertainty fallback should halve the confidence"
        );
        assert!(
            fallback_result[0]
                .reasons
                .iter()
                .any(|r| r.contains("RPL high-uncertainty fallback")),
            "reasons should document the fallback"
        );
    }

    #[test]
    fn check_with_reasoning_none_matches_check() {
        let engine = AutoActivationEngine::new();
        let skill = make_skill("rust-expert", vec!["rust", "cargo"], vec![], vec![], 20);
        let tmp = std::env::temp_dir();

        let base = engine.check("cargo build my Rust project", &tmp, &[skill.clone()], &[]);
        let with_none =
            engine.check_with_reasoning("cargo build my Rust project", &tmp, &[skill], &[], None);

        assert_eq!(
            base.len(),
            with_none.len(),
            "passing None for reasoning should give the same number of results"
        );
        for (b, r) in base.iter().zip(with_none.iter()) {
            assert_eq!(b.skill_name, r.skill_name, "skill names should match");
            assert_eq!(
                b.confidence, r.confidence,
                "confidence should be identical with no reasoning trace"
            );
        }
    }
}
