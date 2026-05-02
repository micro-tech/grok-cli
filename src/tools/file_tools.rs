//! File-system tools — read, write, list, search, and replace file content.
//!
//! Every function takes a [`SecurityPolicy`] reference so the ACP layer can
//! keep calling them with the same signature it already uses.

use crate::acp::security::{PathAccessType, SecurityPolicy};
use crate::cli::approval::{ApprovalDecision, prompt_external_access_approval};
use crate::security::audit::{AuditLogger, create_access_log};
use anyhow::{Result, anyhow};
use glob::glob;
use regex::Regex;
use tokio::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};

use std::path::Path;
use tracing::info;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// read_file
// ─────────────────────────────────────────────────────────────────────────────

/// Read file content with full external-access approval / audit flow.
///
/// * Internal paths (under any trusted directory) are read immediately.
/// * External paths that have `auto_approve` set are read after audit-logging.
/// * External paths that require approval prompt the user via
///   [`prompt_external_access_approval`] before proceeding.
pub async fn read_file(path: &str, security: &SecurityPolicy) -> Result<String> {
    let access_type = security.validate_path_access(path)?;

    let resolved_path = match &access_type {
        PathAccessType::Internal(path) => path.clone(),
        PathAccessType::External(path) => {
            if security.is_external_access_logging_enabled() {
                info!("External file access (auto-approved): {}", path.display());
                if let Ok(logger) = AuditLogger::new(true) {
                    let session_id = Uuid::new_v4().to_string();
                    let log = create_access_log(
                        path.to_str().unwrap_or("unknown"),
                        "read",
                        "allowed",
                        &session_id,
                        None,
                        Some("auto-approved".to_string()),
                    );
                    let _ = logger.log_access(log);
                }
            }
            path.clone()
        }
        PathAccessType::ExternalRequiresApproval(path) => {
            info!("External file access requested: {}", path.display());

            let config_source = if std::env::var("GROK_EXTERNAL_ACCESS_ENABLED").is_ok() {
                "environment variable"
            } else {
                ".grok/.env or config.toml"
            };

            let session_id = Uuid::new_v4().to_string();
            let path_str = path.to_str().unwrap_or("unknown");

            match prompt_external_access_approval(path, config_source) {
                Ok(ApprovalDecision::AllowOnce) => {
                    info!("External file access approved (once): {}", path.display());
                    if security.is_external_access_logging_enabled()
                        && let Ok(logger) = AuditLogger::new(true)
                    {
                        let log = create_access_log(
                            path_str,
                            "read",
                            "approved_once",
                            &session_id,
                            None,
                            Some(config_source.to_string()),
                        );
                        let _ = logger.log_access(log);
                    }
                    path.clone()
                }
                Ok(ApprovalDecision::TrustAlways) => {
                    info!(
                        "External file access approved (session): {}",
                        path.display()
                    );
                    if security.is_external_access_logging_enabled()
                        && let Ok(logger) = AuditLogger::new(true)
                    {
                        let log = create_access_log(
                            path_str,
                            "read",
                            "approved_always",
                            &session_id,
                            None,
                            Some(config_source.to_string()),
                        );
                        let _ = logger.log_access(log);
                    }
                    // NOTE: session-trust mutation requires a mutable policy reference;
                    // callers that need session-trust should call
                    // `security.add_session_trusted_path(path)` before invoking this function.
                    path.clone()
                }
                Ok(ApprovalDecision::Deny) => {
                    use tracing::warn;
                    warn!("External file access denied by user: {}", path.display());
                    if security.is_external_access_logging_enabled()
                        && let Ok(logger) = AuditLogger::new(true)
                    {
                        let log = create_access_log(
                            path_str,
                            "read",
                            "denied",
                            &session_id,
                            Some("User denied access".to_string()),
                            Some(config_source.to_string()),
                        );
                        let _ = logger.log_access(log);
                    }
                    return Err(anyhow!("Access denied by user"));
                }
                Err(e) => {
                    use tracing::warn;
                    warn!("External file access approval failed: {}", e);
                    if security.is_external_access_logging_enabled()
                        && let Ok(logger) = AuditLogger::new(true)
                    {
                        let log = create_access_log(
                            path_str,
                            "read",
                            "error",
                            &session_id,
                            Some(format!("Approval prompt failed: {}", e)),
                            Some(config_source.to_string()),
                        );
                        let _ = logger.log_access(log);
                    }
                    return Err(anyhow!("Approval prompt failed: {}", e));
                }
            }
        }
    };

    if !resolved_path.exists() {
        return Err(anyhow!("File not found: {}", resolved_path.display()));
    }

    fs::read_to_string(&resolved_path).await.map_err(|e| anyhow!("Failed to read file: {}", e))
}

