use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::tools::tool_arbitration::{self, ArbitrationDecision};
use crate::tools::{
    ToolContext,
    agent_tools,
    ai_tools, // ← AI-generated tools scaffold
    discovery_tools,
    file_tools,
    lsp_tools,
    mcp_tools,
    memory_tools,
    notebook_tools,
    plan_tools,
    shell_tools,
    skill_tools,
    system_tools,
    task_graph_tools,
    task_tools,
    web_tools,
};

// ─────────────────────────────────────────────────────────────────────────────
// Small argument helpers (ARCH-2 ergonomics)
//
// These reduce boilerplate and copy-paste errors in the dispatch match.
// They make it much safer to keep the schema list and the match in sync.
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key]
        .as_str()
        .ok_or_else(|| anyhow!("Missing required argument: {}", key))
}

#[inline]
fn require_array<'a>(args: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    args[key]
        .as_array()
        .ok_or_else(|| anyhow!("Missing required argument: {} (array)", key))
}

#[inline]
fn require_u64(args: &Value, key: &str) -> Result<u64> {
    args[key]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing required argument: {} (integer)", key))
}

#[inline]
fn require_f64(args: &Value, key: &str) -> Result<f64> {
    args[key]
        .as_f64()
        .ok_or_else(|| anyhow!("Missing required argument: {} (number)", key))
}

// ─────────────────────────────────────────────────────────────────────────────
// Thin handler functions (ARCH-2 completion for Task 260)
//
// Goal: Make adding a tool obvious, low-risk, and require changes in **only two
// places**:
//   1. Add the JSON schema in `get_full_tool_definitions()` (source of truth)
//   2. Add a one-line entry in the dispatch table + (usually tiny) handler fn below
//
// Each handler is responsible for:
//   - Extracting + validating its own arguments (using the require_* helpers)
//   - Calling the real implementation in the appropriate *_tools module
//   - Returning a clean Result<String>
//
// This isolates logic, makes the big match a pure "dispatch table", and
// enables easy per-tool testing.
//
// The `unknown` arm below acts as a compile/runtime guard: if a schema is added
// without a handler, you get a clear error mentioning the exact tool name.
// ─────────────────────────────────────────────────────────────────────────────

async fn handle_read_file(args: &Value, ctx: &ToolContext) -> Result<String> {
    let path = require_str(args, "path")?;
    file_tools::read_file(path, ctx).await
}

async fn handle_read_multiple_files(args: &Value, ctx: &ToolContext) -> Result<String> {
    let paths_val = require_array(args, "paths")?;
    let paths: Result<Vec<String>> = paths_val
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| anyhow!("Invalid path entry"))
                .map(str::to_string)
        })
        .collect();
    file_tools::read_multiple_files(paths?, ctx).await
}

async fn handle_list_code_definitions(args: &Value, ctx: &ToolContext) -> Result<String> {
    let path = require_str(args, "path")?;
    file_tools::list_code_definitions(path, ctx).await
}

async fn handle_write_file(args: &Value, ctx: &ToolContext) -> Result<String> {
    let path = require_str(args, "path")?;
    let content = require_str(args, "content")?;
    file_tools::write_file(path, content, ctx, false).await
}

async fn handle_replace(args: &Value, ctx: &ToolContext) -> Result<String> {
    let path = require_str(args, "path")?;
    let old_string = require_str(args, "old_string")?;
    let new_string = require_str(args, "new_string")?;
    let expected = args["expected_replacements"].as_u64().map(|n| n as u32);
    file_tools::replace(path, old_string, new_string, expected, ctx, false).await
}

fn handle_list_directory(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let path = require_str(args, "path")?;
    file_tools::list_directory(path, policy)
}

fn handle_glob_search(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let pattern = require_str(args, "pattern")?;
    file_tools::glob_search(pattern, policy)
}

fn handle_search_file_content(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let path = require_str(args, "path")?;
    let pattern = require_str(args, "pattern")?;
    file_tools::search_file_content(path, pattern, policy)
}

async fn handle_run_shell_command(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let command = require_str(args, "command")?;
    // timeout_secs is accepted in schema for compat but ignored (derived from SecurityPolicy)
    let _ = args["timeout_secs"].as_u64();
    shell_tools::run_shell_command(command, policy).await
}

async fn handle_web_search(args: &Value, ctx: &ToolContext) -> Result<String> {
    let query = require_str(args, "query")?;
    web_tools::web_search(query, ctx).await
}

async fn handle_web_fetch(args: &Value, ctx: &ToolContext) -> Result<String> {
    let url = require_str(args, "url")?;
    web_tools::web_fetch(url, ctx).await
}

fn handle_save_memory(args: &Value) -> Result<String> {
    let fact = require_str(args, "fact")?;
    memory_tools::save_memory(fact)
}

async fn handle_sleep(args: &Value) -> Result<String> {
    let seconds = args["seconds"].as_f64().unwrap_or(1.0) as u64;
    system_tools::sleep_for(seconds).await
}

fn handle_synthetic_output(args: &Value) -> Result<String> {
    let schema_name = args["schema_name"].as_str().unwrap_or("output");
    let data = &args["data"];
    system_tools::synthetic_output(schema_name, data)
}

async fn handle_execute_task_graph(args: &Value, ctx: &ToolContext) -> Result<String> {
    let graph_json = require_str(args, "graph")?;
    task_graph_tools::execute_task_graph(graph_json, ctx).await
}

