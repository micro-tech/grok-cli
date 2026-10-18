use crate::config::ExternalAccessConfig;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    trusted_directories: Vec<PathBuf>,
    working_directory: PathBuf,
    external_access_config: ExternalAccessConfig,
    /// Maximum seconds a shell command may run before being killed.
    /// Set from `tools.shell.command_timeout_secs` in config.toml.
    /// The `GROK_SHELL_TIMEOUT_SECS` env var overrides this at runtime.
    shell_timeout_secs: u64,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns true for Windows-style drive-absolute paths such as `C:\\Windows`
/// or `D:/data`.  On Unix these are NOT absolute according to `Path::is_absolute`,
/// but joining them under the working directory is wrong and can leak trust.
fn is_windows_drive_absolute(path: &Path) -> bool {
    let s = path.to_string_lossy();
    let bytes = s.as_bytes();
    // X: or X:\ or X:/
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Minimal POSIX-ish shell word splitter (Task 469).
///
/// Handles single quotes, double quotes, and backslash escapes — enough to
/// collapse quoting tricks (`r''m` → `rm`, `"rm"` → `rm`) for denylist
/// analysis.  This is *not* a full shell parser: no brace expansion, no
/// command substitution unpacking.
fn shell_word_split(command: &str) -> Vec<String> {
    // `${IFS}` / `$IFS` expand to whitespace in a real shell; normalise them
    // to spaces *before* tokenizing so `rm${IFS}-rf${IFS}/` splits into
    // `rm`, `-rf`, `/` like the shell would.
    let mut normalized = command.replace("${IFS}", " ");
    let mut fixed = String::with_capacity(normalized.len());
    let mut rest = normalized.as_str();
    while let Some(pos) = rest.find("$IFS") {
        fixed.push_str(&rest[..pos]);
        fixed.push(' ');
        rest = &rest[pos + 4..];
    }
    fixed.push_str(rest);
    normalized = fixed;

    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut chars = normalized.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                // Single-quoted: literal until closing quote.
                in_word = true;
                for qc in chars.by_ref() {
                    if qc == '\'' {
                        break;
                    }
                    current.push(qc);
                }
            }
            '"' => {
                // Double-quoted: backslash escapes one char, rest literal.
                in_word = true;
                while let Some(qc) = chars.next() {
                    if qc == '"' {
                        break;
                    }
                    if qc == '\\' {
                        if let Some(esc) = chars.next() {
                            current.push(esc);
                        }
                    } else {
                        current.push(qc);
                    }
                }
            }
            '\\' => {
                in_word = true;
                if let Some(esc) = chars.next() {
                    current.push(esc);
                }
            }
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            c => {
                in_word = true;
                current.push(c);
            }
        }
    }
    if in_word {
        words.push(current);
    }
    words
}

