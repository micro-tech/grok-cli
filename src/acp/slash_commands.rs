//! Slash-command definitions for ACP integration.
//!
//! This module implements the ACP slash-command feature described at
//! <https://agentclientprotocol.com/protocol/slash-commands>.
//!
//! # How it works
//!
//! 1. After a new session is created the agent sends an
//!    `available_commands_update` notification (built from [`get_available_commands`]).
//! 2. The client surfaces those commands in its UI (e.g. Zed's `/` palette).
//! 3. When the user invokes a command the client sends a normal
//!    `session/prompt` request whose text begins with `/<name> …`.
//! 4. [`parse_slash_command`] recognises the prefix and returns a typed
//!    [`SlashCommand`] value.
//! 5. Commands that need no AI round-trip (e.g. `/help`, `/clear`) are
//!    handled directly via [`handle_builtin`].
//!    All other commands go through [`command_to_prompt`] which rewrites the
//!    user text into a richer instruction for the model.
//!
//! # Adding a new command
//!
//! 1. Add a variant to [`SlashCommand`].
//! 2. Add a match arm in [`parse_slash_command`].
//! 3. Add an [`AvailableCommand`] entry in [`get_available_commands`].
//! 4. Handle it in [`command_to_prompt`] (AI) **or** [`handle_builtin`] (direct).

use super::protocol::{AvailableCommand, AvailableCommandInput, UnstructuredCommandInput};
use crate::config::ThinkingMode;
use anyhow::anyhow;

// ---------------------------------------------------------------------------
// Command enum
// ---------------------------------------------------------------------------

/// All slash commands supported by grok-cli.
///
/// Variants whose names end with `{ description: String }` accept optional
/// free-text that the user types after the command name.
#[derive(Debug, Clone, PartialEq)]
pub enum SlashCommand {
    /// `/help` — list all available slash commands.
    Help,

    /// `/web <query>` — ask the model to search/research a topic.
    Web { query: String },

    /// `/explain [description]` — ask for a thorough explanation of code or a concept.
    Explain { description: String },

    /// `/review [description]` — request a comprehensive code review.
    Review { description: String },

    /// `/plan <description>` — generate a detailed implementation plan.
    Plan { description: String },

    /// `/test [description]` — help write, run, or debug tests.
    Test { description: String },

    /// `/fix [description]` — ask the model to diagnose and fix a problem.
    Fix { description: String },

    /// `/model [name|show]` — show current model or switch to a different one.
    /// `/model` or `/model show` displays the active model.
    /// `/model <name>` switches the session to that model.
    Model { name: String },

    /// `/clear` — wipe the current conversation history.
    Clear,

    /// `/context` — show session configuration and active context files.
    Context,

    /// `/tools` — list all 32 LLM-callable tools available in this session.
    Tools,

    /// `/bayes show` — display current Bayesian belief state.
    BayesShow,

    /// `/bayes reset` — reset Bayesian belief state.
    BayesReset,

    /// `/bayes explain` — explain current Bayesian reasoning.
    BayesExplain,

    /// `/archives` — list all context archive chunks for this session.
    Archives,

    /// `/recall <N>` — restore archived chunk N back into active context.
    /// `/recall` without an argument lists archives (same as `/archives`).
    Recall { chunk_id: Option<u32> },

    /// `/goal <text>` -- set a persistent session goal.
    /// `/goal` with no argument shows the current goal.
    Goal { text: String },

    /// `/goal clear` -- remove the active goal.
    GoalClear,

    /// `/visualize` — display the pipeline state machine as a DOT graph.
    Visualize,

    /// `/think [off|low|high]` — set or display the reasoning mode.
    /// `/think` without an argument shows the current mode.
    /// `/think off` disables reasoning.
    /// `/think low` enables light reasoning.
    /// `/think high` enables deep reasoning.
    Think { mode: Option<ThinkingMode> },

    /// `/commit [instructions]` — generate a commit message from the current git diff.
    /// `/commit` with no argument uses default Conventional Commits style.
    /// Additional instructions can be provided after the command.
    Commit { instructions: String },

    /// `/diagnostics` — show status of all background systems (Bayesian, DNA, compression, etc.).
    Diagnostics,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Try to parse a user message as a slash command.
///
/// Returns `Some(SlashCommand)` when the (trimmed) message starts with a
/// recognised `/<name>` prefix, `None` for all other messages.
///
/// Unknown `/foo` prefixes return `None` so they are forwarded to the model
/// unchanged — this is intentional; the model may still produce useful output
/// for unrecognised command text.
pub fn parse_slash_command(message: &str) -> Option<SlashCommand> {
    let trimmed = message.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    // Split into command token + optional rest
    let (command_token, rest) = match trimmed.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&trimmed[..pos], trimmed[pos..].trim()),
        None => (trimmed, ""),
    };

    let name = command_token.to_lowercase();
    let args = rest.to_string();

    match name.as_str() {
        "/help" => Some(SlashCommand::Help),
        "/web" => Some(SlashCommand::Web { query: args }),
        "/explain" => Some(SlashCommand::Explain { description: args }),
        "/review" => Some(SlashCommand::Review { description: args }),
        "/plan" => Some(SlashCommand::Plan { description: args }),
        "/test" => Some(SlashCommand::Test { description: args }),
        "/fix" => Some(SlashCommand::Fix { description: args }),
        "/model" => Some(SlashCommand::Model { name: args }),
        "/clear" => Some(SlashCommand::Clear),
        "/context" => Some(SlashCommand::Context),
        "/tools" => Some(SlashCommand::Tools),
        "/bayes" => match args.as_str() {
            "show" | "" => Some(SlashCommand::BayesShow),
            "reset" => Some(SlashCommand::BayesReset),
            "explain" => Some(SlashCommand::BayesExplain),
            _ => None,
        },
        "/archives" => Some(SlashCommand::Archives),
        "/recall" => {
            let id = args.parse::<u32>().ok();
            Some(SlashCommand::Recall { chunk_id: id })
        }
        "/goal" => {
            if args.eq_ignore_ascii_case("clear") {
                Some(SlashCommand::GoalClear)
            } else {
                Some(SlashCommand::Goal { text: args })
            }
        }
        "/visualize" => Some(SlashCommand::Visualize),
        "/think" => {
            if args.is_empty() {
                // `/think` with no arg — show current mode
                Some(SlashCommand::Think { mode: None })
            } else {
                // `/think off|low|high` — None for unknown args (AI responds)
                ThinkingMode::from_str_ci(&args).map(|m| SlashCommand::Think { mode: Some(m) })
            }
        }
        "/commit" => Some(SlashCommand::Commit {
            instructions: args,
        }),
        "/diagnostics" => Some(SlashCommand::Diagnostics),
        _ => None, // unknown command -- let the AI handle the raw text
    }
}

