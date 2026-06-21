//! Tool restriction layer for Explorer mode.

use std::collections::HashSet;

/// Returns the set of tool names that are allowed in Explorer mode.
pub fn allowed_explorer_tools() -> HashSet<&'static str> {
    [
        "fs_glob",
        "fs_read",
        "fs_grep",
        "list_directory",
        "search_file_content",
    ]
    .into_iter()
    .collect()
}

/// Returns true if the given tool name is permitted in Explorer mode.
pub fn is_tool_allowed(name: &str) -> bool {
    allowed_explorer_tools().contains(name)
}

/// Returns true if the tool is a write/patch/execution tool that must be blocked.
pub fn is_write_or_dangerous_tool(name: &str) -> bool {
    matches!(
        name,
        "fs_write"
            | "fs_patch"
            | "shell_exec"
            | "run_terminal_cmd"
            | "edit_file"
            | "apply_patch"
    )
}
