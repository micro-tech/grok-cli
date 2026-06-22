//! Planning layer — goal-driven plan construction for the reasoning engine.
//!
//! This module translates a high-level user goal and a list of available
//! tools into an ordered sequence of [`PlanStep`]s ready for the reasoning
//! engine to execute.
//!
//! # Architecture
//!
//! The central type is [`PlanBuilder`] which is configured with a
//! [`PlanBuilderConfig`].  Tool affinity is expressed through [`ToolHint`]s
//! that map goal keywords (case-insensitive) to registered tool names.
//! When no tool hint fires, the builder falls back to a
//! [`crate::engine::state::StepAction::ModelCall`] so the LLM can decide
//! what to do next.
//!
//! ## Typical usage
//!
//! ```rust
//! use grok_cli::engine::planner::PlanBuilder;
//!
//! let builder = PlanBuilder::default();
//! let steps = builder.build_plan("read the config file", &["read_file"]);
//! // steps[0] → UseTool { tool_name: "read_file", … }
//! // steps[1] → NoOp  (terminator)
//! ```
//!
//! See `docs/engine_architecture.md` for the full design document.

use crate::engine::state::{PlanStep, StepAction, StepStatus};

// ---------------------------------------------------------------------------
// ToolHint
// ---------------------------------------------------------------------------

/// A heuristic rule that maps a keyword in the goal to a tool name.
///
/// When the planner tokenises a goal it evaluates each [`ToolHint`] in
/// descending [`priority`](ToolHint::priority) order.  If the hint's keyword
/// appears as a token in the goal **and** the associated tool is present in
/// the available-tools list, a [`StepAction::UseTool`] step is added to the
/// plan.
#[derive(Debug, Clone)]
pub struct ToolHint {
    /// Case-insensitive keyword matched against individual goal tokens.
    pub keyword: String,

    /// Registered name of the tool to schedule when the keyword matches.
    pub tool_name: String,

    /// Scheduling priority: higher values are evaluated first.
    /// Range: `0` (lowest) – `255` (highest).
    pub priority: u8,
}

impl ToolHint {
    /// Create a new [`ToolHint`].
    ///
    /// # Arguments
    ///
    /// * `keyword` — Word to look for in the tokenised goal (case-insensitive).
    /// * `tool_name` — Name of the tool to use when `keyword` matches.
    /// * `priority` — Evaluation order: higher values are checked first.
    pub fn new(keyword: impl Into<String>, tool_name: impl Into<String>, priority: u8) -> Self {
        Self {
            keyword: keyword.into(),
            tool_name: tool_name.into(),
            priority,
        }
    }
}

// ---------------------------------------------------------------------------
// PlanBuilderConfig
// ---------------------------------------------------------------------------

/// Configuration for [`PlanBuilder`].
///
/// Controls the maximum plan length, the keyword-to-tool hint table, and
/// whether a memory-prefetch step should be prepended to every plan.
#[derive(Debug, Clone)]
pub struct PlanBuilderConfig {
    /// Maximum total number of steps the builder will produce for a single
    /// goal (including the memory prefetch and the `NoOp` terminator).
    ///
    /// Defaults to `10`.
    pub max_steps: usize,

    /// Ordered list of keyword → tool hints evaluated during plan
    /// construction.  See [`PlanBuilderConfig::default_hints`] for the
    /// built-in set.
    pub hints: Vec<ToolHint>,

    /// When `true`, the plan begins with a [`StepAction::QueryMemory`] step
    /// that fetches any facts relevant to the goal before tool steps run.
    ///
    /// Defaults to `false`.
    pub always_start_with_memory: bool,
}

impl Default for PlanBuilderConfig {
    /// Returns a sensible default configuration.
    ///
    /// - `max_steps` = `10`
    /// - `hints` = [`Self::default_hints`]
    /// - `always_start_with_memory` = `false`
    fn default() -> Self {
        Self {
            max_steps: 10,
            hints: Self::default_hints(),
            always_start_with_memory: false,
        }
    }
}

