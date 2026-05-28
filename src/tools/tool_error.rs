//! Unified error type for all tool executions.
//!
//! All tool functions return `anyhow::Result<String>` for simplicity and
//! backwards compatibility with existing call-sites. This enum is exposed for
//! callers that need structured error handling (e.g. the registry).
//!
//! The [`format_tool_error_for_llm`] function converts any tool error into a
//! structured, actionable message that the LLM can use to recover — instead of
//! just receiving a raw error string with no guidance.

use thiserror::Error;

/// Structured error type for tool execution failures.
#[derive(Debug, Error)]
pub enum ToolError {
    /// The requested path is outside trusted directories or was explicitly denied.
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// The file or directory was not found at the given path.
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// An underlying I/O operation failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A required argument was missing or had an unexpected type.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// The tool exceeded its time budget.
    #[error("Command timed out after {0}s")]
    Timeout(u64),

    /// A network-level error (Starlink drop, DNS failure, HTTP error, …).
    #[error("Network error: {0}")]
    Network(String),

    /// A regex pattern or other input was syntactically invalid.
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    /// The requested tool name is not registered.
    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    /// Catch-all for unexpected errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// ---------------------------------------------------------------------------
// Error categorisation
// ---------------------------------------------------------------------------

/// High-level category of a tool failure, used to select recovery suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ErrorCategory {
    /// Path is outside the trusted workspace directories.
    AccessDenied,
    /// File or directory does not exist at the given path.
    PathNotFound,
    /// The tool was called with missing or wrong arguments.
    MissingArgument,
    /// The tool name is not registered.
    UnknownTool,
    /// Network request timed out (Starlink / slow connection).
    NetworkTimeout,
    /// General network / HTTP failure.
    NetworkError,
    /// OS-level permission denied (not a workspace policy error).
    OsPermission,
    /// Catch-all.
    Generic,
}

/// Inspect the error string and classify it into an [`ErrorCategory`].
fn categorise(error: &str) -> ErrorCategory {
    let e = error.to_lowercase();

    if e.contains("external access is disabled")
        || e.contains("access denied")
        || e.contains("trusted director")
        || e.contains("outside the trusted")
    {
        return ErrorCategory::AccessDenied;
    }

    if e.contains("file not found")
        || e.contains("no such file")
        || e.contains("cannot find the path")
        || e.contains("cannot find the file")
        || e.contains("os error 2")
        || e.contains("os error 3")
        || e.contains("failed to resolve path")
        || e.contains("path does not exist")
    {
        return ErrorCategory::PathNotFound;
    }

    if e.contains("missing ")
        || e.contains("invalid argument")
        || e.contains("missing path")
        || e.contains("missing query")
        || e.contains("missing pattern")
        || e.contains("missing content")
        || e.contains("missing url")
    {
        return ErrorCategory::MissingArgument;
    }

    if e.contains("unknown tool") || e.contains("not a registered tool") {
        return ErrorCategory::UnknownTool;
    }

    if e.contains("timed out") || e.contains("timeout") || e.contains("deadline") {
        return ErrorCategory::NetworkTimeout;
    }

    if e.contains("network error")
        || e.contains("connection")
        || e.contains("dns")
        || e.contains("http error")
        || e.contains("reqwest")
    {
        return ErrorCategory::NetworkError;
    }

    if e.contains("permission denied") || e.contains("os error 5") || e.contains("os error 13") {
        return ErrorCategory::OsPermission;
    }

    ErrorCategory::Generic
}

