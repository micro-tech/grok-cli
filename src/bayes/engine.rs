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

use std::cell::RefCell;
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
const DEFAULT_BELIEF_DECAY_RATE: f32 = 0.95;
const DEFAULT_PRIOR_PULL_RATE: f32 = 0.05;

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

    /// Decay factor for belief stabilization (0.0 – 1.0).
    belief_decay_rate: f32,

    /// Pull strength toward long-term priors during decay.
    prior_pull_rate: f32,

    // ── Task 268.3: Intent caching / short-circuit ────────────────────────────
    /// Last user text we fully processed (trimmed). Used for identical-input fast path.
    last_text: Option<String>,
    /// Cached result of the most recent best_intent computation (interior mut for &self query).
    /// Invalidated on any update that mutates the distribution.
    cached_best_intent: RefCell<Option<String>>,
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
            DEFAULT_BELIEF_DECAY_RATE,
            DEFAULT_PRIOR_PULL_RATE,
        )
    }

    pub fn new_with_default_priors() -> Self {
        Self::from_priors(
            default_priors(),
            DEFAULT_CLARIFICATION_THRESHOLD,
            DEFAULT_UNCERTAINTY_THRESHOLD,
            DEFAULT_VAGUENESS_THRESHOLD,
            DEFAULT_INTENT_LIKELIHOOD_WEIGHT,
            DEFAULT_PROFILE_LEARNING_RATE,
            DEFAULT_BELIEF_DECAY_RATE,
            DEFAULT_PRIOR_PULL_RATE,
        )
    }

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
            config.belief_decay_rate,
            config.prior_pull_rate,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "all parameters are distinct threshold/rate values; a builder struct would add complexity without clarity"
    )]
    fn from_priors(
        priors: HashMap<String, f32>,
        clarification_threshold: f32,
        uncertainty_threshold: f32,
        vagueness_threshold: f32,
        intent_likelihood_weight: f32,
        profile_learning_rate: f32,
        belief_decay_rate: f32,
        prior_pull_rate: f32,
    ) -> Self {
        let mut graph = BeliefGraph::new();
        for (k, v) in &priors {
            graph.set(k, *v);
        }
        graph.normalize();
        Self {
            priors,
            graph,
            clarification_threshold,
            uncertainty_threshold,
            vagueness_threshold,
            intent_likelihood_weight,
            profile_learning_rate,
            belief_decay_rate,
            prior_pull_rate,
            last_text: None,
            cached_best_intent: RefCell::new(None),
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
        // Task 268.3: intent caching / short-circuit for identical consecutive inputs
        let trimmed = text.trim().to_string();
        if let Some(ref last) = self.last_text {
            if *last == trimmed {
                // Fast path: identical input, distribution unchanged.
                // Still allow perf reporting of the cheap path.
                if crate::utils::perf::perf_enabled() {
                    // Emit a zero-cost marker for visibility under GROK_PERF
                    eprintln!("[perf] bayes.update_from_text (cached identical)");
                }
                return;
            }
        }

        // Task 268: instrument per-turn Bayesian cost
        let _t = if crate::utils::perf::perf_enabled() {
            Some(crate::utils::perf::start_turn())
        } else {
            None
        };

        let likelihoods = likelihood_from_text(text, self.intent_likelihood_weight);
        bayes_update(
            &mut self.priors,
            &likelihoods,
            self.belief_decay_rate,
            self.prior_pull_rate,
        );
        self.sync_graph();

        // Update cache state for next short-circuit / best_intent
        self.last_text = Some(trimmed);
        self.cached_best_intent.replace(None);

        if let Some(start) = _t {
            crate::utils::perf::report_turn("bayes.update_from_text", start);
        }
    }

    /// Update beliefs from a model-confidence score in `[0.0, 1.0]`.
    pub fn update_from_model_confidence(&mut self, score: f32) {
        // Task 268.3: invalidate caches on any distribution mutation
        self.invalidate_caches();

        let likelihoods = likelihood_from_model_confidence(score);
        bayes_update(
            &mut self.priors,
            &likelihoods,
            self.belief_decay_rate,
            self.prior_pull_rate,
        );
        self.sync_graph();
    }

    /// Update beliefs after a tool call failure.
    pub fn update_from_tool_failure(&mut self) {
        // Task 268.3: invalidate caches on mutation
        self.invalidate_caches();
        let likelihoods = likelihood_from_tool_failure();
        bayes_update(
            &mut self.priors,
            &likelihoods,
            self.belief_decay_rate,
            self.prior_pull_rate,
        );
        self.sync_graph();
    }

    /// Apply a multiplicative boost to a specific intent prior (used by Session DNA).
    /// The caller is responsible for re-normalising afterwards.
    pub fn boost_prior(&mut self, intent: &str, factor: f32) {
        // Task 268.3: invalidate on mutation
        self.invalidate_caches();
        if let Some(p) = self.priors.get_mut(intent) {
            *p *= factor;
        }
    }

    /// Boost the prior for the intent that corresponds to a successfully used
    /// tool, then re-normalise and persist the profile to disk.
    ///
    /// The boost magnitude is `self.profile_learning_rate` (e.g. 10 %).
    pub fn update_profile(&mut self, executed_intent: &str) {
        // Task 268.3: invalidate caches on any distribution mutation
        self.invalidate_caches();

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
    ///
    /// Task 268.3: Returns a cached value when the distribution has not changed
    /// since the last `update_from_text` (or when the identical-input fast path fired).
    pub fn best_intent(&self) -> Option<String> {
        // Task 268.3 fast path: serve from cache if present
        if let Some(cached) = self.cached_best_intent.borrow().as_ref() {
            return Some(cached.clone());
        }

        // Task 268.2: use the zero-cost guard (only emits under GROK_PERF=1)
        crate::perf_guard!("bayes.best_intent (compute)");

        // Compute and cache
        let computed = self.graph.best_key("intent_");
        if let Some(ref key) = computed {
            self.cached_best_intent.replace(Some(key.clone()));
        }
        computed
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

    // ── Task 268.3 helpers ────────────────────────────────────────────────────

    /// Invalidate the identical-input short-circuit and best_intent cache.
    /// Call this before/after any operation that mutates the prior distribution.
    fn invalidate_caches(&mut self) {
        self.last_text = None;
        self.cached_best_intent.replace(None);
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
        let engine = BayesianEngine::new_with_default_priors();
        assert!(engine.probability("intent_question") > 0.0);
        assert_eq!(engine.best_intent(), Some("intent_question".to_string()));
    }

    #[test]
    fn test_engine_update_from_text() {
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
            belief_decay_rate: DEFAULT_BELIEF_DECAY_RATE,
            prior_pull_rate: DEFAULT_PRIOR_PULL_RATE,
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
        // Use from_priors() directly so the test doesn't load the on-disk profile.
        let mut engine = BayesianEngine::from_priors(
            default_priors(),
            0.01,
            DEFAULT_UNCERTAINTY_THRESHOLD,
            DEFAULT_VAGUENESS_THRESHOLD,
            DEFAULT_INTENT_LIKELIHOOD_WEIGHT,
            DEFAULT_PROFILE_LEARNING_RATE,
            DEFAULT_BELIEF_DECAY_RATE,
            DEFAULT_PRIOR_PULL_RATE,
        );
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
            0.5, // 50 % boost
            DEFAULT_BELIEF_DECAY_RATE,
            DEFAULT_PRIOR_PULL_RATE,
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

    #[test]
    fn test_268_3_intent_caching_and_identical_short_circuit() {
        let mut engine = BayesianEngine::new_with_default_priors();

        // First update should compute
        engine.update_from_text("can you edit the config file");
        let first = engine.best_intent();
        assert_eq!(first, Some("intent_edit".to_string()));

        // Identical input: should short-circuit (no change to distribution)
        engine.update_from_text("can you edit the config file");
        let second = engine.best_intent();
        assert_eq!(first, second);

        // Different input invalidates and recomputes
        engine.update_from_text("please search the web for rust");
        let third = engine.best_intent();
        assert_eq!(third, Some("intent_search".to_string()));

        // After a mutating call (update_profile), cache is invalidated
        engine.update_profile("write_file");
        // best_intent should still be consistent (may stay search or change depending on learning)
        let after_profile = engine.best_intent();
        assert!(after_profile.is_some());
    }
}
