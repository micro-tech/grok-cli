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

/// Execute a named tool with the provided JSON arguments and context.
///
/// This is the unified entry-point used by the CPU router tool loop. Every
/// named tool in [`get_tool_definitions`] must have a matching arm here.
pub async fn execute_tool(name: &str, args: &Value, ctx: &ToolContext) -> Result<String> {
    let policy = &ctx.policy;

    // ─────────────────────────────────────────────────────────────────────
    // Tool Arbitration Layer
    // ─────────────────────────────────────────────────────────────────────
    match tool_arbitration::arbitrate_tool_call(name, args)? {
        ArbitrationDecision::Execute { name, args } => {
            // Use normalized name/args from arbitration
            match name.as_str() {
                // ── File tools ──────────────────────────────────────────────
                "read_file" => {
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    file_tools::read_file(path, policy).await
                }
                "read_multiple_files" => {
                    let paths_val = args["paths"]
                        .as_array()
                        .ok_or_else(|| anyhow!("Missing: paths (array)"))?;
                    let paths: Result<Vec<String>> = paths_val
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .ok_or_else(|| anyhow!("Invalid path entry"))
                                .map(str::to_string)
                        })
                        .collect();
                    file_tools::read_multiple_files(paths?, policy).await
                }
                "list_code_definitions" => {
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    file_tools::list_code_definitions(path, policy).await
                }
                "write_file" => {
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    let content = args["content"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: content"))?;
                    file_tools::write_file(path, content, policy, false).await
                }
                "replace" => {
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    let old_string = args["old_string"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: old_string"))?;
                    let new_string = args["new_string"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: new_string"))?;
                    let expected = args["expected_replacements"].as_u64().map(|n| n as u32);
                    file_tools::replace(path, old_string, new_string, expected, policy, false).await
                }
                "list_directory" => {
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    file_tools::list_directory(path, policy)
                }
                "glob_search" => {
                    let pattern = args["pattern"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: pattern"))?;
                    file_tools::glob_search(pattern, policy)
                }
                "search_file_content" => {
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    let pattern = args["pattern"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: pattern"))?;
                    file_tools::search_file_content(path, pattern, policy)
                }

                // ── Shell ───────────────────────────────────────────────────
                "run_shell_command" => {
                    let command = args["command"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: command"))?;
                    // Callers may inject a project-specific timeout via args["timeout_secs"];
                    // fall back to 0 which tells shell_tools to use its 300 s built-in default.
                    let timeout_secs = args["timeout_secs"].as_u64().unwrap_or(0);
                    shell_tools::run_shell_command(command, policy, timeout_secs).await
                }

                // ── Web ─────────────────────────────────────────────────────
                "web_search" => {
                    let query = args["query"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: query"))?;
                    web_tools::web_search(query).await
                }
                "web_fetch" => {
                    let url = args["url"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: url"))?;
                    web_tools::web_fetch(url).await
                }

                // ── Memory ──────────────────────────────────────────────────
                "save_memory" => {
                    let fact = args["fact"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: fact"))?;
                    memory_tools::save_memory(fact)
                }

                // ── System ──────────────────────────────────────────────────
                "sleep" => {
                    let seconds = args["seconds"].as_f64().unwrap_or(1.0) as u64;
                    system_tools::sleep_for(seconds).await
                }
                "synthetic_output" => {
                    let schema_name = args["schema_name"].as_str().unwrap_or("output");
                    let data = &args["data"];
                    system_tools::synthetic_output(schema_name, data)
                }

                // ── Task management ────────────────────────────────────────
                "execute_task_graph" => {
                    let graph_json = args["graph"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: graph"))?;
                    task_graph_tools::execute_task_graph(graph_json, ctx).await
                }
                "task_create" => {
                    let title = args["title"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: title"))?;
                    let description = args["description"].as_str().unwrap_or("");
                    let priority = args["priority"].as_str().unwrap_or("medium");
                    let deps: Vec<f64> = args["dependencies"]
                        .as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                        .unwrap_or_default();
                    let details = args["details"].as_str().unwrap_or("");
                    let test_strategy = args["testStrategy"].as_str().unwrap_or("");
                    let subtasks: Vec<Value> =
                        args["subtasks"].as_array().cloned().unwrap_or_default();
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
                "task_get" => {
                    let id = args["id"].as_f64().ok_or_else(|| anyhow!("Missing: id"))?;
                    task_tools::task_get(id, policy)
                }
                "task_update" => {
                    let id = args["id"].as_f64().ok_or_else(|| anyhow!("Missing: id"))?;
                    let status = args["status"].as_str();
                    let title = args["title"].as_str();
                    let priority = args["priority"].as_str();
                    let details = args["details"].as_str();
                    task_tools::task_update(id, status, title, priority, details, policy)
                }

                // ── Plan mode + worktrees ───────────────────────────────────
                "enter_plan_mode" => plan_tools::enter_plan_mode(),
                "exit_plan_mode" => plan_tools::exit_plan_mode(),
                "enter_worktree" => {
                    let branch = args["branch"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: branch"))?;
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    plan_tools::enter_worktree(branch, path, policy).await
                }
                "exit_worktree" => {
                    let merge = args["merge"].as_bool().unwrap_or(false);
                    plan_tools::exit_worktree(merge, policy).await
                }

                // ── Notebook ────────────────────────────────────────────────
                "notebook_edit" => {
                    let path = args["path"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: path"))?;
                    let cell_index = args["cell_index"].as_u64().unwrap_or(0) as usize;
                    let source = args["source"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: source"))?;
                    let cell_type = args["cell_type"].as_str().unwrap_or("code");
                    notebook_tools::notebook_edit(path, cell_index, source, cell_type, policy)
                }

                // ── Skills ──────────────────────────────────────────────────
                "execute_skill" => {
                    let skill_name = args["skill_name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: skill_name"))?;
                    let input = args["input"].as_str().unwrap_or("");
                    skill_tools::execute_skill(skill_name, input)
                }
                "list_skills" => skill_tools::list_available_skills(),

                // ── Agent coordination ──────────────────────────────────────
                "spawn_agent" => {
                    let task = args["task"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: task"))?;
                    let context = args["context"].as_str().unwrap_or("");
                    let max_tokens = args["max_tokens"].as_u64().unwrap_or(2048) as u32;

                    // Optional per-agent config fields — all backward-compatible.
                    let has_config = args.get("model").is_some()
                        || args.get("system_prompt").is_some()
                        || args.get("allowed_tools").is_some()
                        || args.get("trusted_dirs").is_some()
                        || args.get("max_tool_iterations").is_some();

                    if has_config {
                        let mut builder =
                            crate::agent::SubAgentConfig::builder().max_tokens(max_tokens);
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
                            builder = builder.allowed_tools(names);
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
                "send_message" => {
                    let target = args["target"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: target"))?;
                    let message = args["message"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: message"))?;
                    agent_tools::send_message(target, message)
                }
                "team_create" => {
                    let name = args["name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: name"))?;
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
                "team_delete" => {
                    let name = args["name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: name"))?;
                    agent_tools::team_delete(name)
                }
                "list_agents" => {
                    let parent = args.get("parent_id").and_then(|v| v.as_str());
                    agent_tools::list_agents(parent).await
                }
                "get_agent_status" => {
                    let id = args["agent_id"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: agent_id"))?;
                    agent_tools::get_agent_status(id).await
                }
                "cancel_agent" => {
                    let id = args["agent_id"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: agent_id"))?;
                    agent_tools::cancel_agent(id).await
                }
                "send_message_in_memory" => {
                    let from = args["from"].as_str().unwrap_or("main");
                    let to = args["to"].as_str().ok_or_else(|| anyhow!("Missing: to"))?;
                    let message = args["message"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: message"))?;
                    agent_tools::send_message_in_memory(from, to, message).await
                }
                "receive_messages" => {
                    let target = args["target"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: target"))?;
                    agent_tools::receive_messages(target).await
                }
                "fork_agent" => {
                    let tasks: Vec<String> = args["tasks"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    agent_tools::fork_agent(tasks).await
                }
                "join_agents" => {
                    let ids: Vec<String> = args["agent_ids"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    agent_tools::join_agents(ids).await
                }

                // ── MCP ─────────────────────────────────────────────────────
                "mcp_call" => {
                    let server_command = args["server_command"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: server_command"))?;
                    let tool_name = args["tool_name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: tool_name"))?;
                    let arguments = args["arguments"].clone();
                    mcp_tools::mcp_call(server_command, tool_name, arguments, policy).await
                }

                // List currently connected MCP servers and their discovered tools
                "mcp_list" => {
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

                // ── LSP ─────────────────────────────────────────────────────
                "lsp_query" => {
                    let file = args["file"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: file"))?;
                    let line = args["line"].as_u64().unwrap_or(0) as u32;
                    let character = args["character"].as_u64().unwrap_or(0) as u32;
                    let query_type = args["query_type"].as_str().unwrap_or("diagnostics");
                    lsp_tools::lsp_query(file, line, character, query_type, policy).await
                }

                // ── Discovery ───────────────────────────────────────────────
                "tool_search" => {
                    let query = args["query"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: query"))?;
                    discovery_tools::tool_search(query)
                }
                "cron_create" => {
                    let name = args["name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: name"))?;
                    let schedule = args["schedule"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: schedule"))?;
                    let task = args["task"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: task"))?;
                    discovery_tools::cron_create(name, schedule, task)
                }
                "remote_trigger" => {
                    let endpoint = args["endpoint"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing: endpoint"))?;
                    let payload = args["payload"].clone();
                    let method = args["method"].as_str().unwrap_or("POST");
                    discovery_tools::remote_trigger(endpoint, payload, method).await
                }

                // ── Context recall ──────────────────────────────────────────
                "recall_context" => {
                    let chunk_id = args["chunk_id"].as_u64().ok_or_else(|| {
                        anyhow!("Missing or invalid: chunk_id (must be a positive integer)")
                    })? as u32;

                    match crate::memory::context_archive::ContextArchive::for_session("unknown") {
                        Err(e) => Err(anyhow!("Could not open context archive: {}", e)),
                        Ok(archive) => match archive.load_chunk(chunk_id)? {
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
                                            .map(|f| format!("\u{2022} {f}"))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    )
                                };
                                Ok(format!(
                                    "[Recalled Archive #{id}]\n\
                                     Covered {count} messages archived on {ts}.\n\
                                     Summary: {summary}{facts}\n\n\
                                     Note: The full raw messages have been injected into your \
                                     context by the system. You can now reference the details \
                                     from that earlier conversation.",
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

                // ── AI-generated tools scaffold ─────────────────────────────
                "ai_tool" => {
                    // Generic entrypoint for AI-generated tools.
                    // Implement `ai_tools::run(&Value, &ToolPolicy) -> Result<String>` in src/tools/ai_tools.rs
                    ai_tools::run(&args, policy).await
                }

                // ── Commit message generation (Task 161) ─────────────────────
                "generate_commit_message" => {
                    // This tool is primarily a convenience for the AI to call itself.
                    // The heavy lifting (git diff + prompt) is done in slash_commands.
                    // Here we just return a short instruction so the model knows
                    // it should use the /commit slash command or call the LLM directly.
                    Ok("Use the `/commit` slash command (or call the model with a git diff) to generate a commit message.".to_string())
                }

                // ── Unknown tool (should be rare due to arbitration) ───────
                unknown => Err(anyhow!("Unknown tool: '{}'", unknown)),
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
pub fn get_tool_definitions() -> Vec<&'static str> {
    vec![
        "read_file",
        "read_multiple_files",
        "list_code_definitions",
        "write_file",
        "replace",
        "list_directory",
        "glob_search",
        "search_file_content",
        "run_shell_command",
        "web_search",
        "web_fetch",
        "save_memory",
        "sleep",
        "synthetic_output",
        "execute_task_graph",
        "task_get",
        "task_create",
        "task_update",
        "enter_plan_mode",
        "exit_plan_mode",
        "enter_worktree",
        "exit_worktree",
        "notebook_edit",
        "execute_skill",
        "list_skills",
        "spawn_agent",
        "send_message",
        "team_create",
        "team_delete",
        "list_agents",
        "get_agent_status",
        "cancel_agent",
        "send_message_in_memory",
        "receive_messages",
        "fork_agent",
        "join_agents",
        "mcp_call",
        "mcp_list",
        "lsp_query",
        "tool_search",
        "cron_create",
        "remote_trigger",
        "recall_context",
        "ai_tool",
        "generate_commit_message",
    ]
}

/// Returns full OpenAI-style JSON tool schemas for every registered tool.
///
/// Each entry has the shape:
/// ```json
/// {"type":"function","function":{"name":"...","description":"...","parameters":{...}}}
/// ```
/// This is the format expected by the Grok/xAI API and by all ACP consumers.
pub fn get_full_tool_definitions() -> Vec<serde_json::Value> {
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
    ]
}

/// Returns the full JSON tool definitions (same as get_full_tool_definitions).
/// This alias is used by all ACP consumers that need the schema for the model.
pub fn get_available_tool_definitions() -> Vec<serde_json::Value> {
    get_full_tool_definitions()
}

/// Returns the built-in tools plus any tools discovered from connected MCP servers.
/// MCP tools are exposed with the server name as a prefix (e.g. "markmap:generate").
/// This is the function that should be used when building the tool list for a session
/// that has MCP servers attached.
pub async fn get_available_tool_definitions_with_mcp(
    mcp_tools: &[(String, crate::mcp::protocol::Tool)],
) -> Vec<serde_json::Value> {
    let mut defs = get_full_tool_definitions();

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
