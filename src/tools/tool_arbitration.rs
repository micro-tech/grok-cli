use anyhow::Result;
use serde_json::Value;

/// What the arbiter decided to do with the tool call.
pub enum ArbitrationDecision {
    /// Tool call is valid; proceed with execution.
    Execute { name: String, args: Value },
    /// Tool call is invalid; return a user-facing message instead of executing.
    Reject { message: String },
    /// Tool call is incomplete; ask the LLM/user for more info.
    NeedMoreInfo {
        message: String,
        missing_fields: Vec<String>,
    },
}

/// High-level arbitration entry point.
/// - Validates tool name
/// - Validates required arguments
/// - Optionally normalizes / corrects args
pub fn arbitrate_tool_call(name: &str, args: &Value) -> Result<ArbitrationDecision> {
    // 1) Validate tool name against the known set.
    if !is_known_tool(name) {
        return Ok(ArbitrationDecision::Reject {
            message: format!(
                "I don't have a tool named `{}`. Use /tools or /help to see available tools.",
                name
            ),
        });
    }

    // 2) Tool-specific argument validation.
    let missing = missing_required_fields(name, args);

    if !missing.is_empty() {
        return Ok(ArbitrationDecision::NeedMoreInfo {
            message: format!(
                "The `{}` tool is missing required fields: {}. \
                 Please provide these and try again.",
                name,
                missing.join(", ")
            ),
            missing_fields: missing,
        });
    }

    // 3) (Optional) Argument normalization / correction hook.
    let normalized_args = normalize_args(name, args)?;

    Ok(ArbitrationDecision::Execute {
        name: name.to_string(),
        args: normalized_args,
    })
}

/// Minimal known-tool check. Keep in sync with get_tool_definitions / execute_tool.
fn is_known_tool(name: &str) -> bool {
    matches!(
        name,
        "read_file"
            | "read_multiple_files"
            | "list_code_definitions"
            | "write_file"
            | "replace"
            | "list_directory"
            | "glob_search"
            | "search_file_content"
            | "run_shell_command"
            | "web_search"
            | "web_fetch"
            | "save_memory"
            | "sleep"
            | "synthetic_output"
            | "task_get"
            | "task_create"
            | "task_update"
            | "execute_task_graph"
            | "enter_plan_mode"
            | "exit_plan_mode"
            | "enter_worktree"
            | "exit_worktree"
            | "notebook_edit"
            | "execute_skill"
            | "list_skills"
            | "spawn_agent"
            | "send_message"
            | "team_create"
            | "team_delete"
            | "list_agents"
            | "get_agent_status"
            | "cancel_agent"
            | "send_message_in_memory"
            | "receive_messages"
            | "fork_agent"
            | "join_agents"
            | "mcp_call"
            | "lsp_query"
            | "tool_search"
            | "cron_create"
            | "remote_trigger"
            | "recall_context"
            | "ai_tool"
    )
}

/// Return a list of missing required fields for a given tool.
fn missing_required_fields(name: &str, args: &Value) -> Vec<String> {
    let mut missing = Vec::new();

    let require = |field: &str, args: &Value, missing: &mut Vec<String>| {
        if args.get(field).is_none() || args[field].is_null() {
            missing.push(field.to_string());
        }
    };

    match name {
        "read_file" => {
            require("path", args, &mut missing);
        }
        "read_multiple_files" => {
            require("paths", args, &mut missing);
        }
        "list_code_definitions" => {
            require("path", args, &mut missing);
        }
        "write_file" => {
            require("path", args, &mut missing);
            require("content", args, &mut missing);
        }
        "replace" => {
            require("path", args, &mut missing);
            require("old_string", args, &mut missing);
            require("new_string", args, &mut missing);
        }
        "list_directory" => {
            require("path", args, &mut missing);
        }
        "glob_search" => {
            require("pattern", args, &mut missing);
        }
        "search_file_content" => {
            require("path", args, &mut missing);
            require("pattern", args, &mut missing);
        }
        "run_shell_command" => {
            require("command", args, &mut missing);
        }
        "web_search" => {
            require("query", args, &mut missing);
        }
        "web_fetch" => {
            require("url", args, &mut missing);
        }
        "save_memory" => {
            require("fact", args, &mut missing);
        }
        "sleep" => {
            require("seconds", args, &mut missing);
        }
        "synthetic_output" => {
            require("schema_name", args, &mut missing);
            require("data", args, &mut missing);
        }
        "task_get" => {
            require("id", args, &mut missing);
        }
        "task_create" => {
            require("title", args, &mut missing);
        }
        "task_update" => {
            require("id", args, &mut missing);
        }
        "execute_task_graph" => {
            require("graph", args, &mut missing);
        }
        "enter_worktree" => {
            require("branch", args, &mut missing);
            require("path", args, &mut missing);
        }
        "notebook_edit" => {
            require("path", args, &mut missing);
            require("source", args, &mut missing);
        }
        "execute_skill" => {
            require("skill_name", args, &mut missing);
        }
        "spawn_agent" => {
            require("task", args, &mut missing);
        }
        "send_message" => {
            require("target", args, &mut missing);
            require("message", args, &mut missing);
        }
        "team_create" => {
            require("name", args, &mut missing);
        }
        "team_delete" => {
            require("name", args, &mut missing);
        }
        "mcp_call" => {
            require("server_command", args, &mut missing);
            require("tool_name", args, &mut missing);
        }
        "lsp_query" => {
            require("file", args, &mut missing);
        }
        "tool_search" => {
            require("query", args, &mut missing);
        }
        "cron_create" => {
            require("name", args, &mut missing);
            require("schedule", args, &mut missing);
            require("task", args, &mut missing);
        }
        "remote_trigger" => {
            require("endpoint", args, &mut missing);
        }
        "recall_context" => {
            require("chunk_id", args, &mut missing);
        }
        _ => {}
    }

    missing
}

/// Hook for argument normalization / correction.
/// Right now it's a no-op; you can grow this over time.
fn normalize_args(_name: &str, args: &Value) -> Result<Value> {
    // Example: coerce numeric strings, trim whitespace, etc.
    // For now, just clone.
    Ok(args.clone())
}
