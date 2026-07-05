//! Safety error types for Grok-CLI safety system.

use thiserror::Error;

/// Errors returned by safety validators and hooks.
#[derive(Debug, Error)]
pub enum SafetyError {
    #[error("Full file replacement >200 lines is not allowed")]
    FullReplacementTooLarge,

    #[error("Edit would remove {0:.0}% of the file (>40% limit)")]
    ExcessiveRemoval(f32),

    #[error("Refusing to write empty file over existing content")]
    EmptyFileOverwrite,

    #[error("Refusing write that would make file >10x larger")]
    FileSizeExplosion,

    #[error("Content contains binary junk")]
    BinaryJunk,

    #[error("Invalid {format} syntax")]
    InvalidSyntax { format: String },

    #[error("Refusing to write >200k characters in a single operation")]
    ContentTooLarge,

    #[error("Target is .json but content is not valid JSON")]
    InvalidJsonTarget,

    #[error("SessionDNA shows repeated write failures. Confirm before proceeding.")]
    RepeatedFailuresRequireConfirmation,

    #[error("About to DELETE {path}. Confirm?")]
    DeleteRequiresConfirmation { path: String },

    #[error("{0}")]
    Custom(String),
}

impl SafetyError {
    /// Convert the error into a human-readable message (for legacy String returns).
    pub fn to_message(&self) -> String {
        self.to_string()
    }
}
