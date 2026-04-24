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
            shell_timeout_secs: 300,
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
            shell_timeout_secs: 300,
        }
    }

    pub fn add_trusted_directory<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();
        // Canonicalize the path to resolve symlinks and make it absolute
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.trusted_directories.push(canonical);
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

    /// Check if external access logging is enabled
    pub fn is_external_access_logging_enabled(&self) -> bool {
        self.external_access_config.logging
    }

    /// Resolve a path to its canonical absolute form
    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let path = path.as_ref();

        // Convert to absolute path relative to working directory
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };

        // Canonicalize to resolve symlinks and .. components
        // If the file doesn't exist yet, try to canonicalize the parent
        absolute.canonicalize().or_else(|_| {
            if let Some(parent) = absolute.parent() {
                let canonical_parent = parent.canonicalize()?;
                if let Some(file_name) = absolute.file_name() {
                    Ok(canonical_parent.join(file_name))
                } else {
                    Ok(canonical_parent)
                }
            } else {
                Ok(absolute)
            }
        })
    }

    /// Check if a path is within internal project boundaries
    pub fn is_internal_path<P: AsRef<Path>>(&self, path: P) -> bool {
        // Resolve the path first
        let resolved = match self.resolve_path(path) {
            Ok(p) => p,
            Err(_) => return false,
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
    pub fn validate_shell_command(&self, command: &str) -> Result<()> {
        if command.trim().is_empty() {
            return Err(anyhow!("Command cannot be empty"));
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
}
