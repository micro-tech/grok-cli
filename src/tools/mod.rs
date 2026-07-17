//! Tool implementations for the Grok-CLI agent runtime.
//!
//! # Module Map
//!
//! | Module | Key functions |
//! |---|---|
//! | [`agent_tools`] | `spawn_agent`, `send_message`, `team_create`, `team_delete` |
//! | [`discovery_tools`] | `tool_search`, `cron_create`, `remote_trigger` |
//! | [`file_tools`] | `read_file`, `write_file`, `list_directory`, `replace`, … |
//! | [`lsp_tools`] | `lsp_query` |
//! | [`mcp_tools`] | `mcp_call` |
//! | [`memory_tools`] | `save_memory` |
//! | [`notebook_tools`] | `notebook_edit` |
//! | [`plan_tools`] | `enter_plan_mode`, `exit_plan_mode`, `enter_worktree`, `exit_worktree` |
//! | [`registry`] | `execute_tool`, `get_tool_definitions`, `get_available_tool_definitions` |
//! | [`shell_tools`] | `run_shell_command` |
//! | [`skill_tools`] | `execute_skill`, `list_available_skills` |
//! | [`system_tools`] | `sleep_for`, `synthetic_output` |
//! | [`task_tools`] | `task_create`, `task_update` |
//! | [`tool_context`] | [`ToolContext`] |
//! | [`tool_error`] | [`ToolError`] |
//! | [`web_tools`] | `web_search`, `web_fetch` |
//!
//! # Quick start
//!
//! ```rust,no_run
//! use grok_cli::tools::{ToolContext, registry};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let ctx = ToolContext::default_for_cwd();
//! let result = registry::execute_tool(
//!     "list_directory",
//!     &serde_json::json!({"path": "."}),
//!     &ctx,
//! ).await?;
//! println!("{result}");
//! # Ok(())
//! # }
//! ```
//!
pub mod agent_tools;
pub mod ai_tools;
pub mod discovery_tools;
pub mod file_tools;
pub mod lsp_tools;
pub mod mcp_tools;
pub mod memory_tools;
pub mod notebook_tools;
pub mod okf_tools;
pub mod plan_tools;
pub mod registry;
pub mod sandbox;
pub mod shell_tools;
pub mod skill_tools;
pub mod system_tools;
pub mod task_graph_tools;
pub mod task_tools;
pub mod tool_arbitration;
pub mod tool_context;
pub mod tool_error;
pub mod web_tools;
pub mod image;

// ── Core re-exports ───────────────────────────────────────────────────────────

pub use registry::{
    execute_tool, get_available_tool_definitions, get_full_tool_definitions, get_tool_definitions,
};
pub use tool_context::ToolContext;
pub use tool_error::ToolError;

// ── File tools ────────────────────────────────────────────────────────────────

pub use file_tools::{
    glob_search, list_code_definitions, list_directory, read_file, read_multiple_files, replace,
    search_file_content, write_file,
};

// ── Shell / system ────────────────────────────────────────────────────────────

pub use shell_tools::run_shell_command;
pub use system_tools::{sleep_for, synthetic_output};

// ── Web ───────────────────────────────────────────────────────────────────────

pub use web_tools::{is_web_search_configured, web_fetch, web_search};

// ── Memory ────────────────────────────────────────────────────────────────────

pub use memory_tools::save_memory;
pub use okf_tools::{okf_lookup, okf_get};

// ── Task management ───────────────────────────────────────────────────────────

pub use task_graph_tools::execute_task_graph;
pub use task_tools::{task_create, task_get, task_update};

// ── Plan mode + worktrees ─────────────────────────────────────────────────────

pub use plan_tools::{enter_plan_mode, enter_worktree, exit_plan_mode, exit_worktree};

// ── Notebook ──────────────────────────────────────────────────────────────────

pub use notebook_tools::notebook_edit;

// ── Skills ────────────────────────────────────────────────────────────────────

pub use skill_tools::{execute_skill, list_available_skills};

// ── Agent coordination ────────────────────────────────────────────────────────

pub use agent_tools::{send_message, spawn_agent, team_create, team_delete};

// ── MCP + LSP ─────────────────────────────────────────────────────────────────

pub use lsp_tools::lsp_query;
pub use mcp_tools::mcp_call;

// ── Discovery ─────────────────────────────────────────────────────────────────

pub use discovery_tools::{cron_create, remote_trigger, tool_search};

// ── Image / Vision ────────────────────────────────────────────────────────────

pub use image::{extract_image_from_message, is_image_path, is_image_url, prepare_image_content, print_image_attached_feedback};

// ── Vision support ────────────────────────────────────────────────────────────

pub mod vision;
pub use vision::{is_vision_model, recommended_vision_model, should_use_vision_model};
mod vision_api;
pub use vision_api::{create_vision_message, message_has_image};

// ── Init command ──────────────────────────────────────────────────────────────

pub mod init;
pub use init::run_init;