/// `argv[0]`-based dangerous-command analysis (Task 469).
///
/// Returns `Err(reason)` when the tokenized command is recognisably
/// dangerous: recursive `rm` at filesystem-sensitive targets, `dd` to block
/// devices, `mkfs*`, and privilege-wrapping (`sudo`/`env`) of the same.
/// Quoting tricks are already collapsed by [`shell_word_split`].
fn check_shell_argv0(command: &str) -> Result<(), String> {
    let words = shell_word_split(command);
    if words.is_empty() {
        return Ok(());
    }

    // Skip privilege wrappers to find the real program: `sudo rm -rf /`.
    let mut idx = 0;
    while idx < words.len() && matches!(words[idx].as_str(), "sudo" | "doas" | "env" | "runas") {
        idx += 1;
    }
    // `env VAR=val cmd …` — skip VAR=val assignments too.
    while idx < words.len() {
        let w = &words[idx];
        let is_assignment = !w.starts_with('-')
            && w.split_once('=').is_some_and(|(k, _)| {
                !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
        if is_assignment {
            idx += 1;
        } else {
            break;
        }
    }
    let Some(argv0) = words.get(idx) else {
        return Ok(());
    };
    // Basename: `/bin/rm` → `rm`.  Lowercase for `RM` parity.
    let prog = argv0
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(argv0)
        .to_lowercase();
    let args: &[String] = if idx + 1 < words.len() {
        &words[idx + 1..]
    } else {
        &[]
    };

    // Recursive rm at filesystem- or home-sensitive targets.
    if prog == "rm" {
        let recursive = args.iter().any(|a| {
            a == "--recursive"
                || (a.starts_with('-')
                    && !a.starts_with("--")
                    && a.get(1..).is_some_and(|flags| {
                        flags.chars().any(|c| c == 'r' || c == 'R')
                    }))
        });
        if recursive {
            for target in args.iter().filter(|a| !a.starts_with('-')) {
                let t = target.as_str();
                if t == "/"
                    || t == "/*"
                    || t == "~"
                    || t == "$HOME"
                    || t == "${HOME}"
                    || t.starts_with("~/")
                    || t.starts_with('$')
                {
                    return Err(format!(
                        "recursive rm targeting '{}' (filesystem/home destructive)",
                        target
                    ));
                }
            }
        }
    }

    // dd writing to a block device, e.g. `dd of=/dev/nvme0n1`.
    if prog == "dd" && args.iter().any(|a| a.starts_with("of=/dev/")) {
        return Err("dd writing directly to a block device".to_string());
    }

    // Filesystem formatting.
    if prog == "mkfs" || prog.starts_with("mkfs.") {
        return Err("filesystem formatting command".to_string());
    }

    // PowerShell encoded-command obfuscation via argv (substring layer also covers this).
    if (prog == "powershell" || prog == "pwsh")
        && args.iter().any(|a| {
            let l = a.to_lowercase();
            l == "-enc" || l == "-encodedcommand" || l == "-e"
        })
    {
        return Err("PowerShell encoded command (obfuscation)".to_string());
    }

    Ok(())
}

impl SecurityPolicy {
    pub fn new() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Always trust the working directory at construction time so that the
        // project root where Grok was opened is accessible from the very first
        // tool call — before any session/new or initialize message arrives.
        let canonical_cwd = working_directory
            .canonicalize()
            .unwrap_or_else(|_| working_directory.clone());
        Self {
            trusted_directories: vec![canonical_cwd],
            working_directory,
            external_access_config: ExternalAccessConfig::default(),
            shell_timeout_secs: crate::constants::DEFAULT_SHELL_TIMEOUT_SECS,
        }
    }

    pub fn with_external_access_config(mut self, config: ExternalAccessConfig) -> Self {
        self.external_access_config = config;
        self
    }

    pub fn with_working_directory(working_directory: PathBuf) -> Self {
        // Also trust the supplied working directory immediately so that callers
        // who use this constructor (e.g. tests) don't have to call
        // add_trusted_directory separately.
        let canonical = working_directory
            .canonicalize()
            .unwrap_or_else(|_| working_directory.clone());
        Self {
            trusted_directories: vec![canonical],
            working_directory,
            external_access_config: ExternalAccessConfig::default(),
            shell_timeout_secs: crate::constants::DEFAULT_SHELL_TIMEOUT_SECS,
        }
    }

    pub fn add_trusted_directory<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();
        // Canonicalize the path to resolve symlinks and make it absolute
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        // Deduplicate — the same directory can be registered many times
        // (once in SecurityPolicy::new, once explicitly in GrokAcpAgent::new,
        // and once per session in initialize_session). Pushing duplicates just
        // pollutes the trusted-directory list shown in TOOL-ERROR log entries.
        if !self.trusted_directories.contains(&canonical) {
            self.trusted_directories.push(canonical);
        }
    }

    /// Get the working directory
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// Return the list of trusted directories for diagnostic logging.
    ///
    /// These are the directories that the security policy considers "internal"
    /// (i.e. accessible without user approval).  Exposing them here lets the
    /// tool logger include them in error entries so it is immediately clear why
    /// an "Access denied" failure occurred.
    pub fn trusted_directories(&self) -> &[PathBuf] {
        &self.trusted_directories
    }

    /// Return the configured shell-command timeout in seconds.
    ///
    /// This is the value from `tools.shell.command_timeout_secs` in
    /// `config.toml`.  The `GROK_SHELL_TIMEOUT_SECS` environment variable
    /// takes precedence over this value when set.
    pub fn shell_timeout_secs(&self) -> u64 {
        self.shell_timeout_secs
    }

    /// Set the shell-command timeout (called once at startup from config).
    pub fn set_shell_timeout_secs(&mut self, secs: u64) {
        if secs > 0 {
            self.shell_timeout_secs = secs;
        }
    }

    /// Update the working directory and ensure it is in the trusted list.
    ///
    /// Call this when the Zed session workspace root is received via `session/new`
    /// so that relative paths like `.zed/task_list.json` resolve correctly.
    pub fn set_working_directory(&mut self, cwd: PathBuf) {
        let canonical = cwd.canonicalize().unwrap_or_else(|_| cwd.clone());
        tracing::debug!(
            old_cwd = %self.working_directory.display(),
            new_cwd = %canonical.display(),
            "SecurityPolicy: updating working directory"
        );
        self.working_directory = canonical.clone();
        if !self.trusted_directories.contains(&canonical) {
            self.trusted_directories.push(canonical);
        }
    }

    /// Check if external access logging is enabled
    pub fn is_external_access_logging_enabled(&self) -> bool {
        self.external_access_config.logging
    }

    /// Resolve a path to its canonical absolute form.
    ///
    /// This version is deliberately tolerant of non-existent deep directories
    /// (required by write_file when creating nested paths like deep/nested/file.txt).
    /// It walks upward to the first existing ancestor and re-attaches the suffix.
    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let p = path.as_ref();

        // Make absolute (respect Windows drive letters)
        let mut abs = if p.is_absolute() || is_windows_drive_absolute(p) {
            p.to_path_buf()
        } else {
            self.working_directory.join(p)
        };

        // If it already exists (or is a file that exists), just canonicalize.
        if let Ok(c) = abs.canonicalize() {
            return Ok(c);
        }

        // Walk up until we find an existing directory we can canonicalize.
        let mut suffix = std::path::PathBuf::new();

        loop {
            if abs.exists() {
                break;
            }
            if let Some(name) = abs.file_name() {
                suffix = Path::new(name).join(&suffix);
            }
            match abs.parent() {
                Some(parent) if parent != abs => {
                    abs = parent.to_path_buf();
                }
                _ => break,
            }
        }

        if abs.exists()
            && let Ok(canonical) = abs.canonicalize()
        {
            if suffix.as_os_str().is_empty() {
                return Ok(canonical);
            }
            return Ok(canonical.join(suffix));
        }

        // Final fallback — at least give an absolute path under the working dir.
        // is_internal_path will still compare against trusted directories.
        Ok(if p.is_absolute() || is_windows_drive_absolute(p) {
            p.to_path_buf()
        } else {
            self.working_directory.join(p)
        })
    }

    /// Check if a path is within internal project boundaries
    pub fn is_internal_path<P: AsRef<Path>>(&self, path: P) -> bool {
        // Resolve the path first
        let Ok(resolved) = self.resolve_path(path) else {
            return false;
        };

        // If no trusted directories are set, everything is untrusted (deny by default)
        if self.trusted_directories.is_empty() {
            return false;
        }

        self.trusted_directories
            .iter()
            .any(|trusted| resolved.starts_with(trusted))
    }

    /// Legacy method - kept for backward compatibility
    pub fn is_path_trusted<P: AsRef<Path>>(&self, path: P) -> bool {
        self.is_internal_path(path)
    }

    /// Check if external access is allowed for a path
    pub fn is_external_access_allowed<P: AsRef<Path>>(&self, path: P) -> ExternalAccessResult {
        // If external access is disabled, deny
        if !self.external_access_config.enabled {
            return ExternalAccessResult::Denied(
                "External access is disabled in configuration".to_string(),
            );
        }

        let resolved = match self.resolve_path(&path) {
            Ok(p) => p,
            Err(e) => return ExternalAccessResult::Denied(format!("Cannot resolve path: {}", e)),
        };

        // Check if path matches excluded patterns
        if self.is_path_excluded(&resolved) {
            return ExternalAccessResult::Denied(
                "Path matches excluded pattern (security protection)".to_string(),
            );
        }

        // Check if path is in allowed external paths
        let is_allowed = self
            .external_access_config
            .allowed_paths
            .iter()
            .any(|allowed| {
                // Canonicalize allowed path if possible
                let canonical_allowed = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
                resolved.starts_with(&canonical_allowed)
            });

        // Check session-trusted paths
        let session_trusted = {
            let session_paths = self
                .external_access_config
                .session_trusted_paths
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            session_paths
                .iter()
                .any(|trusted| resolved.starts_with(trusted))
        };

        if !is_allowed && !session_trusted {
            return ExternalAccessResult::Denied(
                "Path is not in allowed external paths or session-trusted paths".to_string(),
            );
        }

        // Check if approval is required
        if self.external_access_config.require_approval && !session_trusted {
            ExternalAccessResult::RequiresApproval(resolved)
        } else {
            ExternalAccessResult::Allowed(resolved)
        }
    }

    /// Check if path matches any excluded pattern
    fn is_path_excluded(&self, path: &Path) -> bool {
        use glob::Pattern;

        let path_str = path.to_string_lossy();
        self.external_access_config
            .excluded_patterns
            .iter()
            .any(|pattern| {
                // Use glob matching
                Pattern::new(pattern)
                    .map(|p| p.matches(&path_str))
                    .unwrap_or(false)
            })
    }

    /// Combined path validation (internal or external)
    pub fn validate_path_access<P: AsRef<Path>>(&self, path: P) -> Result<PathAccessType> {
        let path_ref = path.as_ref();

        // First check if it's internal (project paths)
        if self.is_internal_path(path_ref) {
            return Ok(PathAccessType::Internal(self.resolve_path(path_ref)?));
        }

        // Not internal, check external access
        match self.is_external_access_allowed(path_ref) {
            ExternalAccessResult::Allowed(resolved) => Ok(PathAccessType::External(resolved)),
            ExternalAccessResult::RequiresApproval(resolved) => {
                Ok(PathAccessType::ExternalRequiresApproval(resolved))
            }
            ExternalAccessResult::Denied(reason) => {
                // Build a diagnostic message that shows the caller exactly which
                // directories are currently trusted and what path was resolved.
                // This makes it much easier to understand why access failed when
                // Grok is running as an ACP server for a different project than
                // the one it was launched from.
                let resolved_display = self
                    .resolve_path(path_ref)
                    .map(|p| format!("{}", p.display()))
                    .unwrap_or_else(|_| format!("{}", path_ref.display()));

                let trusted_list = if self.trusted_directories.is_empty() {
                    "  (none — no trusted directories registered yet)".to_string()
                } else {
                    self.trusted_directories
                        .iter()
                        .map(|p| format!("  • {}", p.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                Err(anyhow!(
                    "Access denied: {}\n\
                     Requested path : {}\n\
                     Trusted directories:\n{}\n\
                     Tip: if this file is in your project, make sure Grok is \
                     launched from the project root, or @-mention any file in \
                     the project so the workspace root is auto-detected.",
                    reason,
                    resolved_display,
                    trusted_list,
                ))
            }
        }
    }

    /// Add a path to session-trusted paths (for "Trust Always" during session)
    pub fn add_session_trusted_path<P: AsRef<Path>>(&self, path: P) {
        let path = path.as_ref();
        if let Ok(canonical) = path.canonicalize() {
            let mut session_paths = self
                .external_access_config
                .session_trusted_paths
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if !session_paths.contains(&canonical) {
                session_paths.push(canonical);
            }
        }
    }

    /// Validate a shell command against a denylist of dangerous patterns.
    ///
    /// This is a defence-in-depth measure on top of the user-approval gate.
    /// It blocks the most commonly exploited shell patterns regardless of
    /// whether the user has pre-approved the tool.
    ///
    /// # Blocked categories
    ///
    /// | Category | Examples |
    /// |---|---|
    /// | Catastrophic filesystem destruction | `rm -rf /`, `Remove-Item C:\ -Recurse` |
    /// | Block device / disk wipe | `dd if=… of=/dev/sda`, `> /dev/sda` |
    /// | Remote code execution via pipe | `curl … \| bash`, `wget … \| sh` |
    /// | Reverse shells | `/dev/tcp/`, `nc -e`, `ncat --exec` |
    /// | Base64 obfuscation + execute | `base64 -d \| bash`, `echo … \| base64 -d \| sh` |
    /// | PowerShell encoded commands | `powershell -enc`, `powershell -EncodedCommand` |
    /// | PowerShell download + execute | `IEX`, `Invoke-Expression`, `iwr \| iex` |
    /// | Fork bombs | `:(){ :\|:& };:` |
    /// | Disk formatting | `mkfs`, `Format-Volume` |
    ///
    /// # How it works
    ///
    /// Two layers, both best-effort **defense-in-depth behind the user-approval
    /// gate** — never the sole protection:
    ///
    /// 1. **`argv[0]` analysis** (Task 469) — the command is shell-word split
    ///    (quotes/backslashes honoured, `${IFS}`/`$IFS` normalised to spaces)
    ///    and the real program name is matched: `r''m -rf /`, `"rm" -rf /`,
    ///    `sudo rm -rf /`, `rm${IFS}-rf${IFS}/`, and `rm -rf $HOME` are all
    ///    caught even though the substrings differ.
    /// 2. **Substring denylist** — catches known-dangerous patterns anywhere
    ///    in the command (pipe-to-shell, reverse shells, fork bombs, …).
    ///
    /// Known limits: unexpanded variables other than `IFS` are not resolved,
    /// and `sh -c '<payload>'` style indirection is not unpacked — the
    /// approval gate remains the real control.
    pub fn validate_shell_command(&self, command: &str) -> Result<()> {
        if command.trim().is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        // ── argv[0] layer (Task 469) ─────────────────────────────────────────
        // Tokenize first: quoting tricks like `r''m` or `"rm"` collapse to the
        // real program name, and `${IFS}` games become plain whitespace.
        if let Err(reason) = check_shell_argv0(command) {
            warn!(
                command = %command,
                reason = %reason,
                "Shell command blocked by security denylist (argv[0] analysis)"
            );
            return Err(anyhow!(
                "Command blocked for security reasons: {}.\n\
                 If this is a legitimate operation, run it directly in your terminal.",
                reason
            ));
        }

        // Normalise: lowercase for case-insensitive matching, collapse whitespace
        let normalised = command.to_lowercase();
        let collapsed: String = normalised.split_whitespace().collect::<Vec<_>>().join(" ");

        // ── Denylist entries ─────────────────────────────────────────────────
        // Each entry is (pattern_substring, human_reason).
        // We match against both the original (for symbol patterns) and the
        // collapsed-whitespace lowercase version (for keyword patterns).
        let denied: &[(&str, &str)] = &[
            // Catastrophic recursive deletes
            ("rm -rf /", "recursive deletion of filesystem root"),
            ("rm -rf ~", "recursive deletion of home directory"),
            ("rm -rf *", "recursive deletion of all files in directory"),
            (
                "rm --no-preserve-root",
                "deletion of filesystem root without guard",
            ),
            // PowerShell catastrophic deletes
            (
                "remove-item c:\\ -recurse",
                "recursive deletion of C: drive",
            ),
            (
                "remove-item / -recurse",
                "recursive deletion of filesystem root",
            ),
            // Disk / block device wipes
            ("of=/dev/sda", "writing directly to block device sda"),
            ("of=/dev/sdb", "writing directly to block device sdb"),
            ("of=/dev/nvme", "writing directly to NVMe block device"),
            ("> /dev/sda", "overwriting block device sda"),
            // Disk formatting
            ("mkfs", "filesystem formatting command"),
            ("format-volume", "PowerShell disk format command"),
            // Remote code execution via pipe-to-shell
            ("| bash", "piping remote content directly to bash"),
            ("| sh", "piping remote content directly to sh"),
            ("| zsh", "piping remote content directly to zsh"),
            ("|bash", "piping remote content directly to bash"),
            ("|sh", "piping remote content directly to sh"),
            // Base64 decode + execute
            ("base64 -d | ", "base64-decode piped to shell execution"),
            ("base64 -d|", "base64-decode piped to shell execution"),
            // Reverse shell patterns
            ("/dev/tcp/", "bash /dev/tcp reverse shell"),
            ("/dev/udp/", "bash /dev/udp reverse shell"),
            ("nc -e ", "netcat execute reverse shell"),
            ("nc -e\t", "netcat execute reverse shell"),
            ("ncat --exec", "ncat execute reverse shell"),
            ("ncat -e ", "ncat execute reverse shell"),
            // PowerShell encoded command (common obfuscation)
            ("-enc ", "PowerShell base64-encoded command (obfuscation)"),
            (
                "-encodedcommand",
                "PowerShell base64-encoded command (obfuscation)",
            ),
            // PowerShell download + execute
            (
                "invoke-expression",
                "PowerShell Invoke-Expression (remote code execution)",
            ),
            (" iex ", "PowerShell IEX alias (remote code execution)"),
            ("(iex ", "PowerShell IEX alias (remote code execution)"),
            // Fork bomb
            (":(){ :|:& };:", "shell fork bomb"),
            // Crontab injection
            ("crontab -", "crontab modification"),
            // LD_PRELOAD / library injection
            ("ld_preload=", "LD_PRELOAD library injection"),
        ];

        for (pattern, reason) in denied {
            if collapsed.contains(pattern) || command.to_lowercase().contains(pattern) {
                warn!(
                    command = %command,
                    pattern = %pattern,
                    reason  = %reason,
                    "Shell command blocked by security denylist"
                );
                return Err(anyhow!(
                    "Command blocked for security reasons: {} \
                     (matched pattern '{}').\n\
                     If this is a legitimate operation, run it directly in your terminal.",
                    reason,
                    pattern
                ));
            }
        }

        Ok(())
    }
}

/// Result type for external access checks
#[derive(Debug)]
pub enum ExternalAccessResult {
    Allowed(PathBuf),
    RequiresApproval(PathBuf),
    Denied(String),
}

/// Type of path access (internal project or external)
#[derive(Debug)]
pub enum PathAccessType {
    Internal(PathBuf),
    External(PathBuf),
    ExternalRequiresApproval(PathBuf),
}

pub struct SecurityManager {
    policy: Arc<Mutex<SecurityPolicy>>,
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            policy: Arc::new(Mutex::new(SecurityPolicy::new())),
        }
    }

    pub fn new_with_config(config: ExternalAccessConfig) -> Self {
        let policy = SecurityPolicy::new().with_external_access_config(config);
        Self {
            policy: Arc::new(Mutex::new(policy)),
        }
    }

    pub fn get_policy(&self) -> SecurityPolicy {
        self.policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn update_external_access_config(&self, config: ExternalAccessConfig) {
        self.policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .external_access_config = config;
    }

    pub fn add_trusted_directory<P: AsRef<Path>>(&self, path: P) {
        self.policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .add_trusted_directory(path);
    }

    /// Apply the shell-command timeout from `config.toml` to the policy.
    ///
    /// Call this once in `GrokAcpAgent::new()` after the config is loaded.
    /// The `GROK_SHELL_TIMEOUT_SECS` env var still overrides this at runtime.
    pub fn set_shell_timeout_secs(&self, secs: u64) {
        self.policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_shell_timeout_secs(secs);
    }

    /// Update the policy's working directory to the Zed session workspace root.
    ///
    /// Without this call, relative paths like `.zed/task_list.json` resolve
    /// against the process launch directory instead of the actual project root.
    pub fn set_working_directory(&self, cwd: &Path) {
        self.policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_working_directory(cwd.to_path_buf());
    }

    pub fn check_path_access<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if self.get_policy().is_path_trusted(path) {
            Ok(())
        } else {
            Err(anyhow!("Access denied: Path is not in a trusted directory"))
        }
    }

    pub fn add_session_trusted_path<P: AsRef<Path>>(&self, path: P) {
        self.policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .add_session_trusted_path(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_absolute_path_trusted() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let mut policy = SecurityPolicy::with_working_directory(temp_path.clone());
        policy.add_trusted_directory(&temp_path);

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test").unwrap();

        // Absolute path should be trusted
        assert!(policy.is_path_trusted(&file_path));
    }

    #[test]
    fn test_relative_path_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let mut policy = SecurityPolicy::with_working_directory(temp_path.clone());
        policy.add_trusted_directory(&temp_path);

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test").unwrap();

        // Relative path should be resolved and trusted
        assert!(policy.is_path_trusted("test.txt"));
        assert!(policy.is_path_trusted("./test.txt"));
    }

    #[test]
    fn test_parent_directory_access() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create subdirectory
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        // Create file in parent
        let file_path = temp_path.join("parent.txt");
        fs::write(&file_path, "test").unwrap();

        // Set working directory to subdirectory, but trust parent
        let mut policy = SecurityPolicy::with_working_directory(sub_dir.clone());
        policy.add_trusted_directory(&temp_path);

        // Access file in parent using relative path
        assert!(policy.is_path_trusted("../parent.txt"));
    }

    #[test]
    fn test_path_outside_trusted_denied() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let mut policy = SecurityPolicy::with_working_directory(temp_path.clone());
        policy.add_trusted_directory(&temp_path);

        // Path outside trusted directory should be denied
        #[cfg(target_os = "windows")]
        let outside_path = "C:\\Windows\\System32\\cmd.exe";
        #[cfg(not(target_os = "windows"))]
        let outside_path = "/etc/passwd";

        assert!(!policy.is_path_trusted(outside_path));
    }

    #[test]
    fn test_resolve_path_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let policy = SecurityPolicy::with_working_directory(temp_path.clone());

        // Should resolve path even if file doesn't exist yet
        let result = policy.resolve_path("newfile.txt");
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved, temp_path.join("newfile.txt"));
    }

    #[test]
    fn test_symlink_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a file
        let real_file = temp_path.join("real.txt");
        fs::write(&real_file, "test").unwrap();

        // Create a symlink (skip on Windows if not admin)
        #[cfg(unix)]
        {
            let link_path = temp_path.join("link.txt");
            std::os::unix::fs::symlink(&real_file, &link_path).unwrap();

            let mut policy = SecurityPolicy::with_working_directory(temp_path.clone());
            policy.add_trusted_directory(&temp_path);

            // Symlink should resolve to real path and be trusted
            assert!(policy.is_path_trusted("link.txt"));
        }
    }

    #[test]
    fn test_multiple_trusted_directories() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let temp_path1 = temp_dir1.path().canonicalize().unwrap();
        let temp_path2 = temp_dir2.path().canonicalize().unwrap();

        let mut policy = SecurityPolicy::with_working_directory(temp_path1.clone());
        policy.add_trusted_directory(&temp_path1);
        policy.add_trusted_directory(&temp_path2);

        // Create files in both directories
        let file1 = temp_path1.join("file1.txt");
        let file2 = temp_path2.join("file2.txt");
        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        // Both should be trusted
        assert!(policy.is_path_trusted(&file1));
        assert!(policy.is_path_trusted(&file2));
    }

    #[test]
    fn test_working_directory_auto_trusted() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a test file inside the working directory
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test").unwrap();

        // with_working_directory now auto-trusts the supplied path, so a file
        // inside it should be accessible without calling add_trusted_directory.
        let policy = SecurityPolicy::with_working_directory(temp_path.clone());
        assert!(
            policy.is_path_trusted(&file_path),
            "working directory should be trusted automatically"
        );
    }

    #[test]
    fn test_path_outside_working_directory_not_auto_trusted() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let temp_path1 = temp_dir1.path().canonicalize().unwrap();
        let temp_path2 = temp_dir2.path().canonicalize().unwrap();

        // dir2 is NOT the working directory and was never explicitly trusted
        let file_in_dir2 = temp_path2.join("secret.txt");
        fs::write(&file_in_dir2, "secret").unwrap();

        let policy = SecurityPolicy::with_working_directory(temp_path1.clone());
        assert!(
            !policy.is_path_trusted(&file_in_dir2),
            "a directory that was never trusted should remain inaccessible"
        );
    }

    #[test]
    fn test_security_manager() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let manager = SecurityManager::new();
        manager.add_trusted_directory(&temp_path);

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test").unwrap();

        // Should be able to check access
        assert!(manager.check_path_access(&file_path).is_ok());

        // Path outside should be denied
        #[cfg(target_os = "windows")]
        let outside_path = "C:\\Windows\\System32\\cmd.exe";
        #[cfg(not(target_os = "windows"))]
        let outside_path = "/etc/passwd";

        assert!(manager.check_path_access(outside_path).is_err());
    }

    // ── Task 469: argv[0] denylist bypass regressions ─────────────────────────
    #[test]
    fn test_argv0_blocks_recursive_rm_variants() {
        let policy = SecurityPolicy::new();
        // Classic cases (also caught by the substring layer).
        assert!(policy.validate_shell_command("rm -rf /").is_err());
        // Bypass variants the substring layer missed (Task 469).
        assert!(policy.validate_shell_command("rm -rf /*").is_err());
        assert!(policy.validate_shell_command("rm -rf $HOME").is_err());
        assert!(policy.validate_shell_command("rm -rf ${HOME}").is_err());
        assert!(policy.validate_shell_command("rm -rf ~").is_err());
        assert!(policy.validate_shell_command("r''m -rf /").is_err());
        assert!(policy.validate_shell_command("\"rm\" -rf /").is_err());
        assert!(policy.validate_shell_command("rm${IFS}-rf${IFS}/").is_err());
        assert!(policy.validate_shell_command("sudo rm -rf /").is_err());
        assert!(policy.validate_shell_command("/bin/rm -rf /").is_err());
        assert!(policy.validate_shell_command("rm -rfv /").is_err());
    }

    #[test]
    fn test_argv0_blocks_dd_and_mkfs_variants() {
        let policy = SecurityPolicy::new();
        assert!(policy.validate_shell_command("dd if=/dev/zero of=/dev/sda").is_err());
        // Generalized block-device target (substring layer only knew sda/sdb/nvme).
        assert!(policy.validate_shell_command("dd if=/dev/zero of=/dev/nvme0n1").is_err());
        assert!(policy.validate_shell_command("mkfs.ext4 /dev/sda1").is_err());
    }

    #[test]
    fn test_argv0_allows_benign_commands() {
        let policy = SecurityPolicy::new();
        assert!(policy.validate_shell_command("echo hello").is_ok());
        assert!(policy.validate_shell_command("ls /").is_ok());
        assert!(policy.validate_shell_command("rm -rf ./target/tmp").is_ok());
        assert!(policy.validate_shell_command("rm /tmp/file.txt").is_ok());
        assert!(policy.validate_shell_command("cargo test -- --nocapture").is_ok());
    }

    #[test]
    fn test_shell_word_split_quoting() {
        assert_eq!(shell_word_split("r''m -rf /"), vec!["rm", "-rf", "/"]);
        assert_eq!(shell_word_split("\"rm\" -rf /"), vec!["rm", "-rf", "/"]);
        assert_eq!(
            shell_word_split("rm${IFS}-rf${IFS}/"),
            vec!["rm", "-rf", "/"]
        );
        assert_eq!(shell_word_split("echo 'a b' c"), vec!["echo", "a b", "c"]);
    }
}
