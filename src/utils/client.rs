//! Utility module for initialising Grok clients.
//!
//! # Migration note
//!
//! New call sites should prefer [`initialize_router`], which returns an
//! [`AppRouter`] backed by the full `CpuRouter` + `GrokBackend` stack.
//! [`initialize_client`] is kept for backward-compatibility with modules
//! that have not yet been migrated (e.g. `acp/mod.rs`).

use anyhow::Result;

use crate::GrokClient;
use crate::config::RateLimitConfig;
use crate::router::AppRouter;

/// Initialise a bare [`GrokClient`] with the provided settings.
///
/// **Prefer [`initialize_router`] for new code.**
///
/// This function is retained so that modules that talk directly to
/// `GrokClient` (primarily `acp/mod.rs`) continue to compile without
/// modification until they are migrated.
pub fn initialize_client(
    api_key: &str,
    timeout_secs: u64,
    max_retries: u32,
    rate_limit_config: RateLimitConfig,
) -> Result<GrokClient> {
    GrokClient::with_settings(api_key, timeout_secs, max_retries)
        .map(|client| client.with_rate_limits(rate_limit_config))
}

/// Initialise an [`AppRouter`] backed by the Grok backend.
///
/// This is the preferred entry point for all CLI commands.  The router
/// handles exponential back-off, Starlink-resilient retries, and future
/// multi-backend routing transparently — callers do not need to manage
/// any of that themselves.
///
/// - `api_key`      – xAI API key (must be non-empty).
/// - `timeout_secs` – per-request HTTP timeout; pass `config.timeout_secs`
///                    to honour the user's configuration value end-to-end.
pub fn initialize_router(api_key: &str, timeout_secs: u64) -> Result<AppRouter> {
    AppRouter::new(api_key, timeout_secs)
}
