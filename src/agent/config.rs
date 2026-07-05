//! Per-agent configuration for sub-agent sessions.
//!
//! [`SubAgentConfig`] is the single source of truth for everything that
//! differentiates one sub-agent from another:
//!
//! - **Model** — which xAI model the agent runs on
//! - **Persona** — custom system prompt / role description
//! - **Tool permissions** — whitelist of callable tool names (or none)
//! - **Sandbox** — trusted directories the agent may read/write
//! - **Context budget** — output-token cap + tool-loop iteration limit
//! - **Temperature** — sampling randomness
//!
//! Use [`SubAgentConfig::builder()`] for ergonomic construction, or
//! [`SubAgentConfig::default()`] for a sensible no-tools text-completion agent.

use std::path::PathBuf;

/// Runtime configuration for a single sub-agent session.
///
/// Defaults produce a safe, tools-disabled text-completion agent on
/// `grok-3-mini`.  Override individual fields via the [`SubAgentConfigBuilder`]
/// returned by [`SubAgentConfig::builder()`].
#[derive(Debug, Clone)]
pub struct SubAgentConfig {
    /// Model identifier passed to the xAI API.
    /// Default: `"grok-3-mini"` (fast, cheap, good for focused tasks).
    pub model: String,

    /// System prompt / persona injected as the first message.
    /// `None` uses the default sub-agent system prompt from `agent_tools`.
    pub system_prompt: Option<String>,

    /// Whitelist of tool names the agent may call.
    ///
    /// - `None`           → no tools at all (pure text completion, fastest)
    /// - `Some([])`       → no tools (same as None, explicit)
    /// - `Some(["read_file", "list_directory"])` → read-only filesystem access
    ///
    /// Any tool name not in this list is stripped from the API request before
    /// the call is made, so the model cannot even see unavailable tools.
    pub allowed_tools: Option<Vec<String>>,

    /// Directories the agent's file tools are allowed to access.
    ///
    /// Empty list → only the process CWD is trusted (most restrictive).
    /// These paths are passed to `SecurityPolicy::add_trusted_directory`
    /// so the existing path-validation layer enforces them.
    pub trusted_dirs: Vec<PathBuf>,

    /// Maximum number of generated tokens in the final response.
    /// Clamped to `[256, 8192]` at call time.
    /// Default: 2048.
    pub max_tokens: u32,

    /// Maximum tool-call iterations before the loop errors out.
    /// Default: 10 (conservative — sub-agents should be focused).
    pub max_tool_iterations: u32,

    /// Sampling temperature (0.0 – 2.0).
    /// Default: 0.7.
    pub temperature: f32,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            model: "grok-3-mini".to_string(),
            system_prompt: None,
            allowed_tools: None, // no tools by default — safe
            trusted_dirs: Vec::new(),
            max_tokens: 2048,
            max_tool_iterations: 10,
            temperature: 0.7,
        }
    }
}

impl SubAgentConfig {
    /// Return a [`SubAgentConfigBuilder`] pre-loaded with [`Default`] values.
    pub fn builder() -> SubAgentConfigBuilder {
        SubAgentConfigBuilder::new()
    }

    /// Convenience: a read-only research agent that can read files and search
    /// but cannot write, execute shell commands, or call network tools.
    pub fn research() -> Self {
        Self::builder()
            .system_prompt(
                "You are a thorough research agent. Read files carefully, \
                 cite evidence from the codebase, and return well-structured findings.",
            )
            .allowed_tools(vec![
                "read_file".into(),
                "list_directory".into(),
                "search_file_content".into(),
                "fs_grep".into(),
                "fs_glob".into(),
            ])
            .max_tokens(4096)
            .build()
    }

    /// Convenience: a code-editing agent allowed to read and write files but
    /// not execute shell commands or access the web.
    pub fn coder() -> Self {
        Self::builder()
            .system_prompt(
                "You are an expert software engineer. Write clean, idiomatic Rust code. \
                 Read existing files before modifying them. Make minimal, precise changes.",
            )
            .allowed_tools(vec![
                "read_file".into(),
                "list_directory".into(),
                "write_file".into(),
                "edit_file".into(),
                "search_file_content".into(),
                "fs_grep".into(),
                "fs_glob".into(),
            ])
            .max_tokens(4096)
            .max_tool_iterations(15)
            .build()
    }

    /// Convenience: a reviewer agent — read-only, no writes.
    pub fn reviewer() -> Self {
        Self::builder()
            .system_prompt(
                "You are a senior code reviewer. Identify bugs, security issues, \
                 performance problems, and style violations. Be specific and actionable.",
            )
            .allowed_tools(vec![
                "read_file".into(),
                "list_directory".into(),
                "search_file_content".into(),
                "fs_grep".into(),
                "fs_glob".into(),
            ])
            .max_tokens(4096)
            .temperature(0.3) // lower temp → more deterministic reviews
            .build()
    }

    /// Returns `true` if this config grants the agent any tool access.
    pub fn has_tools(&self) -> bool {
        self.allowed_tools
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
    }

