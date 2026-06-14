//! Dry-Run Mode Support
//!
//! When dry_run=true, file tools compute the diff and return it without writing.
//! The LLM must then explicitly confirm with "Apply this diff."

#[derive(Debug, Clone, Default)]
pub struct DryRunContext {
    pub enabled: bool,
}

impl DryRunContext {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Returns true if the tool should simulate instead of execute
    pub fn should_simulate(&self) -> bool {
        self.enabled
    }
}
