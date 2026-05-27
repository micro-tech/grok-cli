//! Error types for the context subsystem.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("token budget exceeded: used {used} > max {max}")]
    BudgetExceeded { used: u32, max: u32 },

    #[error("invalid token count: {0}")]
    InvalidTokenCount(u32),

    #[error("prompt too large after delta application")]
    PromptTooLarge,

    #[error("cache key generation failed")]
    CacheKeyError,

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type ContextResult<T> = Result<T, ContextError>;
