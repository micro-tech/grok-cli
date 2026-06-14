//! Skill arbitration integration — deep integration with tool planning (Task 98).
//!
//! This module provides the [`ArbitrationEngine`], which combines plan-step
//! relevance scoring with RPL reasoning-trace evaluations to select the best
//! tool for each reasoning step.  When system uncertainty is high it falls
//! back to cheaper, safer alternatives rather than expensive or risky tools.
//!
//! # Architecture
//!
//! ```text
//! ArbitrationEngine
//!        │
//!        ├── rank_tools(plan, trace) ──► Vec<RankedTool>
//!        │         │
//!        │         ├── plan_score : UseTool name match  +  tag match in descriptions
//!        │         └── rpl_score  : trace.tool_evaluations relevance_score
//!        │
//!        ├── select_tool(ranked, uncertainty) ──► Option<&RankedTool>
//!        │         │
//!        │         ├── normal path   :  highest combined score (ranked[0])
//!        │         └── high uncert. :  lowest cost (safe fallback)
//!        │
//!        ├── fallback_step(uncertainty, reason) ──► PlanStep
//!        │         │
//!        │         ├── ≥ 0.9       :  ModelCall  (ask for clarification)
//!        │         ├── ≥ threshold :  QueryMemory (check memory first)
//!        │         └── otherwise   :  NoOp
//!        │
//!        └── commit_selection(ranked, state, rpl_trace)
//!                  ├── state.selected_tools ← names where selected == true
//!                  └── rpl_trace           ← one ToolEvaluation per ranked tool
//! ```
//!
//! # Quick start
//!
//! ```rust,ignore
//! use grok_cli::engine::arbitration::ArbitrationEngine;
//!
//! let engine = ArbitrationEngine::with_defaults();
//! let ranked  = engine.rank_tools(&state.plan, &rpl_trace);
//! if let Some(tool) = engine.select_tool(&ranked, state.uncertainty) {
//!     println!("selected: {}", tool.capability.tool_name);
//! }
//! ```

use crate::engine::state::{PlanStep, ReasoningEngineState, StepAction, StepStatus};
use crate::rpl::{ReasoningTrace, ToolEvaluation};

// ---------------------------------------------------------------------------
// ToolCapability
// ---------------------------------------------------------------------------

/// Metadata describing a tool's capabilities and relative execution cost.
///
/// Capabilities are registered with [`ArbitrationEngine`] either via
/// [`ArbitrationEngine::new`] (explicit list) or
/// [`ArbitrationEngine::with_defaults`] (built-in set of eight tools).
///
/// # Cost scale
///
/// | Range       | Meaning                             |
/// |-------------|-------------------------------------|
/// | `0.0 – 0.1` | Near-free (local read-only ops)     |
/// | `0.2 – 0.4` | Low cost (local write, search)      |
/// | `0.5 – 0.6` | Moderate (network calls)            |
/// | `0.7 – 1.0` | High cost (shell exec, mutations)   |
#[derive(Debug, Clone)]
pub struct ToolCapability {
    /// Registered name of the tool (e.g. `"list_directory"`).
    pub tool_name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// Relative cost in `[0.0, 1.0]`.  `0.0` = free; `1.0` = very expensive.
    ///
    /// The value is clamped to `[0.0, 1.0]` during construction.
    pub cost: f32,
    /// Whether this tool can run concurrently with other tools.
    pub parallelizable: bool,
    /// Keyword tags used for fuzzy matching against plan-step descriptions.
    ///
    /// Matching is case-insensitive substring search: if any tag string
    /// appears anywhere inside a step description the step is considered a
    /// soft match for this capability.
    pub tags: Vec<String>,
}

impl ToolCapability {
    /// Construct a new [`ToolCapability`].
    ///
    /// The `cost` argument is clamped to `[0.0, 1.0]` automatically.
    ///
    /// # Arguments
    ///
    /// * `tool_name`     — registered tool name, e.g. `"web_search"`.
    /// * `description`   — human-readable description.
    /// * `cost`          — relative cost in `[0.0, 1.0]`.
    /// * `parallelizable` — `true` if the tool can run in parallel with others.
    /// * `tags`          — keyword tags for plan-step matching.
    pub fn new(
        tool_name: impl Into<String>,
        description: impl Into<String>,
        cost: f32,
        parallelizable: bool,
        tags: Vec<String>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            description: description.into(),
            cost: cost.clamp(0.0, 1.0),
            parallelizable,
            tags,
        }
    }
}

