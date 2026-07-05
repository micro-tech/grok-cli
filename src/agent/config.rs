//! Per-agent configuration — the complete runtime profile for a sub-agent.
//!
//! [`SubAgentConfig`] bundles everything that differentiates one agent from
//! another into a single, flat, TOML-serialisable struct.  The five sections
//! mirror the Copilot/Claude Code/Codex agent model:
//!
//! | Section | Controls |
//! |---|---|
//! | `tool_permissions` | Which tools the agent may call, which are banned, which need approval |
//! | `persona` | Role, tone, verbosity, reasoning effort, system prompt |
//! | `safety` | Write-size cap, destructive ops, dry-run, intent validation |
//! | `context_budget` | Token limits, compression threshold & mode |
//! | `sandbox` | Filesystem isolation — enabled, path, cleanup |
//!
//! Use [`SubAgentConfig::builder()`] for ergonomic code construction, or load
//! a TOML definition via [`crate::agent::loader::load_agent_config`].

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── ToolPermissions ───────────────────────────────────────────────────────────

/// Per-agent tool surface control.
///
/// Evaluation order: `deny` wins over `allow`.  A tool in `restricted` that
/// is also in `allow` may be called, but the caller must request user approval
/// before execution (this is checked by the safety layer, not here).
///
/// `allow = None` → all tools permitted (permissive default, use with care).
/// `allow = Some([])` → no tools at all (pure text completion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    /// Whitelist of tool names the agent may call.
    /// `None` = unrestricted (all tools available).
    pub allow: Option<Vec<String>>,

    /// Blacklist of tool names that are always blocked, even if in `allow`.
    #[serde(default)]
    pub deny: Vec<String>,

    /// Tools that require explicit user approval before each call.
    #[serde(default)]
    pub restricted: Vec<String>,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            allow: None, // no tools by default — safest
            deny: vec![],
            restricted: vec![],
        }
    }
}

impl ToolPermissions {
    /// Return `true` if `tool_name` is permitted (not in deny, in allow or unrestricted).
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        // Deny wins always.
        if self.deny.iter().any(|d| d == tool_name) {
            return false;
        }
        match &self.allow {
            None => true,                                      // unrestricted
            Some(list) => list.iter().any(|a| a == tool_name), // whitelist
        }
    }

    /// Return `true` if the tool requires user approval before calling.
    pub fn is_restricted(&self, tool_name: &str) -> bool {
        self.restricted.iter().any(|r| r == tool_name)
    }

    /// Filter a list of tool JSON definitions down to the permitted set.
    pub fn filter_tools(&self, tools: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
        tools
            .into_iter()
            .filter(|t| {
                t.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|name| self.is_allowed(name))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Return the effective allowed-tool names (for logging/display).
    pub fn effective_tools<'a>(&'a self, all_tools: &'a [serde_json::Value]) -> Vec<&'a str> {
        all_tools
            .iter()
            .filter_map(|t| {
                t.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .filter(|name| self.is_allowed(name))
            })
            .collect()
    }
}

// ── AgentPersona ──────────────────────────────────────────────────────────────

/// The agent's voice, role, and reasoning style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    /// Semantic role label: `"planner"`, `"coder"`, `"researcher"`, `"verifier"`.
    #[serde(default = "default_role")]
    pub role: String,

    /// Tone of responses: `"precise"`, `"technical"`, `"analytical"`, `"strict"`.
    #[serde(default = "default_tone")]
    pub tone: String,

    /// Response verbosity: `"low"`, `"medium"`, `"high"`.
    #[serde(default = "default_verbosity")]
    pub verbosity: String,

    /// Reasoning effort for thinking-capable models: `"off"`, `"low"`, `"high"`.
    #[serde(default = "default_reasoning_mode")]
    pub reasoning_mode: String,

    /// Full system prompt / persona injected as the first message.
    /// Overrides the global default sub-agent prompt.
    pub system_prompt: Option<String>,
}

