//! Unified error type for all tool executions.
//!
//! All tool functions return `anyhow::Result<String>` for simplicity and
//! backwards compatibility with existing call-sites. This enum is exposed for
//! callers that need structured error handling (e.g. the registry).

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
