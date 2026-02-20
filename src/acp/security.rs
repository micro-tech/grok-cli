use crate::config::ExternalAccessConfig;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    trusted_directories: Vec<PathBuf>,
    working_directory: PathBuf,
    external_access_config: ExternalAccessConfig,
}

impl SecurityPolicy {
    pub fn new() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            trusted_directories: Vec::new(),
            working_directory,
            external_access_config: ExternalAccessConfig::default(),
        }
    }

    pub fn with_external_access_config(mut self, config: ExternalAccessConfig) -> Self {
        self.external_access_config = config;
        self
    }

    pub fn with_working_directory(working_directory: PathBuf) -> Self {
        Self {
            trusted_directories: Vec::new(),
            working_directory,
            external_access_config: ExternalAccessConfig::default(),
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
                .expect("Session trusted paths mutex poisoned");
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
            ExternalAccessResult::Denied(reason) => Err(anyhow!("Access denied: {}", reason)),
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
                .expect("Session trusted paths mutex poisoned");
            if !session_paths.contains(&canonical) {
                session_paths.push(canonical);
            }
        }
    }

    pub fn validate_shell_command(&self, command: &str) -> Result<()> {
        // Basic blacklist for really dangerous things if needed,
        // but mostly we rely on user confirmation + trusted scope.
        // For now, allow all if confirmed.
        if command.trim().is_empty() {
            return Err(anyhow!("Command cannot be empty"));
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
            .expect("SecurityManager mutex poisoned - this is a bug")
            .clone()
    }

    pub fn update_external_access_config(&self, config: ExternalAccessConfig) {
        self.policy
            .lock()
            .expect("SecurityManager mutex poisoned - this is a bug")
            .external_access_config = config;
    }

    pub fn add_trusted_directory<P: AsRef<Path>>(&self, path: P) {
        self.policy
            .lock()
            .expect("SecurityManager mutex poisoned - this is a bug")
            .add_trusted_directory(path);
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
            .expect("SecurityManager mutex poisoned - this is a bug")
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
    fn test_empty_trusted_directories() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let policy = SecurityPolicy::with_working_directory(temp_path.clone());

        // Without any trusted directories, nothing should be trusted
        assert!(!policy.is_path_trusted("test.txt"));
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