fn default_role() -> String {
    "agent".to_string()
}
fn default_tone() -> String {
    "precise".to_string()
}
fn default_verbosity() -> String {
    "medium".to_string()
}
fn default_reasoning_mode() -> String {
    "off".to_string()
}

impl Default for AgentPersona {
    fn default() -> Self {
        Self {
            role: default_role(),
            tone: default_tone(),
            verbosity: default_verbosity(),
            reasoning_mode: default_reasoning_mode(),
            system_prompt: None,
        }
    }
}

impl AgentPersona {
    /// Convert `reasoning_mode` to the xAI API `reasoning_effort` string,
    /// returning `None` for `"off"` (no extended reasoning).
    pub fn reasoning_effort(&self) -> Option<&str> {
        match self.reasoning_mode.as_str() {
            "off" | "" => None,
            other => Some(other),
        }
    }
}

// ── AgentSafety ───────────────────────────────────────────────────────────────

/// Per-agent safety boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSafety {
    /// Maximum bytes a single file-write may produce.
    /// 0 = no writes allowed (enforced before the API call).
    #[serde(default = "default_max_write_size")]
    pub max_write_size: usize,

    /// Whether the agent may perform destructive operations
    /// (delete_file, run_shell_command with side-effects).
    #[serde(default)]
    pub allow_destructive: bool,

    /// When `true`, write operations return a diff preview instead of
    /// applying the change.  The LLM must re-confirm to apply.
    #[serde(default)]
    pub require_dry_run: bool,

    /// Intent validation strictness: `"strict"`, `"standard"`, `"off"`.
    #[serde(default = "default_intent_validation")]
    pub intent_validation: String,
}

fn default_max_write_size() -> usize {
    100_000
}
fn default_intent_validation() -> String {
    "standard".to_string()
}

impl Default for AgentSafety {
    fn default() -> Self {
        Self {
            max_write_size: default_max_write_size(),
            allow_destructive: false,
            require_dry_run: false,
            intent_validation: default_intent_validation(),
        }
    }
}

impl AgentSafety {
    /// Return `true` if a proposed write of `byte_count` bytes is within the limit.
    /// `max_write_size == 0` means "no writes allowed" (returns `false` for any count).
    pub fn write_allowed(&self, byte_count: usize) -> bool {
        if self.max_write_size == 0 {
            false
        } else {
            byte_count <= self.max_write_size
        }
    }

    /// Return `true` if destructive tools (delete, shell) should be blocked.
    pub fn blocks_destructive(&self) -> bool {
        !self.allow_destructive
    }
}

// ── ContextBudget ─────────────────────────────────────────────────────────────

/// Token and compression budget for a sub-agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Hard limit on context window tokens.
    /// 0 = use the model's built-in limit.
    #[serde(default = "default_budget_max_tokens")]
    pub max_tokens: usize,

    /// Token count at which compression / summarisation is triggered.
    /// Should be < `max_tokens`.
    #[serde(default = "default_summary_threshold")]
    pub summary_threshold: usize,

    /// Compression strategy: `"semantic"` (meaning-preserving LLM pass)
    /// or `"token"` (simple oldest-first truncation).
    #[serde(default = "default_compression_mode")]
    pub compression_mode: String,
}

fn default_budget_max_tokens() -> usize {
    32_000
}
fn default_summary_threshold() -> usize {
    24_000
}
fn default_compression_mode() -> String {
    "token".to_string()
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_tokens: default_budget_max_tokens(),
            summary_threshold: default_summary_threshold(),
            compression_mode: default_compression_mode(),
        }
    }
}

// ── AgentSandbox ──────────────────────────────────────────────────────────────

/// Filesystem isolation rules for a sub-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSandbox {
    /// Whether to restrict the agent to the sandbox path.
    #[serde(default)]
    pub enabled: bool,

    /// Path to the sandbox directory.
    /// Empty string or relative path is resolved from CWD.
    #[serde(default)]
    pub path: String,

    /// Whether to keep the sandbox directory after the agent exits.
    #[serde(default)]
    pub keep: bool,
}

