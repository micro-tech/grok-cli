use std::fmt;

use crate::router::{Backend, BackendKind, RouterError, RouterRequest, RouterResponse};

/// CPU-side request router.
///
/// Holds a list of registered backends and dispatches each [`RouterRequest`]
/// to the best matching one.  Currently only the [`BackendKind::Grok`] backend
/// is supported; other model prefixes return [`RouterError::BackendUnavailable`].
pub struct CpuRouter {
    backends: Vec<Box<dyn Backend>>,
}

impl fmt::Debug for CpuRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpuRouter")
            .field("backend_count", &self.backends.len())
            .finish()
    }
}

impl CpuRouter {
    /// Create a new router with the given backend list.
    pub fn new(backends: Vec<Box<dyn Backend>>) -> Self {
        Self { backends }
    }

    /// Route a request to the appropriate backend.
    ///
    /// Routing rules (checked in order):
    /// - Model name starts with `"grok"` → [`BackendKind::Grok`]
    /// - No match → [`RouterError::BackendUnavailable`]
    pub async fn route(&self, req: &RouterRequest) -> Result<RouterResponse, RouterError> {
        let backend = self
            .select_backend(&req.model)
            .ok_or_else(|| RouterError::BackendUnavailable(req.model.clone()))?;

        backend.send(req).await
    }

    /// Return the first available backend that matches the model prefix.
    fn select_backend(&self, model: &str) -> Option<&dyn Backend> {
        if model.starts_with("grok") {
            return self
                .backends
                .iter()
                .find(|b| b.kind() == BackendKind::Grok && b.is_available())
                .map(|b| b.as_ref());
        }

        // Fallback: first available backend regardless of kind
        self.backends
            .iter()
            .find(|b| b.is_available())
            .map(|b| b.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::{RouterError, RouterRequest, RouterResponse};
    use async_trait::async_trait;
    use grok_api::Message;

    struct MockGrokBackend {
        available: bool,
    }

    #[async_trait]
    impl Backend for MockGrokBackend {
        fn kind(&self) -> BackendKind {
            BackendKind::Grok
        }

        fn is_available(&self) -> bool {
            self.available
        }

        async fn send(&self, req: &RouterRequest) -> Result<RouterResponse, RouterError> {
            Ok(RouterResponse::text("mock response", &req.model))
        }
    }

    fn make_request(model: &str) -> RouterRequest {
        RouterRequest::new(
            model,
            vec![Message {
                role: "user".to_string(),
                content: Some(grok_api::MessageContent::Text("hello".to_string())),
                tool_calls: None,
            }],
        )
    }

    #[tokio::test]
    async fn routes_grok_prefix_to_grok_backend() {
        let router = CpuRouter::new(vec![Box::new(MockGrokBackend { available: true })]);
        let req = make_request("grok-3-mini");
        let resp = router.route(&req).await.expect("should route");
        assert_eq!(resp.text.as_deref(), Some("mock response"));
    }

    #[tokio::test]
    async fn returns_unavailable_for_unknown_model_with_no_fallback() {
        let router = CpuRouter::new(vec![]);
        let req = make_request("unknown-model");
        let err = router.route(&req).await.expect_err("should fail");
        assert!(matches!(err, RouterError::BackendUnavailable(_)));
    }

    #[tokio::test]
    async fn unavailable_backend_is_skipped() {
        let router = CpuRouter::new(vec![Box::new(MockGrokBackend { available: false })]);
        let req = make_request("grok-2");
        let err = router.route(&req).await.expect_err("should fail");
        assert!(matches!(err, RouterError::BackendUnavailable(_)));
    }
}