impl PlanBuilderConfig {
    /// Return the built-in set of keyword → tool hints covering common
    /// Grok-CLI tools.
    ///
    /// | Keyword  | Tool               | Priority |
    /// |----------|--------------------|----------|
    /// | `read`   | `read_file`        | 80       |
    /// | `write`  | `write_file`       | 80       |
    /// | `list`   | `list_directory`   | 70       |
    /// | `search` | `web_search`       | 60       |
    /// | `find`   | `search_content`   | 60       |
    /// | `shell`  | `run_shell_command`| 50       |
    /// | `memory` | `save_memory`      | 40       |
    pub fn default_hints() -> Vec<ToolHint> {
        vec![
            ToolHint::new("read", "read_file", 80),
            ToolHint::new("write", "write_file", 80),
            ToolHint::new("list", "list_directory", 70),
            ToolHint::new("search", "web_search", 60),
            ToolHint::new("find", "search_content", 60),
            ToolHint::new("shell", "run_shell_command", 50),
            ToolHint::new("memory", "save_memory", 40),
        ]
    }
}

// ---------------------------------------------------------------------------
// PlanBuilder
// ---------------------------------------------------------------------------

/// Constructs and manages multi-step plans for the reasoning engine.
///
/// Plans are built from a textual `goal` string and a slice of registered
/// tool names.  The builder applies [`ToolHint`]s in descending priority
/// order, matching keywords in the goal to tool steps.  When no hint fires
/// it falls back to a [`StepAction::ModelCall`] so the LLM can decide what
/// to do next.
///
/// # Example
///
/// ```rust,ignore
/// use grok_cli::engine::planner::PlanBuilder;
///
/// let builder = PlanBuilder::default();
/// let steps = builder.build_plan("read the config file", &["read_file"]);
/// // steps[0].action == UseTool { tool_name: "read_file", … }
/// ```
pub struct PlanBuilder {
    config: PlanBuilderConfig,
}

impl PlanBuilder {
    /// Create a new [`PlanBuilder`] from the given configuration.
    pub fn new(config: PlanBuilderConfig) -> Self {
        Self { config }
    }

    /// Build an initial plan for `goal` given `available_tools`.
    ///
    /// # Algorithm
    ///
    /// 1. If [`PlanBuilderConfig::always_start_with_memory`] is `true`, the
    ///    plan starts with a [`StepAction::QueryMemory`] step using `goal` as
    ///    the query string.
    /// 2. The goal is tokenised by splitting on whitespace and ASCII
    ///    punctuation characters (all tokens are lower-cased).
    /// 3. [`ToolHint`]s are evaluated in descending [`priority`][ToolHint::priority]
    ///    order.  For each hint whose `keyword` appears among the goal tokens
    ///    **and** whose `tool_name` is present in `available_tools`, a
    ///    [`StepAction::UseTool`] step is appended — stopping once the total
    ///    step count reaches [`PlanBuilderConfig::max_steps`].
    /// 4. If no tool step was generated a [`StepAction::ModelCall`] fallback
    ///    step is appended (prompt = `goal`), provided there is still room.
    /// 5. A [`StepAction::NoOp`] terminator is appended last if there is
    ///    still room under `max_steps`.
    ///
    /// Returns the ordered step list.
    pub fn build_plan(&self, goal: &str, available_tools: &[&str]) -> Vec<PlanStep> {
        let mut steps: Vec<PlanStep> = Vec::new();

        // ── Step 1: optional memory prefetch ────────────────────────────────
        if self.config.always_start_with_memory {
            steps.push(PlanStep::new(
                "Retrieve relevant memories for goal",
                StepAction::QueryMemory {
                    query: goal.to_string(),
                },
            ));
        }

        // ── Step 2: tokenise the goal ────────────────────────────────────────
        let tokens = tokenize(goal);

        // ── Step 3: apply hints in descending priority order ─────────────────
        let mut sorted_hints = self.config.hints.clone();
        // Stable sort keeps equal-priority hints in their declaration order.
        sorted_hints.sort_by_key(|h| std::cmp::Reverse(h.priority));

        let mut tool_steps_added = 0usize;

        for hint in &sorted_hints {
            if steps.len() >= self.config.max_steps {
                break;
            }

            let kw_lc = hint.keyword.to_lowercase();
            let keyword_matches = tokens.iter().any(|t| t == &kw_lc);
            let tool_available = available_tools.contains(&hint.tool_name.as_str());

            if keyword_matches && tool_available {
                steps.push(PlanStep::new(
                    format!("Use tool {}", hint.tool_name),
                    StepAction::UseTool {
                        tool_name: hint.tool_name.clone(),
                        args: serde_json::Value::Null,
                    },
                ));
                tool_steps_added += 1;
            }
        }

        // ── Step 4: fallback to a ModelCall if no tools matched ──────────────
        if tool_steps_added == 0 && steps.len() < self.config.max_steps {
            steps.push(PlanStep::new(
                format!("Model call: {goal}"),
                StepAction::ModelCall {
                    prompt: goal.to_string(),
                },
            ));
        }

        // ── Step 5: Intelligent sub-agent delegation (Task 127) ─────────────
        // Delegate when the goal looks complex or parallelizable.
        // (Uncertainty-driven delegation can be added by passing
        // ReasoningEngineState into build_plan in a future iteration.)
        let looks_complex = goal.split_whitespace().count() > 12
            || goal.to_lowercase().contains("complex")
            || goal.to_lowercase().contains("multiple")
            || goal.to_lowercase().contains("parallel")
            || goal.to_lowercase().contains("research")
            || goal.to_lowercase().contains("large");

        if looks_complex && steps.len() < self.config.max_steps {
            steps.push(PlanStep::new(
                format!("Delegate to sub-agent: {goal}"),
                StepAction::DelegateToSubAgent {
                    task: goal.to_string(),
                    agent_id: None,
                },
            ));
        }

        // ── Step 6: NoOp terminator ──────────────────────────────────────────
        if steps.len() < self.config.max_steps {
            steps.push(PlanStep::new("Plan complete", StepAction::NoOp));
        }

        steps
    }