// ---------------------------------------------------------------------------
// RankedTool
// ---------------------------------------------------------------------------

/// A [`ToolCapability`] together with its computed arbitration score.
///
/// Produced by [`ArbitrationEngine::rank_tools`].  The returned vector is
/// sorted in **descending** order by [`score`][Self::score].
///
/// The `selected` field is `false` on every element returned by
/// `rank_tools`; the caller can set it to `true` before passing the slice
/// to [`ArbitrationEngine::commit_selection`].
#[derive(Debug, Clone)]
pub struct RankedTool {
    /// The underlying tool capability metadata.
    pub capability: ToolCapability,
    /// Combined arbitration score in `[0.0, 1.0]`.
    ///
    /// Computed from plan relevance, RPL trace score, and a small cost
    /// penalty.  Higher is better.
    pub score: f32,
    /// Whether this tool was ultimately selected for execution.
    ///
    /// Defaults to `false` after [`ArbitrationEngine::rank_tools`].  Set it
    /// to `true` before calling [`ArbitrationEngine::commit_selection`].
    pub selected: bool,
}

// ---------------------------------------------------------------------------
// ArbitrationConfig
// ---------------------------------------------------------------------------

/// Tuning parameters for the [`ArbitrationEngine`].
///
/// All weight fields should be in `[0.0, 1.0]`.  `rpl_weight` and
/// `plan_weight` are applied independently and do **not** need to sum to 1.
#[derive(Debug, Clone)]
pub struct ArbitrationConfig {
    /// Uncertainty level at or above which the engine switches from
    /// "highest-score" selection to "lowest-cost" (safest) selection.
    ///
    /// Default: `0.7`.
    pub fallback_uncertainty_threshold: f32,
    /// Weight of the RPL reasoning-trace relevance score in the combined score.
    ///
    /// Default: `0.4`.
    pub rpl_weight: f32,
    /// Weight of the plan-step match score in the combined score.
    ///
    /// Default: `0.6`.
    pub plan_weight: f32,
}

impl Default for ArbitrationConfig {
    /// Return the recommended defaults: `fallback_uncertainty_threshold = 0.7`,
    /// `rpl_weight = 0.4`, `plan_weight = 0.6`.
    fn default() -> Self {
        Self {
            fallback_uncertainty_threshold: 0.7,
            rpl_weight: 0.4,
            plan_weight: 0.6,
        }
    }
}

// ---------------------------------------------------------------------------
// ArbitrationEngine
// ---------------------------------------------------------------------------

/// Joint reasoning-arbitration engine for tool selection.
///
/// Combines plan-step relevance, RPL reasoning-trace scores, and cost
/// penalties to rank and select the most appropriate tool for each reasoning
/// cycle.
///
/// When aggregate uncertainty is high (≥ [`ArbitrationConfig::fallback_uncertainty_threshold`])
/// the engine switches to the cheapest registered tool to reduce the risk of
/// expensive or irreversible actions.
///
/// # Example
///
/// ```rust,ignore
/// use grok_cli::engine::arbitration::ArbitrationEngine;
///
/// let engine  = ArbitrationEngine::with_defaults();
/// let mut ranked = engine.rank_tools(&state.plan, &rpl_trace);
///
/// if let Some(best) = engine.select_tool(&ranked, state.uncertainty) {
///     // mark it selected before committing
///     if let Some(rt) = ranked.iter_mut().find(|r| r.capability.tool_name == best.capability.tool_name) {
///         rt.selected = true;
///     }
/// }
///
/// engine.commit_selection(&ranked, &mut state, &mut rpl_trace);
/// ```
pub struct ArbitrationEngine {
    config: ArbitrationConfig,
    capabilities: Vec<ToolCapability>,
}

impl ArbitrationEngine {
    /// Construct a new engine with the given `config` and `capabilities`.
    pub fn new(config: ArbitrationConfig, capabilities: Vec<ToolCapability>) -> Self {
        Self {
            config,
            capabilities,
        }
    }