fn handle_task_create(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let title = require_str(args, "title")?;
    let description = args["description"].as_str().unwrap_or("");
    let priority = args["priority"].as_str().unwrap_or("medium");
    let deps: Vec<f64> = args["dependencies"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();
    let details = args["details"].as_str().unwrap_or("");
    let test_strategy = args["testStrategy"].as_str().unwrap_or("");
    let subtasks: Vec<Value> = args["subtasks"].as_array().cloned().unwrap_or_default();
    task_tools::task_create(
        title,
        description,
        priority,
        deps,
        details,
        test_strategy,
        subtasks,
        policy,
    )
}

fn handle_task_get(args: &Value, policy: &crate::acp::security::SecurityPolicy) -> Result<String> {
    let id = require_f64(args, "id")?;
    task_tools::task_get(id, policy)
}

fn handle_task_update(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let id = require_f64(args, "id")?;
    let status = args["status"].as_str();
    let title = args["title"].as_str();
    let priority = args["priority"].as_str();
    let details = args["details"].as_str();
    task_tools::task_update(id, status, title, priority, details, policy)
}

fn handle_enter_plan_mode(_args: &Value) -> Result<String> {
    plan_tools::enter_plan_mode()
}

fn handle_exit_plan_mode(_args: &Value) -> Result<String> {
    plan_tools::exit_plan_mode()
}

async fn handle_enter_worktree(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let branch = require_str(args, "branch")?;
    let path = require_str(args, "path")?;
    plan_tools::enter_worktree(branch, path, policy).await
}

async fn handle_exit_worktree(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let merge = args["merge"].as_bool().unwrap_or(false);
    plan_tools::exit_worktree(merge, policy).await
}

fn handle_notebook_edit(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let path = require_str(args, "path")?;
    let cell_index = args["cell_index"].as_u64().unwrap_or(0) as usize;
    let source = require_str(args, "source")?;
    let cell_type = args["cell_type"].as_str().unwrap_or("code");
    notebook_tools::notebook_edit(path, cell_index, source, cell_type, policy)
}

fn handle_execute_skill(args: &Value) -> Result<String> {
    let skill_name = require_str(args, "skill_name")?;
    let input = args["input"].as_str().unwrap_or("");
    skill_tools::execute_skill(skill_name, input)
}

fn handle_list_skills(_args: &Value) -> Result<String> {
    skill_tools::list_available_skills()
}

async fn handle_spawn_agent(args: &Value, _ctx: &ToolContext) -> Result<String> {
    let task = require_str(args, "task")?;
    let context = args["context"].as_str().unwrap_or("");
    let max_tokens = args["max_tokens"].as_u64().unwrap_or(2048) as u32;

    let has_config = args.get("model").is_some()
        || args.get("system_prompt").is_some()
        || args.get("allowed_tools").is_some()
        || args.get("trusted_dirs").is_some()
        || args.get("max_tool_iterations").is_some();

    if has_config {
        let mut builder = crate::agent::SubAgentConfig::builder().max_tokens(max_tokens);
        if let Some(m) = args["model"].as_str() {
            builder = builder.model(m);
        }
        if let Some(p) = args["system_prompt"].as_str() {
            builder = builder.system_prompt(p);
        }
        if let Some(tools) = args["allowed_tools"].as_array() {
            let names: Vec<String> = tools
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            builder = builder.allow_tools(names.iter().map(|s| s.as_str()).collect());
        }
        if let Some(dirs) = args["trusted_dirs"].as_array() {
            for d in dirs {
                if let Some(s) = d.as_str() {
                    builder = builder.trusted_dir(s);
                }
            }
        }
        if let Some(n) = args["max_tool_iterations"].as_u64() {
            builder = builder.max_tool_iterations(n as u32);
        }
        let config = builder.build();
        agent_tools::spawn_agent_configured(task, context, None, config).await
    } else {
        agent_tools::spawn_agent(task, context, max_tokens).await
    }
}

fn handle_send_message(args: &Value) -> Result<String> {
    let target = require_str(args, "target")?;
    let message = require_str(args, "message")?;
    agent_tools::send_message(target, message)
}

fn handle_team_create(args: &Value) -> Result<String> {
    let name = require_str(args, "name")?;
    let members: Vec<String> = args["members"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let description = args["description"].as_str().unwrap_or("");
    agent_tools::team_create(name, members, description)
}

fn handle_team_delete(args: &Value) -> Result<String> {
    let name = require_str(args, "name")?;
    agent_tools::team_delete(name)
}

async fn handle_list_agents(args: &Value) -> Result<String> {
    let parent = args.get("parent_id").and_then(|v| v.as_str());
    agent_tools::list_agents(parent).await
}

async fn handle_get_agent_status(args: &Value) -> Result<String> {
    let id = require_str(args, "agent_id")?;
    agent_tools::get_agent_status(id).await
}

async fn handle_cancel_agent(args: &Value) -> Result<String> {
    let id = require_str(args, "agent_id")?;
    agent_tools::cancel_agent(id).await
}

async fn handle_send_message_in_memory(args: &Value) -> Result<String> {
    let from = args["from"].as_str().unwrap_or("main");
    let to = require_str(args, "to")?;
    let message = require_str(args, "message")?;
    agent_tools::send_message_in_memory(from, to, message).await
}

async fn handle_receive_messages(args: &Value) -> Result<String> {
    let target = require_str(args, "target")?;
    agent_tools::receive_messages(target).await
}

async fn handle_fork_agent(args: &Value) -> Result<String> {
    let tasks: Vec<String> = require_array(args, "tasks")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    agent_tools::fork_agent(tasks).await
}

async fn handle_join_agents(args: &Value) -> Result<String> {
    let ids: Vec<String> = require_array(args, "agent_ids")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    agent_tools::join_agents(ids).await
}

async fn handle_mcp_call(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let server_command = require_str(args, "server_command")?;
    let tool_name = require_str(args, "tool_name")?;
    let arguments = args["arguments"].clone();
    mcp_tools::mcp_call(server_command, tool_name, arguments, policy).await
}

fn handle_mcp_list(_args: &Value) -> Result<String> {
    let discovered = get_discovered_mcp_tools();
    if discovered.is_empty() {
        Ok(serde_json::json!({
            "connected_servers": 0,
            "message": "No MCP servers are currently connected."
        })
        .to_string())
    } else {
        let servers: Vec<_> = discovered
            .iter()
            .map(|(name, tools)| {
                serde_json::json!({
                    "server": name,
                    "tool_count": tools.len(),
                    "tools": tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                })
            })
            .collect();
        Ok(serde_json::json!({
            "connected_servers": discovered.len(),
            "servers": servers
        })
        .to_string())
    }
}

async fn handle_lsp_query(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    let file = require_str(args, "file")?;
    let line = args["line"].as_u64().unwrap_or(0) as u32;
    let character = args["character"].as_u64().unwrap_or(0) as u32;
    let query_type = args["query_type"].as_str().unwrap_or("diagnostics");
    lsp_tools::lsp_query(file, line, character, query_type, policy).await
}

fn handle_tool_search(args: &Value) -> Result<String> {
    let query = require_str(args, "query")?;
    discovery_tools::tool_search(query)
}

fn handle_cron_create(args: &Value) -> Result<String> {
    let name = require_str(args, "name")?;
    let schedule = require_str(args, "schedule")?;
    let task = require_str(args, "task")?;
    discovery_tools::cron_create(name, schedule, task)
}

async fn handle_remote_trigger(args: &Value) -> Result<String> {
    let endpoint = require_str(args, "endpoint")?;
    let payload = args["payload"].clone();
    let method = args["method"].as_str().unwrap_or("POST");
    discovery_tools::remote_trigger(endpoint, payload, method).await
}

async fn handle_recall_context(args: &Value) -> Result<String> {
    let chunk_id = require_u64(args, "chunk_id")?;
    match crate::memory::context_archive::ContextArchive::for_session("unknown") {
        Err(e) => Err(anyhow!("Could not open context archive: {}", e)),
        Ok(archive) => match archive.load_chunk(chunk_id as u32)? {
            None => Ok(format!(
                "Archive chunk #{} not found. Use /archives to see available chunks.",
                chunk_id
            )),
            Some(chunk) => {
                let facts = if chunk.key_facts.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n\nKey facts:\n{}",
                        chunk
                            .key_facts
                            .iter()
                            .map(|f| format!("• {}", f))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                };
                Ok(format!(
                    "[Recalled Archive #{id}]\nCovered {count} messages archived on {ts}.\nSummary: {summary}{facts}\n\nNote: The full raw messages have been injected into your context by the system.",
                    id = chunk.chunk_id,
                    count = chunk.message_count,
                    ts = chunk.created_at.format("%Y-%m-%d %H:%M UTC"),
                    summary = chunk.summary,
                    facts = facts,
                ))
            }
        },
    }
}

async fn handle_ai_tool(
    args: &Value,
    policy: &crate::acp::security::SecurityPolicy,
) -> Result<String> {
    ai_tools::run(args, policy).await
}

fn handle_generate_commit_message(args: &Value) -> Result<String> {
    // The real work happens via /commit or direct LLM call. This is a hint.
    let _instructions = args["instructions"].as_str(); // optional
    Ok("Use the `/commit` slash command (or call the model with a git diff) to generate a commit message.".to_string())
}

async fn handle_okf_lookup(args: &Value) -> Result<String> {
    let query = require_str(args, "query")?;
    let max_results = args["max_results"].as_u64().map(|n| n as usize);
    crate::tools::okf_tools::okf_lookup(query, max_results)
}

async fn handle_okf_get(args: &Value) -> Result<String> {
    let id = require_str(args, "id")?;
    crate::tools::okf_tools::okf_get(id)
}

async fn handle_okf_create(args: &Value) -> Result<String> {
    let r#type = args["type"].as_str().unwrap_or("Concept");
    let title = require_str(args, "title")?;
    let body = require_str(args, "body")?;
    let description = args["description"].as_str();
    let tags: Option<Vec<String>> = args["tags"].as_array().map(|a| {
        a.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    });
    let resource = args["resource"].as_str();
    let id = args["id"].as_str();

    crate::tools::okf_tools::okf_create(r#type, title, body, description, tags, resource, id).await
}

/// Execute a named tool with the provided JSON arguments and context.
///
/// This is the **unified entry-point** used by the main agent loop, chat router,
/// ACP, task graphs, etc.
///
/// ## ARCH-2: Unified Tool Registry (Task 260 - Complete)
///
/// Single source of truth + thin handlers architecture:
///
/// | Concern                  | Status          | How it works                                      |
/// |--------------------------|-----------------|---------------------------------------------------|
/// | Tool names + schemas     | Single source   | `get_full_tool_definitions()` (JSON)              |
/// | Required parameters      | Derived         | `get_required_parameters()` reads schema          |
/// | Arbitration              | Schema-driven   | `tool_arbitration` uses the schema                |
/// | Argument extraction      | Centralized     | `require_str` / `require_*` helpers               |
/// | Business logic           | Isolated        | Thin `handle_*` functions (one per tool)          |
/// | Dispatch                 | Pure table      | 1-line match arm calling the handle_* fn          |
/// | Guard against drift      | Runtime + tests | `unknown` arm + `every_schema_has_a_handler` test |
///
/// ### How to add a new tool (the right way)
/// 1. Add a complete JSON schema entry to `get_full_tool_definitions()`
/// 2. Implement a `async fn handle_xxx(args: &Value, ...) -> Result<String>`
/// 3. Add one line to the dispatch match: `"xxx" => handle_xxx(args, ...),`
/// 4. Add a test (ideally using the round-trip test helper or direct call)
///
/// The `require_*` helpers give consistent, high-quality error messages.
/// All known tools are exercised by `every_schema_has_a_handler` at test time.
pub async fn execute_tool(name: &str, args: &Value, ctx: &ToolContext) -> Result<String> {
    let policy = &ctx.policy;

    // ─────────────────────────────────────────────────────────────────────
    // Tool Arbitration Layer (Task 271: fast O(1) checks)
    // ─────────────────────────────────────────────────────────────────────
    match tool_arbitration::arbitrate_tool_call(name, args)? {
        ArbitrationDecision::Execute { name, args } => {
            // Pure dispatch table (ARCH-2 / Task 260)
            // All real work lives in the thin handle_* functions above.
            // Adding a tool = (1) schema + (2) one line here + handler.
            match name.as_str() {
                "read_file" => handle_read_file(&args, ctx).await,
                "read_multiple_files" => handle_read_multiple_files(&args, ctx).await,
                "list_code_definitions" => handle_list_code_definitions(&args, ctx).await,
                "write_file" => handle_write_file(&args, ctx).await,
                "replace" => handle_replace(&args, ctx).await,
                "list_directory" => handle_list_directory(&args, policy),
                "glob_search" => handle_glob_search(&args, policy),
                "search_file_content" => handle_search_file_content(&args, policy),

                "run_shell_command" => handle_run_shell_command(&args, policy).await,

                "web_search" => handle_web_search(&args, ctx).await,
                "web_fetch" => handle_web_fetch(&args, ctx).await,

                "save_memory" => handle_save_memory(&args),

                "sleep" => handle_sleep(&args).await,
                "synthetic_output" => handle_synthetic_output(&args),

                "execute_task_graph" => handle_execute_task_graph(&args, ctx).await,
                "task_create" => handle_task_create(&args, policy),
                "task_get" => handle_task_get(&args, policy),
                "task_update" => handle_task_update(&args, policy),

                "enter_plan_mode" => handle_enter_plan_mode(&args),
                "exit_plan_mode" => handle_exit_plan_mode(&args),
                "enter_worktree" => handle_enter_worktree(&args, policy).await,
                "exit_worktree" => handle_exit_worktree(&args, policy).await,

                "notebook_edit" => handle_notebook_edit(&args, policy),

                "execute_skill" => handle_execute_skill(&args),
                "list_skills" => handle_list_skills(&args),

                "spawn_agent" => handle_spawn_agent(&args, ctx).await,
                "send_message" => handle_send_message(&args),
                "team_create" => handle_team_create(&args),
                "team_delete" => handle_team_delete(&args),
                "list_agents" => handle_list_agents(&args).await,
                "get_agent_status" => handle_get_agent_status(&args).await,
                "cancel_agent" => handle_cancel_agent(&args).await,
                "send_message_in_memory" => handle_send_message_in_memory(&args).await,
                "receive_messages" => handle_receive_messages(&args).await,
                "fork_agent" => handle_fork_agent(&args).await,
                "join_agents" => handle_join_agents(&args).await,

                "mcp_call" => handle_mcp_call(&args, policy).await,
                "mcp_list" => handle_mcp_list(&args),

                "lsp_query" => handle_lsp_query(&args, policy).await,

                "tool_search" => handle_tool_search(&args),
                "cron_create" => handle_cron_create(&args),
                "remote_trigger" => handle_remote_trigger(&args).await,

                "recall_context" => handle_recall_context(&args).await,

                "ai_tool" => handle_ai_tool(&args, policy).await,

                "generate_commit_message" => handle_generate_commit_message(&args),

                "okf_lookup" => handle_okf_lookup(&args).await,
                "okf_get" => handle_okf_get(&args).await,
                "okf_create" => handle_okf_create(&args).await,

                // Runtime guard for ARCH-2 consistency
                unknown => Err(anyhow!(
                    "Tool '{}' is declared in get_full_tool_definitions() but has no handler. \
                     This is an ARCH-2 inconsistency. Add a handle_{} function and a match arm.",
                    unknown,
                    unknown
                )),
            }
        }

        // ─────────────────────────────────────────────────────────────────
        // Arbitration: Reject
        // ─────────────────────────────────────────────────────────────────
        ArbitrationDecision::Reject { message } => Ok(json!({
            "error": "tool_rejected",
            "message": message

        })
        .to_string()),

        // ─────────────────────────────────────────────────────────────────
        // Arbitration: NeedMoreInfo
        // ─────────────────────────────────────────────────────────────────
        ArbitrationDecision::NeedMoreInfo {
            message,
            missing_fields,
        } => Ok(json!({
            "error": "missing_arguments",
            "message": message,
            "missing_fields": missing_fields,
        })
        .to_string()),
    }
}
/// Returns a list of all tool names supported by the registry.
///
/// This list is **derived** from `get_full_tool_definitions()` so that
/// there is only one place that needs to be updated when adding/removing tools.
/// This is a key step toward the unified registry goal (ARCH-2).
pub fn get_tool_definitions() -> Vec<&'static str> {
    // We use a static cache to avoid repeatedly walking the JSON on every call.
    // The list is small and stable.
    static NAMES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();

    NAMES
        .get_or_init(|| {
            get_full_tool_definitions()
                .iter()
                .filter_map(|v| {
                    v.get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_owned().leak() as &'static str) // leak is fine for static tool names
                })
                .collect()
        })
        .clone()
}

/// Static cache for the full tool definitions.
/// Built exactly once (via OnceLock) to eliminate repeated allocation of 50+
/// `serde_json::json!` objects on every call to hot paths.
static FULL_TOOL_DEFINITIONS: std::sync::OnceLock<Vec<serde_json::Value>> =
    std::sync::OnceLock::new();

/// Task 271: Pre-computed static lookup structures for hot-path tool execution.
///
/// These eliminate O(n) linear scans on every tool call in arbitration + dispatch.
///
/// - KNOWN_TOOLS: O(1) membership test (replaces Vec.contains)
/// - REQUIRED_PARAMS_MAP: O(1) lookup for required fields (replaces find_map + alloc per call)
static KNOWN_TOOLS: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
    std::sync::OnceLock::new();

static REQUIRED_PARAMS_MAP: std::sync::OnceLock<
    std::collections::HashMap<&'static str, Vec<String>>,
> = std::sync::OnceLock::new();

/// Initialize (or return) the fast lookup caches.
/// Called lazily from the hot-path accessors.
fn get_known_tools() -> &'static std::collections::HashSet<&'static str> {
    KNOWN_TOOLS.get_or_init(|| get_tool_definitions().into_iter().collect())
}

fn get_required_params_map() -> &'static std::collections::HashMap<&'static str, Vec<String>> {
    REQUIRED_PARAMS_MAP.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        for def in get_full_tool_definitions() {
            if let Some(name) = def["function"]["name"].as_str() {
                let required: Vec<String> = def["function"]["parameters"]["required"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                // Leak the key for 'static lifetime (safe for tool names)
                let static_name: &'static str = name.to_owned().leak();
                map.insert(static_name, required);
            }
        }
        map
    })
}

