//! Router module
//!
//! The top-level entry point for application code is [`AppRouter`], which
//! wraps [`CpuRouter`] + [`backends::GrokBackend`] and exposes the same
//! async method signatures as the legacy `GrokClient`.
//!
//! Provides a lightweight CPU-side request router that dispatches inference
//! requests to the appropriate backend based on the model name prefix.
//!
//! # Example
//! ```rust,no_run
//! use grok_cli::router::{CpuRouter, RouterRequest};
//! use grok_cli::router::backends::GrokBackend;
//!
//! let backend = GrokBackend::new("xai-...").unwrap();
//! let router  = CpuRouter::new(vec![Box::new(backend)]);
//! ```

pub mod app_router;
pub mod backend;
pub mod backends;
pub mod cpu_router;
pub mod request;
pub mod response;
pub mod router_error;

pub use app_router::AppRouter;
pub use backend::{Backend, BackendKind};
pub use cpu_router::CpuRouter;
pub use request::{FunctionDefinition, RouterRequest, ToolDefinition};
pub use response::{RouterResponse, UsageStats};
pub use router_error::RouterError;
