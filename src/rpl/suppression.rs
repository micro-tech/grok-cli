//! Reasoning Protocol Layer – suppression and privacy controls.
//!
//! Provides [`RedactionRule`], [`RedactionConfig`], and [`SuppressionLayer`]
//! for guarding reasoning traces from leaking to user-facing output and for
//! redacting sensitive content before any trace is exposed to callers.
//!
//! # Design
//!
//! Every [`ReasoningTrace`] is constructed with `suppressed = true` by
//! default, so internal reasoning is never accidentally surfaced in
//! user-facing output.  The [`SuppressionLayer`] enforces this contract at
//! the point of consumption:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │                  SuppressionLayer                    │
//! │                                                      │
//! │  guard(trace)  ──►  None  (production, suppressed)   │
//! │                ──►  Some  (debug_mode override)       │
//! │                ──►  Some  (trace.suppressed == false) │
//! │                                                      │
//! │  redact(trace) ──►  clone with sensitive fields      │
//! │                     replaced by "[REDACTED]"         │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick start
//!
//! ```rust,ignore
//! use grok_cli::rpl::{SuppressionLayer, ReasoningTrace, ReasoningPhase};
//!
//! let layer = SuppressionLayer::production();
//! let trace = ReasoningTrace::new(ReasoningPhase::Complete);
//!
//! // Returns None – trace is suppressed and we are in production mode.
//! let exposed = layer.guard(&trace);
//! assert!(exposed.is_none());
//!
//! // Apply redaction before exposing a trace in debug mode.
//! let safe = layer.redact(&trace);
//! ```

use regex::Regex;

// ---------------------------------------------------------------------------
// RedactionRule
// ---------------------------------------------------------------------------

/// A single redaction rule that matches patterns in string fields.
///
/// Rules are composable: build a pipeline with [`RedactionConfig`] and apply
/// them all in a single pass via [`RedactionConfig::apply_all`].
#[derive(Debug, Clone)]
pub struct RedactionRule {
    /// Human-readable name used in log messages when this rule fires.
    pub name: String,

    /// Compiled regex pattern that identifies sensitive content.
    pattern: Regex,

    /// Replacement text inserted wherever the pattern matches
    /// (e.g. `"[REDACTED]"`).
    replacement: String,
}

impl RedactionRule {
    /// Create a new rule.
    ///
    /// Returns `Err` if `pattern` is not a valid regular expression.
    ///
    /// # Arguments
    ///
    /// * `name`        – Human-readable label for log output.
    /// * `pattern`     – Regular expression to match sensitive content.
    /// * `replacement` – String that replaces every match (e.g. `"[REDACTED]"`).
    ///
    /// # Errors
    ///
    /// Returns [`regex::Error`] when `pattern` cannot be compiled.
    pub fn new(
        name: impl Into<String>,
        pattern: &str,
        replacement: impl Into<String>,
    ) -> Result<Self, regex::Error> {
        let compiled = Regex::new(pattern)?;
        Ok(Self {
            name: name.into(),
            pattern: compiled,
            replacement: replacement.into(),
        })
    }

    /// Apply this rule to `input`, returning the redacted version.
    ///
    /// All non-overlapping occurrences of the pattern are replaced.  If the
    /// pattern does not match, the original string is returned unchanged.
    pub fn apply(&self, input: &str) -> String {
        self.pattern
            .replace_all(input, self.replacement.as_str())
            .into_owned()
    }
}

// ---------------------------------------------------------------------------
// RedactionConfig
// ---------------------------------------------------------------------------

/// Collection of redaction rules applied to reasoning traces before logging.
///
/// Rules are executed in insertion order; each rule receives the output of
/// the previous one, enabling chained redaction (e.g. a first rule masks
/// an API key, a second rule masks the bearer token scheme).
///
/// ## Default production rule set
///
/// Use [`RedactionConfig::default_rules`] to obtain a pre-built configuration
/// that covers the most common sensitive patterns:
///
/// | Rule name  | Pattern matched                                         |
/// |------------|---------------------------------------------------------|
/// | `api_key`  | `api_key`, `bearer`, or `token` followed by `=`/`:`    |
/// | `secret`   | `secret` followed by `=`/`:`                           |
/// | `password` | `password` followed by `=`/`:`                         |
#[derive(Debug, Clone, Default)]
pub struct RedactionConfig {
    rules: Vec<RedactionRule>,
}