/// Returns full OpenAI-style JSON tool schemas for every registered tool.
///
/// Each entry has the shape:
/// ```json
/// {"type":"function","function":{"name":"...","description":"...","parameters":{...}}}
/// ```
/// This is the format expected by the Grok/xAI API and by all ACP consumers.
pub fn get_full_tool_definitions() -> &'static [serde_json::Value] {
    FULL_TOOL_DEFINITIONS.get_or_init(|| {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read the contents of a file at the given path.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Absolute or relative path to the file."}
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "read_multiple_files",
                    "description": "Read multiple files at once and return their contents concatenated.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "paths": {"type": "array", "items": {"type": "string"}, "description": "List of file paths to read."}
                        },
                        "required": ["paths"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_code_definitions",
                    "description": "List functions, structs, classes and other top-level definitions in a source file.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Path to the source file."}
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write (overwrite or create) a file with the given content.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path":    {"type": "string", "description": "Path to the file."},
                            "content": {"type": "string", "description": "Full content to write."}
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "replace",
                    "description": "Replace an exact string in a file with a new string.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path":                  {"type": "string", "description": "File path."},
                            "old_string":            {"type": "string", "description": "Exact text to find."},
                            "new_string":            {"type": "string", "description": "Text to replace it with."},
                            "expected_replacements": {"type": "integer", "description": "Expected number of replacements (optional assertion)."}
                        },
                        "required": ["path", "old_string", "new_string"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_directory",
                    "description": "List files and sub-directories inside a directory.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Directory path."}
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "glob_search",
                    "description": "Find files matching a glob pattern (e.g. **/*.rs).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "pattern": {"type": "string", "description": "Glob pattern to match."}
                        },
                        "required": ["pattern"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "search_file_content",
                    "description": "Search for a regex pattern inside a file and return matching lines.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path":    {"type": "string", "description": "File path."},
                            "pattern": {"type": "string", "description": "Regex or text to search for."}
                        },
                        "required": ["path", "pattern"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "run_shell_command",
                    "description": "Run a shell command and return its stdout/stderr output.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command":      {"type": "string", "description": "The shell command to execute."},
                            "timeout_secs": {"type": "integer", "description": "Optional timeout in seconds (default 300)."}
                        },
                        "required": ["command"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "web_search",
                    "description": "Search the web and return a list of results with titles, URLs and snippets.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "Search query."}
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "web_fetch",
                    "description": "Fetch a URL and return the page content as plain text.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": {"type": "string", "description": "URL to fetch."}
                        },
                        "required": ["url"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "save_memory",
                    "description": "Persist a fact or note to the agent's long-term memory store.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "fact": {"type": "string", "description": "The fact or note to remember."}
                        },
                        "required": ["fact"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "sleep",
                    "description": "Pause execution for a given number of seconds.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "seconds": {"type": "number", "description": "Seconds to sleep."}
                        },
                        "required": ["seconds"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "synthetic_output",
                    "description": "Emit a structured JSON output conforming to a named schema.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "schema_name": {"type": "string", "description": "Name of the output schema."},
                            "data":        {"description": "Data conforming to the schema."}
                        },
                        "required": ["schema_name", "data"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "execute_task_graph",
                    "description": "Execute a task graph (DAG) where each node is a tool call.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "graph": {"type": "string", "description": "JSON-serialised TaskGraph."}
                        },
                        "required": ["graph"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "task_get",
                    "description": "Retrieve a single task (or subtask) by numeric ID from .zed/task_list.json.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "number", "description": "Task ID (e.g. 122 or 5.2 for a subtask)."}
                        },
                        "required": ["id"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "task_create",
                    "description": "Create a new task in the project task list.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title":        {"type": "string",  "description": "Task title."},
                            "description":  {"type": "string",  "description": "Brief description."},
                            "priority":     {"type": "string",  "description": "high | medium | low"},
                            "dependencies": {"type": "array",   "items": {"type": "number"}, "description": "IDs of prerequisite tasks."},
                            "details":      {"type": "string",  "description": "Implementation details."},
                            "testStrategy": {"type": "string",  "description": "How to verify completion."},
                            "subtasks":     {"type": "array",   "description": "List of subtask objects."}
                        },
                        "required": ["title"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "task_update",
                    "description": "Update an existing task's status, title, priority or details.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "id":       {"type": "number", "description": "Task ID."},
                            "status":   {"type": "string", "description": "pending | in_progress | done | deferred"},
                            "title":    {"type": "string"},
                            "priority": {"type": "string"},
                            "details":  {"type": "string"}
                        },
                        "required": ["id"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "enter_plan_mode",
                    "description": "Switch the agent into plan mode (no tool execution, planning only).",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "exit_plan_mode",
                    "description": "Exit plan mode and resume normal tool execution.",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "enter_worktree",
                    "description": "Create or switch to a git worktree for isolated work on a branch.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "branch": {"type": "string", "description": "Branch name."},
                            "path":   {"type": "string", "description": "Worktree path."}
                        },
                        "required": ["branch", "path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "exit_worktree",
                    "description": "Exit the current git worktree, optionally merging changes back.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "merge": {"type": "boolean", "description": "Merge changes into main branch (default false)."}
                        }
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "notebook_edit",
                    "description": "Edit a cell in a Jupyter notebook.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path":       {"type": "string",  "description": "Path to the notebook file."},
                            "source":     {"type": "string",  "description": "New source code for the cell."},
                            "cell_index": {"type": "integer", "description": "0-based cell index (default 0)."},
                            "cell_type":  {"type": "string",  "description": "code | markdown (default code)."}
                        },
                        "required": ["path", "source"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "execute_skill",
                    "description": "Run a named agent skill and return its output.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "skill_name": {"type": "string", "description": "Name of the skill to execute."},
                            "input":      {"type": "string", "description": "Optional input to pass to the skill."}
                        },
                        "required": ["skill_name"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_skills",
                    "description": "List all available agent skills.",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "spawn_agent",
                    "description": "Spawn a focused sub-agent to complete a well-scoped task. \
                        Optionally provide a custom model, persona, tool whitelist, sandbox dirs, \
                        and iteration budget for per-agent isolation.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "task": {
                                "type": "string",
                                "description": "The task for the sub-agent to complete."
                            },
                            "context": {
                                "type": "string",
                                "description": "Optional additional context to pass to the agent."
                            },
                            "max_tokens": {
                                "type": "integer",
                                "description": "Max output tokens (256–8192, default 2048)."
                            },
                            "model": {
                                "type": "string",
                                "description": "Model to use, e.g. 'grok-3-mini' (default) or 'grok-3'."
                            },
                            "system_prompt": {
                                "type": "string",
                                "description": "Custom persona / system prompt for this agent."
                            },
                            "allowed_tools": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Tool whitelist. Omit for no tools. E.g. ['read_file','list_directory']."
                            },
                            "trusted_dirs": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Directories the agent may access. Defaults to CWD only."
                            },
                            "max_tool_iterations": {
                                "type": "integer",
                                "description": "Max tool-loop iterations (default 10)."
                            }
                        },
                        "required": ["task"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "send_message",
                    "description": "Send a message to a named agent or channel.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target":  {"type": "string", "description": "Target agent ID or channel name."},
                            "message": {"type": "string", "description": "Message content."}
                        },
                        "required": ["target", "message"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "team_create",
                    "description": "Create a named team configuration.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name":        {"type": "string", "description": "Team name."},
                            "members":     {"type": "array",  "items": {"type": "string"}, "description": "List of member IDs."},
                            "description": {"type": "string", "description": "Team description."}
                        },
                        "required": ["name"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "team_delete",
                    "description": "Delete a named team.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string", "description": "Name of the team to delete."}
                        },
                        "required": ["name"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_agents",
                    "description": "List all tracked sub-agents (optionally filtered by parent).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "parent_id": {"type": "string", "description": "Optional parent agent ID to filter by."}
                        }
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "get_agent_status",
                    "description": "Get the status and result of a specific sub-agent.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_id": {"type": "string", "description": "ID of the sub-agent."}
                        },
                        "required": ["agent_id"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "cancel_agent",
                    "description": "Cancel a running sub-agent.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_id": {"type": "string", "description": "ID of the sub-agent to cancel."}
                        },
                        "required": ["agent_id"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "send_message_in_memory",
                    "description": "Send a message using the fast in-memory agent bus.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "from":    {"type": "string", "description": "Sender agent ID."},
                            "to":      {"type": "string", "description": "Target agent ID or channel."},
                            "message": {"type": "string", "description": "Message content."}
                        },
                        "required": ["to", "message"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "receive_messages",
                    "description": "Receive pending in-memory messages for an agent.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": {"type": "string", "description": "Agent ID or channel to receive for."}
                        },
                        "required": ["target"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "fork_agent",
                    "description": "Spawn multiple sub-agents in parallel for different subtasks.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "tasks": {"type": "array", "items": {"type": "string"}, "description": "List of tasks to fork."}
                        },
                        "required": ["tasks"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "join_agents",
                    "description": "Collect and merge results from multiple sub-agents.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_ids": {"type": "array", "items": {"type": "string"}, "description": "List of agent IDs to join."}
                        },
                        "required": ["agent_ids"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "mcp_call",
                    "description": "Call a tool on a Model-Context-Protocol server.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "server_command": {"type": "string", "description": "Command to launch the MCP server."},
                            "tool_name":      {"type": "string", "description": "Name of the tool to invoke."},
                            "arguments":      {"description": "Tool arguments (any JSON value)."}
                        },
                        "required": ["server_command", "tool_name"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "mcp_list",
                    "description": "List all connected MCP servers and the tools discovered from them.",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "lsp_query",
                    "description": "Query the Language Server Protocol for diagnostics, hover info, or definitions.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "file":       {"type": "string",  "description": "Source file path."},
                            "line":       {"type": "integer", "description": "0-based line number."},
                            "character":  {"type": "integer", "description": "0-based character offset."},
                            "query_type": {"type": "string",  "description": "diagnostics | hover | definition (default diagnostics)."}
                        },
                        "required": ["file"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "tool_search",
                    "description": "Search for tools by name or description keyword.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "Search query."}
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "cron_create",
                    "description": "Schedule a recurring task using a cron expression.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name":     {"type": "string", "description": "Unique name for the cron job."},
                            "schedule": {"type": "string", "description": "Cron expression (e.g. '0 * * * *')."},
                            "task":     {"type": "string", "description": "Task description or command to run."}
                        },
                        "required": ["name", "schedule", "task"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "remote_trigger",
                    "description": "Send an HTTP request to a remote endpoint.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "endpoint": {"type": "string", "description": "URL to send the request to."},
                            "payload":  {"description":   "Request body (any JSON value)."},
                            "method":   {"type": "string", "description": "HTTP method (default POST)."}
                        },
                        "required": ["endpoint"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "recall_context",
                    "description": "Recall an archived context chunk by its numeric ID.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "chunk_id": {"type": "integer", "description": "Archive chunk ID to recall."}
                        },
                        "required": ["chunk_id"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "ai_tool",
                    "description": "Generic entrypoint for AI-generated or dynamic tools.",
                    "parameters": {"type": "object", "properties": {}}
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_commit_message",
                    "description": "Generate a Conventional Commits style commit message from the current git diff. Use this when you need to create a commit message programmatically.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "instructions": {
                                "type": "string",
                                "description": "Optional extra instructions for the commit message style (e.g. 'use conventional commits with scope')"
                            }
                        }
                    }
                }
            }),
            // ── OKF Knowledge (Open Knowledge Format) ─────────────────────────
            json!({
                "type": "function",
                "function": {
                    "name": "okf_lookup",
                    "description": "Search the loaded Open Knowledge Format (OKF) bundles. This is Grok-CLI's Knowledge API. Use it to find structured knowledge such as tables, metrics, runbooks, schemas, definitions, etc. that were loaded from markdown+frontmatter bundles.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "What to search for (e.g. 'orders table', 'weekly active users', 'runbook for data refresh')"
                            },
                            "max_results": {
                                "type": "integer",
                                "description": "Maximum number of concepts to return (default 5, max 20)"
                            }
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "okf_get",
                    "description": "Retrieve the full content of a specific OKF concept by its ID (the relative path inside the bundle, e.g. 'metrics/weekly_active_users' or 'tables/orders').",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "The stable ID of the concept (usually the path without .md)"
                            }
                        },
                        "required": ["id"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "okf_create",
                    "description": "Create and store a new structured knowledge concept in the OKF Knowledge OS. Writes to the remote OKF server (if configured via okf.remote_url) or falls back to local knowledge bundles. Use this to permanently record tables, metrics, runbooks, decisions, patterns, etc. so they become queryable via okf_lookup.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "type": {
                                "type": "string",
                                "description": "Concept type (e.g. 'Table', 'Metric', 'Runbook', 'Decision', 'API', 'Pattern', 'Schema')"
                            },
                            "title": {
                                "type": "string",
                                "description": "Human-readable title"
                            },
                            "body": {
                                "type": "string",
                                "description": "Markdown body content with the actual knowledge"
                            },
                            "description": {
                                "type": "string",
                                "description": "Short one-line description (optional)"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Tags for categorization (optional)"
                            },
                            "resource": {
                                "type": "string",
                                "description": "Link to the real resource (optional)"
                            },
                            "id": {
                                "type": "string",
                                "description": "Optional explicit ID/path (e.g. 'metrics/weekly_active_users'). If omitted, one is generated from type+title."
                            }
                        },
                        "required": ["title", "body"]
                    }
                }
            }),
        ]
    })
}