    /// Returns the effective tool count (0 if no tools are allowed).
    pub fn tool_count(&self) -> usize {
        self.allowed_tools.as_ref().map(|t| t.len()).unwrap_or(0)
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Ergonomic builder for [`SubAgentConfig`].
///
/// ```rust
/// use grok_cli::agent::SubAgentConfig;
///
/// let cfg = SubAgentConfig::builder()
///     .model("grok-3")
///     .system_prompt("You are a testing specialist.")
///     .allowed_tools(vec!["read_file".into(), "write_file".into()])
///     .trusted_dir("/home/user/project")
///     .max_tokens(4096)
///     .max_tool_iterations(20)
///     .temperature(0.5)
///     .build();
/// ```
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

    /// Set the model identifier (e.g. `"grok-3-mini"`, `"grok-3"`).
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner.model = model.into();
        self
    }

    /// Set the system prompt / persona.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.inner.system_prompt = Some(prompt.into());
        self
    }

    /// Set the allowed-tools whitelist.
    /// Pass an empty `Vec` to allow no tools (pure text completion).
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.inner.allowed_tools = Some(tools);
        self
    }

    /// Add a single trusted directory to the sandbox.
    /// Accepts anything that converts to a `PathBuf`.
    pub fn trusted_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.inner.trusted_dirs.push(dir.into());
        self
    }

    /// Replace the entire trusted-dirs list.
    pub fn trusted_dirs(mut self, dirs: Vec<impl Into<PathBuf>>) -> Self {
        self.inner.trusted_dirs = dirs.into_iter().map(Into::into).collect();
        self
    }

    /// Set the maximum output tokens (clamped to 256–8192 at call time).
    pub fn max_tokens(mut self, n: u32) -> Self {
        self.inner.max_tokens = n;
        self
    }

    /// Set the maximum tool-loop iterations.
    pub fn max_tool_iterations(mut self, n: u32) -> Self {
        self.inner.max_tool_iterations = n;
        self
    }

    /// Set the sampling temperature (0.0 – 2.0).
    pub fn temperature(mut self, t: f32) -> Self {
        self.inner.temperature = t.clamp(0.0, 2.0);
        self
    }

    /// Consume the builder and return the finished [`SubAgentConfig`].
    pub fn build(self) -> SubAgentConfig {
        self.inner
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_tools() {
        let cfg = SubAgentConfig::default();
        assert!(!cfg.has_tools());
        assert_eq!(cfg.tool_count(), 0);
    }

    #[test]
    fn builder_sets_all_fields() {
        let cfg = SubAgentConfig::builder()
            .model("grok-3")
            .system_prompt("You are helpful.")
            .allowed_tools(vec!["read_file".into(), "write_file".into()])
            .trusted_dir("/tmp/sandbox")
            .max_tokens(4096)
            .max_tool_iterations(20)
            .temperature(0.3)
            .build();

        assert_eq!(cfg.model, "grok-3");
        assert_eq!(cfg.system_prompt.as_deref(), Some("You are helpful."));
        assert_eq!(cfg.allowed_tools.as_ref().unwrap().len(), 2);
        assert_eq!(cfg.trusted_dirs.len(), 1);
        assert_eq!(cfg.max_tokens, 4096);
        assert_eq!(cfg.max_tool_iterations, 20);
        assert!((cfg.temperature - 0.3).abs() < 1e-6);
        assert!(cfg.has_tools());
        assert_eq!(cfg.tool_count(), 2);
    }

    #[test]
    fn temperature_is_clamped() {
        let cfg = SubAgentConfig::builder().temperature(5.0).build();
        assert!((cfg.temperature - 2.0).abs() < 1e-6);

        let cfg2 = SubAgentConfig::builder().temperature(-1.0).build();
        assert!((cfg2.temperature - 0.0).abs() < 1e-6);
    }

    #[test]
    fn empty_allowed_tools_means_no_tools() {
        let cfg = SubAgentConfig::builder().allowed_tools(vec![]).build();
        assert!(!cfg.has_tools());
        assert_eq!(cfg.tool_count(), 0);
    }

    #[test]
    fn preset_research_has_read_only_tools() {
        let cfg = SubAgentConfig::research();
        assert!(cfg.has_tools());
        let tools = cfg.allowed_tools.as_ref().unwrap();
        assert!(tools.contains(&"read_file".to_string()));
        assert!(!tools.contains(&"write_file".to_string()));
        assert!(!tools.contains(&"run_shell_command".to_string()));
    }

    #[test]
    fn preset_coder_has_write_tools() {
        let cfg = SubAgentConfig::coder();
        let tools = cfg.allowed_tools.as_ref().unwrap();
        assert!(tools.contains(&"write_file".to_string()));
        assert!(!tools.contains(&"run_shell_command".to_string()));
    }

    #[test]
    fn preset_reviewer_is_read_only() {
        let cfg = SubAgentConfig::reviewer();
        let tools = cfg.allowed_tools.as_ref().unwrap();
        assert!(!tools.contains(&"write_file".to_string()));
        assert!(!tools.contains(&"edit_file".to_string()));
    }

    #[test]
    fn trusted_dirs_builder() {
        let cfg = SubAgentConfig::builder()
            .trusted_dir("/tmp/a")
            .trusted_dir("/tmp/b")
            .build();
        assert_eq!(cfg.trusted_dirs.len(), 2);
    }
}
