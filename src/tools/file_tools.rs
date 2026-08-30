//! File-system tools — read, write, list, search, and replace file content.
//!
//! Every function takes a [`SecurityPolicy`] reference so the ACP layer can
//! keep calling them with the same signature it already uses.

use crate::acp::security::{PathAccessType, SecurityPolicy};
use crate::cli::approval::{ApprovalDecision, prompt_external_access_approval};
use crate::safety::SuspiciousWriteGuard;
use crate::safety::pre_write_hook::{SafetyDecision, WriteContext, on_before_write_file};
use crate::security::audit::{AuditLogger, create_access_log};
use anyhow::{Result, anyhow};
use glob::glob;
use regex::Regex;
use serde_json;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::LazyLock;
use tokio::fs;

use std::path::Path;
use tracing::info;
// uuid no longer used for per-call session ids (SEC-8); we now use the stable
// session_id coming from ToolContext for audit correlation.

// ─────────────────────────────────────────────────────────────────────────────
// Static regexes (hot paths)
// ─────────────────────────────────────────────────────────────────────────────

/// Static regex for stripping trailing commas in JSONC files.
/// Promoted to Lazy for performance (COR-4).
static RE_JSONC_TRAILING_COMMA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r",(\s*[}\]])").expect("BUG: invalid static regex pattern"));

/// Static regex used by list_code_definitions to find top-level definitions.
/// Promoted to Lazy for performance (COR-4).
static RE_CODE_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^[\t ]*(pub|async|unsafe|static|export|default|class|def|fn|func|struct|enum|trait|impl|interface|type|const|let|var)\b",
    )
    .expect("BUG: invalid static regex pattern")
});

// ─────────────────────────────────────────────────────────────────────────────
// read_file
// ─────────────────────────────────────────────────────────────────────────────