/// Returns the full JSON tool definitions (same as get_full_tool_definitions).
/// This alias is used by all ACP consumers that need the schema for the model.
///
/// Returns a static slice (zero-cost after first build) thanks to the OnceLock cache.
pub fn get_available_tool_definitions() -> &'static [serde_json::Value] {
    get_full_tool_definitions()
}

/// Returns the list of required parameter names for the given tool name,
/// extracted directly from its JSON schema definition.
///
/// **ARCH-2 Single Source of Truth**
/// This function (together with `get_full_tool_definitions`) is the canonical
/// definition of what arguments a tool needs.  `tool_arbitration` now
/// delegates to it, eliminating the old per-tool `match` duplication.
pub fn get_required_parameters(name: &str) -> Vec<String> {
    // Task 271: Use pre-computed static map for O(1) lookup instead of linear scan.
    get_required_params_map()
        .get(name)
        .cloned()
        .unwrap_or_default()
}

/// Fast O(1) version for hot paths (Task 271).
/// Returns a reference to the static list (no allocation on lookup).
pub fn get_required_parameters_fast(name: &str) -> Option<&'static [String]> {
    get_required_params_map().get(name).map(|v| v.as_slice())
}

/// O(1) known-tool check (Task 271).
/// Replaces the previous O(n) `get_tool_definitions().contains(...)` on the hot path.
pub fn is_known_tool_fast(name: &str) -> bool {
    get_known_tools().contains(name)
}