// ---------------------------------------------------------------------------
// ACP advertisement
// ---------------------------------------------------------------------------

/// Build the list of [`AvailableCommand`] entries sent to ACP clients via the
/// `available_commands_update` notification immediately after session creation.
/// Commands are returned in alphabetical order.
pub fn get_available_commands() -> Vec<AvailableCommand> {
    // Helper closure to keep call sites concise.
    // Wraps a hint string in the crate's AvailableCommandInput::Unstructured variant.
    let input =
        |hint: &str| AvailableCommandInput::Unstructured(UnstructuredCommandInput::new(hint));

    let mut cmds = vec![
        AvailableCommand::new(
            "archives",
            "List all archived context chunks for this session",
        ),
        AvailableCommand::new(
            "bayes",
            "Inspect or manage the Bayesian belief-state for this session (show / reset / explain)",
        )
        .input(input("show | reset | explain — omit to show the current state")),
        AvailableCommand::new("clear", "Clear the current conversation history"),
        AvailableCommand::new(
            "commit",
            "Generate a high-quality commit message from the current git diff (Conventional Commits by default)",
        )
        .input(input("optional extra instructions for the commit message style")),
        AvailableCommand::new(
            "context",
            "Show current session configuration and active context",
        ),
        AvailableCommand::new(
            "diagnostics",
            "Show status of all background systems (Bayesian engine, DNA, compression, knowledge packs, etc.)",
        ),
        AvailableCommand::new(
            "explain",
            "Get a thorough explanation of code, a file, or a concept",
        )
        .input(input("code, file path, or concept to explain")),
        AvailableCommand::new("fix", "Diagnose and fix a bug or error")
            .input(input("problem description, error message, or file path")),
        AvailableCommand::new(
            "goal",
            "Set, view, or clear the persistent session goal that shapes all message interpretation",
        )
        .input(input(
            "goal text — e.g. 'refactor auth'. Type 'clear' to remove the active goal, or omit to show it",
        )),
        AvailableCommand::new("help", "Show all available slash commands and their usage"),
        AvailableCommand::new("model", "Switch to a different Grok model for this session")
            .input(input("model name (e.g. grok-3, grok-4.3, grok-3-mini)")),
        AvailableCommand::new("plan", "Create a detailed step-by-step implementation plan")
            .input(input("description of what to plan")),
        AvailableCommand::new(
            "recall",
            "Restore an archived context chunk back into the active window",
        )
        .input(input("chunk number (e.g. 1, 2, 3) -- omit to list archives")),
        AvailableCommand::new(
            "review",
            "Comprehensive code review: bugs, security, performance, style",
        )
        .input(input("code or file path to review")),
        AvailableCommand::new("test", "Help write, run, or debug tests")
            .input(input("test description or file path (optional)")),
        AvailableCommand::new(
            "think",
            "Set or show the reasoning mode (off / low / high).  \"high\" gives the most thorough answers; \"off\" (default) gives the fastest.",
        )
        .input(input("off | low | high -- omit to show the current mode")),
        AvailableCommand::new(
            "tools",
            "List all LLM-callable tools available in this session (file, shell, web, task, …)",
        ),
        AvailableCommand::new(
            "visualize",
            "Display the Grok-CLI routing pipeline as a DOT/Graphviz graph",
        ),
        AvailableCommand::new("web", "Research a topic or search the web for information")
            .input(input("query to research")),
    ];

    // Ensure alphabetical order by command name
    cmds.sort_by(|a, b| a.name.cmp(&b.name));
    cmds
}

// ---------------------------------------------------------------------------
// AI prompt builder
// ---------------------------------------------------------------------------

