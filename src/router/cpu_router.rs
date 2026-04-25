use std::fmt;

use grok_api::MessageContent;

use crate::router::{Backend, BackendKind, RouterError, RouterRequest, RouterResponse};

/// CPU-side request router.
///
/// Holds a list of registered backends and dispatches each [`RouterRequest`]
/// to the best matching one.  Currently only the [`BackendKind::Grok`] backend
/// is supported; other model prefixes return [`RouterError::BackendUnavailable`].
///
/// An optional [`crate::rpl::RplLayer`] can be attached via [`CpuRouter::with_rpl`]
/// to enable structured reasoning traces on every
/// [`route_with_tools_traced`][CpuRouter::route_with_tools_traced] call.
pub struct CpuRouter {
    backends: Vec<Box<dyn Backend>>,
    /// Optional Reasoning Protocol Layer.
    ///
    /// When `Some`, [`route_with_tools_traced`][CpuRouter::route_with_tools_traced]
    /// uses this layer to record lifecycle hooks.  When `None` a temporary
    /// default layer is created per-call so callers always receive a valid
    /// [`crate::rpl::ReasoningTrace`].
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
    ///
    /// No [`crate::rpl::RplLayer`] is attached by default; call
    /// [`with_rpl`][Self::with_rpl] to enable structured reasoning traces.
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
    ///
    /// [`route_with_tools_traced`]: Self::route_with_tools_traced
    /// [`route_with_tools`]: Self::route_with_tools
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
        // We work with raw JSON internally so tool-result messages can carry
        // the `tool_call_id` field that the Grok API requires for function
        // calling.  The typed `grok_api::Message` struct does not have this
        // field, so raw JSON is the safest cross-version approach.
        let mut messages_json: Vec<serde_json::Value> = req
            .messages
            .iter()
            .filter_map(|m| serde_json::to_value(m).ok())
            .collect();