    /// Construct an engine with [`ArbitrationConfig::default`] and the
    /// built-in set of eight tool capabilities.
    ///
    /// | Tool name           | Cost | Parallel | Tags                        |
    /// |---------------------|------|----------|-----------------------------|
    /// | `read_file`         | 0.1  | false    | read, file, text            |
    /// | `write_file`        | 0.2  | false    | write, file, edit           |
    /// | `list_directory`    | 0.1  | true     | list, dir, directory        |
    /// | `search_content`    | 0.3  | true     | search, find, grep          |
    /// | `web_search`        | 0.5  | true     | search, web, internet       |
    /// | `web_fetch`         | 0.5  | false    | fetch, url, http            |
    /// | `run_shell_command` | 0.7  | false    | shell, run, exec            |
    /// | `save_memory`       | 0.2  | false    | memory, save, remember      |
    pub fn with_defaults() -> Self {
        let caps = vec![
            ToolCapability::new(
                "read_file",
                "Read the contents of a file from the file system.",
                0.1,
                false,
                vec!["read".into(), "file".into(), "text".into()],
            ),
            ToolCapability::new(
                "write_file",
                "Write or overwrite a file on the file system.",
                0.2,
                false,
                vec!["write".into(), "file".into(), "edit".into()],
            ),
            ToolCapability::new(
                "list_directory",
                "List the contents of a directory.",
                0.1,
                true,
                vec!["list".into(), "dir".into(), "directory".into()],
            ),
            ToolCapability::new(
                "search_content",
                "Search file contents using a pattern or keyword (grep-style).",
                0.3,
                true,
                vec!["search".into(), "find".into(), "grep".into()],
            ),
            ToolCapability::new(
                "web_search",
                "Search the internet for information.",
                0.5,
                true,
                vec!["search".into(), "web".into(), "internet".into()],
            ),
            ToolCapability::new(
                "web_fetch",
                "Fetch the content of a URL via HTTP.",
                0.5,
                false,
                vec!["fetch".into(), "url".into(), "http".into()],
            ),
            ToolCapability::new(
                "run_shell_command",
                "Execute an arbitrary shell command on the host system.",
                0.7,
                false,
                vec!["shell".into(), "run".into(), "exec".into()],
            ),
            ToolCapability::new(
                "save_memory",
                "Persist a key-value pair to the long-term memory store.",
                0.2,
                false,
                vec!["memory".into(), "save".into(), "remember".into()],
            ),
        ];

        Self::new(ArbitrationConfig::default(), caps)
    }

    /// Score and rank all registered capabilities against `plan` and `trace`.
    ///
    /// Optionally accepts a DNA-derived tool weight multiplier (from SessionDna)
    /// that is applied to the final score of every tool.
    pub fn rank_tools(
        &self,
        plan: &[PlanStep],
        trace: &ReasoningTrace,
        dna_tool_weight: Option<f32>,
    ) -> Vec<RankedTool> {
        let mut ranked: Vec<RankedTool> = self
            .capabilities
            .iter()
            .map(|cap| {
                let plan_score = Self::compute_plan_score(cap, plan);
                let rpl_score = Self::compute_rpl_score(cap, trace);
                let combined =
                    self.config.plan_weight * plan_score + self.config.rpl_weight * rpl_score;
                let cost_penalty = cap.cost * 0.1;
                let mut score = (combined - cost_penalty).clamp(0.0, 1.0);

                // Apply DNA weight if provided (Task 151)
                if let Some(w) = dna_tool_weight {
                    score = (score * w).clamp(0.0, 1.0);
                }

                RankedTool {
                    capability: cap.clone(),
                    score,
                    selected: false,
                }
            })
            .collect();

        // Sort descending: highest score first.
        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ranked
    }

