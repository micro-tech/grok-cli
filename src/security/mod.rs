//! Security utilities for grok-cli
//!
//! This module provides security-related functionality including:
//! - Audit logging for external file access
//! - Security policy management (in acp/security.rs)

pub mod audit;

// Re-export commonly used types for convenience
pub use audit::{AuditLogger, ExternalAccessLog, create_access_log};
