//! Bayesian inference engine for intent routing.
//!
//! The engine maintains a probability distribution over possible user intents
//! and meta-states (confidence, vagueness, clarification need).  It is updated
//! on every user message and after each tool call, and its internal state is
//! persisted to `~/.grok/bayes_profile.json` so the router learns from usage
//! over time.
//!
//! ## Configuration
//!
//! All tunable values can be set via the `[bayesian]` section of
//! `config.toml`.  Call [`BayesianEngine::new_with_config`] to apply them;
//! [`BayesianEngine::new`] uses the compiled-in defaults and is kept for
//! backward compatibility and tests.

use std::collections::HashMap;

use crate::bayes::belief_graph::BeliefGraph;
use crate::bayes::likelihoods::{
    DEFAULT_INTENT_LIKELIHOOD_WEIGHT, likelihood_from_model_confidence, likelihood_from_text,
    likelihood_from_tool_failure,
};
use crate::bayes::priors::{default_priors, priors_from_config};
use crate::bayes::updater::bayes_update;

/// Threshold defaults (mirrored from `config::BayesianConfig` defaults).
const DEFAULT_CLARIFICATION_THRESHOLD: f32 = 0.4;
const DEFAULT_UNCERTAINTY_THRESHOLD: f32 = 0.6;
const DEFAULT_VAGUENESS_THRESHOLD: f32 = 0.6;

const DEFAULT_PROFILE_LEARNING_RATE: f32 = 0.1;

/// The core Bayesian inference engine.
#[derive(Debug, Clone)]
pub struct BayesianEngine {
    priors: HashMap<String, f32>,
    graph: BeliefGraph,

    // ── Configurable thresholds ───────────────────────────────────────────────
    /// `P(need_clarification)` above which the router fires the clarification gate.
    clarification_threshold: f32,

    /// `P(need_clarification | low_confidence)` above which system uncertainty
    /// notes are injected into the prompt.
    uncertainty_threshold: f32,

    /// `P(is_vague)` above which a "request is vague" system note is injected.
    vagueness_threshold: f32,

    /// Strength of keyword → intent likelihood spikes in `likelihood_from_text`.
    /// Higher = more decisive routing on a keyword match.
    intent_likelihood_weight: f32,

    /// Fractional boost applied to a prior when the corresponding tool is used
    /// successfully.  `0.1` = 10 % boost per call.
    profile_learning_rate: f32,
}

