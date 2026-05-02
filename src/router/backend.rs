use async_trait::async_trait;

use crate::router::{RouterError, RouterRequest, RouterResponse};

/// Which AI provider a backend connects to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendKind {
    Grok,
}

/// The core abstraction every backend must implement.
///
/// Backends are async so that HTTP round-trips don't block the executor.
/// Use `Box<dyn Backend>` inside [`CpuRouter`] for runtime polymorphism.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Identify which provider this backend targets.
    fn kind(&self) -> BackendKind;

    /// Return `true` when the backend is configured and reachable.
    /// This is a cheap, synchronous check (e.g. "do I have an API key?").
    fn is_available(&self) -> bool;

    /// Send a request to the backend and await a response.
    async fn send(&self, req: &RouterRequest) -> Result<RouterResponse, RouterError>;
}
