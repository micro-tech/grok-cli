//! OKF (Open Knowledge Format + Workflow Trace Forwarder) configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

/// OKF (Open Knowledge Format + Workflow Trace Forwarder) configuration.
///
/// This section serves two purposes:
/// 1. Workflow trace forwarding (observability) to a central server.
/// 2. Open Knowledge Format (OKF) bundles for structured, portable knowledge.
///
/// Google's OKF (markdown + YAML frontmatter knowledge directories) can be
/// loaded at session start and queried as a "Knowledge OS" / "Knowledge API".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkfConfig {
    /// Master switch for the entire OKF subsystem (both forwarding and knowledge).
    #[serde(default)]
    pub enabled: bool,

    // ── Trace Forwarder (original use) ──────────────────────────────────────
    /// Server address for trace forwarding. Can be IP or hostname.
    #[serde(default)]
    pub server: String,

    /// Port the OKF server listens on
    #[serde(default = "default_okf_port")]
    pub port: u16,

    /// Optional bearer token / API key for the trace forwarder
    #[serde(default)]
    pub api_key: Option<String>,

    /// Ingestion endpoint path for traces
    #[serde(default = "default_okf_endpoint")]
    pub endpoint: String,

    /// Use HTTPS for trace forwarding
    #[serde(default)]
    pub use_https: bool,

    /// Timeout for the HTTP POST to OKF (trace forwarder)
    #[serde(default = "default_okf_timeout")]
    pub timeout_secs: u64,

    /// Whether to buffer traces locally when the OKF server is unreachable
    #[serde(default = "default_true")]
    pub buffer_on_failure: bool,

    /// Maximum number of traces to keep in the local buffer
    #[serde(default = "default_okf_buffer_size")]
    pub max_buffer_size: usize,

    // ── Open Knowledge Format (Knowledge OS / Bundles) ──────────────────────
    /// Directories containing OKF bundles (Google's Open Knowledge Format).
    ///
    /// Each directory is treated as one bundle. Markdown files with YAML
    /// frontmatter inside will be loaded as structured knowledge.
    ///
    /// These are loaded at session start and become part of the agent's
    /// "Knowledge OS". They can also be queried via the `okf_lookup` tool.
    ///
    /// Examples:
    ///   knowledge_bundles = [
    ///     "~/.grok-cli/knowledge",
    ///     "./.grok/knowledge",
    ///     "/opt/okf-bundles/company-metrics"
    ///   ]
    #[serde(default)]
    pub knowledge_bundles: Vec<String>,

    /// Remote OKF server base URL for reading/writing knowledge bundles.
    /// Example: "http://192.168.1.106:8080"
    ///
    /// When set, Grok-CLI can fetch bundles and push new concepts
    /// (via okf_create / okf_push) to the central server instead of (or in
    /// addition to) local files.
    #[serde(default)]
    pub remote_url: Option<String>,

    /// Default bundle name to use when writing knowledge to the remote server.
    /// Typical values: "grok-cli", "shared"
    #[serde(default = "default_okf_bundle")]
    pub default_bundle: String,
}

fn default_okf_bundle() -> String {
    "grok-cli".to_string()
}

fn default_okf_port() -> u16 {
    8080
}

fn default_okf_endpoint() -> String {
    "/api/traces".to_string()
}

fn default_okf_timeout() -> u64 {
    10
}

fn default_okf_buffer_size() -> usize {
    100
}

fn default_true() -> bool {
    true
}

impl Default for OkfConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server: String::new(),
            port: default_okf_port(),
            api_key: None,
            endpoint: default_okf_endpoint(),
            use_https: false,
            timeout_secs: default_okf_timeout(),
            buffer_on_failure: true,
            max_buffer_size: default_okf_buffer_size(),
            knowledge_bundles: vec![
                "~/.grok-cli/knowledge".to_string(),
                ".grok/knowledge".to_string(),
            ],
            remote_url: None,
            default_bundle: default_okf_bundle(),
        }
    }
}