        for iteration in 0..max_iterations {
            // Re-parse JSON → typed messages for this iteration's request.
            // Unknown fields (e.g. `tool_call_id`) are ignored by serde's
            // default behaviour, which is intentional here.
            let typed_messages: Vec<grok_api::Message> = messages_json
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();

            let iter_req = RouterRequest {
                model: req.model.clone(),
                messages: typed_messages,
                tools: req.tools.clone(),
                max_tokens: req.max_tokens,
                temperature: req.temperature,
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
                    finish_reason: Some("stop".to_string()),
                });
            }

            // ── Execute each tool call ────────────────────────────────────────
            for tool_call in &tool_calls {
                let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                // Soft errors are returned as tool result text so the model
                // can react (e.g. "File not found — try a different path").
                // Hard / unexpected errors propagate as RouterError::ToolError.
                // format_tool_error_for_llm() categorises the failure and adds
                // actionable recovery suggestions so the model does not blindly
                // retry the exact same failing call.
                // ── Execute tool, log result, forward structured error to LLM ────────────
                let t0 = std::time::Instant::now();
                let outcome =
                    crate::tools::registry::execute_tool(&tool_call.function.name, &args, context)
                        .await;
                let elapsed_us = t0.elapsed().as_micros();

                let result = match outcome {
                    Ok(r) => {
                        crate::utils::tool_logger::log_tool_success(
                            &tool_call.function.name,
                            &args,
                            r.len(),
                            elapsed_us,
                        );
                        r
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let cwd = std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("."));
                        crate::utils::tool_logger::log_tool_error(
                            &tool_call.function.name,
                            &args,
                            &err_str,
                            elapsed_us,
                            &cwd,
                            context.policy.trusted_directories(),
                        );
                        crate::tools::tool_error::format_tool_error_for_llm(
                            &tool_call.function.name,
                            &args,
                            &err_str,
                        )
                    }
                };

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

    /// Like [`route_with_tools`] but also returns the [`crate::rpl::ReasoningTrace`]
    /// captured during the tool-execution loop.
    ///
    /// If no [`RplLayer`] is attached (see [`with_rpl`]), a default trace is
    /// still returned (created via `RplLayer::with_default_config()`), so callers
    /// can always pattern-match on the trace without checking for `None`.
    ///
    /// # Reasoning lifecycle hooks called
    ///
    /// | Point in loop                   | Hook                      |
    /// |---------------------------------|---------------------------|
    /// | Before first backend call       | `on_pre_evaluate()`       |
    /// | Each tool call in the loop      | `on_tool_selection()`     |
    /// | After loop exits (any reason)   | `on_complete()`           |
    ///
    /// # Errors
    /// Returns the same error variants as [`route_with_tools`].  In every
    /// error path [`on_complete`][crate::rpl::RplLayer::on_complete] is still
    /// called on the trace before the error is returned, so the trace always
    /// reaches the [`Complete`][crate::rpl::ReasoningPhase::Complete] phase.
    ///
    /// [`route_with_tools`]: Self::route_with_tools
    /// [`with_rpl`]: Self::with_rpl
    /// [`RplLayer`]: crate::rpl::RplLayer
    pub async fn route_with_tools_traced(
        &self,
        req: RouterRequest,
        context: &crate::tools::ToolContext,
        max_iterations: u32,
    ) -> Result<(RouterResponse, crate::rpl::ReasoningTrace), RouterError> {
        // ── Task 88.3: Determinism guard ─────────────────────────────────────
        // A per-call UUID lets us correlate every tracing event emitted inside
        // this method, even when concurrent calls interleave in the log.
        let call_id = uuid::Uuid::new_v4().to_string();
        tracing::debug!(
            call_id = %call_id,
            model   = %req.model,
            "rpl: route_with_tools_traced start"
        );

        // ── Resolve the RPL layer ─────────────────────────────────────────────
        // `RplLayer` is neither `Clone` nor `Default`, so we cannot store a
        // fallback in `self`.  Instead we construct a temporary one here and
        // borrow from it when no layer has been attached.  The lifetime of
        // `_default_layer` covers the entire function, so the borrow is valid.
        let _default_layer = crate::rpl::RplLayer::with_default_config();
        let layer: &crate::rpl::RplLayer = self.rpl.as_ref().unwrap_or(&_default_layer);

        // ── Extract the goal for on_pre_evaluate ──────────────────────────────
        // Use the text content of the last message that carries text.  Most
        // routing calls have the user turn last; this covers the common case
        // without requiring callers to pass the goal separately.
        let goal: Option<&str> = req.messages.iter().rev().find_map(|m| match &m.content {
            Some(MessageContent::Text(t)) => Some(t.as_str()),
            _ => None,
        });

        let mut trace = layer.on_pre_evaluate(goal, None);

        // ── Build the mutable JSON message history ────────────────────────────
        let mut messages_json: Vec<serde_json::Value> = req
            .messages
            .iter()
            .filter_map(|m| serde_json::to_value(m).ok())
            .collect();

        // Track the number of completed iterations for the exit log event.
        let mut iteration_count: u32 = 0;

        for iteration in 0..max_iterations {
            iteration_count = iteration + 1;

            // Re-parse JSON → typed messages for this iteration's request.
            let typed_messages: Vec<grok_api::Message> = messages_json
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();

            let iter_req = RouterRequest {
                model: req.model.clone(),
                messages: typed_messages,
                tools: req.tools.clone(),
                max_tokens: req.max_tokens,
                temperature: req.temperature,
            };

            // Call the backend.  On network / auth errors, complete the trace
            // before propagating so it is always in the `Complete` phase.
            let resp = match self.route(&iter_req).await {
                Ok(r) => r,
                Err(e) => {
                    layer.on_complete(&mut trace);
                    tracing::debug!(
                        call_id    = %call_id,
                        iterations = iteration_count,
                        "rpl: route_with_tools_traced complete"
                    );
                    return Err(e);
                }
            };

            // Clone tool calls before the response is consumed by
            // `into_message_with_finish_reason`.
            let tool_calls = resp.tool_calls.clone();
            let has_tool_calls = !tool_calls.is_empty();

            // Serialize the assistant turn into the running history.
            let assistant_msg = resp.into_message_with_finish_reason().message;
            if let Ok(v) = serde_json::to_value(&assistant_msg) {
                messages_json.push(v);
            }

            if !has_tool_calls {
                // The model returned a final text response — complete the trace
                // and return both the response and the trace.
                let text = match assistant_msg.content {
                    Some(MessageContent::Text(t)) => Some(t),
                    _ => None,
                };

                layer.on_complete(&mut trace);
                tracing::debug!(
                    call_id    = %call_id,
                    iterations = iteration_count,
                    "rpl: route_with_tools_traced complete"
                );

                return Ok((
                    RouterResponse {
                        text,
                        tool_calls: vec![],
                        raw: serde_json::Value::Null,
                        model: req.model.clone(),
                        usage: None,
                        finish_reason: Some("stop".to_string()),
                    },
                    trace,
                ));
            }

            // ── Execute each tool call ────────────────────────────────────────
            for tool_call in &tool_calls {
                // Record the tool selection in the RPL trace *before*
                // execution so the trace reflects intent, not only outcome.
                layer.on_tool_selection(&mut trace, &tool_call.function.name, true, None);

                let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                // Soft errors become tool result text; the model can then
                // decide how to react (e.g. retry with a different path).
                // ── Execute tool, log result, forward structured error to LLM ────────────
                let t0 = std::time::Instant::now();
                let outcome =
                    crate::tools::registry::execute_tool(&tool_call.function.name, &args, context)
                        .await;
                let elapsed_us = t0.elapsed().as_micros();

                let result = match outcome {
                    Ok(r) => {
                        crate::utils::tool_logger::log_tool_success(
                            &tool_call.function.name,
                            &args,
                            r.len(),
                            elapsed_us,
                        );
                        r
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let cwd = std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("."));
                        crate::utils::tool_logger::log_tool_error(
                            &tool_call.function.name,
                            &args,
                            &err_str,
                            elapsed_us,
                            &cwd,
                            context.policy.trusted_directories(),
                        );
                        crate::tools::tool_error::format_tool_error_for_llm(
                            &tool_call.function.name,
                            &args,
                            &err_str,
                        )
                    }
                };

                messages_json.push(serde_json::json!({
                    "role":        "tool",
                    "tool_call_id": tool_call.id,
                    "content":     result,
                }));
            }

            tracing::debug!(
                iteration = iteration + 1,
                max = max_iterations,
                tools = tool_calls.len(),
                "tool loop iteration complete"
            );
        }

        // ── Max iterations reached ────────────────────────────────────────────
        layer.on_complete(&mut trace);
        tracing::debug!(
            call_id    = %call_id,
            iterations = iteration_count,
            "rpl: route_with_tools_traced complete"
        );
        Err(RouterError::MaxToolIterations(max_iterations))
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

    // ── Original tests (must continue to pass) ────────────────────────────────

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
                    finish_reason: Some("tool_calls".to_string()),
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

    // ── Task 88: RPL integration tests ───────────────────────────────────────

    /// Without an attached [`crate::rpl::RplLayer`], `route_with_tools_traced`
    /// must still return a valid, completed trace built by the default layer.
    #[tokio::test]
    async fn route_with_tools_traced_returns_trace_with_no_rpl() {
        let router = CpuRouter::new(vec![Box::new(MockGrokBackend { available: true })]);
        let req = make_request("grok-3-mini");
        let ctx = crate::tools::ToolContext::default_for_cwd();

        let (resp, trace) = router
            .route_with_tools_traced(req, &ctx, 10)
            .await
            .expect("should succeed");

        assert_eq!(resp.text.as_deref(), Some("mock response"));
        // The trace must have a non-empty UUID correlation ID.
        assert!(!trace.trace_id.is_empty());
        // on_complete must have been called, advancing the phase.
        assert_eq!(trace.phase, crate::rpl::ReasoningPhase::Complete);
    }

    /// With an attached [`crate::rpl::RplLayer`], `route_with_tools_traced`
    /// uses that layer's configuration and still returns a completed trace.
    #[tokio::test]
    async fn route_with_tools_traced_with_rpl_attached() {
        let rpl = crate::rpl::RplLayer::with_default_config();
        let router =
            CpuRouter::new(vec![Box::new(MockGrokBackend { available: true })]).with_rpl(rpl);
        let req = make_request("grok-3-mini");
        let ctx = crate::tools::ToolContext::default_for_cwd();

        let (resp, trace) = router
            .route_with_tools_traced(req, &ctx, 10)
            .await
            .expect("should succeed");

        assert_eq!(resp.text.as_deref(), Some("mock response"));
        assert_eq!(trace.phase, crate::rpl::ReasoningPhase::Complete);
    }

    /// The `Debug` impl must expose `rpl_enabled` so operators can confirm
    /// the layer is attached without inspecting private fields.
    #[test]
    fn debug_shows_rpl_enabled_field() {
        let router_no_rpl = CpuRouter::new(vec![]);
        let debug_no_rpl = format!("{:?}", router_no_rpl);
        assert!(
            debug_no_rpl.contains("rpl_enabled: false"),
            "expected 'rpl_enabled: false' in: {debug_no_rpl}"
        );

        let router_with_rpl =
            CpuRouter::new(vec![]).with_rpl(crate::rpl::RplLayer::with_default_config());
        let debug_with_rpl = format!("{:?}", router_with_rpl);
        assert!(
            debug_with_rpl.contains("rpl_enabled: true"),
            "expected 'rpl_enabled: true' in: {debug_with_rpl}"
        );
    }
}