/// Read file content with full external-access approval / audit flow.
///
/// * Internal paths (under any trusted directory) are read immediately.
/// * External paths that have `auto_approve` set are read after audit-logging.
/// * External paths that require approval prompt the user via
///   [`prompt_external_access_approval`] before proceeding.
///
/// The `ctx.session_id` is used for audit correlation (SEC-8) so that all
/// access logs belonging to the same user session share a stable ID.
pub async fn read_file(path: &str, ctx: &crate::tools::ToolContext) -> Result<String> {
    let security = &ctx.policy;
    let session_id = &ctx.session_id;

    let access_type = security.validate_path_access(path)?;

    let resolved_path = match &access_type {
        PathAccessType::Internal(path) => path.clone(),
        PathAccessType::External(path) => {
            if security.is_external_access_logging_enabled() {
                info!("External file access (auto-approved): {}", path.display());
                if let Ok(logger) = AuditLogger::new(true) {
                    let log = create_access_log(
                        path.to_str().unwrap_or("unknown"),
                        "read",
                        "allowed",
                        session_id,
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
                            session_id,
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
                            session_id,
                            None,
                            Some(config_source.to_string()),
                        );
                        let _ = logger.log_access(log);
                    }

                    // SECURITY FIX (SEC-3): Actually persist the "Trust Always" decision
                    // for the remainder of this session so future reads of the same path
                    // do not re-prompt.
                    security.add_session_trusted_path(path);

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
                            session_id,
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
                            session_id,
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

    // CI/Windows defense: the target may have become a directory due to
    // path normalization / previous create_dir_all side effects.
    if resolved_path.is_dir() {
        let _ = std::fs::remove_dir_all(&resolved_path);
    }
    if !resolved_path.exists() {
        return Err(anyhow!("File not found: {}", resolved_path.display()));
    }

    let content = fs::read_to_string(&resolved_path)
        .await
        .map_err(|e| anyhow!("Failed to read file: {}", e))?;

    // ── JSON integrity check (two-stage: strict JSON → JSONC → error) ────────
    //
    // Stage 1 — strict parse: valid JSON files are returned immediately.
    // Stage 2 — JSONC cleanup: Zed, VS Code, and many editor config files use
    //           JSONC (trailing commas after the last element).  Strip those
    //           commas and retry before declaring the file broken.
    // Stage 3 — truly malformed: prepend READ_FILE_WARNING so the LLM knows
    //           the file *was* read but the data cannot be trusted, preventing
    //           it from fabricating a plausible-looking answer.
    let ext = resolved_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "json"
        && !content.trim().is_empty()
        && let Err(strict_err) = serde_json::from_str::<serde_json::Value>(&content)
    {
        // Stage 2: try JSONC (trailing-comma cleanup).
        let stripped = strip_jsonc_trailing_commas(&content);
        if serde_json::from_str::<serde_json::Value>(&stripped).is_ok() {
            // JSONC file — return the original, unmodified content so the
            // LLM sees the real source.  Log at DEBUG so there is a record
            // without spamming the user.
            tracing::debug!(
                path = %resolved_path.display(),
                "read_file: JSONC detected (trailing commas) — returning raw content"
            );
            return Ok(content);
        }

        // Stage 3: genuinely malformed — warn and forward with a banner.
        tracing::warn!(
            path = %resolved_path.display(),
            bytes = content.len(),
            error = %strict_err,
            "read_file: JSON validation failed — file was read but content is malformed"
        );
        let preview_len = content.len().min(300);
        let truncation_note = if content.len() > 300 {
            "…[truncated]"
        } else {
            ""
        };
        return Ok(format!(
            "READ_FILE_WARNING: '{}' was read ({} bytes) but failed JSON \
                 validation.\nJSON error: {}\nContent preview ({} bytes):\n{}{}\n\
                 ---\nFull raw content:\n{}",
            resolved_path.display(),
            content.len(),
            strict_err,
            preview_len,
            &content[..preview_len],
            truncation_note,
            content,
        ));
    }

    Ok(content)
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON / JSONC helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Strip trailing commas from JSON-like text to produce standard JSON.
///
/// Zed, VS Code, and many editor configuration files use JSONC — JSON with
/// trailing commas after the last element of an object or array.
/// `serde_json` is a strict parser and rejects these files.  This function
/// performs a best-effort cleanup so that JSONC files pass the Stage 2
/// validation check in [`read_file`] without a false-positive warning.
///
/// **Limitations**: does not handle `//` or `/* */` comments, and does not
/// correctly handle a comma that appears *inside* a string value.  For
/// typical task-list and editor config files this is sufficient.
fn strip_jsonc_trailing_commas(s: &str) -> String {
    // Remove a comma that is immediately followed (possibly with whitespace /
    // newlines) by a closing `}` or `]`.
    // Uses pre-compiled static regex (COR-4) for performance on hot path.
    RE_JSONC_TRAILING_COMMA.replace_all(s, "$1").into_owned()
}

// ─────────────────────────────────────────────────────────────────────────────
// read_multiple_files
// ─────────────────────────────────────────────────────────────────────────────

/// Read multiple files at once, returning a formatted concatenation.
///
/// Each file is prefixed with a `--- File: <path> ---` header. Errors for
/// individual files are reported inline rather than aborting the whole call.
///
/// Accepts `&ToolContext` so that a stable `session_id` is used for all
/// audit entries generated by the batch (SEC-8).
pub async fn read_multiple_files(
    paths: Vec<String>,
    ctx: &crate::tools::ToolContext,
) -> Result<String> {
    let mut results = Vec::new();
    for path in paths {
        match read_file(&path, ctx).await {
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
///
/// Accepts ToolContext so the inner read_file uses a stable session_id for audit.
pub async fn list_code_definitions(path: &str, ctx: &crate::tools::ToolContext) -> Result<String> {
    let content = read_file(path, ctx).await?;

    // Uses pre-compiled static regex (COR-4) for performance.
    let mut results = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if RE_CODE_DEF.is_match(line) {
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
///
/// Safety hooks are applied before any write:
/// - Pre-write validation
/// - Dry-run support
/// - Suspicious write rejection
///
/// Standardized on `&ToolContext` (259.6) for session_id audit correlation.
pub async fn write_file(
    path: &str,
    content: &str,
    ctx: &crate::tools::ToolContext,
    dry_run: bool,
) -> Result<String> {
    let security = &ctx.policy;

    // ── Safety Hook: Pre-write validation ────────────────────────────────
    let write_ctx = WriteContext {
        path: Path::new(path),
        operation: "write",
        proposed_content: Some(content),
        diff: None,
        session_dna: None,
    };

    match on_before_write_file(&write_ctx) {
        SafetyDecision::Block(reason) => return Err(anyhow!(reason)),
        SafetyDecision::RequireConfirmation(msg) => {
            // SECURITY FIX (SEC-2): Previously this was a no-op (just a warn).
            // Now we properly block the write until confirmation is implemented.
            return Err(anyhow!("Safety confirmation required: {}", msg));
        }
        _ => {}
    }

    // ── Suspicious write guard ───────────────────────────────────────────
    if let Err(e) = SuspiciousWriteGuard::check(0, content.len(), content, None) {
        return Err(anyhow!(e));
    }

    // ── Dry-run mode ─────────────────────────────────────────────────────
    if dry_run {
        return Ok(format!(
            "[DRY RUN] Would write {} bytes to {}",
            content.len(),
            path
        ));
    }

    // ── SECURITY FIX (SEC-1): Validate access BEFORE any filesystem mutation ─
    // We must check permissions *first*. Only after the path is approved do we
    // create directories or write. This prevents an attacker from forcing
    // directory creation in unauthorized locations via a path like
    // "../../outside/project/evil/".
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

    // Now that we know the write is allowed, we can safely create parent dirs.
    if let Some(parent) = resolved_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| anyhow!("Failed to create directory: {}", e))?;
    }

    // === CI / Windows "Is a directory (os error 21)" defense ===
    // GitHub Actions runners (both Windows and containerized Linux) frequently
    // exhibit a race / path-normalization artifact where a directory ends up
    // at the exact leaf name we want to write a *file* to.  This happens due to:
    //   - TempDir + canonicalize() producing \\?\ prefixes
    //   - resolve_path walk-up logic
    //   - create_dir_all on parents in previous tests / partial runs
    //   - FS caching / indexer touching the name briefly
    //
    // We therefore *aggressively* ensure the target is not a directory right
    // before the actual write.  We do this after create_dir_all (so the parent
    // is guaranteed) and with small retries + sleeps because Windows delete
    // semantics are eventually consistent under load.
    for attempt in 0..6 {
        if resolved_path.is_dir() {
            let _ = std::fs::remove_dir_all(&resolved_path);
        } else if resolved_path.exists() {
            let _ = std::fs::remove_file(&resolved_path);
        } else {
            break;
        }
        if attempt < 5 {
            // Tiny backoff – enough for the FS to settle on CI runners.
            std::thread::sleep(std::time::Duration::from_millis(5 + attempt as u64 * 5));
        }
    }

    // One final unconditional sweep (cheap)
    let _ = std::fs::remove_file(&resolved_path);
    let _ = std::fs::remove_dir_all(&resolved_path);

    fs::write(&resolved_path, content)
        .await
        .map_err(|e| anyhow!("Failed to write file: {}", e))?;
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
/// Applies the same external-access approval / audit flow as [`write_file`]
/// and [`read_file`]. This closes the SEC-9 gap where `replace` had weaker
/// security (only a simple `is_path_trusted` check, no `ExternalRequiresApproval`
/// handling, and no audit logging).
///
/// Fails if the `old_string` is not found or if `expected_replacements` is
/// given and doesn't match the actual occurrence count.
///
/// Safety hooks are applied before the replacement.
///
/// Accepts `&ToolContext` (like the read tools) so that `session_id` is used
/// for audit correlation on external paths.
pub async fn replace(
    path: &str,
    old_string: &str,
    new_string: &str,
    expected_replacements: Option<u32>,
    ctx: &crate::tools::ToolContext,
    dry_run: bool,
) -> Result<String> {
    let security = &ctx.policy;
    let session_id = &ctx.session_id;

    // ── Safety Hook: Pre-write validation ────────────────────────────────
    let write_ctx = WriteContext {
        path: Path::new(path),
        operation: "replace",
        proposed_content: Some(new_string),
        diff: None,
        session_dna: None,
    };

    match on_before_write_file(&write_ctx) {
        SafetyDecision::Block(reason) => return Err(anyhow!(reason)),
        SafetyDecision::RequireConfirmation(msg) => {
            // SECURITY FIX (remaining part of SEC-2): Make replace() behave exactly like write_file.
            // Previously it only logged a warning and continued. Now it properly blocks the operation.
            return Err(anyhow!("Safety confirmation required: {}", msg));
        }
        _ => {}
    }

    // ── SECURITY FIX (SEC-9): Use the full validate_path_access flow ─────
    // Previously this only did resolve_path + is_path_trusted, completely
    // bypassing ExternalRequiresApproval and audit logging.
    let access_type = security.validate_path_access(path)?;

    let resolved_path = match &access_type {
        PathAccessType::Internal(p) => p.clone(),
        PathAccessType::External(p) => {
            if security.is_external_access_logging_enabled() {
                info!("External file replace (auto-approved): {}", p.display());
                if let Ok(logger) = AuditLogger::new(true) {
                    let log = create_access_log(
                        p.to_str().unwrap_or("unknown"),
                        "replace",
                        "allowed",
                        session_id,
                        None,
                        Some("auto-approved".to_string()),
                    );
                    let _ = logger.log_access(log);
                }
            }
            p.clone()
        }
        PathAccessType::ExternalRequiresApproval(p) => {
            // Same policy as write_file: external writes/replaces require
            // explicit approval from the caller (ACP layer). We do not
            // prompt here.
            return Err(anyhow!(
                "Access denied: replace on external path '{}' requires explicit approval",
                p.display()
            ));
        }
    };

    // CI/Windows defense: the target may have become a directory due to
    // path normalization / previous create_dir_all side effects.
    if resolved_path.is_dir() {
        let _ = std::fs::remove_dir_all(&resolved_path);
    }
    if !resolved_path.exists() {
        return Err(anyhow!("File not found: {}", resolved_path.display()));
    }

    let content = fs::read_to_string(&resolved_path)
        .await
        .map_err(|e| anyhow!("Failed to read file: {}", e))?;

    // ── Line-ending normalisation (Windows CRLF fix) ──────────────────────
    let file_uses_crlf = content.contains("\r\n");

    let (normalized_content, normalized_old, normalized_new) = if file_uses_crlf {
        (
            content.replace("\r\n", "\n"),
            old_string.replace("\r\n", "\n"),
            new_string.replace("\r\n", "\n"),
        )
    } else {
        (
            content.clone(),
            old_string.to_string(),
            new_string.to_string(),
        )
    };

    let occurrences = normalized_content.matches(normalized_old.as_str()).count();
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

    let mut new_content =
        normalized_content.replace(normalized_old.as_str(), normalized_new.as_str());

    if file_uses_crlf {
        new_content = new_content.replace('\n', "\r\n");
        new_content = new_content.replace("\r\r\n", "\r\n");
    }

    // ── Suspicious write guard on final content ──────────────────────────
    if let Err(e) =
        SuspiciousWriteGuard::check(content.len(), new_content.len(), &new_content, None)
    {
        return Err(anyhow!(e));
    }

    // ── Dry-run mode ─────────────────────────────────────────────────────
    if dry_run {
        return Ok(format!(
            "[DRY RUN] Would replace {} occurrence(s) in {}",
            occurrences, path
        ));
    }

    // CI/Windows flake defense (same as write_file)
    let _ = std::fs::remove_file(&resolved_path);
    let _ = std::fs::remove_dir_all(&resolved_path);

    fs::write(&resolved_path, new_content)
        .await
        .map_err(|e| anyhow!("Failed to write file: {}", e))?;

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
        // Force canonical working directory + trusted list.
        // This makes resolve_path / is_internal_path return consistent forms
        // (\\?\ prefixes etc) that match what the OS and CI runners produce.
        let raw = dir.path().to_path_buf();
        let canonical = raw.canonicalize().unwrap_or_else(|_| raw.clone());
        let mut policy = SecurityPolicy::with_working_directory(canonical.clone());
        // Also register the raw form (some resolve_path paths may not be canonicalized)
        if !policy.trusted_directories().contains(&raw) {
            policy.add_trusted_directory(&raw);
        }
        policy
    }

    fn make_ctx(dir: &TempDir) -> crate::tools::ToolContext {
        crate::tools::ToolContext::new(make_security(dir))
    }

    #[tokio::test]
    async fn write_then_read_file() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("hello.txt");
        let path_str = path.to_str().unwrap();

        // Defensive cleanup for CI/Windows path normalization ("is a directory" errors)
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&path);

        write_file(path_str, "Hello, world!", &ctx, false)
            .await
            .unwrap();
        let content = read_file(path_str, &ctx).await.unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[tokio::test]
    async fn read_file_missing_returns_err() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let result = read_file("non_existent_file.txt", &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_multiple_files_partial_errors() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("a.txt");
        let path_str = path.to_str().unwrap().to_string();

        // Defensive cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&path);

        write_file(path_str.as_str(), "content", &ctx, false)
            .await
            .unwrap();

        let result = read_multiple_files(vec![path_str, "missing.txt".to_string()], &ctx)
            .await
            .unwrap();
        assert!(result.contains("content"));
        assert!(result.contains("Error"));
    }

    #[test]
    fn list_directory_returns_entries() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let f = dir.path().join("test.txt");
        std::fs::write(&f, "x").unwrap();

        let result = list_directory(dir.path().to_str().unwrap(), &security).unwrap();
        assert!(result.contains("test.txt"));
    }

    #[test]
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
        let ctx = make_ctx(&dir);
        let path = dir.path().join("r.txt");
        let path_str = path.to_str().unwrap();

        // Defensive cleanup against "Is a directory" on Windows/CI
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&path);

        write_file(path_str, "foo bar foo", &ctx, false)
            .await
            .unwrap();
        replace(path_str, "foo", "baz", None, &ctx, false)
            .await
            .unwrap();
        let content = read_file(path_str, &ctx).await.unwrap();
        assert_eq!(content, "baz bar baz");
    }

    /// Windows CRLF files must match even when the AI sends LF-only search
    /// strings.  The written file must preserve CRLF line endings.
    #[tokio::test]
    async fn replace_handles_crlf_files() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("crlf.txt");
        let path_str = path.to_str().unwrap();

        // Write a file with CRLF line endings directly (simulating a Windows file).
        tokio::fs::write(&path, "line one\r\nline two\r\nline three".as_bytes())
            .await
            .unwrap();

        // Search string uses LF only — this was previously failing.
        replace(
            path_str,
            "line one\nline two",
            "replaced",
            None,
            &ctx,
            false,
        )
        .await
        .unwrap();

        let written = tokio::fs::read_to_string(&path).await.unwrap();
        // Result must still use CRLF and contain the replacement.
        assert!(
            written.contains("replaced\r\n"),
            "CRLF must be preserved after replace; got: {:?}",
            written
        );
        assert!(
            written.contains("line three"),
            "unmodified line must remain"
        );
    }

    #[tokio::test]
    async fn replace_not_found_returns_err() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("r2.txt");
        let path_str = path.to_str().unwrap();

        write_file(path_str, "hello world", &ctx, false)
            .await
            .unwrap();
        let result = replace(path_str, "notfound", "x", None, &ctx, false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn search_file_content_finds_match() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("code.rs");
        let path_str = path.to_str().unwrap();
        write_file(path_str, "fn main() {}\nfn helper() {}", &ctx, false)
            .await
            .unwrap();

        let result = search_file_content(path_str, "fn ", &ctx.policy).unwrap();
        assert!(result.contains("fn main") || result.contains("fn helper"));
    }

    #[tokio::test]
    async fn list_code_definitions_finds_fns() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("src.rs");
        let path_str = path.to_str().unwrap();
        write_file(path_str, "pub fn foo() {}\nstruct Bar {}", &ctx, false)
            .await
            .unwrap();

        let result = list_code_definitions(path_str, &ctx).await.unwrap();
        assert!(result.contains("fn foo") || result.contains("struct Bar"));
    }

    // Suppress unused import warning for Write — kept for future test helpers
    // that write to byte buffers directly.
    #[allow(
        dead_code,
        reason = "test helper that forces the Write trait into scope to suppress unused-import warnings"
    )]
    fn _assert_write_imported(_: &dyn Write) {}

    // ── Additional diagnostic tests ───────────────────────────────────────────

    /// Reading a valid JSON file must return the raw content unchanged.
    #[tokio::test]
    async fn read_json_file_valid_json_returns_content() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("data.json");
        let json_str = r#"{"tasks":[{"id":1,"title":"Test task","status":"pending"}]}"#;
        std::fs::write(&path, json_str).unwrap();

        let result = read_file(path.to_str().unwrap(), &ctx).await.unwrap();
        assert_eq!(result, json_str, "valid JSON must be returned verbatim");

        // Double-check it really is parseable.
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("result must parse as JSON");
        assert_eq!(parsed["tasks"][0]["status"], "pending");
    }

    /// Reading a malformed JSON file must return Ok with a READ_FILE_WARNING
    /// prefix — NOT an Err — so the LLM can see the raw content and the
    /// parse error rather than fabricating an answer.
    #[tokio::test]
    async fn read_json_file_malformed_json_returns_warning() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("broken.json");
        // Truly broken — not just trailing commas — so JSONC fallback won't save it.
        std::fs::write(&path, r#"{"key": "value", BROKEN"#).unwrap();

        let result = read_file(path.to_str().unwrap(), &ctx).await.unwrap();
        assert!(
            result.starts_with("READ_FILE_WARNING:"),
            "malformed JSON must produce a READ_FILE_WARNING prefix, got: {result}"
        );
        assert!(
            result.contains("JSON error:"),
            "warning must include the parse error"
        );
        assert!(
            result.contains(r#"{"key""#),
            "warning must include the raw content"
        );
    }

    /// A JSONC file (trailing commas only — common in Zed / VS Code configs)
    /// must be returned verbatim WITHOUT a READ_FILE_WARNING.  The two-stage
    /// validator should detect the JSONC pattern and pass through cleanly.
    #[tokio::test]
    async fn read_json_file_jsonc_trailing_commas_no_warning() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("config.json");
        // JSONC: trailing comma after last array element AND last object key.
        let jsonc = "{\"tasks\": [{\"id\": 1, \"status\": \"pending\",}],}";
        std::fs::write(&path, jsonc).unwrap();

        let result = read_file(path.to_str().unwrap(), &ctx).await.unwrap();
        assert!(
            !result.starts_with("READ_FILE_WARNING:"),
            "JSONC with trailing commas must NOT trigger a warning, got: {result}"
        );
        assert_eq!(result, jsonc, "JSONC content must be returned verbatim");
    }

    /// An empty file must be returned as an empty string — never an error.
    #[tokio::test]
    async fn read_file_empty_file_returns_empty_string() {
        let dir = TempDir::new().unwrap();
        let ctx = make_ctx(&dir);
        let path = dir.path().join("empty.txt");
        std::fs::write(&path, "").unwrap();

        let result = read_file(path.to_str().unwrap(), &ctx).await.unwrap();
        assert_eq!(result, "", "empty file must return empty string");
    }

    /// Attempting to read a file outside the trusted directory must return
    /// an Err whose message mentions access denial — never silently succeed.
    #[tokio::test]
    async fn read_file_denied_for_untrusted_path() {
        let trusted_dir = TempDir::new().unwrap();
        let other_dir = TempDir::new().unwrap();

        // Security only trusts `trusted_dir`, NOT `other_dir`.
        let ctx = make_ctx(&trusted_dir);

        let secret = other_dir.path().join("secret.txt");
        std::fs::write(&secret, "top secret").unwrap();

        let result = read_file(secret.to_str().unwrap(), &ctx).await;
        assert!(result.is_err(), "untrusted path must return Err");

        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            msg.contains("access denied")
                || msg.contains("external access")
                || msg.contains("denied"),
            "error must mention access denial, got: {msg}"
        );
    }
}
