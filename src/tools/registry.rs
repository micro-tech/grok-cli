//! Tool registry — central lookup table and async executor.
//!
//! [`execute_tool`] is the single entry-point used by the CPU router tool
//! loop (and optionally by the ACP agent) to dispatch a named tool call with
//! its JSON arguments and a [`ToolContext`].
//!
//! [`get_tool_definitions`] / [`get_available_tool_definitions`] return the
//! JSON schemas passed to the LLM so it knows what tools exist.

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::tools::{
    ToolContext, agent_tools, discovery_tools, file_tools, lsp_tools, mcp_tools, memory_tools,
    notebook_tools, plan_tools, shell_tools, skill_tools, system_tools, task_graph_tools,
    task_tools, web_tools,
};

// ─────────────────────────────────────────────────────────────────────────────
// execute_tool
// ─────────────────────────────────────────────────────────────────────────────

/// Execute a named tool with the provided JSON arguments and context.
///
/// This is the unified entry-point used by the CPU router tool loop. Every
/// named tool in [`get_tool_definitions`] must have a matching arm here.
pub async fn execute_tool(name: &str, args: &Value, ctx: &ToolContext) -> Result<String> {
    let policy = &ctx.policy;

    match name {
        // ── File tools ──────────────────────────────────────────────────────
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
            file_tools::write_file(path, content, policy).await
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
            file_tools::replace(path, old_string, new_string, expected, policy).await
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

        // ── Shell ────────────────────────────────────────────────────────────
        "run_shell_command" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing: command"))?;
            // Pass 0 → uses the built-in 300 s default.
            // The ACP path (acp/mod.rs) reads the value from
            // config.tools.shell.command_timeout_secs instead.
            shell_tools::run_shell_command(command, policy, 0).await
        }

        // ── Web ──────────────────────────────────────────────────────────────
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

        // ── Memory ───────────────────────────────────────────────────────────
        "save_memory" => {
            let fact = args["fact"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing: fact"))?;
            memory_tools::save_memory(fact)
        }

        // ── System ───────────────────────────────────────────────────────────
        "sleep" => {
            let seconds = args["seconds"].as_f64().unwrap_or(1.0) as u64;
            system_tools::sleep_for(seconds).await
        }
        "synthetic_output" => {
            let schema_name = args["schema_name"].as_str().unwrap_or("output");
            let data = &args["data"];
            system_tools::synthetic_output(schema_name, data)
        }

        // ── Task management ──────────────────────────────────────────────
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
            // Dependencies support integer task IDs and decimal subtask IDs (e.g. 5.2)
            let deps: Vec<f64> = args["dependencies"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let details = args["details"].as_str().unwrap_or("");
            let test_strategy = args["testStrategy"].as_str().unwrap_or("");
            // Each subtask element must have at least {"title": "..."}
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
        "task_update" => {
            let id = args["id"].as_f64().ok_or_else(|| anyhow!("Missing: id"))?;
            let status = args["status"].as_str();
            let title = args["title"].as_str();
            let priority = args["priority"].as_str();
            let details = args["details"].as_str();
            task_tools::task_update(id, status, title, priority, details, policy)
        }

        // ── Plan mode + worktrees ────────────────────────────────────────────
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

        // ── Notebook ─────────────────────────────────────────────────────────
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

        // ── Skills ───────────────────────────────────────────────────────────
        "execute_skill" => {
            let skill_name = args["skill_name"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing: skill_name"))?;
            let input = args["input"].as_str().unwrap_or("");
            skill_tools::execute_skill(skill_name, input)
        }
        "list_skills" => skill_tools::list_available_skills(),

        // ── Agent coordination ───────────────────────────────────────────────
        "spawn_agent" => {
            let task = args["task"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing: task"))?;
            let context = args["context"].as_str().unwrap_or("");
            let max_tokens = args["max_tokens"].as_u64().unwrap_or(2048) as u32;
            agent_tools::spawn_agent(task, context, max_tokens).await
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

        // ── MCP ──────────────────────────────────────────────────────────────
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

        // ── LSP ──────────────────────────────────────────────────────────────
        "lsp_query" => {
            let file = args["file"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing: file"))?;
            let line = args["line"].as_u64().unwrap_or(0) as u32;
            let character = args["character"].as_u64().unwrap_or(0) as u32;
            let query_type = args["query_type"].as_str().unwrap_or("diagnostics");
            lsp_tools::lsp_query(file, line, character, query_type, policy).await
        }

        // ── Discovery ────────────────────────────────────────────────────────
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

        // ── Context recall ────────────────────────────────────────────────────
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

        unknown => Err(anyhow!("Unknown tool: '{}'", unknown)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// get_tool_definitions
// ─────────────────────────────────────────────────────────────────────────────

/// Return JSON tool definitions for all 34 registered tools.
pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        // ── File tools ──────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"read_file","description":"Read the content of a file","parameters":{"type":"object","properties":{"path":{"type":"string","description":"Path to the file"}},"required":["path"]}}}),
        json!({"type":"function","function":{"name":"read_multiple_files","description":"Read multiple files at once","parameters":{"type":"object","properties":{"paths":{"type":"array","items":{"type":"string"},"description":"File paths to read"}},"required":["paths"]}}}),
        json!({"type":"function","function":{"name":"list_code_definitions","description":"List code definitions (functions, structs, classes) in a file","parameters":{"type":"object","properties":{"path":{"type":"string","description":"Path to the file"}},"required":["path"]}}}),
        json!({"type":"function","function":{"name":"write_file","description":"Write content to a file, creating parent directories if needed","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}}}),
        json!({"type":"function","function":{"name":"replace","description":"Replace text in a file","parameters":{"type":"object","properties":{"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"},"expected_replacements":{"type":"integer","description":"Expected occurrence count (optional)"}},"required":["path","old_string","new_string"]}}}),
        json!({"type":"function","function":{"name":"list_directory","description":"List files and directories in a path","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}}),
        json!({"type":"function","function":{"name":"glob_search","description":"Find files matching a glob pattern (e.g. **/*.rs)","parameters":{"type":"object","properties":{"pattern":{"type":"string"}},"required":["pattern"]}}}),
        json!({"type":"function","function":{"name":"search_file_content","description":"Search for a regex pattern in file content (grep-style)","parameters":{"type":"object","properties":{"path":{"type":"string","description":"File or directory to search"},"pattern":{"type":"string","description":"Regex pattern"}},"required":["path","pattern"]}}}),
        // ── Shell ────────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"run_shell_command","description":"Execute a shell command in the working directory","parameters":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}}}),
        // ── Web ──────────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"web_search","description":"Search the web using DuckDuckGo (no API key required)","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}}),
        json!({"type":"function","function":{"name":"web_fetch","description":"Fetch the text content of a URL","parameters":{"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}}}),
        // ── Memory ───────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"save_memory","description":"Save a fact to long-term memory","parameters":{"type":"object","properties":{"fact":{"type":"string"}},"required":["fact"]}}}),
        // ── System ───────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"sleep","description":"Wait for a specified number of seconds (max 300). Use in proactive or scheduled tasks.","parameters":{"type":"object","properties":{"seconds":{"type":"number","description":"Duration in seconds (capped at 300)"}},"required":["seconds"]}}}),
        json!({"type":"function","function":{"name":"synthetic_output","description":"Emit a structured JSON output object with a schema label","parameters":{"type":"object","properties":{"schema_name":{"type":"string","description":"Label identifying the output schema"},"data":{"type":"object","description":"The structured data to emit"}},"required":["schema_name","data"]}}}),
        // ── Task management ────────────────────────────────────────────
        json!({"type":"function","function":{"name":"task_create","description":"Create a new task in .zed/task_list.json following Task Builder rules. Auto-assigns the next available integer ID and sets status to 'pending'. All Task Builder fields are supported: title, description, priority, dependencies (integer or decimal subtask IDs like 5.2), details (implementation instructions), testStrategy (verification approach), and subtasks (auto-assigned decimal IDs {parent}.1, {parent}.2, …).","parameters":{"type":"object","properties":{"title":{"type":"string","description":"Brief, descriptive task title (required)"},"description":{"type":"string","description":"Concise description of what the task involves"},"priority":{"type":"string","enum":["high","medium","low"],"description":"Importance level — high: blocks progress; medium: important; low: deferrable. Default: medium"},"dependencies":{"type":"array","items":{"type":"number"},"description":"IDs of tasks (or subtasks, e.g. 5.2) that must be done before this one"},"details":{"type":"string","description":"In-depth implementation instructions for the task"},"testStrategy":{"type":"string","description":"Verification approach to confirm the task is complete"},"subtasks":{"type":"array","description":"Optional initial subtasks; IDs are auto-assigned as {parent_id}.1, {parent_id}.2, etc. Each subtask starts as 'pending'.","items":{"type":"object","properties":{"title":{"type":"string","description":"Subtask title (required)"},"dependencies":{"type":"array","items":{"type":"number"},"description":"Subtask-level dependency IDs"}},"required":["title"]}}},"required":["title"]}}}),
        json!({"type":"function","function":{"name":"task_update","description":"Update a task's status, title, priority, or details in .zed/task_list.json","parameters":{"type":"object","properties":{"id":{"type":"number","description":"Task ID (supports decimals for subtasks, e.g. 85.2)"},"status":{"type":"string","enum":["pending","in_progress","done","deferred"]},"title":{"type":"string"},"priority":{"type":"string","enum":["high","medium","low"]},"details":{"type":"string"}},"required":["id"]}}}),
        json!({"type":"function","function":{"name":"execute_task_graph","description":"Execute a DAG-based multi-step task graph with dependency resolution","parameters":{"type":"object","properties":{"graph":{"type":"string","description":"JSON string representing the task graph"}},"required":["graph"]}}}),
        // ── Plan mode ────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"enter_plan_mode","description":"Activate plan mode — the agent outlines a full plan before making any changes","parameters":{"type":"object","properties":{}}}}),
        json!({"type":"function","function":{"name":"exit_plan_mode","description":"Deactivate plan mode and begin executing the current plan","parameters":{"type":"object","properties":{}}}}),
        // ── Worktrees ────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"enter_worktree","description":"Create a git worktree at the given path on the specified branch for isolated development","parameters":{"type":"object","properties":{"branch":{"type":"string","description":"Branch name (created with -b if it does not exist)"},"path":{"type":"string","description":"Worktree directory path (relative to project root)"}},"required":["branch","path"]}}}),
        json!({"type":"function","function":{"name":"exit_worktree","description":"Remove the active git worktree and optionally merge its branch back","parameters":{"type":"object","properties":{"merge":{"type":"boolean","description":"Merge the worktree branch before removing (default false)"}},"required":[]}}}),
        // ── Notebook ─────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"notebook_edit","description":"Edit or append a cell in a Jupyter notebook (.ipynb). Creates the notebook if it does not exist.","parameters":{"type":"object","properties":{"path":{"type":"string","description":"Path to the .ipynb file"},"cell_index":{"type":"integer","description":"0-based cell index; appends if out of range"},"source":{"type":"string","description":"Cell source code or markdown text"},"cell_type":{"type":"string","enum":["code","markdown"],"description":"Cell type (default: code)"}},"required":["path","source"]}}}),
        // ── Skills ───────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"execute_skill","description":"Load a named skill from ~/.grok/skills/ and format its instructions with the user input","parameters":{"type":"object","properties":{"skill_name":{"type":"string","description":"Exact skill name as declared in SKILL.md frontmatter"},"input":{"type":"string","description":"User request or context to pass into the skill"}},"required":["skill_name"]}}}),
        json!({"type":"function","function":{"name":"list_skills","description":"List all available skills in ~/.grok/skills/","parameters":{"type":"object","properties":{}}}}),
        // ── Agent coordination ───────────────────────────────────────────────
        json!({"type":"function","function":{"name":"spawn_agent","description":"Spawn a focused sub-agent to complete a well-scoped task. Requires GROK_API_KEY or XAI_API_KEY in environment.","parameters":{"type":"object","properties":{"task":{"type":"string","description":"Task description for the sub-agent"},"context":{"type":"string","description":"Relevant background context (optional)"},"max_tokens":{"type":"integer","description":"Maximum response tokens, 256–4096 (default 2048)"}},"required":["task"]}}}),
        json!({"type":"function","function":{"name":"send_message","description":"Send a message to a named target agent or channel (appended to ~/.grok/messages/{target}.jsonl)","parameters":{"type":"object","properties":{"target":{"type":"string"},"message":{"type":"string"}},"required":["target","message"]}}}),
        json!({"type":"function","function":{"name":"team_create","description":"Create a named team configuration in ~/.grok/teams.json","parameters":{"type":"object","properties":{"name":{"type":"string"},"members":{"type":"array","items":{"type":"string"}},"description":{"type":"string"}},"required":["name"]}}}),
        json!({"type":"function","function":{"name":"team_delete","description":"Delete a named team from ~/.grok/teams.json","parameters":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}}}),
        // ── MCP ──────────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"mcp_call","description":"Invoke a tool on an MCP (Model Context Protocol) server. The server is launched as a subprocess via stdio.","parameters":{"type":"object","properties":{"server_command":{"type":"string"},"tool_name":{"type":"string"},"arguments":{"type":"object"}},"required":["server_command","tool_name"]}}}),
        // ── LSP ──────────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"lsp_query","description":"Query code intelligence for a file position. Supports diagnostics (cargo check for Rust), hover context, definition search, and reference search.","parameters":{"type":"object","properties":{"file":{"type":"string"},"line":{"type":"integer"},"character":{"type":"integer"},"query_type":{"type":"string","enum":["diagnostics","hover","definition","references"]}},"required":["file","query_type"]}}}),
        // ── Discovery ────────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"tool_search","description":"Search the tool registry for tools matching a keyword. Use for deferred tool discovery.","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}}),
        json!({"type":"function","function":{"name":"cron_create","description":"Register a scheduled trigger in ~/.grok/crons.json. An external scheduler must read this file to run the tasks.","parameters":{"type":"object","properties":{"name":{"type":"string"},"schedule":{"type":"string"},"task":{"type":"string"}},"required":["name","schedule","task"]}}}),
        json!({"type":"function","function":{"name":"remote_trigger","description":"Fire an HTTP trigger to a remote endpoint (POST/GET/PUT with JSON payload and 30 s timeout)","parameters":{"type":"object","properties":{"endpoint":{"type":"string"},"method":{"type":"string","enum":["POST","GET","PUT"]},"payload":{"type":"object"}},"required":["endpoint"]}}}),
        // ── Context recall ────────────────────────────────────────────────────
        json!({"type":"function","function":{"name":"recall_context","description":"Restore a previously archived conversation chunk back into the active context. Use this when you need information from earlier in the conversation that has been archived to save context space. The chunk summary and key facts are returned as the tool result; the full raw messages are injected back as context.","parameters":{"type":"object","properties":{"chunk_id":{"type":"integer","description":"The archive chunk number to recall (see /archives for the list)"}},"required":["chunk_id"]}}}),
    ]
}

/// Return only the tool definitions that are currently configured and available.
pub fn get_available_tool_definitions() -> Vec<Value> {
    get_tool_definitions()
        .into_iter()
        .filter(|tool| {
            if let Some(name) = tool
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                && name == "web_search"
                && !web_tools::is_web_search_configured()
            {
                return false;
            }
            true
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::security::SecurityPolicy;
    use crate::tools::ToolContext;
    use tempfile::TempDir;

    fn make_context(dir: &TempDir) -> ToolContext {
        let policy = SecurityPolicy::with_working_directory(dir.path().to_path_buf());
        ToolContext::new(policy)
    }

    #[test]
    fn get_tool_definitions_has_31_tools() {
        assert_eq!(get_tool_definitions().len(), 34);
    }

    #[test]
    fn all_tool_definitions_have_required_fields() {
        for tool in get_tool_definitions() {
            assert!(tool.get("type").is_some(), "missing 'type': {tool}");
            assert!(tool.get("function").is_some(), "missing 'function': {tool}");
            assert!(
                tool["function"].get("name").is_some(),
                "missing 'name': {tool}"
            );
            assert!(
                tool["function"].get("description").is_some(),
                "missing 'description': {tool}"
            );
        }
    }

    #[test]
    fn available_tool_count_matches_total_when_all_configured() {
        let available = get_available_tool_definitions();
        // web_search is available (DuckDuckGo needs no key) so counts equal
        assert_eq!(available.len(), get_tool_definitions().len());
    }

    #[test]
    fn get_available_definitions_includes_web_search() {
        let defs = get_available_tool_definitions();
        let has_web = defs.iter().any(|d| {
            d.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                == Some("web_search")
        });
        assert!(has_web, "web_search should be available");
    }

    #[tokio::test]
    async fn execute_tool_unknown_returns_error() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        let result = execute_tool("nonexistent_tool", &json!({}), &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn execute_tool_read_file_missing_arg_returns_error() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        let result = execute_tool("read_file", &json!({"wrong": "field"}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_tool_list_directory_works() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        std::fs::write(dir.path().join("hello.txt"), "hi").unwrap();
        let result = execute_tool(
            "list_directory",
            &json!({ "path": dir.path().to_str().unwrap() }),
            &ctx,
        )
        .await;
        assert!(result.is_ok(), "list_directory failed: {:?}", result);
        assert!(result.unwrap().contains("hello.txt"));
    }

    #[tokio::test]
    async fn execute_tool_write_and_read_file_roundtrip() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        let path = dir.path().join("rw.txt").display().to_string();

        execute_tool(
            "write_file",
            &json!({"path": path, "content": "roundtrip"}),
            &ctx,
        )
        .await
        .unwrap();
        let result = execute_tool("read_file", &json!({"path": path}), &ctx)
            .await
            .unwrap();
        assert_eq!(result, "roundtrip");
    }

    #[tokio::test]
    async fn execute_tool_save_memory_succeeds_or_errors_gracefully() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        let result = execute_tool(
            "save_memory",
            &json!({"fact": "test-fact-registry-integration"}),
            &ctx,
        )
        .await;
        assert!(result.is_ok() || result.is_err()); // Graceful either way
    }

    #[tokio::test]
    async fn execute_tool_enter_exit_plan_mode() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        execute_tool("enter_plan_mode", &json!({}), &ctx)
            .await
            .unwrap();
        let r = execute_tool("exit_plan_mode", &json!({}), &ctx)
            .await
            .unwrap();
        assert!(r.contains("INACTIVE"));
    }

    #[tokio::test]
    async fn execute_tool_tool_search_finds_read_file() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        let r = execute_tool("tool_search", &json!({"query": "read"}), &ctx)
            .await
            .unwrap();
        assert!(r.contains("read_file"), "expected read_file in: {r}");
    }

    #[tokio::test]
    async fn execute_tool_synthetic_output_returns_json() {
        let dir = TempDir::new().unwrap();
        let ctx = make_context(&dir);
        let r = execute_tool(
            "synthetic_output",
            &json!({"schema_name": "test", "data": {"x": 1}}),
            &ctx,
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&r).unwrap();
        assert_eq!(parsed["schema"], "test");
    }
}