// ─────────────────────────────────────────────────────────────────────────────
// read_multiple_files
// ─────────────────────────────────────────────────────────────────────────────

/// Read multiple files at once, returning a formatted concatenation.
///
/// Each file is prefixed with a `--- File: <path> ---` header. Errors for
/// individual files are reported inline rather than aborting the whole call.
pub async fn read_multiple_files(paths: Vec<String>, security: &SecurityPolicy) -> Result<String> {
    let mut results = Vec::new();
    for path in paths {
        match read_file(&path, security).await {
            Ok(content) => {
                results.push(format!("--- File: {} ---\n{}\n", path, content));
            }
            Err(e) => {
                results.push(format!("--- File: {} ---\nError: {}\n", path, e));
            }
        }
    }
    Ok(results.join("\n"))
}

// ─────────────────────────────────────────────────────────────────────────────
// list_code_definitions
// ─────────────────────────────────────────────────────────────────────────────

/// List top-level code definitions (functions, structs, classes, etc.) in a file.
///
/// Uses a heuristic regex that recognises common definition keywords across
/// Rust, JavaScript, TypeScript, Python, Go, and C++.
pub async fn list_code_definitions(path: &str, security: &SecurityPolicy) -> Result<String> {
    let content = read_file(path, security).await?;

    let re = Regex::new(
        r"(?m)^[\t ]*(pub|async|unsafe|static|export|default|class|def|fn|func|struct|enum|trait|impl|interface|type|const|let|var)\b",
    )
    .map_err(|e| anyhow!("Invalid regex: {}", e))?;

    let mut results = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if re.is_match(line) {
            let trimmed = line.trim();
            if !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*')
            {
                results.push(format!("{}: {}", i + 1, trimmed));
            }
        }
    }

    if results.is_empty() {
        Ok("No definitions found matching common patterns.".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// write_file
// ─────────────────────────────────────────────────────────────────────────────

/// Write content to a file, creating parent directories as needed.
///
/// Applies the same external-access / audit flow as [`read_file`].
/// Writes to paths that `ExternalRequiresApproval` are blocked — the caller
/// (ACP dispatch) handles the approval before invoking this function.
pub async fn write_file(path: &str, content: &str, security: &SecurityPolicy) -> Result<String> {
    let path_ref = Path::new(path);
    let absolute_path = if path_ref.is_absolute() {
        path_ref.to_path_buf()
    } else {
        security.working_directory().join(path_ref)
    };

    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| anyhow!("Failed to create directory: {}", e))?;
    }

    let access_type = security.validate_path_access(path)?;

    let resolved_path = match &access_type {
        PathAccessType::Internal(p) => p.clone(),
        PathAccessType::External(p) => {
            if security.is_external_access_logging_enabled() {
                info!("External file write (auto-approved): {}", p.display());
            }
            p.clone()
        }
        PathAccessType::ExternalRequiresApproval(p) => {
            return Err(anyhow!(
                "Access denied: write to external path '{}' requires explicit approval",
                p.display()
            ));
        }
    };

    fs::write(&resolved_path, content).await.map_err(|e| anyhow!("Failed to write file: {}", e))?;
    info!(
        "Wrote {} bytes to {}",
        content.len(),
        resolved_path.display()
    );
    Ok(format!("Successfully wrote to {}", resolved_path.display()))
}

