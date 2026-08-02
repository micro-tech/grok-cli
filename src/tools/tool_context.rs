//! Context passed into every tool execution.
//!
//! [`ToolContext`] bundles together the [`SecurityPolicy`] governing path
//! access and shell validation. It is deliberately cheap to clone so it can
//! be shared across the tool execution loop without an `Arc`.

use crate::acp::security::SecurityPolicy;

/// Runtime context provided to every tool call.
///
/// Build one from an existing [`SecurityPolicy`] or from scratch via
/// [`ToolContext::default_for_cwd`] when running in a simple context
/// (e.g. inside the CPU router tool loop).
///
/// The `session_id` is used for audit log correlation (SEC-8) so that
/// all external access logs from tools within the same logical session
/// share a stable identifier instead of a fresh UUID per call.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Security policy governing path trust, external-access rules, and
    /// shell-command validation.
    pub policy: SecurityPolicy,
    /// Stable session identifier for audit correlation and logging.
    /// When not provided by a higher-level session manager, a fresh UUID
    /// is generated for this context instance.
    pub session_id: String,
}

impl ToolContext {
    /// Create a `ToolContext` from an existing [`SecurityPolicy`].
    /// A new random session_id is generated (suitable for short-lived or test use).
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create a `ToolContext` with an explicit session identifier.
    pub fn with_session_id(policy: SecurityPolicy, session_id: impl Into<String>) -> Self {
        Self {
            policy,
            session_id: session_id.into(),
        }
    }

    /// Create a default `ToolContext` that trusts the current working directory.
    /// Generates a fresh session_id (for CLI fallbacks, tests, routers without
    /// a real ACP/ chat session context).
    pub fn default_for_cwd() -> Self {
        Self {
            policy: SecurityPolicy::new(),
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl From<SecurityPolicy> for ToolContext {
    fn from(policy: SecurityPolicy) -> Self {
        Self::new(policy)
    }
}