/// Recovery suggestions to append to the error message, keyed by category.
fn recovery_suggestions(
    category: &ErrorCategory,
    tool_name: &str,
    args: &serde_json::Value,
) -> String {
    let path_hint = args["path"]
        .as_str()
        .or_else(|| args["url"].as_str())
        .or_else(|| args["pattern"].as_str())
        .unwrap_or("");

    match category {
        ErrorCategory::AccessDenied => {
            let mut msg = String::from(
                "Suggestions:\n\
                • Use list_directory(\".\") to see files available inside the workspace.\n\
                • Use glob_search with pattern \"**/*\" to locate files by name.\n\
                • Check that the path is relative to the project root, not an absolute path.",
            );
            if !path_hint.is_empty() {
                msg.push_str(&format!(
                    "\n• The path you used was: \"{}\". \
                    If this file is in your project, make sure Grok was launched from the \
                    correct project root directory.",
                    path_hint
                ));
            }
            msg.push_str(
                "\n• If the file is genuinely outside the project, ask the user to \
                enable external access in their Grok configuration.",
            );
            msg
        }

        ErrorCategory::PathNotFound => {
            let mut msg = String::from(
                "Suggestions:\n\
                • The path does not exist. Verify the exact file or directory name.\n\
                • Use list_directory(\".\") to browse the project root.\n\
                • Use glob_search with a pattern like \"**/*.rs\" to find files by extension.\n\
                • Use search_file_content to search by content if you are unsure of the filename.",
            );
            if !path_hint.is_empty() {
                msg.push_str(&format!(
                    "\n• Path that failed: \"{}\". Check for typos or incorrect relative path.",
                    path_hint
                ));
            }
            msg
        }

        ErrorCategory::MissingArgument => format!(
            "Suggestions:\n\
            • The tool \"{}\" was called with missing or incorrect arguments.\n\
            • Check the required argument names and types for this tool.\n\
            • Do not pass null or empty strings for required fields.\n\
            • Retry with all required arguments populated.",
            tool_name
        ),

        ErrorCategory::UnknownTool => String::from(
            "Suggestions:\n\
            • The tool name is not recognised. Available tools include:\n\
              read_file, write_file, list_directory, glob_search, search_file_content,\n\
              replace, run_shell_command, web_search, web_fetch, save_memory,\n\
              read_multiple_files, list_code_definitions.\n\
            • Check for typos in the tool name and retry.",
        ),

        ErrorCategory::NetworkTimeout => String::from(
            "Suggestions:\n\
            • The request timed out (possibly a Starlink satellite handover or slow connection).\n\
            • Wait a few seconds and retry the same tool call.\n\
            • If the problem persists, ask the user to check their internet connection.",
        ),

        ErrorCategory::NetworkError => String::from(
            "Suggestions:\n\
            • A network error occurred. This may be a temporary connectivity issue.\n\
            • Retry the request after a short delay.\n\
            • For web_search or web_fetch, verify the URL or query is valid.\n\
            • If the problem persists, inform the user that the network is unavailable.",
        ),

        ErrorCategory::OsPermission => format!(
            "Suggestions:\n\
            • The operating system denied access to the file or directory.\n\
            • The file at \"{}\" may be locked by another process or require elevated privileges.\n\
            • Try a different path, or ask the user to check file permissions.",
            path_hint
        ),

        ErrorCategory::Generic => format!(
            "Suggestions:\n\
            • An unexpected error occurred in tool \"{}\". \
            Review the error message above for details.\n\
            • If this is a file operation, verify the path exists and is accessible.\n\
            • If this is a network operation, check connectivity and retry.\n\
            • If the error persists, report it to the user and try an alternative approach.",
            tool_name
        ),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Format a tool failure into a structured, LLM-readable error message.
///
/// Both the ACP tool loop (`src/acp/mod.rs`) and the CPU router tool loop
/// (`src/router/cpu_router.rs`) call this function so the model always
/// receives the same style of actionable guidance when a tool fails.
///
/// # Arguments
///
/// * `tool_name` — the registered tool name (e.g. `"read_file"`)
/// * `args`      — the JSON arguments the tool was called with
/// * `error`     — the error message string produced by the tool
///
/// # Output format
///
/// ```text
/// TOOL ERROR — read_file
/// Arguments: {"path": "src/foo.rs"}
/// Error:     Access denied: External access is disabled in configuration
///
/// Cause:     The path is outside the trusted workspace directories.
/// Suggestions:
/// • Use list_directory(".") to see files available inside the workspace.
/// • ...
/// ```
pub fn format_tool_error_for_llm(tool_name: &str, args: &serde_json::Value, error: &str) -> String {
    let category = categorise(error);

    let cause = match &category {
        ErrorCategory::AccessDenied => {
            "The path is outside the trusted workspace directories, or external \
            access is disabled in the Grok configuration."
        }
        ErrorCategory::PathNotFound => {
            "The specified file or directory does not exist at the given path."
        }
        ErrorCategory::MissingArgument => {
            "The tool was called with one or more missing or invalid arguments."
        }
        ErrorCategory::UnknownTool => "The tool name is not registered in Grok CLI.",
        ErrorCategory::NetworkTimeout => {
            "The network request exceeded the timeout limit (Starlink drop or slow connection)."
        }
        ErrorCategory::NetworkError => {
            "A network-level error prevented the request from completing."
        }
        ErrorCategory::OsPermission => {
            "The operating system denied permission to access the file or resource."
        }
        ErrorCategory::Generic => "An unexpected error occurred during tool execution.",
    };

    let args_display = if args.is_null() || args == &serde_json::Value::Object(Default::default()) {
        "(none)".to_string()
    } else {
        args.to_string()
    };

    let suggestions = recovery_suggestions(&category, tool_name, args);

    format!(
        "TOOL ERROR — {tool_name}\n\
        Arguments: {args_display}\n\
        Error:     {error}\n\
        \n\
        Cause:     {cause}\n\
        {suggestions}\n\
        \n\
        Do NOT repeat the exact same tool call. Use the suggestions above to \
        adjust your approach before retrying."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn categorises_access_denied() {
        assert_eq!(
            categorise("Access denied: External access is disabled in configuration"),
            ErrorCategory::AccessDenied
        );
    }

    #[test]
    fn categorises_path_not_found_os_error_3() {
        assert_eq!(
            categorise(
                "Failed to resolve path 'src/foo.rs': The system cannot find the path specified. (os error 3)"
            ),
            ErrorCategory::PathNotFound
        );
    }

    #[test]
    fn categorises_path_not_found_os_error_2() {
        assert_eq!(
            categorise("No such file or directory (os error 2)"),
            ErrorCategory::PathNotFound
        );
    }

    #[test]
    fn categorises_missing_argument() {
        assert_eq!(
            categorise("Missing path argument"),
            ErrorCategory::MissingArgument
        );
    }

    #[test]
    fn categorises_unknown_tool() {
        assert_eq!(
            categorise("Unknown tool: frobulate"),
            ErrorCategory::UnknownTool
        );
    }

    #[test]
    fn categorises_network_timeout() {
        assert_eq!(
            categorise("request timed out after 30s"),
            ErrorCategory::NetworkTimeout
        );
    }

    #[test]
    fn categorises_network_error() {
        assert_eq!(
            categorise("Network error: connection refused"),
            ErrorCategory::NetworkError
        );
    }

    #[test]
    fn categorises_os_permission() {
        assert_eq!(
            categorise("Permission denied (os error 13)"),
            ErrorCategory::OsPermission
        );
    }

    #[test]
    fn categorises_generic_fallback() {
        assert_eq!(
            categorise("Something completely unexpected happened"),
            ErrorCategory::Generic
        );
    }

    #[test]
    fn format_contains_tool_name_and_error() {
        let msg = format_tool_error_for_llm(
            "read_file",
            &json!({"path": "src/main.rs"}),
            "Access denied: External access is disabled",
        );
        assert!(msg.contains("TOOL ERROR — read_file"), "missing header");
        assert!(msg.contains("Access denied"), "missing raw error");
        assert!(msg.contains("list_directory"), "missing suggestion");
        assert!(msg.contains("Do NOT repeat"), "missing retry warning");
    }

    #[test]
    fn format_path_not_found_includes_path_hint() {
        let msg = format_tool_error_for_llm(
            "read_file",
            &json!({"path": "src/missing.rs"}),
            "Failed to resolve path 'src/missing.rs': os error 3",
        );
        assert!(
            msg.contains("src/missing.rs"),
            "path hint missing from suggestion"
        );
        assert!(
            msg.contains("glob_search"),
            "glob_search suggestion missing"
        );
    }

    #[test]
    fn format_unknown_tool_lists_available_tools() {
        let msg = format_tool_error_for_llm(
            "frobulate",
            &serde_json::Value::Null,
            "Unknown tool: frobulate",
        );
        assert!(msg.contains("read_file"), "tool list missing");
        assert!(msg.contains("web_search"), "tool list incomplete");
    }

    #[test]
    fn format_timeout_suggests_retry() {
        let msg = format_tool_error_for_llm(
            "web_fetch",
            &json!({"url": "https://example.com"}),
            "request timed out after 30s",
        );
        assert!(msg.contains("Starlink"), "Starlink hint missing");
        assert!(msg.contains("retry"), "retry suggestion missing");
    }
}