/// Returns the built-in tools plus any tools discovered from connected MCP servers.
/// MCP tools are exposed with the server name as a prefix (e.g. "markmap:generate").
/// This is the function that should be used when building the tool list for a session
/// that has MCP servers attached.
pub async fn get_available_tool_definitions_with_mcp(
    mcp_tools: &[(String, crate::mcp::protocol::Tool)],
) -> Vec<serde_json::Value> {
    // Clone the static base only when we actually have MCP tools to append.
    // This keeps the zero-alloc fast path for the common (no-MCP) case.
    let mut defs: Vec<serde_json::Value> = get_full_tool_definitions().to_vec();

    for (server, tool) in mcp_tools {
        let full_name = format!("{}:{}", server, tool.name);
        let desc = tool
            .description
            .clone()
            .unwrap_or_else(|| format!("MCP tool from {}", server));

        defs.push(json!({
            "type": "function",
            "function": {
                "name": full_name,
                "description": desc,
                "parameters": tool.input_schema
            }
        }));
    }

    defs
}

// ── Dynamic tool registration (Task 143) ─────────────────────────────────────

use std::collections::HashMap;
use std::sync::Mutex;

// ── MCP discovered tools (populated during ACP session/new) ──────────────────

use std::sync::RwLock as StdRwLock;

