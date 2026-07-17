//! OKF (Observability / Workflow Trace Forwarder) client.
//!
//! Forwards completed `WorkflowTrace` records to a central OKF server
//! (e.g. running on a Proxmox host or dedicated logging cluster).
//!
//! Controlled by `[okf]` section in config + `GROK_OKF_*` environment variables.
//!
//! Features:
//! - Best-effort delivery (never blocks the main workflow)
//! - Optional local buffering when the server is unreachable
//! - Bearer token support
//! - Respects timeout and HTTPS settings

use crate::config::OkfConfig;
use crate::workflow::WorkflowTrace;
use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::sync::Mutex;
use std::time::Duration;
use tracing::{debug, warn};

/// In-memory buffer for traces that could not be delivered.
/// Bounded by the config's `max_buffer_size` at push time.
static OKF_BUFFER: Lazy<Mutex<Vec<WorkflowTrace>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Attempt to forward a workflow trace to the OKF server if enabled.
///
/// This function is **best-effort**:
/// - Returns immediately (never blocks).
/// - Never panics, even when called outside a Tokio runtime (e.g. unit tests).
/// - Only spawns work when a runtime handle is available.
pub fn maybe_forward_trace(trace: WorkflowTrace) {
    // Safely obtain a handle. This prevents panics in plain `#[test]` functions
    // (such as the cargo validation workflow test) that call `save_trace`.
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn(async move {
                // Load config inside the task (hierarchical + env overrides)
                match crate::config::Config::load_hierarchical().await {
                    Ok(cfg) => {
                        if let Err(e) = forward_workflow_trace(&trace, &cfg.okf).await {
                            debug!("OKF forward internal error: {}", e);
                        }
                    }
                    Err(e) => {
                        debug!("Could not load config for OKF forwarding: {}", e);
                    }
                }
            });
        }
        Err(_) => {
            // No active Tokio runtime (common in unit tests).
            // We silently skip forwarding — the trace is still persisted locally.
            debug!("OKF: no active Tokio runtime, skipping trace forward");
        }
    }
}

/// Core async forward implementation.
///
/// Returns `Ok(())` on success or when forwarding is disabled.
/// On transient failure it may buffer the trace (see `OkfConfig`).
pub async fn forward_workflow_trace(trace: &WorkflowTrace, cfg: &OkfConfig) -> Result<()> {
    if !cfg.enabled {
        return Ok(());
    }

    let server = cfg.server.trim();
    if server.is_empty() {
        debug!("OKF enabled but no server configured; skipping");
        return Ok(());
    }

    // Perform the actual send
    let success = send_trace_once(trace, cfg).await;

    if success {
        debug!("OKF trace forwarded successfully");
        // Best-effort flush of anything that was buffered earlier
        flush_buffer(cfg).await;
    } else if cfg.buffer_on_failure {
        push_to_buffer(trace, cfg.max_buffer_size);
    }

    Ok(())
}

/// Internal helper that performs a single HTTP POST of a trace.
/// Returns `true` only on HTTP 2xx.
async fn send_trace_once(trace: &WorkflowTrace, cfg: &OkfConfig) -> bool {
    let scheme = if cfg.use_https { "https" } else { "http" };
    let host = cfg.server.trim_end_matches('/');
    let url = format!("{}://{}:{}{}", scheme, host, cfg.port, cfg.endpoint);

    let client = match Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs.max(1)))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to build OKF HTTP client: {}", e);
            return false;
        }
    };

    let mut request = client.post(&url).json(trace);

    if let Some(key) = &cfg.api_key {
        if !key.trim().is_empty() {
            request = request.bearer_auth(key.trim());
        }
    }

    match request.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                true
            } else {
                let body = resp.text().await.unwrap_or_default();
                warn!("OKF server returned {}: {}", status, body);
                false
            }
        }
        Err(e) => {
            warn!("OKF forward failed to {}: {}", url, e);
            false
        }
    }
}

/// Push a trace into the local buffer, respecting max size.
fn push_to_buffer(trace: &WorkflowTrace, max_size: usize) {
    let mut buf = OKF_BUFFER.lock().unwrap();
    buf.push(trace.clone());

    // Enforce size limit (drop oldest)
    let effective_max = max_size.max(1);
    while buf.len() > effective_max {
        buf.remove(0);
    }

    debug!("OKF buffer now contains {} trace(s)", buf.len());
}

/// Attempt to flush the local buffer to the OKF server.
/// Called after a successful send.
///
/// Uses the internal `send_trace_once` to avoid recursion.
async fn flush_buffer(cfg: &OkfConfig) {
    let traces: Vec<WorkflowTrace> = {
        let mut buf = OKF_BUFFER.lock().unwrap();
        if buf.is_empty() {
            return;
        }
        std::mem::take(&mut *buf)
    };

    debug!("Flushing {} buffered OKF trace(s)", traces.len());

    for trace in traces {
        if send_trace_once(&trace, cfg).await {
            // success — continue flushing
        } else {
            // server still unreachable — re-buffer
            if cfg.buffer_on_failure {
                push_to_buffer(&trace, cfg.max_buffer_size);
            }
        }
    }
}

/// Returns how many traces are currently sitting in the local OKF buffer.
/// Useful for diagnostics / status commands.
pub fn buffered_trace_count() -> usize {
    OKF_BUFFER.lock().unwrap().len()
}

/// Force a flush of any buffered traces (best-effort, non-blocking).
/// Useful from CLI commands or admin endpoints.
pub fn trigger_okf_flush() {
    tokio::spawn(async {
        if let Ok(cfg) = crate::config::Config::load_hierarchical().await {
            flush_buffer(&cfg.okf).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{WorkflowStep, WorkflowTrace};

    fn sample_trace() -> WorkflowTrace {
        let mut t = WorkflowTrace::new();
        t.push(WorkflowStep::UserPrompt("test".into()));
        t.push(WorkflowStep::Decision { passed: true });
        t
    }

    #[test]
    fn buffer_respects_max_size() {
        let trace = sample_trace();
        // Push more than the limit
        for _ in 0..5 {
            push_to_buffer(&trace, 2);
        }
        assert!(buffered_trace_count() <= 2);
    }

    #[tokio::test]
    async fn disabled_config_does_nothing() {
        let mut cfg = OkfConfig::default();
        cfg.enabled = false;
        let trace = sample_trace();

        let res = forward_workflow_trace(&trace, &cfg).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn empty_server_skips() {
        let mut cfg = OkfConfig::default();
        cfg.enabled = true;
        cfg.server = "".to_string();
        let trace = sample_trace();

        let res = forward_workflow_trace(&trace, &cfg).await;
        assert!(res.is_ok());
    }
}
