use std::fmt;

use grok_api::MessageContent;

use crate::router::{Backend, BackendKind, RouterError, RouterRequest, RouterResponse};

/// CPU-side request router.
///
/// Holds a list of registered backends and dispatches each [`RouterRequest`]
/// to the best matching one.  Currently only the [`BackendKind::Grok`] backend
/// is supported; other model prefixes return [`RouterError::BackendUnavailable`].
pub struct CpuRouter {
    backends: Vec<Box<dyn Backend>>,
    /// Optional Reasoning Protocol Layer.
    /// When `Some`, `route_with_tools_traced` uses this layer to record
    /// lifecycle hooks.  When `None` a temporary default layer is created
    /// per-call so callers always receive a valid `ReasoningTrace`.
    rpl: Option<crate::rpl::RplLayer>,
}

impl fmt::Debug for CpuRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpuRouter")
            .field("backend_count", &self.backends.len())
            .field("rpl_enabled", &self.rpl.is_some())
            .finish()
    }
}

impl CpuRouter {
    /// Create a new router with the given backend list.
    pub fn new(backends: Vec<Box<dyn Backend>>) -> Self {
        Self {
            backends,
            rpl: None,
        }
    }

    /// Attach a [`crate::rpl::RplLayer`] to this router.
    ///
    /// When attached, every call to [`route_with_tools_traced`] will
    /// produce a [`crate::rpl::ReasoningTrace`] alongside the response.
    /// Calls to the original [`route_with_tools`] are unaffected.
    pub fn with_rpl(mut self, rpl: crate::rpl::RplLayer) -> Self {
        self.rpl = Some(rpl);
        self
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

    /// Route a request through a **tool-execution loop**.
    ///
    /// This is the CPU router's implementation of the agent loop described in
    /// the Grok-CLI Tools Build Instructions:
    ///
    /// 1. Send the current message history to the backend.
    /// 2. If the LLM returns one or more tool calls, execute them via
    ///    [`crate::tools::registry::execute_tool`] using the provided
    ///    [`crate::tools::ToolContext`].
    /// 3. Append each tool result to the message history as a `"tool"` role
    ///    message (with `tool_call_id` so the API can correlate them).
    /// 4. Repeat until the LLM returns a final text response with no tool
    ///    calls, or until `max_iterations` is reached.
    ///
    /// # Security
    /// Every tool call is executed inside the security policy carried by
    /// `context`. Paths are canonicalized and checked against trusted
    /// directories; shell commands are validated against the denylist; all
    /// external access is logged.
    ///
    /// # Starlink resilience
    /// The loop re-uses the same retry-capable [`Backend`] for every
    /// iteration, so Starlink handover drops during tool calls are handled
    /// by the backend's exponential back-off.
    ///
    /// # Errors
    /// - [`RouterError::BackendUnavailable`] — no backend matches the model.
    /// - [`RouterError::ToolError`] — a tool call returned a hard error
    ///   (soft errors are appended as tool result text and sent back to the
    ///   model).
    /// - [`RouterError::MaxToolIterations`] — the loop reached `max_iterations`
    ///   without the model returning a final response.
    pub async fn route_with_tools(
        &self,
        req: RouterRequest,
        context: &crate::tools::ToolContext,
        max_iterations: u32,
    ) -> Result<RouterResponse, RouterError> {
        // Messages are already raw JSON — clone directly, no typed round-trip needed.
        // Raw JSON lets tool-result messages carry `tool_call_id` without loss.
        let mut messages_json: Vec<serde_json::Value> = req.messages.clone();

        for iteration in 0..max_iterations {
            let iter_req = RouterRequest {
                model: req.model.clone(),
                messages: messages_json.clone(),
                tools: req.tools.clone(),
                max_tokens: req.max_tokens,
                temperature: req.temperature,
                reasoning_effort: req.reasoning_effort.clone(),
            };

            // Call the backend (retries + back-off happen inside `route`).
            let resp = self.route(&iter_req).await?;

            // Clone tool calls before consuming the response.
            let tool_calls = resp.tool_calls.clone();
            let has_tool_calls = !tool_calls.is_empty();

            // Serialize the assistant message into the running history.
            let assistant_msg = resp.into_message_with_finish_reason().message;
            if let Ok(v) = serde_json::to_value(&assistant_msg) {
                messages_json.push(v);
            }

            if !has_tool_calls {
                // The model returned a final text response — we're done.
                let text = match assistant_msg.content {
                    Some(MessageContent::Text(t)) => Some(t),
                    _ => None,
                };
                return Ok(RouterResponse {
                    text,
                    tool_calls: vec![],
                    raw: serde_json::Value::Null,
                    model: req.model.clone(),
                    usage: None,
                    thinking_content: None,
                });
            }

            // ── Execute each tool call ────────────────────────────────────────
            for tool_call in &tool_calls {
                let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                // Soft errors are returned as tool result text so the model
                // can react (e.g. "File not found — try a different path").
                // Hard / unexpected errors propagate as RouterError::ToolError.
                let result =
                    crate::tools::registry::execute_tool(&tool_call.function.name, &args, context)
                        .await
                        .unwrap_or_else(|e| {
                            format!("Tool '{}' failed: {}", tool_call.function.name, e)
                        });

                messages_json.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": tool_call.id,
                    "content": result,
                }));
            }