    /// Build a DNA-conditioned plan.
    ///
    /// This is the DNA-aware version of `build_plan`. It first builds the
    /// base plan, then lets the provided `SessionDna` reshape the final
    /// structure and tone via `shape_plan`.
    pub fn build_dna_plan(
        &self,
        goal: &str,
        available_tools: &[&str],
        dna: Option<&crate::session::dna::SessionDna>,
    ) -> Vec<PlanStep> {
        let base_steps = self.build_plan(goal, available_tools);

        if let Some(dna) = dna {
            // We don't mutate the steps themselves here; instead we return
            // the same steps but the caller (or a higher layer) can use
            // `dna.shape_plan(...)` on a textual representation of the plan.
            // For now we simply return the base plan — the DNA shaping is
            // applied at the prompt / logging layer (see acp/mod.rs).
            tracing::debug!("DNA mode for planning: {}", dna.get_mode());
        }

        base_steps
    }
    ///
    /// Useful when no tools are registered or when the goal requires
    /// free-form model reasoning without any tool invocations.
    pub fn build_model_only_plan(&self, goal: &str) -> Vec<PlanStep> {
        vec![
            PlanStep::new(
                format!("Model call: {goal}"),
                StepAction::ModelCall {
                    prompt: goal.to_string(),
                },
            ),
            PlanStep::new("Plan complete", StepAction::NoOp),
        ]
    }