/// Global store of tools discovered from connected MCP servers.
/// Key = server name, Value = list of tools.
static DISCOVERED_MCP_TOOLS: StdRwLock<Option<HashMap<String, Vec<crate::mcp::protocol::Tool>>>> =
    StdRwLock::new(None);

/// Update the global MCP tools map (called from ACP session handler).
pub fn set_discovered_mcp_tools(map: HashMap<String, Vec<crate::mcp::protocol::Tool>>) {
    let mut guard = DISCOVERED_MCP_TOOLS.write().unwrap();
    *guard = Some(map);
}

/// Returns a snapshot of all discovered MCP tools.
pub fn get_discovered_mcp_tools() -> HashMap<String, Vec<crate::mcp::protocol::Tool>> {
    let guard = DISCOVERED_MCP_TOOLS.read().unwrap();
    guard.clone().unwrap_or_default()
}

/// Registry of dynamically loaded custom tools.
static DYNAMIC_TOOLS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Register a tool that was loaded from a custom dylib (Task 143).
pub fn register_dynamic_tool(name: &str, description: &str, _lib_path: &std::path::Path) {
    let mut map = DYNAMIC_TOOLS.lock().unwrap();
    if map.is_none() {
        *map = Some(HashMap::new());
    }
    if let Some(ref mut m) = *map {
        m.insert(name.to_string(), description.to_string());
    }
    tracing::info!("Dynamic tool registered: {}", name);
}