impl Default for AgentSandbox {
    fn default() -> Self {
        Self {
            enabled: false,
            path: String::new(),
            keep: false,
        }
    }
}

impl AgentSandbox {
    /// Resolve the sandbox to an absolute `PathBuf`, or `None` if disabled
    /// or the path is empty.
    pub fn resolved_path(&self) -> Option<PathBuf> {
        if !self.enabled || self.path.is_empty() {
            return None;
        }
        let p = PathBuf::from(&self.path);
        if p.is_absolute() {
            Some(p)
        } else {
            std::env::current_dir().ok().map(|cwd| cwd.join(p))
        }
    }
}

// ── SubAgentConfig ────────────────────────────────────────────────────────────

/// Complete runtime configuration for a single sub-agent session.
///
/// All five sections have sensible defaults so you only need to override what
/// differs from the baseline.  Serialisable to/from TOML for agent definition
/// files in `.grok/agents/*.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    // ── Core ─────────────────────────────────────────────────────────────────
    /// Model identifier. Default: `"grok-3-mini"`.
    #[serde(default = "default_model")]
    pub model: String,

    /// Sampling temperature (0.0 – 2.0). Default: 0.7.
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum output tokens (clamped to 256–8192 at call time).
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Maximum tool-loop iterations before the session errors out.
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: u32,

    // ── Five sections ────────────────────────────────────────────────────────
    #[serde(default)]
    pub tool_permissions: ToolPermissions,

    #[serde(default)]
    pub persona: AgentPersona,

    #[serde(default)]
    pub safety: AgentSafety,

    #[serde(default)]
    pub context_budget: ContextBudget,

    #[serde(default)]
    pub sandbox: AgentSandbox,

    // ── Legacy compat ────────────────────────────────────────────────────────
    /// Explicit trusted directories (merged with sandbox path if sandbox is enabled).
    /// Prefer `sandbox.path` for isolation; use this for multi-directory access.
    #[serde(default)]
    pub trusted_dirs: Vec<PathBuf>,
}

fn default_model() -> String {
    "grok-3-mini".to_string()
}
fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    2048
}
fn default_max_tool_iterations() -> u32 {
    10
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            max_tool_iterations: default_max_tool_iterations(),
            tool_permissions: ToolPermissions::default(),
            persona: AgentPersona::default(),
            safety: AgentSafety::default(),
            context_budget: ContextBudget::default(),
            sandbox: AgentSandbox::default(),
            trusted_dirs: Vec::new(),
        }
    }
}

// ── Derived helpers ───────────────────────────────────────────────────────────

impl SubAgentConfig {
    /// Start building a config from scratch.
    pub fn builder() -> SubAgentConfigBuilder {
        SubAgentConfigBuilder::new()
    }

    /// Load from a named preset in `.grok/agents/<name>.toml`.
    /// Falls back to `SubAgentConfig::default()` if the file is missing.
    pub fn load_preset(name: &str) -> Self {
        crate::agent::loader::load_agent_config(name).unwrap_or_else(|_| Self::default())
    }