    /// Revise a plan after a step has failed.
    ///
    /// # Strategy
    ///
    /// 1. All steps **before** `failed_at_index` whose status is
    ///    [`StepStatus::Completed`] are cloned verbatim into the revised plan.
    /// 2. The failed step at `failed_at_index` is cloned with its status
    ///    overwritten to [`StepStatus::Skipped`].
    /// 3. A [`StepAction::ModelCall`] step is inserted with the prompt
    ///    `"Re-evaluate after step failure: {reason}"`.
    /// 4. Any steps that appear **after** `failed_at_index` in the original
    ///    plan and whose status is still [`StepStatus::Pending`] are appended
    ///    in their original order.
    /// 5. A [`StepAction::NoOp`] terminator is appended unless one is already
    ///    the last step in the revised plan.
    ///
    /// The input `current_plan` is **never** mutated; a fresh `Vec` is
    /// returned.
    pub fn revise_plan(
        &self,
        current_plan: &[PlanStep],
        failed_at_index: usize,
        reason: &str,
    ) -> Vec<PlanStep> {
        let mut revised: Vec<PlanStep> = Vec::new();

        // 1. Keep all Completed steps that precede the failure.
        for step in current_plan.iter().take(failed_at_index) {
            if step.status == StepStatus::Completed {
                revised.push(step.clone());
            }
        }

        // 2. Clone the failed step and mark it Skipped.
        if let Some(failed_step) = current_plan.get(failed_at_index) {
            let mut skipped = failed_step.clone();
            skipped.status = StepStatus::Skipped;
            revised.push(skipped);
        }

        // 3. Insert a re-evaluation ModelCall.
        revised.push(PlanStep::new(
            format!("Re-evaluate after step failure: {reason}"),
            StepAction::ModelCall {
                prompt: format!("Re-evaluate after step failure: {reason}"),
            },
        ));

        // 4. Append remaining Pending steps from after the failed index.
        for step in current_plan.iter().skip(failed_at_index + 1) {
            if step.status == StepStatus::Pending {
                revised.push(step.clone());
            }
        }

        // 5. Ensure a NoOp terminator is present.
        let already_has_noop = matches!(revised.last().map(|s| &s.action), Some(StepAction::NoOp));
        if !already_has_noop {
            revised.push(PlanStep::new("Plan complete", StepAction::NoOp));
        }

        revised
    }
}

