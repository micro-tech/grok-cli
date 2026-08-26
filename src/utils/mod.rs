//! Utility modules for grok-cli
//!
//! This module contains various utility functions and helpers used throughout
//! the application, including network utilities, file handling, and other
//! common functionality.

pub mod auth;
pub mod chat_logger;
pub mod client;
pub mod context;
pub mod history_compressor;
pub mod http;   // Centralized reqwest client (Task 281)
pub mod messages;
pub mod network;
pub mod perf;
pub mod rate_limiter;
pub mod session;
pub mod shell_permissions;
pub mod telemetry;
pub mod tool_logger;

// Re-export commonly used utilities

// Re-export the central HTTP client for convenience
pub use http::{get_http_client, http_client_builder};