impl BayesianEngine {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a new engine using compiled-in defaults.
    ///
    /// The prior distribution is loaded from the saved profile on disk
    /// (`~/.grok/bayes_profile.json`) and falls back to the built-in default
    /// priors when no profile exists yet.
    pub fn new() -> Self {
        let priors = crate::bayes::profile::load_profile().unwrap_or_else(default_priors);
        Self::from_priors(
            priors,
            DEFAULT_CLARIFICATION_THRESHOLD,
            DEFAULT_UNCERTAINTY_THRESHOLD,
            DEFAULT_VAGUENESS_THRESHOLD,
            DEFAULT_INTENT_LIKELIHOOD_WEIGHT,
            DEFAULT_PROFILE_LEARNING_RATE,
        )
    }

<<<<<<< HEAD
    /// Create a new engine using the compiled-in default priors.
    ///
    /// Unlike [`new`], this constructor never reads from the on-disk saved profile,
    /// making it suitable for unit tests that require deterministic baseline behaviour.
=======
    /// Create a new engine using only the compiled-in default priors,
    /// **without** loading the on-disk profile.
    ///
    /// This constructor is intended for unit tests that need a deterministic,
    /// isolated starting distribution that cannot be polluted by a saved
    /// `~/.grok/bayes_profile.json` from real usage on the developer's machine.
    #[cfg(test)]
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a
    pub fn new_with_default_priors() -> Self {
        Self::from_priors(
            default_priors(),
            DEFAULT_CLARIFICATION_THRESHOLD,
            DEFAULT_UNCERTAINTY_THRESHOLD,
            DEFAULT_VAGUENESS_THRESHOLD,
            DEFAULT_INTENT_LIKELIHOOD_WEIGHT,
            DEFAULT_PROFILE_LEARNING_RATE,
        )
    }

<<<<<<< HEAD
=======
    /// Create an engine from a [`BayesianConfig`] but using only the config's
    /// **compiled-in priors**, without loading the on-disk profile.
    ///
    /// This lets tests verify threshold / weight behaviour under controlled,
    /// deterministic conditions — the on-disk `~/.grok/bayes_profile.json`
    /// from real usage cannot dilute or skew the starting distribution.
    #[cfg(test)]
    pub fn new_from_config_no_profile(config: &crate::config::BayesianConfig) -> Self {
        use crate::bayes::priors::priors_from_config;
        Self::from_priors(
            priors_from_config(&config.priors),
            config.clarification_threshold,
            config.uncertainty_threshold,
            config.vagueness_threshold,
            config.intent_likelihood_weight,
            config.profile_learning_rate,
        )
    }

>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a
    /// Create a new engine using values from `[bayesian]` config.
    ///
    /// The prior distribution is loaded from the saved profile on disk first;
    /// the config priors are only used as the fallback when no saved profile
    /// exists yet.  This preserves learned behaviour across config changes.
    pub fn new_with_config(config: &crate::config::BayesianConfig) -> Self {
        let priors = crate::bayes::profile::load_profile()
            .unwrap_or_else(|| priors_from_config(&config.priors));
        Self::from_priors(
            priors,
            config.clarification_threshold,
            config.uncertainty_threshold,
            config.vagueness_threshold,
            config.intent_likelihood_weight,
            config.profile_learning_rate,
        )
    }

    fn from_priors(
        priors: HashMap<String, f32>,
        clarification_threshold: f32,
        uncertainty_threshold: f32,
        vagueness_threshold: f32,
        intent_likelihood_weight: f32,
        profile_learning_rate: f32,
    ) -> Self {
        let mut graph = BeliefGraph::new();
        for (k, v) in &priors {
            graph.set(k, *v);
        }
        // Normalise at construction so that `probability()` always returns
        // a proper probability (sums to 1.0) even before the first update.
        // The raw `priors` HashMap is intentionally left un-normalised here;
        // `bayes_update` + `sync_graph` will normalise both on every update.
        graph.normalize();
        Self {
            priors,
            graph,
            clarification_threshold,
            uncertainty_threshold,
            vagueness_threshold,
            intent_likelihood_weight,
            profile_learning_rate,
        }
    }

    // ── Threshold accessors ───────────────────────────────────────────────────

    /// The clarification gate fires when `P(need_clarification)` exceeds this.
    pub fn clarification_threshold(&self) -> f32 {
        self.clarification_threshold
    }

    /// Uncertainty system notes are injected when either
    /// `P(need_clarification)` or `P(low_confidence)` exceeds this.
    pub fn uncertainty_threshold(&self) -> f32 {
        self.uncertainty_threshold
    }

    /// Vagueness notes are injected when `P(is_vague)` exceeds this.
    pub fn vagueness_threshold(&self) -> f32 {
        self.vagueness_threshold
    }

    // ── Convenience threshold checks ──────────────────────────────────────────

    /// Returns `true` when the clarification gate should fire.
    pub fn needs_clarification(&self) -> bool {
        self.probability("need_clarification") > self.clarification_threshold
    }

    /// Returns `true` when uncertainty is high enough to inject a system note.
    pub fn is_high_uncertainty(&self) -> bool {
        self.probability("need_clarification") > self.uncertainty_threshold
            || self.probability("low_confidence") > self.uncertainty_threshold
    }

    /// Returns `true` when the input is probably vague.
    pub fn is_vague(&self) -> bool {
        self.probability("is_vague") > self.vagueness_threshold
    }

