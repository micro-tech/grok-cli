//! Reasoning Protocol Layer (RPL).
//!
//! The RPL module provides structured reasoning traces that capture every
//! decision made during a `CpuRouter::route_with_tools` invocation: which
//! tools were considered, which memories were consulted, what plan was formed,
//! and how confident the router was in its choices.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                      RplLayer                           │
//! │                                                         │
//! │  on_pre_evaluate()  ──►  creates ReasoningTrace         │
//! │  on_tool_selection() ──► appends ToolEvaluation         │
//! │  on_complete()      ──►  validates + logs via log_trace │
//! └─────────────────────────────────────────────────────────┘
//!          │                          │
//!          ▼                          ▼
//!      schema.rs                validation.rs
//!   (data types)             (ValidationError + validate)
//!          │
//!          ▼
//!      logging.rs
//!  (ReasoningLogLevel + log_trace)
//! ```
//!
//! # Quick start
//!
//! ```rust,ignore
//! use grok_cli::rpl::{RplLayer, RplConfig, ReasoningLogLevel};
//!
//! let layer = RplLayer::new(RplConfig {
//!     log_level: ReasoningLogLevel::Debug,
//!     lenient_validation: true,
//! });
//!
//! // Inside route_with_tools:
//! let mut trace = layer.on_pre_evaluate(Some("list /tmp"), None);
//! layer.on_tool_selection(&mut trace, "list_directory", true, Some("path arg"));
//! layer.on_complete(&mut trace);
//! ```
//!
//! # Feature flags
//!
//! This module has no optional feature flags.  All types are always compiled.
//!
//! # Stability
//!
//! The on-disk serialisation format is versioned via [`RPL_SCHEMA_VERSION`].
//! Increment that constant and update [`validation::validate`] whenever a
//! breaking change is made to [`ReasoningTrace`]'s serialised layout.

// ---------------------------------------------------------------------------
// Submodule declarations
// ---------------------------------------------------------------------------

/// Lifecycle management for a single reasoning trace.
///
/// See [`RplLayer`] for the primary entry point.
pub mod layer;

/// Trace logging via [`tracing`] at configurable verbosity levels.
///
/// See [`log_trace`] and [`ReasoningLogLevel`].
pub mod logging;

/// Versioned data types that represent a single reasoning trace.
///
/// See [`ReasoningTrace`], [`ToolEvaluation`], [`MemoryConsideration`],
/// [`ReasoningPhase`], and [`RPL_SCHEMA_VERSION`].
pub mod schema;

/// Non-short-circuiting validation of [`schema::ReasoningTrace`] instances.
///
/// See [`validate`] and [`ValidationError`].
pub mod validation;

/// Suppression and redaction controls for reasoning traces.
///
/// See [`SuppressionLayer`] and [`RedactionConfig`].
pub mod suppression;

// ---------------------------------------------------------------------------
// Flat re-exports for ergonomic use at the crate root
// ---------------------------------------------------------------------------

/// Re-export of [`layer::RplConfig`] and [`layer::RplLayer`].
pub use layer::{RplConfig, RplLayer};

/// Re-export of [`logging::ReasoningLogLevel`] and [`logging::log_trace`].
pub use logging::{ReasoningLogLevel, log_trace};

/// Re-export of all public schema types and the version constant.
pub use schema::{
    MemoryConsideration, RPL_SCHEMA_VERSION, ReasoningPhase, ReasoningTrace, ToolEvaluation,
};

/// Re-export of [`validation::ValidationError`] and [`validation::validate`].
pub use validation::{ValidationError, validate};

/// Re-export of [`suppression::RedactionConfig`], [`suppression::RedactionRule`],
/// and [`suppression::SuppressionLayer`].
pub use suppression::{RedactionConfig, RedactionRule, SuppressionLayer};