impl RedactionConfig {
    /// Create an empty [`RedactionConfig`] with no rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append `rule` to the end of the pipeline.
    ///
    /// Rules are applied in insertion order by [`apply_all`][Self::apply_all].
    pub fn add_rule(&mut self, rule: RedactionRule) {
        self.rules.push(rule);
    }

    /// Apply all rules sequentially to `input` and return the result.
    ///
    /// Each rule receives the output of the previous rule.  If no rules have
    /// been added the input is returned unchanged.
    pub fn apply_all(&self, input: &str) -> String {
        self.rules
            .iter()
            .fold(input.to_owned(), |acc, rule| rule.apply(&acc))
    }

    /// Build the default production rule set.
    ///
    /// The following three rules are included, in order:
    ///
    /// 1. **`api_key`** – matches `api_key`, `bearer`, or `token` followed by
    ///    optional whitespace, a `:` or `=` separator, optional whitespace,
    ///    and a non-whitespace value.
    ///    Pattern: `(?i)(api[-_]?key|bearer|token)\s*[:=]\s*\S+`
    ///
    /// 2. **`secret`** – matches `secret` followed by a `:` or `=` separator
    ///    and a non-whitespace value.
    ///    Pattern: `(?i)secret\s*[:=]\s*\S+`
    ///
    /// 3. **`password`** – matches `password` followed by a `:` or `=`
    ///    separator and a non-whitespace value.
    ///    Pattern: `(?i)password\s*[:=]\s*\S+`
    ///
    /// All matches are replaced with `[REDACTED]`.
    ///
    /// # Panics
    ///
    /// This function panics only if the hard-coded patterns fail to compile,
    /// which indicates a programming error and is not expected at runtime.
    pub fn default_rules() -> Self {
        let mut config = Self::new();

        let api_key = RedactionRule::new(
            "api_key",
            r"(?i)(api[-_]?key|bearer|token)\s*[:=]\s*\S+",
            "[REDACTED]",
        )
        .expect("api_key redaction pattern must compile");

        let secret = RedactionRule::new("secret", r"(?i)secret\s*[:=]\s*\S+", "[REDACTED]")
            .expect("secret redaction pattern must compile");

        let password = RedactionRule::new("password", r"(?i)password\s*[:=]\s*\S+", "[REDACTED]")
            .expect("password redaction pattern must compile");

        config.add_rule(api_key);
        config.add_rule(secret);
        config.add_rule(password);
        config
    }
}

// ---------------------------------------------------------------------------
// SuppressionLayer
// ---------------------------------------------------------------------------

/// Guards reasoning traces from leaking to user-facing output.
///
/// A trace is *suppressed* when [`ReasoningTrace::suppressed`] is `true`.
/// In normal operation all traces start suppressed (the default set by
/// [`ReasoningTrace::new`]).  Only an explicit `debug_mode = true` in this
/// layer allows suppressed traces through.
///
/// # Usage pattern
///
/// ```rust,ignore
/// use grok_cli::rpl::{SuppressionLayer, RplLayer, RplConfig, ReasoningLogLevel};
///
/// let rpl = RplLayer::new(RplConfig::default());
/// let sup = SuppressionLayer::production();
///
/// let mut trace = rpl.on_pre_evaluate(Some("do something"), None);
/// rpl.on_complete(&mut trace);
///
/// // Suppressed in production — None is returned.
/// if let Some(t) = sup.guard(&trace) {
///     println!("trace visible: {}", t.trace_id);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SuppressionLayer {
    /// When `true`, suppressed traces are returned by [`guard`][Self::guard]
    /// anyway (for debug / development use).
    pub debug_mode: bool,

    /// Redaction rules applied to string fields before any trace is exposed.
    pub redaction: RedactionConfig,
}

impl SuppressionLayer {
    /// Create a new [`SuppressionLayer`] with the given settings.
    pub fn new(debug_mode: bool, redaction: RedactionConfig) -> Self {
        Self {
            debug_mode,
            redaction,
        }
    }

