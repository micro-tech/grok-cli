use thiserror::Error;

/// All errors that can be produced by the CPU router or any backend.
#[derive(Debug, Error)]
pub enum RouterError {
    /// No registered backend is able to handle the requested model.
    #[error("Backend unavailable for model: {0}")]
    BackendUnavailable(String),

    /// The backend returned an application-level error.
    #[error("Backend error: {0}")]
    BackendError(String),

    /// A JSON serialization / deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// A network-level failure (timeout, connection refused, Starlink drop, …).
    /// The inner string includes the attempt number so callers can log it.
    #[error("Network error: {0}")]
    Network(String),

    /// The backend rejected the request due to invalid or missing credentials.
    #[error("Authentication error: {0}")]
    Auth(String),

    /// The backend is rate-limiting the caller.  The caller should back off
    /// before retrying.
    #[error("Rate limit exceeded")]
    RateLimit,

    /// A tool execution failed during the tool loop.
    ///
    /// The inner string contains the tool name and the underlying error message.
    #[error("Tool execution error: {0}")]
    ToolError(String),

    /// The tool-execution loop hit the maximum iteration cap without the LLM
    /// returning a final text response.
    #[error("Max tool loop iterations reached ({0} iterations)")]
    MaxToolIterations(u32),

    /// Catch-all for truly unexpected situations.
    #[error("Unknown router error")]
    Unknown,
}
