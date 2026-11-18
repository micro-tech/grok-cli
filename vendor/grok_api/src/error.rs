//! Error types for the Grok API client

/// Result type alias for Grok API operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the Grok API client
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Network error (connection issues, timeouts, etc.)
    #[error("Network error: {0}")]
    Network(String),

    /// HTTP error with status code
    #[error("HTTP error {status}: {message}")]
    Http { status: u16, message: String },

    /// Authentication failed (invalid API key)
    #[error("Authentication failed: Invalid or missing API key")]
    Authentication,

    /// Rate limit exceeded. Carries the response body and the `Retry-After`
    /// header (seconds) when the server sent one, so callers can honor it.
    /// (H5 backport — the published 0.2.4 discards both.)
    #[error("Rate limit exceeded. Please try again later. Body: {body}. Retry-After: {retry_after_secs:?}")]
    RateLimit {
        /// Raw response body from the 429 (often a JSON error with a message).
        body: String,
        /// `Retry-After` header value in seconds, when present.
        retry_after_secs: Option<u64>,
    },

    /// Model not found or not available
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Invalid request (bad parameters, malformed request, etc.)
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Server error (5xx responses)
    #[error("Server error: {0}")]
    ServerError(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Request timed out. The `u64` is the configured timeout in seconds,
    /// or `0` if the exact value is not known at the call site.
    #[error("Request timed out ({0}s limit)")]
    Timeout(u64),

    /// Connection dropped (Starlink-specific)
    #[error("Connection dropped - possible network instability")]
    ConnectionDropped,

    /// Maximum retries exceeded
    #[error("Maximum retries exceeded ({0} attempts)")]
    MaxRetriesExceeded(u32),

    /// Empty or invalid API key
    #[error("API key cannot be empty")]
    EmptyApiKey,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Network(_)
                | Error::Timeout(_)
                | Error::ConnectionDropped
                | Error::Http {
                    status: 502 | 503 | 504 | 520 | 521 | 522 | 523 | 524,
                    ..
                }
        )
    }

    /// Check if the error is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Error::RateLimit { .. })
    }

    /// Check if the error is an authentication error
    pub fn is_auth_error(&self) -> bool {
        matches!(self, Error::Authentication)
    }

    /// Check if the error indicates a Starlink network drop
    pub fn is_starlink_drop(&self) -> bool {
        match self {
            Error::ConnectionDropped => true,
            Error::Network(msg) => {
                // Error strings from std/tokio/reqwest are lowercase ASCII;
                // searching the original string avoids a heap allocation.
                msg.contains("connection reset")
                    || msg.contains("broken pipe")
                    || msg.contains("network unreachable")
            }
            _ => false,
        }
    }

    /// Convert from reqwest::Error
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        let error_string = err.to_string();

        // reqwest error strings are lowercase ASCII — no need to allocate a
        // lowercased copy; search the original directly.
        if err.is_timeout() || error_string.contains("timeout") {
            // Exact configured timeout not available here; callers that know
            // their timeout can construct Error::Timeout directly.
            return Error::Timeout(0);
        }

        if err.is_connect() || error_string.contains("connection") {
            return Error::Network(error_string);
        }

        // Check for status codes
        if let Some(status) = err.status() {
            let code = status.as_u16();
            return match code {
                401 => Error::Authentication,
                // No body/header available from a reqwest::Error — preserve
                // the shape with empty values (H5 backport).
                429 => Error::RateLimit {
                    body: String::new(),
                    retry_after_secs: None,
                },
                404 => Error::InvalidRequest("Endpoint not found".to_string()),
                400 => Error::InvalidRequest(error_string),
                500..=599 => Error::ServerError(error_string),
                _ => Error::Http {
                    status: code,
                    message: error_string,
                },
            };
        }

        // Default to network error
        Error::Network(error_string)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::from_reqwest(err)
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        assert!(Error::Network("connection failed".to_string()).is_retryable());
        assert!(Error::Timeout(0).is_retryable());  // 0 = sentinel for unknown duration
        assert!(Error::Timeout(90).is_retryable()); // explicit duration also retryable
        assert!(Error::ConnectionDropped.is_retryable());
        assert!(!Error::Authentication.is_retryable());
        assert!(
            !Error::RateLimit {
                body: String::new(),
                retry_after_secs: None
            }
            .is_retryable()
        );
    }

    #[test]
    fn test_error_is_starlink_drop() {
        assert!(Error::ConnectionDropped.is_starlink_drop());
        assert!(Error::Network("connection reset by peer".to_string()).is_starlink_drop());
        assert!(Error::Network("broken pipe".to_string()).is_starlink_drop());
        assert!(!Error::Authentication.is_starlink_drop());
    }

    #[test]
    fn test_error_display() {
        let err = Error::Authentication;
        assert_eq!(
            err.to_string(),
            "Authentication failed: Invalid or missing API key"
        );

        let err = Error::ModelNotFound("grok-99".to_string());
        assert_eq!(err.to_string(), "Model not found: grok-99");
    }
}