    /// Production-safe default: `debug_mode = false`, default redaction rules.
    ///
    /// Suppressed traces (the default) will **never** be returned by
    /// [`guard`][Self::guard] when this layer is used.
    pub fn production() -> Self {
        Self::new(false, RedactionConfig::default_rules())
    }

    /// Debug-enabled layer: `debug_mode = true`, default redaction rules.
    ///
    /// Suppressed traces **will** be returned by [`guard`][Self::guard],
    /// allowing developers to inspect internal reasoning.
    pub fn debug() -> Self {
        Self::new(true, RedactionConfig::default_rules())
    }

    /// Returns `Some(&trace)` only if the trace should be exposed to the caller.
    ///
    /// Decision table:
    ///
    /// | `trace.suppressed` | `self.debug_mode` | Result    |
    /// |--------------------|-------------------|-----------|
    /// | `false`            | any               | `Some`    |
    /// | `true`             | `true`            | `Some`    |
    /// | `true`             | `false`           | `None`    |
    ///
    /// Note: this method does **not** apply redaction.  Call
    /// [`redact`][Self::redact] before exposing the trace to any output
    /// pipeline.
    pub fn guard<'a>(
        &self,
        trace: &'a crate::rpl::ReasoningTrace,
    ) -> Option<&'a crate::rpl::ReasoningTrace> {
        if !trace.suppressed {
            // Explicitly un-suppressed: always expose.
            Some(trace)
        } else if self.debug_mode {
            // Debug override: expose even though the trace is suppressed.
            Some(trace)
        } else {
            // Production mode + suppressed trace: hide from the caller.
            None
        }
    }

    /// Apply redaction to all sensitive string fields and return a sanitised
    /// clone of `trace`.
    ///
    /// Fields redacted:
    /// - [`goal`][crate::rpl::ReasoningTrace::goal]
    /// - [`context`][crate::rpl::ReasoningTrace::context]
    /// - [`plan`][crate::rpl::ReasoningTrace::plan]
    /// - [`reason`][crate::rpl::ToolEvaluation::reason] for every tool evaluation
    /// - [`summary`][crate::rpl::MemoryConsideration::summary] for every memory consideration
    ///
    /// Fields intentionally **not** redacted:
    /// - `trace_id` – needed for log correlation across systems.
    pub fn redact(&self, trace: &crate::rpl::ReasoningTrace) -> crate::rpl::ReasoningTrace {
        let mut result = trace.clone();

        // Top-level string fields.
        result.goal = result.goal.as_deref().map(|s| self.redaction.apply_all(s));

        result.context = result
            .context
            .as_deref()
            .map(|s| self.redaction.apply_all(s));

        result.plan = result.plan.as_deref().map(|s| self.redaction.apply_all(s));

        // Per-tool evaluation reasons.
        for eval in &mut result.tool_evaluations {
            eval.reason = eval.reason.as_deref().map(|s| self.redaction.apply_all(s));
        }

        // Per-memory consideration summaries.
        for mem in &mut result.memory_considerations {
            mem.summary = mem.summary.as_deref().map(|s| self.redaction.apply_all(s));
        }

        result
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpl::{MemoryConsideration, ReasoningPhase, ReasoningTrace, ToolEvaluation};

    // ── RedactionRule ────────────────────────────────────────────────────────

    /// `new` must return `Err` for an invalid regex.
    #[test]
    fn redaction_rule_rejects_invalid_pattern() {
        let result = RedactionRule::new("bad", r"(?i)[invalid", "[REDACTED]");
        assert!(result.is_err(), "invalid regex must produce Err");
    }

    /// `new` must return `Ok` for a valid regex.
    #[test]
    fn redaction_rule_accepts_valid_pattern() {
        let result = RedactionRule::new("ok", r"hello", "[REDACTED]");
        assert!(result.is_ok(), "valid regex must produce Ok");
    }

    /// `apply` must replace matching content with the replacement string.
    #[test]
    fn redaction_rule_apply_replaces_match() {
        let rule = RedactionRule::new("test", r"secret\s*=\s*\S+", "[REDACTED]")
            .expect("pattern must compile");
        let output = rule.apply("secret=hunter2 other=value");
        assert_eq!(output, "[REDACTED] other=value");
    }

    /// `apply` must return the original string when there is no match.
    #[test]
    fn redaction_rule_apply_passthrough_on_no_match() {
        let rule = RedactionRule::new("test", r"password\s*=\s*\S+", "[REDACTED]")
            .expect("pattern must compile");
        let input = "nothing sensitive here";
        assert_eq!(rule.apply(input), input);
    }

    // ── RedactionConfig ──────────────────────────────────────────────────────

    /// `default_rules` must contain at least three rules.
    #[test]
    fn default_rules_has_at_least_three_rules() {
        let config = RedactionConfig::default_rules();
        assert!(
            config.rules.len() >= 3,
            "default_rules must contain at least 3 rules, got {}",
            config.rules.len()
        );
    }

    /// `apply_all` must chain rules in insertion order.
    ///
    /// Rule 1: `hello` → `world`
    /// Rule 2: `world` → `rust`
    /// Input `"hello world"` should become `"rust rust"`.
    #[test]
    fn apply_all_chains_multiple_rules_correctly() {
        let mut config = RedactionConfig::new();
        config
            .add_rule(RedactionRule::new("first", "hello", "world").expect("pattern must compile"));
        config
            .add_rule(RedactionRule::new("second", "world", "rust").expect("pattern must compile"));

        // "hello world"
        //   → rule 1 → "world world"
        //   → rule 2 → "rust rust"
        assert_eq!(
            config.apply_all("hello world"),
            "rust rust",
            "apply_all must apply rules in insertion order, feeding each rule the previous output"
        );
    }

    /// `apply_all` on an empty config must return the input unchanged.
    #[test]
    fn apply_all_with_no_rules_is_identity() {
        let config = RedactionConfig::new();
        let input = "no rules applied here";
        assert_eq!(config.apply_all(input), input);
    }

    /// The `api_key` rule must redact a `token:` assignment.
    #[test]
    fn default_rules_redacts_token_assignment() {
        let config = RedactionConfig::default_rules();
        let output = config.apply_all("token: sk-abc123");
        assert!(
            !output.contains("sk-abc123"),
            "api_key rule must redact the token value; got: {output:?}"
        );
    }

    /// The `secret` rule must redact a `secret=` assignment.
    #[test]
    fn default_rules_redacts_secret_assignment() {
        let config = RedactionConfig::default_rules();
        let output = config.apply_all("secret=hunter2");
        assert!(
            !output.contains("hunter2"),
            "secret rule must redact the value; got: {output:?}"
        );
    }

    /// The `password` rule must redact a `password=` assignment.
    #[test]
    fn default_rules_redacts_password_assignment() {
        let config = RedactionConfig::default_rules();
        let output = config.apply_all("password=s3cr3t");
        assert!(
            !output.contains("s3cr3t"),
            "password rule must redact the value; got: {output:?}"
        );
    }

    // ── SuppressionLayer::guard ──────────────────────────────────────────────

    /// In production mode, a suppressed trace must return `None`.
    #[test]
    fn guard_returns_none_for_suppressed_trace_in_production() {
        let sup = SuppressionLayer::production();
        let trace = ReasoningTrace::new(ReasoningPhase::Complete);
        assert!(trace.suppressed, "trace must be suppressed by default");
        assert!(
            sup.guard(&trace).is_none(),
            "suppressed trace must be hidden in production"
        );
    }

    /// In debug mode, a suppressed trace must be returned.
    #[test]
    fn guard_returns_some_for_suppressed_trace_in_debug_mode() {
        let sup = SuppressionLayer::debug();
        let trace = ReasoningTrace::new(ReasoningPhase::Complete);
        assert!(trace.suppressed);
        assert!(
            sup.guard(&trace).is_some(),
            "suppressed trace must be visible in debug mode"
        );
    }

    /// An explicitly un-suppressed trace must always be returned.
    #[test]
    fn guard_returns_some_for_unsuppressed_trace_in_production() {
        let sup = SuppressionLayer::production();
        let mut trace = ReasoningTrace::new(ReasoningPhase::Complete);
        trace.suppressed = false;
        assert!(
            sup.guard(&trace).is_some(),
            "un-suppressed trace must always be visible"
        );
    }

    /// `guard` must return a reference to the *same* trace (not a clone).
    #[test]
    fn guard_returns_reference_to_same_trace() {
        let sup = SuppressionLayer::debug();
        let trace = ReasoningTrace::new(ReasoningPhase::Complete);
        let original_id = trace.trace_id.clone();

        let exposed = sup.guard(&trace).expect("debug mode must expose the trace");
        assert_eq!(
            exposed.trace_id, original_id,
            "guard must return a reference to the original trace"
        );
    }

    // ── SuppressionLayer::redact ─────────────────────────────────────────────

    /// `redact` must not modify `trace_id`.
    #[test]
    fn redact_preserves_trace_id() {
        let sup = SuppressionLayer::production();
        let trace = ReasoningTrace::new(ReasoningPhase::Complete).with_goal("token: sk-abc123");
        let original_id = trace.trace_id.clone();

        let redacted = sup.redact(&trace);
        assert_eq!(
            redacted.trace_id, original_id,
            "redact must not alter trace_id"
        );
    }

    /// `redact` must also sanitise `context` and `plan`.
    #[test]
    fn redact_sanitises_context_and_plan() {
        let sup = SuppressionLayer::production();
        let trace = ReasoningTrace::new(ReasoningPhase::Complete)
            .with_context("api_key: AAABBBCCC")
            .with_plan("use password=s3cr3t to authenticate");

        let redacted = sup.redact(&trace);
        let ctx = redacted.context.expect("context must be present");
        let plan = redacted.plan.expect("plan must be present");

        assert!(
            !ctx.contains("AAABBBCCC"),
            "context must be redacted; got: {ctx:?}"
        );
        assert!(
            !plan.contains("s3cr3t"),
            "plan must be redacted; got: {plan:?}"
        );
    }

    /// `redact` must sanitise tool evaluation reasons.
    #[test]
    fn redact_sanitises_tool_evaluation_reasons() {
        let sup = SuppressionLayer::production();
        let mut trace = ReasoningTrace::new(ReasoningPhase::ToolSelection);
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "auth_tool".to_string(),
            relevance_score: 0.9,
            reason: Some("bearer=secret_token_xyz".to_string()),
            selected: true,
        });

        let redacted = sup.redact(&trace);
        let reason = redacted.tool_evaluations[0]
            .reason
            .as_deref()
            .expect("reason must be present");

        assert!(
            !reason.contains("secret_token_xyz"),
            "tool evaluation reason must be redacted; got: {reason:?}"
        );
    }

    /// `redact` must sanitise memory consideration summaries.
    #[test]
    fn redact_sanitises_memory_consideration_summaries() {
        let sup = SuppressionLayer::production();
        let mut trace = ReasoningTrace::new(ReasoningPhase::MemoryLookup);
        trace.add_memory_consideration(MemoryConsideration {
            memory_key: "auth".to_string(),
            relevance_score: 0.8,
            summary: Some("stored token=abc-xyz-secret".to_string()),
        });

        let redacted = sup.redact(&trace);
        let summary = redacted.memory_considerations[0]
            .summary
            .as_deref()
            .expect("summary must be present");

        assert!(
            !summary.contains("abc-xyz-secret"),
            "memory consideration summary must be redacted; got: {summary:?}"
        );
    }

    /// `redact` must not modify the original trace (it clones first).
    #[test]
    fn redact_does_not_modify_original_trace() {
        let sup = SuppressionLayer::production();
        let trace =
            ReasoningTrace::new(ReasoningPhase::Complete).with_goal("token: original-secret");
        let original_goal = trace.goal.clone();

        let _redacted = sup.redact(&trace);
        assert_eq!(
            trace.goal, original_goal,
            "redact must not modify the original trace"
        );
    }

    /// `redact` must leave `None` fields as `None`.
    #[test]
    fn redact_leaves_none_fields_as_none() {
        let sup = SuppressionLayer::production();
        let trace = ReasoningTrace::new(ReasoningPhase::PreEvaluation);

        assert!(trace.goal.is_none());
        assert!(trace.context.is_none());
        assert!(trace.plan.is_none());

        let redacted = sup.redact(&trace);
        assert!(
            redacted.goal.is_none(),
            "None goal must remain None after redact"
        );
        assert!(
            redacted.context.is_none(),
            "None context must remain None after redact"
        );
        assert!(
            redacted.plan.is_none(),
            "None plan must remain None after redact"
        );
    }
}