impl Default for PlanBuilder {
    /// Returns a [`PlanBuilder`] using [`PlanBuilderConfig::default`].
    fn default() -> Self {
        Self::new(PlanBuilderConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Tokenise `text` by splitting on whitespace and ASCII punctuation, then
/// lower-case-normalise each token.  Empty strings are discarded.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{StepAction, StepStatus};

    // ── PlanBuilderConfig ────────────────────────────────────────────────────

    /// The default configuration must cap plans at ten steps.
    #[test]
    fn default_config_has_max_steps_ten() {
        let cfg = PlanBuilderConfig::default();
        assert_eq!(cfg.max_steps, 10);
    }

    // ── build_plan ───────────────────────────────────────────────────────────

    /// When no tool hint fires (no available tools), the plan must contain a
    /// `ModelCall` step followed by a `NoOp` terminator.
    #[test]
    fn build_plan_returns_model_call_for_no_tool_match() {
        let builder = PlanBuilder::default();
        let steps = builder.build_plan("summarize this document", &[]);

        assert!(
            steps.len() >= 2,
            "expected at least 2 steps, got {}",
            steps.len()
        );
        assert!(
            matches!(steps[0].action, StepAction::ModelCall { .. }),
            "first step should be a ModelCall, got {:?}",
            steps[0].action
        );
        assert!(
            matches!(steps.last().unwrap().action, StepAction::NoOp),
            "last step should be a NoOp"
        );
    }

    /// The keyword `"read"` in the goal must produce a `UseTool { "read_file" }`
    /// step when `"read_file"` is in the available-tools list.
    #[test]
    fn build_plan_matches_read_keyword_to_read_file() {
        let builder = PlanBuilder::default();
        let steps = builder.build_plan("read the config file", &["read_file"]);

        let has_read_tool = steps.iter().any(|s| {
            matches!(&s.action, StepAction::UseTool { tool_name, .. } if tool_name == "read_file")
        });
        assert!(
            has_read_tool,
            "expected a UseTool(read_file) step in {:?}",
            steps
        );
    }

    /// When `always_start_with_memory` is `true`, the very first step must be
    /// a `QueryMemory` step.
    #[test]
    fn build_plan_prepends_memory_step_when_configured() {
        let config = PlanBuilderConfig {
            always_start_with_memory: true,
            ..Default::default()
        };
        let builder = PlanBuilder::new(config);
        let steps = builder.build_plan("list files in the project", &["list_directory"]);

        assert!(
            matches!(steps[0].action, StepAction::QueryMemory { .. }),
            "first step should be QueryMemory, got {:?}",
            steps[0].action
        );
    }

    /// The builder must never produce more steps than `config.max_steps`.
    #[test]
    fn build_plan_respects_max_steps() {
        let config = PlanBuilderConfig {
            max_steps: 2,
            ..Default::default()
        };
        let builder = PlanBuilder::new(config);

        // All seven keyword/tool pairs match — but we must stop at 2.
        let steps = builder.build_plan(
            "read write list search find shell memory",
            &[
                "read_file",
                "write_file",
                "list_directory",
                "web_search",
                "search_content",
                "run_shell_command",
                "save_memory",
            ],
        );

        assert!(
            steps.len() <= 2,
            "plan has {} steps, expected <= 2",
            steps.len()
        );
    }

    // ── revise_plan ──────────────────────────────────────────────────────────

    /// The step at `failed_at_index` must appear in the revised plan with
    /// [`StepStatus::Skipped`].
    #[test]
    fn revise_plan_skips_failed_step() {
        let builder = PlanBuilder::default();

        let mut step = PlanStep::new("do something", StepAction::NoOp);
        step.status = StepStatus::Pending;

        let revised = builder.revise_plan(&[step], 0, "network timeout");

        let skipped = revised.iter().find(|s| s.status == StepStatus::Skipped);
        assert!(skipped.is_some(), "expected at least one Skipped step");
    }

    /// A `ModelCall` step containing the failure reason must be inserted
    /// immediately after the skipped failed step.
    #[test]
    fn revise_plan_appends_model_call() {
        let builder = PlanBuilder::default();
        let step = PlanStep::new("do something", StepAction::NoOp);

        let revised = builder.revise_plan(&[step], 0, "some error");

        let model_call = revised.iter().find(|s| {
            matches!(
                &s.action,
                StepAction::ModelCall { prompt } if prompt.contains("some error")
            )
        });
        assert!(
            model_call.is_some(),
            "expected a ModelCall step containing 'some error' in the revised plan"
        );
    }

    /// Steps before the failure that are `Completed` must be preserved in the
    /// revised plan.
    #[test]
    fn revise_plan_preserves_completed_steps() {
        let builder = PlanBuilder::default();

        let mut step0 = PlanStep::new("completed work", StepAction::NoOp);
        step0.status = StepStatus::Completed;

        let step1 = PlanStep::new("failing step", StepAction::NoOp);

        let revised = builder.revise_plan(&[step0, step1], 1, "oops");

        let preserved = revised.iter().find(|s| s.status == StepStatus::Completed);
        assert!(
            preserved.is_some(),
            "expected the Completed step to be preserved in the revised plan"
        );
    }

    // ── build_model_only_plan ────────────────────────────────────────────────

    /// `build_model_only_plan` must return exactly two steps: a `ModelCall`
    /// followed by a `NoOp` terminator.
    #[test]
    fn build_model_only_plan_returns_model_call() {
        let builder = PlanBuilder::default();
        let steps = builder.build_model_only_plan("think carefully about this");

        assert_eq!(steps.len(), 2, "expected exactly 2 steps");
        assert!(
            matches!(steps[0].action, StepAction::ModelCall { .. }),
            "first step should be ModelCall"
        );
        assert!(
            matches!(steps[1].action, StepAction::NoOp),
            "second step should be NoOp"
        );
    }

    // ── Task 127: Sub-agent delegation ─────────────────────────────────────

    /// Complex goals should trigger a `DelegateToSubAgent` step.
    #[test]
    fn build_plan_delegates_on_complex_goal() {
        let builder = PlanBuilder::default();
        let steps = builder.build_plan(
            "Research and implement a complex authentication system with multiple providers and parallel token validation",
            &[],
        );

        let has_delegation = steps.iter().any(|s| {
            matches!(s.action, StepAction::DelegateToSubAgent { .. })
        });

        assert!(
            has_delegation,
            "Expected a DelegateToSubAgent step for a complex goal, got: {:?}",
            steps
        );
    }
}