    /// Select the best tool from a pre-ranked slice given the current `uncertainty`.
    ///
    /// - `uncertainty >= config.fallback_uncertainty_threshold` → returns
    ///   the tool with the **lowest cost** (safest fallback option).
    /// - Otherwise → returns the tool with the **highest score** (the first
    ///   element after [`rank_tools`][Self::rank_tools] sorts descending).
    /// - Returns `None` if `ranked` is empty.
    pub fn select_tool<'a>(
        &self,
        ranked: &'a [RankedTool],
        uncertainty: f32,
    ) -> Option<&'a RankedTool> {
        if ranked.is_empty() {
            return None;
        }

        if uncertainty >= self.config.fallback_uncertainty_threshold {
            // Fallback: cheapest (lowest-cost) tool.
            ranked.iter().min_by(|a, b| {
                a.capability
                    .cost
                    .partial_cmp(&b.capability.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        } else {
            // Normal: highest-scoring tool is already first after rank_tools.
            ranked.first()
        }
    }

    /// Build a fallback [`PlanStep`] appropriate for the current `uncertainty`.
    ///
    /// | Uncertainty range                     | Action produced              |
    /// |---------------------------------------|------------------------------|
    /// | `uncertainty >= 0.9`                  | [`StepAction::ModelCall`]    |
    /// | `uncertainty >= fallback_threshold`   | [`StepAction::QueryMemory`]  |
    /// | otherwise                             | [`StepAction::NoOp`]         |
    ///
    /// The `reason` string is embedded in the step description and in the
    /// action payload where applicable.
    pub fn fallback_step(&self, uncertainty: f32, reason: &str) -> PlanStep {
        if uncertainty >= 0.9 {
            PlanStep::new(
                format!("Model call for clarification (uncertainty={uncertainty:.2}): {reason}"),
                StepAction::ModelCall {
                    prompt: format!(
                        "Uncertainty is very high ({uncertainty:.2}). \
                         Please clarify or provide additional context. \
                         Reason: {reason}"
                    ),
                },
            )
        } else if uncertainty >= self.config.fallback_uncertainty_threshold {
            PlanStep::new(
                format!("Query memory before retrying (uncertainty={uncertainty:.2}): {reason}"),
                StepAction::QueryMemory {
                    query: format!("Relevant context for: {reason} (uncertainty={uncertainty:.2})"),
                },
            )
        } else {
            PlanStep::new(
                format!("No-op fallback (uncertainty={uncertainty:.2}): {reason}"),
                StepAction::NoOp,
            )
        }
    }

    /// Apply the selection decisions from `ranked` to `state` and `rpl_trace`.
    ///
    /// After this call:
    ///
    /// - `state.selected_tools` contains the names of every tool whose
    ///   `selected` field is `true` in `ranked`.
    /// - One [`ToolEvaluation`] is appended to `rpl_trace.tool_evaluations`
    ///   for **every** tool in `ranked`, using the arbitration `score` as
    ///   `relevance_score`.
    pub fn commit_selection(
        &self,
        ranked: &[RankedTool],
        state: &mut ReasoningEngineState,
        rpl_trace: &mut ReasoningTrace,
    ) {
        // Update selected_tools with names of tools marked selected.
        state.selected_tools = ranked
            .iter()
            .filter(|rt| rt.selected)
            .map(|rt| rt.capability.tool_name.clone())
            .collect();

        // Append a ToolEvaluation for every ranked tool.
        for rt in ranked {
            rpl_trace.add_tool_evaluation(ToolEvaluation {
                tool_name: rt.capability.tool_name.clone(),
                relevance_score: rt.score,
                reason: Some(format!(
                    "arbitration: cost={:.2}, parallelizable={}, final_score={:.4}",
                    rt.capability.cost, rt.capability.parallelizable, rt.score
                )),
                selected: rt.selected,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Compute the plan-step contribution to the score for `cap`.
    ///
    /// Returns `1.0` if a `Pending` `UseTool` step names this capability, or
    /// if any tag appears in any step description; otherwise `0.0`.
    fn compute_plan_score(cap: &ToolCapability, plan: &[PlanStep]) -> f32 {
        // Name-based: count Pending UseTool steps that name this capability.
        let name_count = plan
            .iter()
            .filter(|s| s.status == StepStatus::Pending)
            .filter(|s| {
                matches!(
                    &s.action,
                    StepAction::UseTool { tool_name, .. }
                        if tool_name == &cap.tool_name
                )
            })
            .count();
        // Divide by max(1, count) → 0.0 or 1.0.
        let name_score = (name_count as f32 / (name_count.max(1)) as f32).clamp(0.0, 1.0);

        // Tag-based: any tag substring in any step description (case-insensitive).
        let tag_match = plan.iter().any(|s| {
            let desc_lower = s.description.to_lowercase();
            cap.tags.iter().any(|tag| desc_lower.contains(tag.as_str()))
        });
        let tag_score = if tag_match { 1.0_f32 } else { 0.0_f32 };

        name_score.max(tag_score)
    }

    /// Compute the RPL-trace contribution to the score for `cap`.
    ///
    /// Looks up the first matching entry in `trace.tool_evaluations`; returns
    /// its `relevance_score`, or `0.0` if the tool is not present.
    fn compute_rpl_score(cap: &ToolCapability, trace: &ReasoningTrace) -> f32 {
        trace
            .tool_evaluations
            .iter()
            .find(|ev| ev.tool_name == cap.tool_name)
            .map(|ev| ev.relevance_score)
            .unwrap_or(0.0)
    }
}

impl Default for ArbitrationEngine {
    /// Delegates to [`ArbitrationEngine::with_defaults`].
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{PlanStep, ReasoningEngineState, StepAction};
    use crate::rpl::{ReasoningPhase, ReasoningTrace, ToolEvaluation};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Build a minimal empty trace in the `PreEvaluation` phase.
    fn empty_trace() -> ReasoningTrace {
        ReasoningTrace::new(ReasoningPhase::PreEvaluation)
    }

    // -----------------------------------------------------------------------
    // Task 98.1 — ToolCapability / with_defaults
    // -----------------------------------------------------------------------

    #[test]
    fn with_defaults_has_eight_capabilities() {
        let engine = ArbitrationEngine::with_defaults();
        assert_eq!(
            engine.capabilities.len(),
            8,
            "expected 8 built-in capabilities, got {}",
            engine.capabilities.len()
        );

        // Spot-check names are present.
        let names: Vec<&str> = engine
            .capabilities
            .iter()
            .map(|c| c.tool_name.as_str())
            .collect();
        for expected in &[
            "read_file",
            "write_file",
            "list_directory",
            "search_content",
            "web_search",
            "web_fetch",
            "run_shell_command",
            "save_memory",
        ] {
            assert!(names.contains(expected), "missing capability: {expected}");
        }
    }

    // -----------------------------------------------------------------------
    // Task 98.2 — rank_tools
    // -----------------------------------------------------------------------

    #[test]
    fn rank_tools_empty_plan_uses_rpl_scores() {
        let engine = ArbitrationEngine::with_defaults();
        let mut trace = empty_trace();
        // Give read_file a very high RPL score; all others default to 0.
        trace.add_tool_evaluation(ToolEvaluation {
            tool_name: "read_file".to_owned(),
            relevance_score: 0.9,
            reason: None,
            selected: false,
        });

        let ranked = engine.rank_tools(&[], &trace);

        assert!(!ranked.is_empty(), "ranked list must not be empty");
        assert_eq!(
            ranked[0].capability.tool_name, "read_file",
            "read_file should rank first due to high RPL score; got {}",
            ranked[0].capability.tool_name
        );
        // Verify numeric correctness:
        // plan_score=0.0, rpl_score=0.9, combined=0.4*0.9=0.36, penalty=0.01 → 0.35
        let expected = 0.4_f32 * 0.9 - 0.1 * 0.1;
        assert!(
            (ranked[0].score - expected).abs() < 1e-4,
            "score={} expected≈{expected}",
            ranked[0].score
        );
    }

    #[test]
    fn rank_tools_plan_match_boosts_score() {
        let engine = ArbitrationEngine::with_defaults();

        // Use a description ("Enumerate directory contents") that only
        // matches list_directory's tags ("dir", "directory") and no other
        // capability's tags.
        let plan = vec![PlanStep::new(
            "Enumerate directory contents",
            StepAction::UseTool {
                tool_name: "list_directory".to_owned(),
                args: serde_json::Value::Null,
            },
        )];

        let ranked = engine.rank_tools(&plan, &empty_trace());

        // list_directory should rank first.
        assert_eq!(
            ranked[0].capability.tool_name, "list_directory",
            "list_directory should rank first; got {}",
            ranked[0].capability.tool_name
        );

        let list_dir = ranked
            .iter()
            .find(|r| r.capability.tool_name == "list_directory")
            .expect("list_directory must be present");

        // plan_weight=0.6, plan_score=1.0, rpl_score=0.0, cost=0.1
        // score = 0.6 * 1.0 + 0.4 * 0.0 - 0.1 * 0.1 = 0.59
        assert!(
            list_dir.score > 0.5,
            "list_directory score should be > 0.5; got {}",
            list_dir.score
        );
    }

    #[test]
    fn select_tool_normal_returns_highest_score() {
        let engine = ArbitrationEngine::with_defaults();

        // Description avoids triggering other tools' tag matches while the
        // explicit name match ensures read_file scores highest.
        let plan = vec![PlanStep::new(
            "Retrieve document content",
            StepAction::UseTool {
                tool_name: "read_file".to_owned(),
                args: serde_json::Value::Null,
            },
        )];

        let ranked = engine.rank_tools(&plan, &empty_trace());

        // Uncertainty well below the fallback threshold (0.7).
        let selected = engine
            .select_tool(&ranked, 0.1)
            .expect("must return a tool");

        assert_eq!(
            selected.capability.tool_name, ranked[0].capability.tool_name,
            "normal selection should return the highest-scored tool"
        );
    }

    #[test]
    fn select_tool_high_uncertainty_returns_lowest_cost() {
        let engine = ArbitrationEngine::with_defaults();
        // Empty plan + empty trace → all scores driven by −cost_penalty → 0.0
        let ranked = engine.rank_tools(&[], &empty_trace());

        // Uncertainty above the fallback threshold (0.7).
        let selected = engine
            .select_tool(&ranked, 0.95)
            .expect("must return a tool");

        let min_cost = engine
            .capabilities
            .iter()
            .map(|c| c.cost)
            .fold(f32::INFINITY, f32::min);

        assert!(
            (selected.capability.cost - min_cost).abs() < f32::EPSILON,
            "high-uncertainty selection should pick cheapest tool; \
             got cost={} but min_cost={min_cost}",
            selected.capability.cost
        );
    }

    #[test]
    fn select_tool_returns_none_for_empty() {
        let engine = ArbitrationEngine::with_defaults();
        assert!(
            engine.select_tool(&[], 0.5).is_none(),
            "empty ranked list must yield None"
        );
    }

    // -----------------------------------------------------------------------
    // Task 98.3 — fallback_step
    // -----------------------------------------------------------------------

    #[test]
    fn fallback_step_very_high_uncertainty_returns_model_call() {
        let engine = ArbitrationEngine::with_defaults();
        let step = engine.fallback_step(0.95, "no suitable tools found");
        assert!(
            matches!(step.action, StepAction::ModelCall { .. }),
            "uncertainty=0.95 should produce ModelCall; got {:?}",
            step.action
        );
    }

    #[test]
    fn fallback_step_medium_uncertainty_returns_query_memory() {
        let engine = ArbitrationEngine::with_defaults();
        // Default threshold is 0.7; 0.75 is above threshold but below 0.9.
        let step = engine.fallback_step(0.75, "check memory for context");
        assert!(
            matches!(step.action, StepAction::QueryMemory { .. }),
            "uncertainty=0.75 should produce QueryMemory; got {:?}",
            step.action
        );
    }

    #[test]
    fn fallback_step_low_uncertainty_returns_noop() {
        let engine = ArbitrationEngine::with_defaults();
        // Uncertainty well below the 0.7 threshold.
        let step = engine.fallback_step(0.3, "low uncertainty, safe to no-op");
        assert!(
            matches!(step.action, StepAction::NoOp),
            "uncertainty=0.3 should produce NoOp; got {:?}",
            step.action
        );
    }

    // -----------------------------------------------------------------------
    // Task 98.3 — commit_selection
    // -----------------------------------------------------------------------

    #[test]
    fn commit_selection_updates_state_selected_tools() {
        let engine = ArbitrationEngine::with_defaults();
        let mut ranked = engine.rank_tools(&[], &empty_trace());

        // Mark the first two ranked tools as selected.
        ranked[0].selected = true;
        ranked[1].selected = true;
        let expected_names = vec![
            ranked[0].capability.tool_name.clone(),
            ranked[1].capability.tool_name.clone(),
        ];

        let mut state = ReasoningEngineState::default();
        let mut trace = empty_trace();

        engine.commit_selection(&ranked, &mut state, &mut trace);

        assert_eq!(
            state.selected_tools.len(),
            2,
            "state.selected_tools should have exactly 2 entries; got {:?}",
            state.selected_tools
        );
        for name in &expected_names {
            assert!(
                state.selected_tools.contains(name),
                "state.selected_tools missing '{}'; have {:?}",
                name,
                state.selected_tools
            );
        }
    }

    #[test]
    fn commit_selection_appends_tool_evaluations_to_trace() {
        let engine = ArbitrationEngine::with_defaults();
        let ranked = engine.rank_tools(&[], &empty_trace());

        let mut state = ReasoningEngineState::default();
        let mut trace = empty_trace();
        let before = trace.tool_evaluations.len();

        engine.commit_selection(&ranked, &mut state, &mut trace);

        assert_eq!(
            trace.tool_evaluations.len(),
            before + ranked.len(),
            "commit_selection should append one ToolEvaluation per ranked tool; \
             before={before}, ranked={}, after={}",
            ranked.len(),
            trace.tool_evaluations.len()
        );

        // Spot-check: every entry should have a non-empty tool_name.
        for ev in &trace.tool_evaluations {
            assert!(
                !ev.tool_name.is_empty(),
                "ToolEvaluation tool_name must not be empty"
            );
            assert!(
                ev.relevance_score >= 0.0 && ev.relevance_score <= 1.0,
                "relevance_score {} out of [0,1] for tool '{}'",
                ev.relevance_score,
                ev.tool_name
            );
        }
    }
}