// ─────────────────────────────────────────────────────────────────────────────
// replace
// ─────────────────────────────────────────────────────────────────────────────

/// Replace text in a file.
///
/// Fails if the `old_string` is not found or if `expected_replacements` is
/// given and doesn't match the actual occurrence count.
pub async fn replace(
    path: &str,
    old_string: &str,
    new_string: &str,
    expected_replacements: Option<u32>,
    security: &SecurityPolicy,
) -> Result<String> {
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    if !resolved_path.exists() {
        return Err(anyhow!("File not found: {}", resolved_path.display()));
    }

    let content =
        fs::read_to_string(&resolved_path).await.map_err(|e| anyhow!("Failed to read file: {}", e))?;

    let occurrences = content.matches(old_string).count();
    if occurrences == 0 {
        return Err(anyhow!(
            "Failed to replace: '{}' not found in file. Use read_file to verify content.",
            old_string
        ));
    }

    if let Some(expected) = expected_replacements
        && occurrences != expected as usize
    {
        return Err(anyhow!(
            "Failed to replace: Expected {} occurrences, but found {}.",
            expected,
            occurrences
        ));
    }

    let new_content = content.replace(old_string, new_string);
    fs::write(&resolved_path, new_content).await.map_err(|e| anyhow!("Failed to write file: {}", e))?;

    Ok(format!(
        "Successfully replaced {} occurrence(s) in {}",
        occurrences,
        resolved_path.display()
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// list_directory
// ─────────────────────────────────────────────────────────────────────────────

/// List files and sub-directories in a directory.
///
/// Directories are suffixed with `/`.
pub fn list_directory(path: &str, security: &SecurityPolicy) -> Result<String> {
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    if !resolved_path.exists() {
        return Err(anyhow!("Directory not found: {}", resolved_path.display()));
    }

    if !resolved_path.is_dir() {
        return Err(anyhow!(
            "Path is not a directory: {}",
            resolved_path.display()
        ));
    }

    let mut entries = Vec::new();
    for entry in
        std::fs::read_dir(&resolved_path).map_err(|e| anyhow!("Failed to read directory: {}", e))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let is_dir = path.is_dir();
        entries.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
    }

    Ok(entries.join("\n"))
}

// ─────────────────────────────────────────────────────────────────────────────
// glob_search
// ─────────────────────────────────────────────────────────────────────────────

/// Find files matching a glob pattern (e.g. `**/*.rs`).
///
/// Only paths that are inside a trusted directory are returned.
pub fn glob_search(pattern: &str, security: &SecurityPolicy) -> Result<String> {
    let mut matches = Vec::new();
    for entry in glob(pattern).map_err(|e| anyhow!("Failed to read glob pattern: {}", e))? {
        match entry {
            Ok(path) => {
                if security.is_path_trusted(&path) {
                    matches.push(path.display().to_string());
                }
            }
            Err(e) => return Err(anyhow!("Error matching glob: {}", e)),
        }
    }

    if matches.is_empty() {
        Ok("No files found matching pattern".to_string())
    } else {
        Ok(matches.join("\n"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// search_file_content
// ─────────────────────────────────────────────────────────────────────────────

/// Search for a regex pattern in file content (grep-style).
///
/// If `path` is a directory, the search is performed recursively.
/// Lines that cannot be decoded as UTF-8 are silently skipped.
pub fn search_file_content(path: &str, pattern: &str, security: &SecurityPolicy) -> Result<String> {
    let resolved_path = security
        .resolve_path(path)
        .map_err(|e| anyhow!("Failed to resolve path '{}': {}", path, e))?;

    if !security.is_path_trusted(&resolved_path) {
        return Err(anyhow!("Access denied: Path is not in a trusted directory"));
    }

    let re = Regex::new(pattern).map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

    if resolved_path.is_dir() {
        let mut results = Vec::new();
        for entry in walkdir::WalkDir::new(&resolved_path) {
            let entry = entry.map_err(|e| anyhow!("Error walking directory: {}", e))?;
            if entry.file_type().is_file() {
                let entry_path = entry.path();
                if !security.is_path_trusted(entry_path) {
                    continue;
                }
                let file =
                    File::open(entry_path).map_err(|e| anyhow!("Failed to open file: {}", e))?;
                let reader = BufReader::new(file);
                for (i, line) in reader.lines().enumerate() {
                    match line {
                        Ok(line) => {
                            if re.is_match(&line) {
                                results.push(format!(
                                    "{}:{}: {}",
                                    entry_path.display(),
                                    i + 1,
                                    line
                                ));
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
        if results.is_empty() {
            Ok("No matches found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    } else {
        let file = File::open(&resolved_path).map_err(|e| anyhow!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut results = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            match line {
                Ok(line) => {
                    if re.is_match(&line) {
                        results.push(format!("{}:{}: {}", resolved_path.display(), i + 1, line));
                    }
                }
                Err(_) => continue,
            }
        }
        if results.is_empty() {
            Ok("No matches found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_security(dir: &TempDir) -> SecurityPolicy {
        SecurityPolicy::with_working_directory(dir.path().to_path_buf())
    }

    #[tokio::test]
    async fn write_then_read_file() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("hello.txt");
        let path_str = path.to_str().unwrap();

        write_file(path_str, "Hello, world!", &security).await.unwrap();
        let content = read_file(path_str, &security).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[tokio::test]
    async fn read_file_missing_returns_err() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let result = read_file("non_existent_file.txt", &security).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_multiple_files_partial_errors() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("a.txt");
        let path_str = path.to_str().unwrap().to_string();
        write_file(path_str.as_str(), "content", &security).await.unwrap();

        let result =
            read_multiple_files(vec![path_str, "missing.txt".to_string()], &security).await.unwrap();
        assert!(result.contains("content"));
        assert!(result.contains("Error"));
    }

    #[tokio::test]
    fn list_directory_returns_entries() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let f = dir.path().join("test.txt");
        std::fs::write(&f, "x").unwrap();

        let result = list_directory(dir.path().to_str().unwrap(), &security).unwrap();
        assert!(result.contains("test.txt"));
    }

    #[tokio::test]
    fn glob_search_finds_files() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        std::fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();

        let pattern = format!("{}/*.rs", dir.path().display());
        let result = glob_search(&pattern, &security).unwrap();
        assert!(result.contains("a.rs"), "expected a.rs in: {}", result);
    }

    #[tokio::test]
    async fn replace_text_in_file() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("r.txt");
        let path_str = path.to_str().unwrap();

        write_file(path_str, "foo bar foo", &security).unwrap();
        replace(path_str, "foo", "baz", None, &security).await.unwrap();
        let content = read_file(path_str, &security).unwrap();
        assert_eq!(content, "baz bar baz");
    }

    #[tokio::test]
    async fn replace_not_found_returns_err() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("r2.txt");
        let path_str = path.to_str().unwrap();

        write_file(path_str, "hello world", &security).unwrap();
        let result = replace(path_str, "notfound", "x", None, &security).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    fn search_file_content_finds_match() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("code.rs");
        let path_str = path.to_str().unwrap();
        write_file(path_str, "fn main() {}\nfn helper() {}", &security).await.unwrap();

        let result = search_file_content(path_str, "fn ", &security).unwrap();
        assert!(result.contains("fn main") || result.contains("fn helper"));
    }

    #[tokio::test]
    async fn list_code_definitions_finds_fns() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("src.rs");
        let path_str = path.to_str().unwrap();
        write_file(path_str, "pub fn foo() {}\nstruct Bar {}", &security).await.unwrap();

        let result = list_code_definitions(path_str, &security).await.unwrap();
        assert!(result.contains("fn foo") || result.contains("struct Bar"));
    }

    // Suppress unused import warning for Write — kept for future test helpers
    // that write to byte buffers directly.
    #[allow(dead_code)]
    fn _assert_write_imported(_: &dyn Write) {}
}
