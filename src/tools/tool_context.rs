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
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Security policy governing path trust, external-access rules, and
    /// shell-command validation.
    pub policy: SecurityPolicy,
}

impl ToolContext {
    /// Create a `ToolContext` from an existing [`SecurityPolicy`].
    pub fn new(policy: SecurityPolicy) -> Self {
        Self { policy }
    }

    /// Create a default `ToolContext` that trusts the current working directory.
    pub fn default_for_cwd() -> Self {
        Self {
            policy: SecurityPolicy::new(),
        }
    }
}

impl From<SecurityPolicy> for ToolContext {
    fn from(policy: SecurityPolicy) -> Self {
        Self { policy }
    }
}
