//! Safety hooks for Grok-CLI
//!
//! Provides mandatory pre-write validation, dry-run mode, diff-only editing,
//! intent validation, suspicious write rejection, DNA-aware safety, and
//! tool health monitoring.

pub mod diff_validator;
pub mod dna_safety;
pub mod dry_run;
pub mod error;
pub mod intent_validator;
pub mod pre_write_hook;
pub mod suspicious_write_guard;
#[cfg(test)]
pub mod tests;
pub mod tool_health_monitor;

pub use diff_validator::DiffValidator;
pub use dna_safety::DnaSafetyController;
pub use dry_run::DryRunContext;
pub use error::SafetyError;
pub use intent_validator::IntentValidator;
pub use pre_write_hook::{SafetyDecision, WriteContext, on_before_write_file};
pub use suspicious_write_guard::SuspiciousWriteGuard;
pub use tool_health_monitor::ToolHealthMonitor;