    /// Returns `true` when `P(low_confidence)` exceeds the uncertainty
    /// threshold (used by the planner / router for the self-correction loop).
    pub fn is_low_confidence(&self) -> bool {
        self.probability("low_confidence") > self.uncertainty_threshold
    }

    // ── Update methods ────────────────────────────────────────────────────────

    /// Update beliefs from a text event (user message) using the configured
    /// likelihood weight.
    pub fn update_from_text(&mut self, text: &str) {
        let likelihoods = likelihood_from_text(text, self.intent_likelihood_weight);
        bayes_update(&mut self.priors, &likelihoods);
        self.sync_graph();
    }

    /// Update beliefs from a model-confidence score in `[0.0, 1.0]`.
    pub fn update_from_model_confidence(&mut self, score: f32) {
        let likelihoods = likelihood_from_model_confidence(score);
        bayes_update(&mut self.priors, &likelihoods);
        self.sync_graph();
    }

    /// Update beliefs after a tool call failure.
    pub fn update_from_tool_failure(&mut self) {
        let likelihoods = likelihood_from_tool_failure();
        bayes_update(&mut self.priors, &likelihoods);
        self.sync_graph();
    }

    /// Boost the prior for the intent that corresponds to a successfully used
    /// tool, then re-normalise and persist the profile to disk.
    ///
    /// The boost magnitude is `self.profile_learning_rate` (e.g. 10 %).
    pub fn update_profile(&mut self, executed_intent: &str) {
        let intent_key = match executed_intent {
            "replace" | "write_file" => "intent_edit",
            "run_shell_command" => "intent_shell",
            "web_search" | "web_fetch" => "intent_search",
            _ => "intent_question",
        };

        if let Some(prior) = self.priors.get_mut(intent_key) {
            *prior *= 1.0 + self.profile_learning_rate;
        }

        // Re-normalise so probabilities sum to 1.
        let total: f32 = self.priors.values().sum();
        if total > f32::EPSILON {
            for value in self.priors.values_mut() {
                *value /= total;
            }
        }

        self.sync_graph();
        let _ = crate::bayes::profile::save_profile(&self.priors);
    }

    // ── Query methods ─────────────────────────────────────────────────────────

    /// Return the current probability for `key`.  Returns `0.0` for unknown keys.
    pub fn probability(&self, key: &str) -> f32 {
        self.graph.get(key)
    }

    /// Return the intent key (`"intent_*"`) with the highest probability, or
    /// `None` when the graph is empty.
    pub fn best_intent(&self) -> Option<String> {
        self.graph.best_key("intent_")
    }