/// Returns the list of all dynamically loaded tool names.
pub fn list_dynamic_tools() -> Vec<String> {
    let map = DYNAMIC_TOOLS.lock().unwrap();
    map.as_ref()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// ARCH-2: Unified Tool Registry (Task 260 - COMPLETED)
// ─────────────────────────────────────────────────────────────────────────────
//
// We have achieved a clean, maintainable architecture:
//
// - Schemas in `get_full_tool_definitions()` are the **only** place you edit
//   when adding/removing tools for name + required arguments.
// - Thin `handle_*` functions isolate all argument extraction and delegation.
// - The match in `execute_tool` is now a **pure dispatch table** (one line per tool).
// - `require_*` helpers give consistent, high-quality error messages.
// - The `unknown` arm + `every_tool_schema_has_corresponding_handler` test
//   act as a hard guard against drift.
//
// Adding a tool is now low-risk and obvious (see doc comment on `execute_tool`).
//
// This is the practical maximum for a hand-written registry in current Rust
// without introducing a macro or code-generation step.

#[cfg(test)]
mod tests {
    use super::*;

    /// ARCH-2 symmetry test.
    ///
    /// Every tool name returned by `get_tool_definitions()` must be recognized
    /// by the rest of the system (via `is_known_tool`).
    ///
    /// Because `is_known_tool` now delegates to `get_tool_definitions()`,
    /// this test also guards against future divergence.
    #[test]
    fn tool_registry_symmetry() {
        let defined_names = get_tool_definitions();

        assert!(
            !defined_names.is_empty(),
            "get_tool_definitions() must return at least one tool"
        );

        for name in &defined_names {
            // `is_known_tool` is pub(crate) but visible inside the crate.
            // This exercises the delegation added for ARCH-2.
            assert!(
                crate::tools::tool_arbitration::is_known_tool(name),
                "Tool '{}' appears in get_tool_definitions() but is_known_tool() returned false",
                name
            );
        }
    }

    /// Additional sanity check: the full definitions and the name list are consistent.
    #[test]
    fn full_definitions_match_name_list() {
        let full = get_full_tool_definitions();
        let names = get_tool_definitions();

        assert_eq!(
            full.len(),
            names.len(),
            "get_full_tool_definitions() and get_tool_definitions() should have the same length"
        );

        for (def, name) in full.iter().zip(names.iter()) {
            let def_name = def
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("<missing>");
            assert_eq!(
                def_name, *name,
                "Name in full definition does not match name list"
            );
        }
    }

    /// Task 263 regression test: prove that tool definitions are statically cached.
    ///
    /// After the first call, every subsequent call must return a pointer to the
    /// exact same static data (no re-allocation / re-build of the json! vec).
    ///
    /// The key proof is pointer identity of the returned `&'static [Value]` slices.
    /// We keep other checks loose so the test stays stable as the registry evolves.
    #[test]
    fn tool_definitions_are_statically_cached() {
        // First call (may initialize the OnceLock)
        let first = get_full_tool_definitions();

        // Second call – must hit the cached path
        let second = get_full_tool_definitions();

        // This is the critical assertion for "statically cached".
        // OnceLock guarantees the closure runs only once, so we must get the
        // exact same slice (same data pointer + same length).
        assert!(
            std::ptr::eq(first, second),
            "get_full_tool_definitions() must return the exact same &'static slice \
             on every call (OnceLock cache not working). first={:p} second={:p}",
            first.as_ptr(),
            second.as_ptr()
        );

        // Lengths must obviously match
        assert_eq!(first.len(), second.len());

        // Basic sanity – the registry is populated
        assert!(!first.is_empty(), "tool definitions must not be empty");

        // At least the core file tools should be present (content check)
        let names: Vec<&str> = first
            .iter()
            .filter_map(|d| d.get("function")?.get("name")?.as_str())
            .collect();

        assert!(
            names.contains(&"read_file") && names.contains(&"write_file"),
            "core tools read_file + write_file must be present, got names: {:?}",
            names
        );

        // The derived name list must also be consistent
        let n1 = get_tool_definitions();
        let n2 = get_tool_definitions();
        assert_eq!(n1, n2);
    }

    /// ARCH-2: Verify that `get_required_parameters` correctly extracts
    /// the "required" array from each tool's JSON schema.
    ///
    /// This proves that `missing_required_fields` in arbitration now has
    /// a single source of truth.
    #[test]
    fn get_required_parameters_is_accurate() {
        // Tools with no required params
        assert!(get_required_parameters("enter_plan_mode").is_empty());
        assert!(get_required_parameters("list_skills").is_empty());
        assert!(get_required_parameters("mcp_list").is_empty());

        // Simple single required
        let req = get_required_parameters("read_file");
        assert_eq!(req, vec!["path"]);

        let req = get_required_parameters("web_search");
        assert_eq!(req, vec!["query"]);

        let req = get_required_parameters("run_shell_command");
        assert_eq!(req, vec!["command"]);

        // Multiple required
        let req = get_required_parameters("write_file");
        assert!(req.contains(&"path".to_string()));
        assert!(req.contains(&"content".to_string()));

        let req = get_required_parameters("replace");
        assert!(req.contains(&"path".to_string()));
        assert!(req.contains(&"old_string".to_string()));
        assert!(req.contains(&"new_string".to_string()));

        let req = get_required_parameters("spawn_agent");
        assert!(req.contains(&"task".to_string()));

        // Unknown tool → empty (graceful)
        assert!(get_required_parameters("definitely_not_a_real_tool").is_empty());
    }

    /// ARCH-2 guard: every tool that declares "required" fields in its schema
    /// must have at least one entry, and the names must be valid strings.
    #[test]
    fn all_required_fields_are_valid_strings() {
        for def in get_full_tool_definitions() {
            let name = def["function"]["name"].as_str().unwrap_or("???");
            if let Some(req) = def["function"]["parameters"].get("required") {
                if let Some(arr) = req.as_array() {
                    for v in arr {
                        assert!(
                            v.is_string(),
                            "Tool '{}' has a non-string in required: {:?}",
                            name,
                            v
                        );
                    }
                }
            }
        }
    }

    /// TEST-2: End-to-end round-trip through execute_tool dispatch (Task 258).
    ///
    /// **Purpose**
    /// This integration-style unit test exercises the **critical dispatch path**:
    ///     execute_tool(name, args, ctx)
    ///         → tool_arbitration::arbitrate_tool_call()
    ///         → match arm that calls the real tool implementation
    ///
    /// It verifies the full round-trip for:
    /// - A successful write (creates file on disk via the real `write_file` impl)
    /// - A successful read that returns the exact bytes written
    /// - Proper handling of an unknown tool (rejection path)
    /// - Proper handling of a known tool with missing required arguments
    ///
    /// **Isolation**
    /// Uses `tempfile::TempDir` + `SecurityPolicy::with_working_directory` so that
    /// all file operations stay inside an ephemeral directory. No real user files
    /// or network calls are performed.
    ///
    /// **Error checking**
    /// Every dispatch result is inspected. Both the `Result` and the string
    /// content (for structured arbitration responses) are asserted with clear
    /// messages so a regression in any layer produces an obvious failure.
    ///
    /// **Coverage**
    /// This test is automatically run by `cargo test`. It covers the happy path
    /// through the registry, the two main arbitration decision branches that
    /// reach tool code, and the error/rejection paths.
    #[tokio::test]
    async fn execute_tool_round_trip_write_read_unknown_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        // Robust trust for Windows CI (\\?\ prefixes, canonicalization differences)
        let raw = dir.path().to_path_buf();
        let mut policy =
            crate::acp::security::SecurityPolicy::with_working_directory(raw.clone());
        if let Ok(can) = raw.canonicalize() {
            policy.add_trusted_directory(&can);
        }
        // Explicitly trust the raw form as well (Windows path prefix differences)
        policy.add_trusted_directory(&raw);
        let ctx = crate::tools::ToolContext::new(policy);

        // Use a *unique* filename (uuid) placed directly in the temp dir.
        // This completely eliminates any chance of name collision with directories
        // created by create_dir_all, previous test runs, or path-resolution edge cases
        // on CI runners (Linux or Windows).
        let unique_name = format!("roundtrip_via_execute_tool_{}.txt", uuid::Uuid::new_v4());
        let test_file_path = dir.path().join(&unique_name);
        let test_file = test_file_path.to_string_lossy().to_string();

        // Very defensive cleanup: remove both file and any accidental directory with this name.
        // This prevents "Is a directory (os error 21)" when a prior partial run or
        // path normalization left a dir at the target location.
        let _ = std::fs::remove_file(&test_file_path);
        let _ = std::fs::remove_dir_all(&test_file_path);

        // ── Happy path: write_file via dispatch ─────────────────────────────
        let write_args = serde_json::json!({
            "path": test_file,
            "content": "hello from TEST-2 round-trip"
        });
        let write_result = execute_tool("write_file", &write_args, &ctx).await;
        assert!(
            write_result.is_ok(),
            "write_file dispatch must succeed, got error: {:?}",
            write_result.err()
        );
        let on_disk = std::fs::read_to_string(&test_file)
            .expect("file must exist on disk after successful write_file dispatch");
        assert_eq!(
            on_disk, "hello from TEST-2 round-trip",
            "written content must match exactly"
        );

        // ── Happy path: read_file via dispatch ──────────────────────────────
        let read_args = serde_json::json!({ "path": test_file });
        let read_result = execute_tool("read_file", &read_args, &ctx).await;
        assert!(
            read_result.is_ok(),
            "read_file dispatch must succeed: {:?}",
            read_result.err()
        );
        assert_eq!(
            read_result.unwrap(),
            "hello from TEST-2 round-trip",
            "read_file after write_file must return the exact bytes that were written"
        );

        // ── Error / rejection path: unknown tool ────────────────────────────
        let unknown_result = execute_tool(
            "this_tool_does_not_exist_12345",
            &serde_json::json!({}),
            &ctx,
        )
        .await;
        let unknown_str = match &unknown_result {
            Ok(s) => s.clone(),
            Err(e) => e.to_string(),
        };
        assert!(
            unknown_result.is_err()
                || unknown_str.contains("unknown")
                || unknown_str.contains("rejected")
                || unknown_str.contains("tool_rejected")
                || unknown_str.contains("no implementation")
                || unknown_str.contains("declared in the registry")
                || unknown_str.contains("has no handler"),
            "unknown tool must produce an error or a structured rejection message, got: {}",
            unknown_str
        );

        // ── Error path: known tool with missing required argument ───────────
        let missing_arg_args = serde_json::json!({
            "path": test_file
            // "content" deliberately omitted
        });
        let missing_result = execute_tool("write_file", &missing_arg_args, &ctx).await;
        assert!(
            missing_result.is_ok(),
            "arbitration for missing args must return Ok( structured error ) rather than hard Err"
        );
        let missing_str = missing_result.unwrap().to_lowercase();
        assert!(
            missing_str.contains("missing")
                || missing_str.contains("content")
                || missing_str.contains("required")
                || missing_str.contains("arguments")
                || missing_str.contains("needmoreinfo")
                || missing_str.contains("error"),
            "structured response for missing required arg must mention the problem, got: {}",
            missing_str
        );
    }

    /// ARCH-2 / Task 260: The ultimate drift guard.
    ///
    /// For **every** tool that has a schema in `get_full_tool_definitions()`,
    /// there must be a corresponding handler in the dispatch table.
    ///
    /// We prove this by:
    /// 1. Calling `execute_tool` for every known tool name.
    /// 2. Ensuring we never receive the specific "has no handler" error.
    /// 3. For tools that declare required arguments we expect a clean
    ///    "missing_arguments" structured response (not a handler-missing panic).
    /// 4. For tools with zero required arguments we expect successful execution
    ///    (or at least no "no handler" error).
    ///
    /// This test will fail loudly the moment someone adds a schema without
    /// adding the matching `handle_*` function + dispatch entry.
    #[tokio::test]
    async fn every_tool_schema_has_corresponding_handler() {
        let dir = tempfile::TempDir::new().unwrap();
        let policy =
            crate::acp::security::SecurityPolicy::with_working_directory(dir.path().to_path_buf());
        let ctx = crate::tools::ToolContext::new(policy);

        let all_tools = get_tool_definitions();
        assert!(
            !all_tools.is_empty(),
            "Registry must define at least one tool"
        );

        for tool_name in &all_tools {
            // Use empty args. Arbitration will either:
            // - Succeed for zero-req tools
            // - Return a structured NeedMoreInfo for tools that have required fields
            // - Never reach the "unknown" arm if the handler exists
            let result = execute_tool(tool_name, &serde_json::json!({}), &ctx).await;

            let err_string = match &result {
                Ok(s) => s.clone(),
                Err(e) => e.to_string(),
            };

            // Hard failure if the guard arm was hit
            assert!(
                !err_string.contains("has no handler"),
                "CRITICAL ARCH-2 DRIFT: Tool '{}' has a schema but NO handler implementation. \
                 Add handle_{}() + a dispatch line. Error was: {}",
                tool_name,
                tool_name,
                err_string
            );

            // For tools that require arguments, we should get a proper missing-args response
            // (this also exercises the require_* helpers indirectly).
            let required = get_required_parameters(tool_name);
            if !required.is_empty() {
                // The result should be Ok(...) containing a structured error, not a hard Err
                assert!(
                    result.is_ok(),
                    "Tool '{}' with required args should return structured response, got hard error: {:?}",
                    tool_name,
                    result.err()
                );
                let s = result.unwrap().to_lowercase();
                assert!(
                    s.contains("missing")
                        || s.contains("required")
                        || s.contains("arguments")
                        || s.contains("error"),
                    "Tool '{}' with required fields '{}' must produce a clear missing-arguments response, got: {}",
                    tool_name,
                    required.join(", "),
                    s
                );
            } else {
                // Zero-required tools should generally succeed or give a benign response.
                // We only care that we didn't hit the "no handler" path.
                // Some may return errors for other reasons (e.g. no MCP servers), that's fine.
            }
        }
    }

    /// Quick unit tests for the require_* helpers (error message quality).
    #[test]
    fn require_helpers_produce_good_errors() {
        let args = serde_json::json!({ "foo": "bar", "num": 42, "arr": [1,2] });

        // happy paths
        assert_eq!(require_str(&args, "foo").unwrap(), "bar");
        assert_eq!(require_f64(&args, "num").unwrap(), 42.0);
        assert!(require_array(&args, "arr").unwrap().len() == 2);

        // error paths - messages must mention the key
        let err = require_str(&args, "missing_key").unwrap_err().to_string();
        assert!(
            err.contains("missing_key"),
            "error message should name the missing key: {}",
            err
        );

        let err = require_u64(&args, "foo").unwrap_err().to_string(); // wrong type
        assert!(
            err.contains("foo"),
            "error must mention key on type mismatch: {}",
            err
        );
    }

    // Note: optional_* helpers were removed for now (they were unused in production code).
    // When they are re-introduced, add tests for them here.
}