/// Rewrite a slash command into an enhanced prompt for the Grok model.
///
/// Returns `None` for commands that are handled entirely on the agent side
/// (built-ins such as `/help`, `/clear`, `/model`, `/context`) — the caller
/// must NOT forward these to the AI.
///
/// For AI-assisted commands the returned `String` replaces the raw user
/// message that would otherwise be sent to the model, adding richer
/// instructions so the response is more targeted and complete.
pub fn command_to_prompt(cmd: &SlashCommand) -> Option<String> {
    match cmd {
        // --- built-ins: handled without AI ---
        SlashCommand::Help
        | SlashCommand::Clear
        | SlashCommand::Model { .. }
        | SlashCommand::Context
        | SlashCommand::Tools
        | SlashCommand::BayesShow
        | SlashCommand::BayesReset
        | SlashCommand::BayesExplain
        | SlashCommand::Archives
        | SlashCommand::Recall { .. }
        | SlashCommand::Goal { .. }
        | SlashCommand::GoalClear
        | SlashCommand::Visualize
        | SlashCommand::Think { .. }
        | SlashCommand::Diagnostics => None,

        // --- AI-assisted commands ---
        SlashCommand::Web { query } => {
            let topic = if query.is_empty() {
                "the topic in the conversation".to_string()
            } else {
                query.clone()
            };
            Some(format!(
                "Please research and provide comprehensive, accurate, up-to-date information \
                 about: {topic}\n\n\
                 Structure your answer with:\n\
                 1. A clear summary of the key facts\n\
                 2. Relevant details and nuances\n\
                 3. Any important caveats or conflicting views\n\
                 4. References or sources where applicable"
            ))
        }

        SlashCommand::Explain { description } => {
            let subject = if description.is_empty() {
                "the code or concept in the conversation".to_string()
            } else {
                description.clone()
            };
            Some(format!(
                "Please provide a thorough, clear explanation of: {subject}\n\n\
                 Cover the following:\n\
                 - **What** it is / does\n\
                 - **How** it works (step by step if helpful)\n\
                 - **Why** it is designed this way (trade-offs, history)\n\
                 - Key concepts and terminology\n\
                 - Important edge cases or gotchas\n\
                 - Practical usage examples where appropriate"
            ))
        }

        SlashCommand::Review { description } => {
            let target = if description.is_empty() {
                "the code in the conversation".to_string()
            } else {
                description.clone()
            };
            Some(format!(
                "Please perform a comprehensive code review of: {target}\n\n\
                 Examine and report on:\n\
                 - **Bugs & logic errors** — incorrect behaviour, off-by-ones, etc.\n\
                 - **Security vulnerabilities** — injection, unsafe unwraps, secret exposure\n\
                 - **Performance issues** — unnecessary allocations, blocking calls, O(n²) loops\n\
                 - **Error handling** — unchecked Results, panics in production paths\n\
                 - **Readability & style** — naming, structure, idiomatic patterns\n\
                 - **Maintainability** — coupling, test coverage, documentation gaps\n\n\
                 For each issue provide: severity (critical / major / minor / nit), \
                 a description, and a concrete fix."
            ))
        }

        SlashCommand::Plan { description } => {
            let subject = if description.is_empty() {
                "the feature or change described in the conversation".to_string()
            } else {
                description.clone()
            };
            Some(format!(
                "Please create a detailed, actionable implementation plan for: {subject}\n\n\
                 The plan should include:\n\
                 1. **Architecture overview** — key components and how they interact\n\
                 2. **Ordered task list** — numbered steps with clear deliverables\n\
                 3. **Dependencies** — which tasks must complete before others\n\
                 4. **Complexity estimate** — rough effort for each step (S/M/L/XL)\n\
                 5. **Risks & mitigations** — what could go wrong and how to prevent it\n\
                 6. **Testing strategy** — how to verify each step and the whole feature\n\
                 7. **Open questions** — anything that needs clarification before starting"
            ))
        }

        SlashCommand::Test { description } => {
            let target = if description.is_empty() {
                "the code in the conversation".to_string()
            } else {
                description.clone()
            };
            Some(format!(
                "Please help write comprehensive tests for: {target}\n\n\
                 Include:\n\
                 - **Unit tests** for individual functions and methods\n\
                 - **Integration tests** where components interact\n\
                 - **Edge cases** — empty inputs, boundary values, overflow, Unicode, etc.\n\
                 - **Error paths** — ensure error conditions are handled and tested\n\
                 - **Property-based tests** using `proptest` where they add value\n\n\
                 Follow Rust testing best practices:\n\
                 - Use `#[cfg(test)]` modules for unit tests\n\
                 - Use `tests/` directory for integration tests\n\
                 - Name tests descriptively: `test_<function>_<scenario>_<expected>`\n\
                 - Use `assert_eq!` / `assert!` / `assert_matches!` appropriately\n\
                 - Mock external dependencies where needed"
            ))
        }

        SlashCommand::Fix { description } => {
            let problem = if description.is_empty() {
                "the issue described or shown in the conversation".to_string()
            } else {
                description.clone()
            };
            Some(format!(
                "Please diagnose and fix: {problem}\n\n\
                 Approach:\n\
                 1. **Root cause analysis** — identify the underlying problem, not just symptoms\n\
                 2. **Minimal reproduction** — if possible, narrow it to the smallest failing case\n\
                 3. **Fix** — provide the corrected code with a clear explanation\n\
                 4. **Why this works** — explain what was wrong and why the fix is correct\n\
                 5. **Regression test** — suggest a test that would catch this bug in the future\n\
                 6. **Related issues** — flag any similar problems nearby that should also be fixed"
            ))
        }

        SlashCommand::Commit { instructions } => {
            // Fetch the actual git diff so the model has something to write about.
            let diff = get_git_diff_for_commit().unwrap_or_else(|e| {
                format!("[Could not obtain git diff: {}]", e)
            });

            let extra = if instructions.trim().is_empty() {
                String::new()
            } else {
                format!("\n\nAdditional instructions from user: {}", instructions)
            };

            Some(format!(
                "Generate a high-quality git commit message for the following changes.\n\n\
                 Requirements:\n\
                 1. Follow the Conventional Commits specification by default:\n\
                    <type>(<scope>): <description>\n\n\
                 2. Use the present tense (\"add feature\" not \"added feature\")\n\
                 3. Keep the subject line under 72 characters\n\
                 4. Provide a longer body when the change is complex\n\
                 5. Reference any related issues or PRs if applicable\n\n\
                 --- BEGIN DIFF ---\n{}\n--- END DIFF ---\n\n\
                 Recent conversation context and any active goals will also be provided.{extra}",
                diff
            ))
        }
    }
}