    /// Return an ASCII bar-chart visualisation of the current belief state.
    pub fn visualize(&self) -> String {
        self.graph.visualize()
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn sync_graph(&mut self) {
        for (k, v) in &self.priors {
            self.graph.set(k, *v);
        }
        self.graph.normalize();
    }
}

impl Default for BayesianEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BayesianConfig, BayesianPriorsConfig};

    #[test]
    fn test_engine_initialization() {
<<<<<<< HEAD
=======
        // Use the test constructor to avoid loading ~/.grok/bayes_profile.json,
        // which may have intent_edit dominant from real usage on this machine.
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a
        let engine = BayesianEngine::new_with_default_priors();
        assert!(engine.probability("intent_question") > 0.0);
        assert_eq!(engine.best_intent(), Some("intent_question".to_string()));
    }

    #[test]
    fn test_engine_update_from_text() {
<<<<<<< HEAD
=======
        // Use the test constructor so the starting distribution is always the
        // compiled-in defaults, not whatever the saved profile holds.
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a
        let mut engine = BayesianEngine::new_with_default_priors();
        assert_eq!(engine.best_intent(), Some("intent_question".to_string()));
        engine.update_from_text("can you edit the config file");
        assert_eq!(engine.best_intent(), Some("intent_edit".to_string()));
    }

    #[test]
    fn test_default_thresholds() {
        let engine = BayesianEngine::new();
        assert!(
            (engine.clarification_threshold() - DEFAULT_CLARIFICATION_THRESHOLD).abs()
                < f32::EPSILON
        );
        assert!(
            (engine.uncertainty_threshold() - DEFAULT_UNCERTAINTY_THRESHOLD).abs() < f32::EPSILON
        );
        assert!((engine.vagueness_threshold() - DEFAULT_VAGUENESS_THRESHOLD).abs() < f32::EPSILON);
    }

    #[test]
    fn test_custom_thresholds_from_config() {
        let config = BayesianConfig {
            enabled: true,
            clarification_threshold: 0.2,
            uncertainty_threshold: 0.8,
            vagueness_threshold: 0.75,
            intent_likelihood_weight: 8.0,
            profile_learning_rate: 0.05,
            priors: BayesianPriorsConfig::default(),
            show_belief_graph: false,
        };
        let engine = BayesianEngine::new_with_config(&config);
        assert!((engine.clarification_threshold() - 0.2).abs() < f32::EPSILON);
        assert!((engine.uncertainty_threshold() - 0.8).abs() < f32::EPSILON);
        assert!((engine.vagueness_threshold() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_needs_clarification_gate() {
        // With a very low threshold the clarification gate should fire easily.
<<<<<<< HEAD
        // Use from_priors() directly so the test doesn't load the on-disk profile.
        let mut engine = BayesianEngine::from_priors(
            default_priors(),
            0.01, // very low threshold — fires easily
            DEFAULT_UNCERTAINTY_THRESHOLD,
            DEFAULT_VAGUENESS_THRESHOLD,
            DEFAULT_INTENT_LIKELIHOOD_WEIGHT,
            DEFAULT_PROFILE_LEARNING_RATE,
        );
=======
        let config = BayesianConfig {
            clarification_threshold: 0.01, // fires with almost any need_clarification signal
            ..BayesianConfig::default()
        };
        // Use the isolated constructor so the on-disk profile cannot dilute
        // need_clarification below the 0.01 threshold being tested here.
        // With default priors (need_clarification = 0.1) and a 10× likelihood
        // spike from "don't delete", the posterior is ~0.33 — well above 0.01.
        let mut engine = BayesianEngine::new_from_config_no_profile(&config);
>>>>>>> db2d87496180036f3bda9bedaa4199b5dcfcd07a
        engine.update_from_text("be careful, don't delete");
        assert!(engine.needs_clarification());
    }

    #[test]
    fn test_high_threshold_suppresses_clarification() {
        // With a very high threshold the clarification gate should NOT fire.
        let config = BayesianConfig {
            clarification_threshold: 0.99,
            ..BayesianConfig::default()
        };
        let mut engine = BayesianEngine::new_with_config(&config);
        engine.update_from_text("be careful, don't delete");
        assert!(!engine.needs_clarification());
    }

    #[test]
    fn test_is_low_confidence_uses_uncertainty_threshold() {
        let engine = BayesianEngine::new();
        // Fresh engine starts below the uncertainty threshold.
        assert!(!engine.is_low_confidence());
    }

    #[test]
    fn test_profile_learning_rate_applied() {
        // Use from_priors directly to avoid loading the on-disk saved profile.
        let mut engine = BayesianEngine::from_priors(
            default_priors(),
            DEFAULT_CLARIFICATION_THRESHOLD,
            DEFAULT_UNCERTAINTY_THRESHOLD,
            DEFAULT_VAGUENESS_THRESHOLD,
            DEFAULT_INTENT_LIKELIHOOD_WEIGHT,
            0.5, // 50 % boost — noticeable in test
        );
        let before = engine.probability("intent_edit");
        engine.update_profile("write_file");
        let after = engine.probability("intent_edit");
        // After the boost + renormalisation the intent_edit probability
        // should be strictly higher than before.
        assert!(after > before, "expected {} > {}", after, before);
    }

    #[test]
    fn test_visualize_returns_nonempty() {
        let engine = BayesianEngine::new();
        assert!(!engine.visualize().is_empty());
    }
}
