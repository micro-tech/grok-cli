//! Safety hooks for Grok-CLI
//!
//! Provides mandatory pre-write validation, dry-run mode, diff-only editing,
//! intent validation, suspicious write rejection, DNA-aware safety, and
//! tool health monitoring.

pub mod pre_write_hook;
pub mod dry_run;
pub mod diff_validator;
pub mod intent_validator;
pub mod suspicious_write_guard;
pub mod dna_safety;
pub mod tool_health_monitor;
pub mod error;
#[cfg(test)]
pub mod tests;

pub use pre_write_hook::{on_before_write_file, SafetyDecision, WriteContext};
pub use dry_run::DryRunContext;
pub use diff_validator::DiffValidator;
pub use intent_validator::IntentValidator;
pub use suspicious_write_guard::SuspiciousWriteGuard;
pub use dna_safety::DnaSafetyController;
pub use tool_health_monitor::ToolHealthMonitor;
pub use error::SafetyError;