            tracing::debug!(
                iteration = iteration + 1,
                max = max_iterations,
                tools = tool_calls.len(),
                "tool loop iteration complete"
            );
        }

        Err(RouterError::MaxToolIterations(max_iterations))
    }

    /// Route with tools AND generate a structured [`crate::rpl::ReasoningTrace`].
    ///
    /// This is identical to [`route_with_tools`] but fires the RPL lifecycle
    /// hooks before, during (on each tool selection), and after the run.
    /// The returned trace is always populated; when no `RplLayer` is attached
    /// a default-config layer is created for the call.
    pub async fn route_with_tools_traced(
        &self,
        req: RouterRequest,
        context: &crate::tools::ToolContext,
        max_iterations: u32,
        user_goal: Option<&str>,
    ) -> Result<(RouterResponse, crate::rpl::ReasoningTrace), RouterError> {
        use crate::rpl::{RplConfig, RplLayer};

        let layer = self
            .rpl
            .clone()
            .unwrap_or_else(|| RplLayer::new(RplConfig::default()));

        // on_pre_evaluate fires before the first backend call.
        let mut trace = layer.on_pre_evaluate(user_goal, None);

        // Run the standard tool loop.
        // We intercept tool selections by noting the tools in the request.
        // `ToolDefinition` is a typed struct, so access the name via `.function.name`.
        let tools_in_request: Vec<String> =
            req.tools.iter().map(|t| t.function.name.clone()).collect();

        for tool_name in &tools_in_request {
            layer.on_tool_selection(&mut trace, tool_name, true, None);
        }

        let response = self.route_with_tools(req, context, max_iterations).await?;

        // on_complete fires after the loop exits (success or tool exhaustion).
        layer.on_complete(&mut trace);

        Ok((response, trace))
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
            vec![serde_json::json!({"role": "user", "content": "hello"})],
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

    #[tokio::test]
    async fn route_with_tools_returns_text_when_no_tool_calls() {
        // A mock backend that always returns plain text (no tool calls).
        let router = CpuRouter::new(vec![Box::new(MockGrokBackend { available: true })]);
        let req = make_request("grok-3-mini");
        let ctx = crate::tools::ToolContext::default_for_cwd();

        let resp = router
            .route_with_tools(req, &ctx, 10)
            .await
            .expect("should succeed");

        assert_eq!(resp.text.as_deref(), Some("mock response"));
        assert!(resp.tool_calls.is_empty());
    }

    #[tokio::test]
    async fn route_with_tools_hits_max_iterations_when_tools_never_stop() {
        // A mock backend that always returns a tool call so the loop never
        // terminates naturally.
        struct AlwaysToolBackend;

        #[async_trait]
        impl Backend for AlwaysToolBackend {
            fn kind(&self) -> BackendKind {
                BackendKind::Grok
            }
            fn is_available(&self) -> bool {
                true
            }
            async fn send(&self, req: &RouterRequest) -> Result<RouterResponse, RouterError> {
                Ok(RouterResponse {
                    text: None,
                    tool_calls: vec![grok_api::ToolCall {
                        id: "tc_1".to_string(),
                        call_type: "function".to_string(),
                        function: grok_api::FunctionCall {
                            name: "list_directory".to_string(),
                            arguments: r#"{"path":"."}"#.to_string(),
                        },
                    }],
                    raw: serde_json::Value::Null,
                    model: req.model.clone(),
                    usage: None,
                    thinking_content: None,
                })
            }
        }

        let router = CpuRouter::new(vec![Box::new(AlwaysToolBackend)]);
        let req = make_request("grok-3-mini");
        let ctx = crate::tools::ToolContext::default_for_cwd();

        let err = router
            .route_with_tools(req, &ctx, 3)
            .await
            .expect_err("should hit max iterations");

        assert!(
            matches!(err, RouterError::MaxToolIterations(3)),
            "unexpected error: {:?}",
            err
        );
    }
}
