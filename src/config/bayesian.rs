//! Bayesian intent router configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

/// Top-level configuration for the Bayesian intent router.
///
/// All thresholds are probabilities in the range `[0.0, 1.0]`.
/// Lowering a threshold makes the corresponding behaviour fire more readily;
/// raising it makes it more conservative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayesianConfig {
    /// Master on/off switch.  When `false` the router is bypassed entirely.
    #[serde(default)]
    pub enabled: bool,

    /// Show the real-time ASCII belief-graph after each message.
    #[serde(default)]
    pub show_belief_graph: bool,

    /// The router asks for clarification when `P(need_clarification)` exceeds
    /// this value.  Default: `0.4`.  Lower → more cautious; higher → more
    /// permissive.
    #[serde(default = "default_clarification_threshold")]
    pub clarification_threshold: f32,

    /// System uncertainty notes are injected into the prompt when
    /// `P(need_clarification)` or `P(low_confidence)` exceeds this value.
    /// Default: `0.6`.
    #[serde(default = "default_uncertainty_threshold")]
    pub uncertainty_threshold: f32,

    /// A "request is vague" note is injected when `P(is_vague)` exceeds this
    /// value.  Default: `0.6`.
    #[serde(default = "default_vagueness_threshold")]
    pub vagueness_threshold: f32,

    /// Strength of keyword → intent likelihood spikes.  Higher values make the
    /// router commit to an intent more decisively on a keyword match.
    /// Default: `5.0`.
    #[serde(default = "default_intent_likelihood_weight")]
    pub intent_likelihood_weight: f32,

    /// Fractional boost applied to a prior each time the corresponding tool
    /// is used successfully.  `0.1` = 10 % boost per call.  Default: `0.1`.
    #[serde(default = "default_profile_learning_rate")]
    pub profile_learning_rate: f32,

    /// Decay factor applied to current beliefs during the stabilization step.
    /// Higher values (closer to 1.0) = slower decay toward priors.
    /// Default: `0.95` (5% pull toward prior each update).
    #[serde(default = "default_belief_decay_rate")]
    pub belief_decay_rate: f32,

    /// Strength of the pull toward the long-term prior during decay.
    /// `0.05` means beliefs are gently regressed 5% toward their base priors.
    /// Default: `0.05`.
    #[serde(default = "default_prior_pull_rate")]
    pub prior_pull_rate: f32,

    /// Starting prior weights used when no saved profile exists on disk.
    #[serde(default)]
    pub priors: BayesianPriorsConfig,
}

fn default_clarification_threshold() -> f32 {
    0.4
}
fn default_uncertainty_threshold() -> f32 {
    0.6
}
fn default_vagueness_threshold() -> f32 {
    0.6
}
fn default_intent_likelihood_weight() -> f32 {
    5.0
}
fn default_profile_learning_rate() -> f32 {
    0.1
}
fn default_belief_decay_rate() -> f32 {
    0.95
}
fn default_prior_pull_rate() -> f32 {
    0.05
}

impl Default for BayesianConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_belief_graph: false,
            clarification_threshold: default_clarification_threshold(),
            uncertainty_threshold: default_uncertainty_threshold(),
            vagueness_threshold: default_vagueness_threshold(),
            intent_likelihood_weight: default_intent_likelihood_weight(),
            profile_learning_rate: default_profile_learning_rate(),
            belief_decay_rate: default_belief_decay_rate(),
            prior_pull_rate: default_prior_pull_rate(),
            priors: BayesianPriorsConfig::default(),
        }
    }
}

/// Default prior weights (starting beliefs before any input is seen).
///
/// These are used only when no saved profile exists at
/// `~/.grok-cli/bayes_profile.json`.  Once the engine learns from the user's
/// tool usage, the learned values take over.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayesianPriorsConfig {
    /// Prior probability of an edit/write intent.  Default: `0.2`.
    #[serde(default = "prior_intent_edit")]
    pub intent_edit: f32,
    /// Prior probability of a shell/run intent.  Default: `0.2`.
    #[serde(default = "prior_intent_shell")]
    pub intent_shell: f32,
    /// Prior probability of a search/web intent.  Default: `0.2`.
    #[serde(default = "prior_intent_search")]
    pub intent_search: f32,
    /// Prior probability of a question/chat intent.  Default: `0.3`.
    #[serde(default = "prior_intent_question")]
    pub intent_question: f32,
    /// Prior probability that the input needs clarification.  Default: `0.1`.
    #[serde(default = "prior_need_clarification")]
    pub need_clarification: f32,
    /// Prior probability of low model confidence.  Default: `0.2`.
    #[serde(default = "prior_low_confidence")]
    pub low_confidence: f32,
    /// Prior probability that the input is vague.  Default: `0.1`.
    #[serde(default = "prior_is_vague")]
    pub is_vague: f32,
}

fn prior_intent_edit() -> f32 {
    0.2
}
fn prior_intent_shell() -> f32 {
    0.2
}
fn prior_intent_search() -> f32 {
    0.2
}
fn prior_intent_question() -> f32 {
    0.3
}
fn prior_need_clarification() -> f32 {
    0.1
}
fn prior_low_confidence() -> f32 {
    0.2
}
fn prior_is_vague() -> f32 {
    0.1
}

impl Default for BayesianPriorsConfig {
    fn default() -> Self {
        Self {
            intent_edit: prior_intent_edit(),
            intent_shell: prior_intent_shell(),
            intent_search: prior_intent_search(),
            intent_question: prior_intent_question(),
            need_clarification: prior_need_clarification(),
            low_confidence: prior_low_confidence(),
            is_vague: prior_is_vague(),
        }
    }
}
