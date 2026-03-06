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

use super::protocol::{AvailableCommand, AvailableCommandInput};

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

    /// `/model <name>` — switch the active model for this session.
    Model { name: String },

    /// `/clear` — wipe the current conversation history.
    Clear,

    /// `/context` — show session configuration and active context files.
    Context,
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
        _ => None, // unknown command — let the AI handle the raw text
    }
}

// ---------------------------------------------------------------------------
// ACP advertisement
// ---------------------------------------------------------------------------

/// Build the list of [`AvailableCommand`] entries sent to ACP clients via the
/// `available_commands_update` notification immediately after session creation.
pub fn get_available_commands() -> Vec<AvailableCommand> {
    vec![
        AvailableCommand::new("help", "Show all available slash commands and their usage"),
        AvailableCommand::new("web", "Research a topic or search the web for information")
            .with_input("query to research"),
        AvailableCommand::new(
            "explain",
            "Get a thorough explanation of code, a file, or a concept",
        )
        .with_input("code, file path, or concept to explain"),
        AvailableCommand::new(
            "review",
            "Comprehensive code review: bugs, security, performance, style",
        )
        .with_input("code or file path to review"),
        AvailableCommand::new("plan", "Create a detailed step-by-step implementation plan")
            .with_input("description of what to plan"),
        AvailableCommand::new("test", "Help write, run, or debug tests")
            .with_input("test description or file path (optional)"),
        AvailableCommand::new("fix", "Diagnose and fix a bug or error")
            .with_input("problem description, error message, or file path"),
        AvailableCommand::new("model", "Switch to a different Grok model for this session")
            .with_input("model name (e.g. grok-3, grok-4-0709, grok-3-mini)"),
        AvailableCommand::new("clear", "Clear the current conversation history"),
        AvailableCommand::new(
            "context",
            "Show current session configuration and active context",
        ),
    ]
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
        | SlashCommand::Context => None,

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
    /// Display session context/config info — the caller supplies the text.
    ShowContext,
}

/// Handle a built-in slash command, returning `Some(BuiltinResult)` if the
/// command is handled here, or `None` if it should go to the AI.
pub fn handle_builtin(cmd: &SlashCommand) -> Option<BuiltinResult> {
    match cmd {
        SlashCommand::Help => Some(BuiltinResult::Text(format_help_text())),
        SlashCommand::Clear => Some(BuiltinResult::ClearHistory),
        SlashCommand::Model { name } => {
            if name.trim().is_empty() {
                Some(BuiltinResult::Text(format_model_list()))
            } else {
                Some(BuiltinResult::SwitchModel(name.trim().to_string()))
            }
        }
        SlashCommand::Context => Some(BuiltinResult::ShowContext),
        _ => None, // AI-assisted command
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

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
            .map(|i| format!(" `<{}>`", i.hint))
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
        ("grok-4-1-fast-reasoning", "Default — fast + reasoning"),
        ("grok-4-1-fast-non-reasoning", "Fast, no reasoning overhead"),
        ("grok-code-fast-1", "Optimised for code tasks"),
        ("grok-4-fast-reasoning", "Grok 4 with reasoning"),
        ("grok-4-fast-non-reasoning", "Grok 4 standard"),
        ("grok-4-0709", "Grok 4 (July 2025 checkpoint)"),
        ("grok-3", "Grok 3 — large context"),
        ("grok-3-mini", "Grok 3 Mini — lightweight"),
        ("grok-2-vision-1212", "Grok 2 with vision"),
        ("grok-2", "Grok 2 — fallback"),
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
            "help", "web", "explain", "review", "plan", "test", "fix", "clear",
        ] {
            assert!(
                names.contains(&required.to_string()),
                "command '/{required}' missing from advertised list"
            );
        }
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
}