    /// Return `true` if the agent has any allowed tools.
    pub fn has_tools(&self) -> bool {
        self.tool_permissions
            .allow
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false) // None = unrestricted but we track as "has tools"
    }

    /// Effective allowed-tool count (0 if explicitly empty, usize::MAX if unrestricted).
    pub fn tool_count(&self) -> usize {
        self.tool_permissions
            .allow
            .as_ref()
            .map(|t| t.len())
            .unwrap_or(0)
    }

    /// Resolve all trusted directories: sandbox path (if enabled) + explicit trusted_dirs.
    pub fn effective_trusted_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = self.trusted_dirs.clone();
        if let Some(sandbox_path) = self.sandbox.resolved_path() {
            if !dirs.contains(&sandbox_path) {
                dirs.push(sandbox_path);
            }
        }
        dirs
    }

    /// The system prompt to inject: persona.system_prompt or the default.
    pub fn effective_system_prompt<'a>(&'a self, default: &'a str) -> &'a str {
        self.persona.system_prompt.as_deref().unwrap_or(default)
    }

    // ── Preset constructors ───────────────────────────────────────────────────

    /// Read-only research agent — web + filesystem reads, no writes.
    pub fn research() -> Self {
        Self::builder()
            .role("researcher")
            .system_prompt(
                "You are a thorough research agent. Read files carefully, cite evidence, \
                 and return well-structured findings. Never modify files.",
            )
            .allow_tools(vec![
                "read_file",
                "list_directory",
                "search_file_content",
                "fs_grep",
                "fs_glob",
            ])
            .max_tokens(4096)
            .build()
    }

    /// Code-editing agent — reads and writes files, no shell or network.
    pub fn coder() -> Self {
        Self::builder()
            .role("coder")
            .system_prompt(
                "You are an expert software engineer. Write clean, idiomatic Rust. \
                 Read existing files before modifying. Make minimal, precise changes.",
            )
            .allow_tools(vec![
                "read_file",
                "list_directory",
                "search_file_content",
                "fs_grep",
                "fs_glob",
                "write_file",
                "edit_file",
            ])
            .max_tokens(4096)
            .max_tool_iterations(15)
            .build()
    }

    /// Read-only code reviewer.
    pub fn reviewer() -> Self {
        Self::builder()
            .role("verifier")
            .system_prompt(
                "You are a senior code reviewer. Identify bugs, security issues, \
                 performance problems, and style violations. Be specific and actionable.",
            )
            .allow_tools(vec![
                "read_file",
                "list_directory",
                "search_file_content",
                "fs_grep",
                "fs_glob",
            ])
            .max_tokens(4096)
            .temperature(0.3)
            .build()
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Ergonomic builder for [`SubAgentConfig`].
#[derive(Debug, Default)]
pub struct SubAgentConfigBuilder {
    inner: SubAgentConfig,
}

impl SubAgentConfigBuilder {
    pub fn new() -> Self {
        Self {
            inner: SubAgentConfig::default(),
        }
    }

    // ── Core ─────────────────────────────────────────────────────────────────

    pub fn model(mut self, m: impl Into<String>) -> Self {
        self.inner.model = m.into();
        self
    }
    pub fn temperature(mut self, t: f32) -> Self {
        self.inner.temperature = t.clamp(0.0, 2.0);
        self
    }
    pub fn max_tokens(mut self, n: u32) -> Self {
        self.inner.max_tokens = n;
        self
    }
    pub fn max_tool_iterations(mut self, n: u32) -> Self {
        self.inner.max_tool_iterations = n;
        self
    }

    // ── Tool permissions ─────────────────────────────────────────────────────

    /// Shorthand: set allowed tools by name slice.
    pub fn allow_tools(mut self, tools: Vec<&str>) -> Self {
        self.inner.tool_permissions.allow = Some(tools.into_iter().map(str::to_string).collect());
        self
    }
    pub fn deny_tools(mut self, tools: Vec<&str>) -> Self {
        self.inner.tool_permissions.deny = tools.into_iter().map(str::to_string).collect();
        self
    }
    pub fn restricted_tools(mut self, tools: Vec<&str>) -> Self {
        self.inner.tool_permissions.restricted = tools.into_iter().map(str::to_string).collect();
        self
    }
    pub fn tool_permissions(mut self, p: ToolPermissions) -> Self {
        self.inner.tool_permissions = p;
        self
    }

    // ── Persona ───────────────────────────────────────────────────────────────

    pub fn role(mut self, r: impl Into<String>) -> Self {
        self.inner.persona.role = r.into();
        self
    }
    pub fn tone(mut self, t: impl Into<String>) -> Self {
        self.inner.persona.tone = t.into();
        self
    }
    pub fn verbosity(mut self, v: impl Into<String>) -> Self {
        self.inner.persona.verbosity = v.into();
        self
    }
    pub fn reasoning_mode(mut self, m: impl Into<String>) -> Self {
        self.inner.persona.reasoning_mode = m.into();
        self
    }
    pub fn system_prompt(mut self, p: impl Into<String>) -> Self {
        self.inner.persona.system_prompt = Some(p.into());
        self
    }
    pub fn persona(mut self, p: AgentPersona) -> Self {
        self.inner.persona = p;
        self
    }

    // ── Safety ────────────────────────────────────────────────────────────────

    pub fn max_write_size(mut self, n: usize) -> Self {
        self.inner.safety.max_write_size = n;
        self
    }
    pub fn allow_destructive(mut self, b: bool) -> Self {
        self.inner.safety.allow_destructive = b;
        self
    }
    pub fn require_dry_run(mut self, b: bool) -> Self {
        self.inner.safety.require_dry_run = b;
        self
    }
    pub fn safety(mut self, s: AgentSafety) -> Self {
        self.inner.safety = s;
        self
    }

    // ── Context budget ────────────────────────────────────────────────────────

    pub fn context_max_tokens(mut self, n: usize) -> Self {
        self.inner.context_budget.max_tokens = n;
        self
    }
    pub fn summary_threshold(mut self, n: usize) -> Self {
        self.inner.context_budget.summary_threshold = n;
        self
    }
    pub fn compression_mode(mut self, m: impl Into<String>) -> Self {
        self.inner.context_budget.compression_mode = m.into();
        self
    }
    pub fn context_budget(mut self, b: ContextBudget) -> Self {
        self.inner.context_budget = b;
        self
    }

    // ── Sandbox ───────────────────────────────────────────────────────────────

    pub fn sandbox_enabled(mut self, enabled: bool) -> Self {
        self.inner.sandbox.enabled = enabled;
        self
    }
    pub fn sandbox_path(mut self, path: impl Into<String>) -> Self {
        self.inner.sandbox.path = path.into();
        self.inner.sandbox.enabled = true;
        self
    }
    pub fn sandbox_keep(mut self, keep: bool) -> Self {
        self.inner.sandbox.keep = keep;
        self
    }
    pub fn sandbox(mut self, s: AgentSandbox) -> Self {
        self.inner.sandbox = s;
        self
    }

    // ── Legacy trusted dirs ───────────────────────────────────────────────────

    pub fn trusted_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.inner.trusted_dirs.push(dir.into());
        self
    }

    /// Consume the builder and return the finished config.
    pub fn build(self) -> SubAgentConfig {
        self.inner
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_tools_and_is_safe() {
        let cfg = SubAgentConfig::default();
        assert!(!cfg.has_tools());
        assert!(!cfg.safety.allow_destructive);
        assert!(!cfg.sandbox.enabled);
    }

    #[test]
    fn tool_permissions_deny_wins_over_allow() {
        let p = ToolPermissions {
            allow: Some(vec!["write_file".into()]),
            deny: vec!["write_file".into()],
            restricted: vec![],
        };
        assert!(!p.is_allowed("write_file"), "deny must beat allow");
        assert!(!p.is_allowed("read_file"), "not in allow list");
    }

    #[test]
    fn tool_permissions_unrestricted_allows_everything() {
        let p = ToolPermissions {
            allow: None,
            deny: vec![],
            restricted: vec![],
        };
        assert!(p.is_allowed("write_file"));
        assert!(p.is_allowed("run_shell_command"));
    }

    #[test]
    fn tool_permissions_filter_tools() {
        let p = ToolPermissions {
            allow: Some(vec!["read_file".into()]),
            deny: vec![],
            restricted: vec![],
        };
        let tools = vec![
            serde_json::json!({"type":"function","function":{"name":"read_file","description":"","parameters":{}}}),
            serde_json::json!({"type":"function","function":{"name":"write_file","description":"","parameters":{}}}),
        ];
        let filtered = p.filter_tools(tools);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["function"]["name"], "read_file");
    }

    #[test]
    fn builder_sets_all_sections() {
        let cfg = SubAgentConfig::builder()
            .model("grok-3")
            .temperature(0.3)
            .max_tokens(4096)
            .max_tool_iterations(20)
            .allow_tools(vec!["read_file", "write_file"])
            .deny_tools(vec!["run_shell_command"])
            .restricted_tools(vec!["write_file"])
            .role("coder")
            .tone("technical")
            .verbosity("low")
            .reasoning_mode("high")
            .system_prompt("You are a coder.")
            .max_write_size(50_000)
            .allow_destructive(false)
            .require_dry_run(true)
            .context_max_tokens(80_000)
            .summary_threshold(60_000)
            .compression_mode("semantic")
            .sandbox_path(".grok/sandbox")
            .sandbox_keep(false)
            .trusted_dir("/tmp/project")
            .build();

        assert_eq!(cfg.model, "grok-3");
        assert!((cfg.temperature - 0.3).abs() < 1e-6);
        assert_eq!(cfg.tool_permissions.allow.as_ref().unwrap().len(), 2);
        assert_eq!(cfg.tool_permissions.deny, vec!["run_shell_command"]);
        assert!(cfg.tool_permissions.is_restricted("write_file"));
        assert_eq!(cfg.persona.role, "coder");
        assert_eq!(cfg.persona.reasoning_mode, "high");
        assert_eq!(cfg.persona.reasoning_effort(), Some("high"));
        assert_eq!(cfg.safety.max_write_size, 50_000);
        assert!(cfg.safety.require_dry_run);
        assert!(!cfg.safety.allow_destructive);
        assert_eq!(cfg.context_budget.max_tokens, 80_000);
        assert_eq!(cfg.context_budget.compression_mode, "semantic");
        assert!(cfg.sandbox.enabled);
        assert_eq!(cfg.trusted_dirs.len(), 1);
    }

    #[test]
    fn preset_research_is_read_only() {
        let cfg = SubAgentConfig::research();
        assert!(!cfg.tool_permissions.is_allowed("write_file"));
        assert!(cfg.tool_permissions.is_allowed("read_file"));
    }

    #[test]
    fn preset_coder_has_write_no_shell() {
        let cfg = SubAgentConfig::coder();
        assert!(cfg.tool_permissions.is_allowed("write_file"));
        assert!(!cfg.tool_permissions.is_allowed("run_shell_command"));
    }

    #[test]
    fn safety_write_allowed_respects_limit() {
        let s = AgentSafety {
            max_write_size: 100,
            ..Default::default()
        };
        assert!(s.write_allowed(99));
        assert!(s.write_allowed(100));
        assert!(!s.write_allowed(101));
    }

    #[test]
    fn safety_write_size_zero_means_no_writes() {
        let s = AgentSafety {
            max_write_size: 0,
            ..Default::default()
        };
        assert!(!s.write_allowed(1));
    }

    #[test]
    fn sandbox_resolved_path_empty_is_none() {
        let s = AgentSandbox {
            enabled: true,
            path: String::new(),
            keep: false,
        };
        assert!(s.resolved_path().is_none());
    }

    #[test]
    fn sandbox_disabled_returns_none() {
        let s = AgentSandbox {
            enabled: false,
            path: ".grok/sandbox".into(),
            keep: false,
        };
        assert!(s.resolved_path().is_none());
    }

    #[test]
    fn effective_trusted_dirs_merges_sandbox_and_extra() {
        let cfg = SubAgentConfig::builder()
            .sandbox_path(".grok/sandbox")
            .trusted_dir("/tmp/extra")
            .build();
        let dirs = cfg.effective_trusted_dirs();
        assert!(dirs.len() >= 2);
    }

    #[test]
    fn persona_reasoning_effort_off_is_none() {
        let p = AgentPersona {
            reasoning_mode: "off".into(),
            ..Default::default()
        };
        assert!(p.reasoning_effort().is_none());
    }

    #[test]
    fn config_is_toml_serialisable() {
        let cfg = SubAgentConfig::coder();
        let toml_str = toml::to_string(&cfg).expect("should serialise to TOML");
        let back: SubAgentConfig = toml::from_str(&toml_str).expect("should round-trip");
        assert_eq!(back.model, cfg.model);
        assert_eq!(back.persona.role, cfg.persona.role);
    }
}