/// Run `git diff --cached` (staged changes). If nothing is staged, fall back to
/// `git diff` (unstaged). Returns an error if we are not inside a git repo.
fn get_git_diff_for_commit() -> anyhow::Result<String> {
    use std::process::Command;

    // First try staged changes
    let output = Command::new("git")
        .args(["diff", "--cached", "--no-color"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let diff = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !diff.is_empty() {
                return Ok(diff);
            }
            // Nothing staged — fall back to unstaged diff
        }
        _ => {}
    }

    // Fallback: unstaged changes
    let output = Command::new("git")
        .args(["diff", "--no-color"])
        .output()
        .map_err(|e| anyhow!("git diff failed: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!(
            "git returned non-zero exit code: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let diff = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if diff.is_empty() {
        Ok("[No changes detected — working tree is clean]".to_string())
    } else {
        Ok(diff)
    }
}

// ---------------------------------------------------------------------------
// Built-in (non-AI) command handler
// ---------------------------------------------------------------------------

/// Result of handling a built-in slash command directly (without calling the AI).
#[derive(Debug, Clone)]
pub enum BuiltinResult {
    /// Return this text as the assistant response immediately.
    Text(String),
    /// Clear the conversation history, then return this confirmation text.
    ClearHistory,
    /// Switch to the given model name, then return a confirmation text.
    SwitchModel(String),

    /// Show the currently active model for the session.
    ShowCurrentModel,
    /// Display session context/config info — the caller supplies the text.
    ShowContext,
    /// Recall an archived context chunk. `None` = list all archives.
    RecallArchive(Option<u32>),
    /// Display the current Bayesian belief-state graph for this session.
    ShowBayes,
    /// Reset the Bayesian priors for this session back to defaults.
    ResetBayes,
    /// Explain what the current Bayesian state means in plain English.
    ExplainBayes,
    /// Set the active session goal -- handled in the ACP session layer.
    SetGoal(String),
    /// Clear the active goal.
    ClearGoal,
    /// Show the current goal.
    ShowGoal,
    /// Show the pipeline visualizer graph.
    ShowVisualizer,
    /// Set the thinking/reasoning mode for this session.
    /// `None` means the user typed `/think` with no argument (show current mode).
    SetThinkingMode(Option<crate::config::ThinkingMode>),

    /// Show diagnostics report for all background systems.
    ShowDiagnostics,
}

/// Handle a built-in slash command, returning `Some(BuiltinResult)` if the
/// command is handled here, or `None` if it should go to the AI.
pub fn handle_builtin(cmd: &SlashCommand) -> Option<BuiltinResult> {
    match cmd {
        SlashCommand::Help => Some(BuiltinResult::Text(format_help_text())),
        SlashCommand::Clear => Some(BuiltinResult::ClearHistory),
        SlashCommand::Model { name } => {
            let arg = name.trim().to_lowercase();
            if arg.is_empty() || arg == "show" {
                Some(BuiltinResult::ShowCurrentModel)
            } else {
                Some(BuiltinResult::SwitchModel(arg))
            }
        }
        SlashCommand::Context => Some(BuiltinResult::ShowContext),
        SlashCommand::Tools => Some(BuiltinResult::Text(format_tools_text())),
        SlashCommand::Archives => Some(BuiltinResult::Text(format_archives_text(None))),
        SlashCommand::Recall { chunk_id } => Some(BuiltinResult::RecallArchive(*chunk_id)),
        SlashCommand::BayesShow => Some(BuiltinResult::ShowBayes),
        SlashCommand::BayesReset => Some(BuiltinResult::ResetBayes),
        SlashCommand::BayesExplain => Some(BuiltinResult::ExplainBayes),
        SlashCommand::Goal { text } => {
            if text.trim().is_empty() {
                Some(BuiltinResult::ShowGoal)
            } else {
                Some(BuiltinResult::SetGoal(text.clone()))
            }
        }
        SlashCommand::GoalClear => Some(BuiltinResult::ClearGoal),
        SlashCommand::Visualize => Some(BuiltinResult::ShowVisualizer),
        SlashCommand::Think { mode } => Some(BuiltinResult::SetThinkingMode(mode.clone())),
        SlashCommand::Diagnostics => Some(BuiltinResult::Text(format_diagnostics_text())),
        _ => None, // AI-assisted command
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

/// Format the `/tools` response — a markdown table of every LLM-callable tool
/// drawn live from [`crate::tools::registry::get_available_tool_definitions`].
///
/// This always reflects the current registry state, so newly added tools
/// appear automatically the next time a user types `/tools`.
pub fn format_tools_text() -> String {
    let tool_defs = crate::tools::registry::get_available_tool_definitions();

    // Group tools by the first underscore-separated prefix (e.g. "read_file" → "File")
    // for a more readable presentation.
    let section_label = |name: &str| -> &'static str {
        if name.starts_with("read")
            || name.starts_with("write")
            || name.starts_with("list_dir")
            || name.starts_with("list_code")
            || name.starts_with("glob")
            || name.starts_with("search_file")
            || name.starts_with("replace")
        {
            "📁 File"
        } else if name.starts_with("run_shell") {
            "🐚 Shell"
        } else if name.starts_with("web") {
            "🌐 Web"
        } else if name.starts_with("save_memory") || name.starts_with("recall_context") {
            "🧠 Memory"
        } else if name.starts_with("sleep") || name.starts_with("synthetic") {
            "⚙️  System"
        } else if name.starts_with("task") {
            "📋 Tasks"
        } else if name.starts_with("enter_plan")
            || name.starts_with("exit_plan")
            || name.starts_with("enter_work")
            || name.starts_with("exit_work")
        {
            "🗂️  Plan / Worktree"
        } else if name.starts_with("notebook") {
            "📓 Notebook"
        } else if name.starts_with("execute_skill") || name.starts_with("list_skill") {
            "🎓 Skills"
        } else if name.starts_with("spawn")
            || name.starts_with("send_msg")
            || name.starts_with("team")
        {
            "🤖 Agents"
        } else if name.starts_with("mcp") {
            "🔌 MCP"
        } else if name.starts_with("lsp") {
            "🔍 LSP"
        } else if name.starts_with("tool_search")
            || name.starts_with("cron")
            || name.starts_with("remote")
        {
            "🔎 Discovery"
        } else {
            "🛠️  Other"
        }
    };

    let mut lines: Vec<String> = vec![
        "## Grok CLI — Available Tools".to_string(),
        String::new(),
        format!(
            "**{} tools** are available to the AI during this session.",
            tool_defs.len()
        ),
        String::new(),
    ];

    // Collect rows with their section label so we can sort by section
    let mut rows: Vec<(&'static str, String, String)> = tool_defs
        .iter()
        .filter_map(|v| {
            let func = v.get("function")?;
            let name = func.get("name")?.as_str()?;
            let desc = func
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            Some((section_label(name), name.to_string(), desc.to_string()))
        })
        .collect();

    rows.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(&b.1)));

    let mut current_section = "";
    for (section, name, desc) in &rows {
        if *section != current_section {
            if !current_section.is_empty() {
                lines.push(String::new());
            }
            lines.push(format!("### {}", section));
            lines.push(String::new());
            lines.push("| Tool | Description |".to_string());
            lines.push("|------|-------------|".to_string());
            current_section = section;
        }
        lines.push(format!("| `{}` | {} |", name, desc));
    }

    lines.push(String::new());
    lines.push(
        "> **Tip:** These tools are invoked automatically by the AI. \
         Use `/help` to see slash commands you can type directly."
            .to_string(),
    );

    lines.join("\n")
}

/// Format the `/help` response — a markdown list of all available commands.
pub fn format_help_text() -> String {
    let commands = get_available_commands();

    let mut lines: Vec<String> = vec![
        "## Grok CLI — Slash Commands".to_string(),
        String::new(),
        "Use these commands for quick access to Grok's capabilities:".to_string(),
        String::new(),
    ];

    for cmd in &commands {
        let input_hint = cmd
            .input
            .as_ref()
            .map(|i| {
                // AvailableCommandInput is a non-exhaustive enum from the crate;
                // match on the only current variant to extract the hint.
                if let AvailableCommandInput::Unstructured(u) = i {
                    format!(" `<{}>`", u.hint)
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();
        lines.push(format!(
            "- **`/{}{}`** — {}",
            cmd.name, input_hint, cmd.description
        ));
    }

    lines.push(String::new());
    lines.push(
        "> Commands can be combined with additional context, \
         @-mentioned files, or selected code."
            .to_string(),
    );

    lines.join("\n")
}

/// Format the model list shown when `/model` is called with no argument.
pub fn format_model_list() -> String {
    let models = [
        ("grok-4.3", "Latest flagship (1M context)"),
        ("grok-4.20-0309-reasoning", "Reasoning variant (recommended for thinking)"),
        ("grok-4.20-0309-non-reasoning", "Non-reasoning variant"),
        ("grok-4.20-multi-agent-0309", "Multi-agent variant"),
        ("grok-build-0.1", "Build / experimental model"),
    ];

    let mut lines: Vec<String> = vec![
        "## Available Models".to_string(),
        String::new(),
        "Usage: `/model <name>`".to_string(),
        String::new(),
    ];

    for (name, description) in &models {
        lines.push(format!("- **`{name}`** — {description}"));
    }

    lines.push(String::new());
    lines.push("Example: `/model grok-3` — switches the current session to Grok 3.".to_string());

    lines.join("\n")
}

/// Format the `/diagnostics` report — a comprehensive status of all background systems.
pub fn format_diagnostics_text() -> String {
    let mut lines = vec![
        "## 🩺 Grok CLI — Diagnostics".to_string(),
        String::new(),
        "Background systems status (approximate — live values shown when available):".to_string(),
        String::new(),
    ];

    // Bayesian engine
    lines.push("### 🧠 Bayesian Engine".to_string());
    lines.push("- Tracks user intent via simple Bayesian inference".to_string());
    lines.push("- Used for: vagueness detection, repetition, uncertainty gating".to_string());
    lines.push("- Commands: `/bayes show`, `/bayes explain`, `/bayes reset`".to_string());
    lines.push(String::new());

    // Session DNA
    lines.push("### 🧬 Session DNA".to_string());
    lines.push("- Personality & behavior feedback loop".to_string());
    lines.push("- Adapts tone, verbosity, and risk tolerance from tool results".to_string());
    lines.push("- Injected into system prompt on every turn".to_string());
    lines.push(String::new());

    // Context compression
    lines.push("### 📦 Context Compression".to_string());
    lines.push("- Automatically summarises old messages when context grows large".to_string());
    lines.push("- Archives raw messages to `~/.grok/sessions/<id>/`".to_string());
    lines.push("- Commands: `/archives`, `/recall N`".to_string());
    lines.push(String::new());

    // Knowledge packs
    lines.push("### 📚 Knowledge Packs".to_string());
    lines.push("- Loads `knowledge/*.md` and `knowledge/*.json` from project root".to_string());
    lines.push("- Injected as system context at session start".to_string());
    lines.push(String::new());

    // Hooks
    lines.push("### 🪝 Hook System".to_string());
    lines.push("- `before_tool` / `after_tool` hooks for custom logic".to_string());
    lines.push("- Loaded from project `.grok/hooks/` directory".to_string());
    lines.push(String::new());

    // Session persistence
    lines.push("### 💾 Session Persistence".to_string());
    lines.push("- Sessions saved to `~/.grok/sessions/<id>.json` after each turn".to_string());
    lines.push("- Restored automatically on reconnect (if client re-uses session ID)".to_string());
    lines.push(String::new());

    // Status bar / thinking
    lines.push("### 📊 Status Bar & Thinking".to_string());
    lines.push("- Dynamic status line with model, tokens, thinking mode".to_string());
    lines.push("- Structured thinking blocks emitted when `stream_thinking = true`".to_string());
    lines.push(String::new());

    lines.push("> Run `/help` for the full command list.".to_string());
    lines.join("\n")
}

/// Format the `/archives` listing for a session.
///
/// `session_id` is `None` when called from `handle_builtin` (the ACP caller
/// will supply it); pass `Some(id)` to load live data for a specific session.
pub fn format_archives_text(session_id: Option<&str>) -> String {
    let sid = match session_id {
        Some(s) => s.to_string(),
        None => return "📦 **Context Archives**\n\n_(session ID required to list archives — use `/archives` from within a session)_".to_string(),
    };

    match crate::memory::context_archive::ContextArchive::for_session(&sid) {
        Err(e) => format!("❌ Could not open archive: {}", e),
        Ok(archive) => {
            let chunks = archive.list_chunks();
            if chunks.is_empty() {
                return "📦 **Context Archives**\n\nNo archived chunks yet for this session.\n\
                         Archives are created automatically when the context window fills up."
                    .to_string();
            }

            let mut lines = vec![
                "📦 **Context Archives**".to_string(),
                String::new(),
                format!(
                    "**{}** chunk(s) | **~{}** tokens archived total",
                    chunks.len(),
                    archive.total_tokens_archived()
                ),
                String::new(),
                "| # | Date | Messages | Tokens Saved | Summary |".to_string(),
                "|---|------|----------|-------------|---------|".to_string(),
            ];

            for c in chunks {
                lines.push(format!(
                    "| {} | {} | {} | ~{} | {} |",
                    c.chunk_id,
                    c.created_at.format("%m-%d %H:%M"),
                    c.message_count,
                    c.estimated_tokens_saved,
                    c.summary_preview,
                ));
            }

            lines.push(String::new());
            lines.push("Type `/recall N` to restore a chunk into your active context.".to_string());
            lines.join("\n")
        }
    }
}

/// Build a context summary string for the `/context` command.
/// Accepts individual pieces of session state rather than the full session
/// struct so this module stays decoupled from `GrokAcpAgent`.
pub fn format_context_text(
    session_id: &str,
    model: &str,
    temperature: f32,
    max_tokens: u32,
    message_count: usize,
) -> String {
    format!(
        "## Current Session Context\n\
         \n\
         | Field | Value |\n\
         |-------|-------|\n\
         | Session ID | `{session_id}` |\n\
         | Model | `{model}` |\n\
         | Temperature | `{temperature}` |\n\
         | Max Tokens | `{max_tokens}` |\n\
         | Messages in history | `{message_count}` |\n\
         \n\
         Use `/model <name>` to switch models, or `/clear` to reset the conversation."
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_slash_command ---

    #[test]
    fn test_parse_tools() {
        let cmd = parse_slash_command("/tools");
        assert_eq!(cmd, Some(SlashCommand::Tools));
    }

    #[test]
    fn test_tools_is_builtin() {
        let result = handle_builtin(&SlashCommand::Tools);
        assert!(result.is_some());
        match result.unwrap() {
            BuiltinResult::Text(text) => {
                assert!(text.contains("tools"), "expected tool list in: {text}");
                assert!(text.contains("read_file"), "expected read_file in: {text}");
                assert!(
                    text.contains("web_search"),
                    "expected web_search in: {text}"
                );
            }
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_format_tools_text_covers_all_registry_tools() {
        let text = format_tools_text();
        let all_defs = crate::tools::registry::get_available_tool_definitions();
        // Every registered tool should appear in the formatted output
        for def in &all_defs {
            if let Some(name) = def
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
            {
                assert!(
                    text.contains(name),
                    "tool '{}' missing from /tools output",
                    name
                );
            }
        }
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_slash_command("/help"), Some(SlashCommand::Help));
        assert_eq!(parse_slash_command("  /HELP  "), Some(SlashCommand::Help));
    }

    #[test]
    fn test_parse_web_with_query() {
        let cmd = parse_slash_command("/web rust async runtime");
        assert_eq!(
            cmd,
            Some(SlashCommand::Web {
                query: "rust async runtime".to_string()
            })
        );
    }

    #[test]
    fn test_parse_web_empty() {
        let cmd = parse_slash_command("/web");
        assert_eq!(
            cmd,
            Some(SlashCommand::Web {
                query: String::new()
            })
        );
    }

    #[test]
    fn test_parse_model_with_name() {
        let cmd = parse_slash_command("/model grok-3");
        assert_eq!(
            cmd,
            Some(SlashCommand::Model {
                name: "grok-3".to_string()
            })
        );
    }

    #[test]
    fn test_parse_clear() {
        assert_eq!(parse_slash_command("/clear"), Some(SlashCommand::Clear));
    }

    #[test]
    fn test_parse_regular_message() {
        assert_eq!(parse_slash_command("hello world"), None);
        assert_eq!(parse_slash_command("fix the bug"), None);
    }

    #[test]
    fn test_parse_unknown_command() {
        // Unknown commands fall through to the AI unchanged
        assert_eq!(parse_slash_command("/unknown-command"), None);
    }

    // --- command_to_prompt ---

    #[test]
    fn test_help_is_builtin() {
        assert!(command_to_prompt(&SlashCommand::Help).is_none());
    }

    #[test]
    fn test_clear_is_builtin() {
        assert!(command_to_prompt(&SlashCommand::Clear).is_none());
    }

    #[test]
    fn test_web_produces_prompt() {
        let prompt = command_to_prompt(&SlashCommand::Web {
            query: "tokio vs async-std".to_string(),
        });
        assert!(prompt.is_some());
        let text = prompt.unwrap();
        assert!(text.contains("tokio vs async-std"));
        assert!(text.contains("research"));
    }

    #[test]
    fn test_review_produces_prompt() {
        let prompt = command_to_prompt(&SlashCommand::Review {
            description: String::new(),
        });
        assert!(prompt.is_some());
        let text = prompt.unwrap();
        assert!(text.to_lowercase().contains("review"));
    }

    #[test]
    fn test_plan_produces_prompt() {
        let prompt = command_to_prompt(&SlashCommand::Plan {
            description: "add OAuth2 login".to_string(),
        });
        assert!(prompt.is_some());
        let text = prompt.unwrap();
        assert!(text.contains("add OAuth2 login"));
        assert!(text.contains("plan"));
    }

    // --- get_available_commands ---

    #[test]
    fn test_available_commands_not_empty() {
        let cmds = get_available_commands();
        assert!(!cmds.is_empty());
    }

    #[test]
    fn test_available_commands_have_required_fields() {
        for cmd in get_available_commands() {
            assert!(!cmd.name.is_empty(), "command name must not be empty");
            assert!(
                !cmd.description.is_empty(),
                "command description must not be empty"
            );
        }
    }

    #[test]
    fn test_all_commands_are_advertised() {
        let names: Vec<String> = get_available_commands()
            .into_iter()
            .map(|c| c.name)
            .collect();
        for required in &[
            "help",
            "web",
            "explain",
            "review",
            "plan",
            "test",
            "fix",
            "clear",
            "bayes",
            "archives",
            "recall",
            "goal",
            "visualize",
            "think",
        ] {
            assert!(
                names.contains(&required.to_string()),
                "command '/{required}' missing from advertised list"
            );
        }
    }

    // ── Bayes slash commands ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_bayes_show() {
        assert_eq!(
            parse_slash_command("/bayes show"),
            Some(SlashCommand::BayesShow)
        );
        assert_eq!(parse_slash_command("/bayes"), Some(SlashCommand::BayesShow));
        assert_eq!(parse_slash_command("/BAYES"), Some(SlashCommand::BayesShow));
    }

    #[test]
    fn test_parse_bayes_reset() {
        assert_eq!(
            parse_slash_command("/bayes reset"),
            Some(SlashCommand::BayesReset)
        );
    }

    #[test]
    fn test_parse_bayes_explain() {
        assert_eq!(
            parse_slash_command("/bayes explain"),
            Some(SlashCommand::BayesExplain)
        );
    }

    #[test]
    fn test_parse_bayes_unknown_subcommand_returns_none() {
        // Unknown /bayes sub-commands should fall through to the AI.
        assert_eq!(parse_slash_command("/bayes foobar"), None);
    }

    #[test]
    fn test_bayes_show_is_builtin() {
        let result = handle_builtin(&SlashCommand::BayesShow);
        assert!(matches!(result, Some(BuiltinResult::ShowBayes)));
    }

    #[test]
    fn test_bayes_reset_is_builtin() {
        let result = handle_builtin(&SlashCommand::BayesReset);
        assert!(matches!(result, Some(BuiltinResult::ResetBayes)));
    }

    #[test]
    fn test_bayes_explain_is_builtin() {
        let result = handle_builtin(&SlashCommand::BayesExplain);
        assert!(matches!(result, Some(BuiltinResult::ExplainBayes)));
    }

    #[test]
    fn test_bayes_commands_no_ai_prompt() {
        // All bayes variants must be handled without an AI round-trip.
        assert!(command_to_prompt(&SlashCommand::BayesShow).is_none());
        assert!(command_to_prompt(&SlashCommand::BayesReset).is_none());
        assert!(command_to_prompt(&SlashCommand::BayesExplain).is_none());
    }

    // --- format helpers ---

    #[test]
    fn test_format_help_text_contains_commands() {
        let help = format_help_text();
        assert!(help.contains("/help"));
        assert!(help.contains("/web"));
        assert!(help.contains("/plan"));
        assert!(help.contains("/clear"));
    }

    #[test]
    fn test_format_context_text() {
        let ctx = format_context_text("sess_123", "grok-3", 0.7, 4096, 10);
        assert!(ctx.contains("sess_123"));
        assert!(ctx.contains("grok-3"));
        assert!(ctx.contains("10"));
    }

    // --- Goal slash commands (Task 106) ---

    #[test]
    fn test_parse_goal_with_text() {
        let cmd = parse_slash_command("/goal refactor auth");
        assert_eq!(
            cmd,
            Some(SlashCommand::Goal {
                text: "refactor auth".to_string()
            })
        );
    }

    #[test]
    fn test_parse_goal_clear() {
        let cmd = parse_slash_command("/goal clear");
        assert_eq!(cmd, Some(SlashCommand::GoalClear));
    }

    #[test]
    fn test_parse_goal_clear_case_insensitive() {
        assert_eq!(
            parse_slash_command("/goal CLEAR"),
            Some(SlashCommand::GoalClear)
        );
        assert_eq!(
            parse_slash_command("/goal Clear"),
            Some(SlashCommand::GoalClear)
        );
    }

    #[test]
    fn test_parse_goal_empty() {
        // Empty /goal (no argument) parses to Goal { text: "" }
        let cmd = parse_slash_command("/goal");
        assert_eq!(
            cmd,
            Some(SlashCommand::Goal {
                text: String::new()
            })
        );
    }

    #[test]
    fn test_parse_goal_empty_routes_to_show_goal() {
        // handle_builtin turns an empty text into ShowGoal
        let cmd = SlashCommand::Goal {
            text: String::new(),
        };
        let result = handle_builtin(&cmd);
        assert!(matches!(result, Some(BuiltinResult::ShowGoal)));
    }

    #[test]
    fn test_goal_is_builtin() {
        let cmd = SlashCommand::Goal {
            text: "improve test coverage".to_string(),
        };
        let result = handle_builtin(&cmd);
        assert!(matches!(result, Some(BuiltinResult::SetGoal(_))));
        // Must not produce an AI prompt
        assert!(command_to_prompt(&cmd).is_none());
    }

    #[test]
    fn test_goal_clear_is_builtin() {
        let result = handle_builtin(&SlashCommand::GoalClear);
        assert!(matches!(result, Some(BuiltinResult::ClearGoal)));
        // Must not produce an AI prompt
        assert!(command_to_prompt(&SlashCommand::GoalClear).is_none());
    }

    #[test]
    fn test_goal_set_result_contains_text() {
        let goal = "refactor the auth module for safety".to_string();
        let cmd = SlashCommand::Goal { text: goal.clone() };
        match handle_builtin(&cmd) {
            Some(BuiltinResult::SetGoal(text)) => assert_eq!(text, goal),
            other => panic!("expected SetGoal, got {:?}", other),
        }
    }

    // ── Task 110: /think slash command tests ────────────────────────────────────────

    #[test]
    fn test_parse_think_no_arg_shows_current_mode() {
        let result = parse_slash_command("/think");
        assert!(matches!(result, Some(SlashCommand::Think { mode: None })));
    }

    #[test]
    fn test_parse_think_off() {
        let result = parse_slash_command("/think off");
        assert!(matches!(
            result,
            Some(SlashCommand::Think {
                mode: Some(crate::config::ThinkingMode::Off)
            })
        ));
    }

    #[test]
    fn test_parse_think_low() {
        let result = parse_slash_command("/think low");
        assert!(matches!(
            result,
            Some(SlashCommand::Think {
                mode: Some(crate::config::ThinkingMode::Low)
            })
        ));
    }

    #[test]
    fn test_parse_think_high() {
        let result = parse_slash_command("/think high");
        assert!(matches!(
            result,
            Some(SlashCommand::Think {
                mode: Some(crate::config::ThinkingMode::High)
            })
        ));
    }

    #[test]
    fn test_parse_think_case_insensitive() {
        assert!(matches!(
            parse_slash_command("/think HIGH"),
            Some(SlashCommand::Think {
                mode: Some(crate::config::ThinkingMode::High)
            })
        ));
    }

    #[test]
    fn test_parse_think_unknown_arg_returns_none() {
        // Unknown argument — fall through to AI
        assert!(parse_slash_command("/think ultra").is_none());
    }

    #[test]
    fn test_think_is_builtin() {
        let cmd = SlashCommand::Think {
            mode: Some(crate::config::ThinkingMode::High),
        };
        let result = handle_builtin(&cmd);
        assert!(matches!(
            result,
            Some(BuiltinResult::SetThinkingMode(Some(_)))
        ));
        // Must not produce an AI prompt
        assert!(command_to_prompt(&cmd).is_none());
    }

    #[test]
    fn test_thinking_mode_serialises_correctly() {
        use crate::config::ThinkingMode;
        assert_eq!(ThinkingMode::Off.as_api_str(), None);
        assert_eq!(ThinkingMode::Low.as_api_str(), Some("low"));
        assert_eq!(ThinkingMode::High.as_api_str(), Some("high"));
    }

    #[test]
    fn test_thinking_mode_from_str_ci() {
        use crate::config::ThinkingMode;
        assert_eq!(ThinkingMode::from_str_ci("off"), Some(ThinkingMode::Off));
        assert_eq!(ThinkingMode::from_str_ci("none"), Some(ThinkingMode::Off));
        assert_eq!(ThinkingMode::from_str_ci("LOW"), Some(ThinkingMode::Low));
        assert_eq!(ThinkingMode::from_str_ci("High"), Some(ThinkingMode::High));
        assert_eq!(ThinkingMode::from_str_ci("ultra"), None);
    }

    // --- AvailableCommandInput wire-format verification ---

    #[test]
    fn test_available_command_input_serializes_to_hint_object() {
        use crate::acp::protocol::{AvailableCommandInput, UnstructuredCommandInput};
        let input = AvailableCommandInput::Unstructured(UnstructuredCommandInput::new("my hint"));
        let json = serde_json::to_string(&input).expect("serialization failed");
        // Must produce {"hint":"my hint"} — NOT {"Unstructured":{"hint":"my hint"}}
        assert_eq!(
            json, r#"{"hint":"my hint"}"#,
            "AvailableCommandInput wire format changed — check serde tag"
        );
    }

    #[test]
    fn test_available_command_round_trips_with_input() {
        use crate::acp::protocol::{
            AvailableCommand, AvailableCommandInput, UnstructuredCommandInput,
        };
        let cmd = AvailableCommand::new("web", "Search the web").input(
            AvailableCommandInput::Unstructured(UnstructuredCommandInput::new("query")),
        );
        let json = serde_json::to_value(&cmd).expect("serialization failed");
        assert_eq!(json["name"], "web");
        assert_eq!(json["input"]["hint"], "query");
    }
}
