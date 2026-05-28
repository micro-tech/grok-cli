//! Language Server Protocol query tool.
//!
//! Provides practical code-intelligence queries without requiring a
//! persistent LSP server connection:
//!
//! | `query_type`  | Implementation                                        |
//! |---------------|-------------------------------------------------------|
//! | `diagnostics` | Runs `cargo check --message-format=json` (Rust)      |
//! | `hover`       | Returns a context window around the target line       |
//! | `definition`  | Regex-based symbol-definition search in the file      |
//! | `references`  | Substring search for the symbol across the file       |
//!
//! For non-Rust projects the `diagnostics` query reports file existence only.
//! Full LSP server integration (e.g. rust-analyzer stdio protocol) is a
//! planned future extension.

use crate::acp::security::SecurityPolicy;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

// ── public entry point ────────────────────────────────────────────────────────

/// Query code intelligence for a file position.
///
/// * `file`       — path to the source file (resolved against SecurityPolicy).
/// * `line`       — 0-based line number.
/// * `character`  — 0-based character offset within the line.
/// * `query_type` — one of `"diagnostics"`, `"hover"`, `"definition"`,
///   `"references"`.
pub async fn lsp_query(
    file: &str,
    line: u32,
    character: u32,
    query_type: &str,
    security: &SecurityPolicy,
) -> Result<String> {
    let resolved = security.resolve_path(file).map_err(|e| {
        let err = anyhow!("Failed to resolve '{}': {}", file, e);
        tracing::warn!(
            tool  = "lsp_query",
            error = %err,
            "lsp_tools: failed to resolve path"
        );
        err
    })?;

    if !security.is_path_trusted(&resolved) {
        let e = anyhow!(
            "Access denied: '{}' is not in a trusted directory",
            resolved.display()
        );
        tracing::warn!(
            tool  = "lsp_query",
            path  = %resolved.display(),
            error = %e,
            "lsp_tools: access denied — path is not in a trusted directory"
        );
        return Err(e);
    }

    match query_type {
        "diagnostics" => get_diagnostics(&resolved, security).await,
        "hover" => get_hover(&resolved, line, character),
        "definition" => find_definition(&resolved, line, character),
        "references" => find_references(&resolved, line, character),
        other => {
            let e = anyhow!(
                "Unknown query_type '{}'. Valid options: diagnostics, hover, definition, references",
                other
            );
            tracing::warn!(
                tool       = "lsp_query",
                query_type = other,
                error      = %e,
                "lsp_tools: unknown query type requested"
            );
            Err(e)
        }
    }
}

// ── diagnostics ───────────────────────────────────────────────────────────────

async fn get_diagnostics(path: &Path, security: &SecurityPolicy) -> Result<String> {
    let project_root = security.working_directory();

    // For Rust projects use `cargo check`; otherwise report file existence.
    if project_root.join("Cargo.toml").exists() {
        let output = timeout(
            Duration::from_secs(60),
            Command::new("cargo")
                .args(["check", "--message-format=json", "--quiet"])
                .current_dir(project_root)
                .output(),
        )
        .await
        .map_err(|_| {
            tracing::warn!(
                timeout_secs = 60u64,
                path         = %path.display(),
                "lsp_tools: cargo check timed out — diagnostics may be incomplete"
            );
            anyhow!("cargo check timed out after 60 s")
        })?
        .map_err(|e| {
            tracing::warn!(
                tool  = "lsp_query",
                error = %e,
                "lsp_tools: failed to spawn cargo check"
            );
            anyhow!("Failed to run cargo check: {}", e)
        })?;

        // Warn when cargo check exits non-zero (compilation errors are expected,
        // but an unexpected exit code may indicate a broken toolchain or project).
        if !output.status.success() {
            tracing::warn!(
                tool      = "lsp_query",
                exit_code = ?output.status.code(),
                "lsp_tools: cargo check exited non-zero — some diagnostics may be incomplete"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let file_stem = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        let mut diagnostics: Vec<String> = Vec::new();
        for json_line in stdout.lines() {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(json_line) {
                if msg["reason"] != "compiler-message" {
                    continue;
                }
                let compiler_msg = &msg["message"];
                let level = compiler_msg["level"].as_str().unwrap_or("note");
                let text = compiler_msg["message"].as_str().unwrap_or("");

                if let Some(spans) = compiler_msg["spans"].as_array() {
                    for span in spans {
                        let span_file = span["file_name"].as_str().unwrap_or("");
                        if span_file.contains(file_stem) {
                            let span_line = span["line_start"].as_u64().unwrap_or(0);
                            diagnostics.push(format!("[{}] line {}: {}", level, span_line, text));
                        }
                    }
                }
            }
        }

        if diagnostics.is_empty() {
            Ok(format!("No diagnostics for '{}'.", path.display()))
        } else {
            Ok(format!(
                "Diagnostics for '{}':\n{}",
                path.display(),
                diagnostics.join("\n")
            ))
        }
    } else {
        // Non-Rust project: just confirm the file exists.
        if path.exists() {
            Ok(format!(
                "File '{}' exists. Full LSP diagnostics require a language server \
                 for this file type.",
                path.display()
            ))
        } else {
            let e = anyhow!("File not found: '{}'", path.display());
            tracing::warn!(
                tool  = "lsp_query",
                path  = %path.display(),
                error = %e,
                "lsp_tools: file not found during non-Rust diagnostics"
            );
            Err(e)
        }
    }
}

// ── hover ─────────────────────────────────────────────────────────────────────

fn get_hover(path: &Path, line: u32, _character: u32) -> Result<String> {
    if !path.exists() {
        let e = anyhow!("File not found: '{}'", path.display());
        tracing::warn!(
            tool  = "lsp_query",
            path  = %path.display(),
            error = %e,
            "lsp_tools: file not found for hover query"
        );
        return Err(e);
    }

    let content = fs::read_to_string(path).map_err(|e| {
        let err = anyhow!("Failed to read '{}': {}", path.display(), e);
        tracing::warn!(
            tool  = "lsp_query",
            path  = %path.display(),
            error = %err,
            "lsp_tools: failed to read file for hover query"
        );
        err
    })?;
    let lines: Vec<&str> = content.lines().collect();

    let line_idx = line as usize;
    if line_idx >= lines.len() {
        let e = anyhow!(
            "Line {} is out of range (file has {} lines).",
            line + 1,
            lines.len()
        );
        tracing::warn!(
            tool      = "lsp_query",
            path      = %path.display(),
            requested = line + 1,
            total     = lines.len(),
            error     = %e,
            "lsp_tools: requested line is out of range"
        );
        return Err(e);
    }

    // Return a ±3 line context window with an arrow marking the target line.
    let start = line_idx.saturating_sub(3);
    let end = (line_idx + 4).min(lines.len());

    let mut result = format!("Context around {}:{}\n```\n", path.display(), line + 1);
    for (i, l) in lines[start..end].iter().enumerate() {
        let actual = start + i + 1; // 1-based for display
        let marker = if actual == line_idx + 1 { "→" } else { " " };
        result.push_str(&format!("{} {:5} | {}\n", marker, actual, l));
    }
    result.push_str("```");

    Ok(result)
}

// ── definition ────────────────────────────────────────────────────────────────

fn find_definition(path: &Path, line: u32, character: u32) -> Result<String> {
    if !path.exists() {
        let e = anyhow!("File not found: '{}'", path.display());
        tracing::warn!(
            tool  = "lsp_query",
            path  = %path.display(),
            error = %e,
            "lsp_tools: file not found for definition query"
        );
        return Err(e);
    }

    let content = fs::read_to_string(path).map_err(|e| {
        let err = anyhow!("Failed to read '{}': {}", path.display(), e);
        tracing::warn!(
            tool  = "lsp_query",
            path  = %path.display(),
            error = %err,
            "lsp_tools: failed to read file for definition query"
        );
        err
    })?;
    let lines: Vec<&str> = content.lines().collect();

    let symbol = extract_symbol(&lines, line, character)?;
    if symbol.is_empty() {
        return Ok("No symbol found at the given position.".to_string());
    }

    let patterns: Vec<String> = [
        format!("fn {}", symbol),
        format!("struct {}", symbol),
        format!("enum {}", symbol),
        format!("trait {}", symbol),
        format!("impl {}", symbol),
        format!("type {} =", symbol),
        format!("const {}: ", symbol),
        format!("let {} =", symbol),
        format!("let mut {} =", symbol),
        format!("pub fn {}", symbol),
        format!("pub struct {}", symbol),
        format!("pub enum {}", symbol),
        format!("pub trait {}", symbol),
    ]
    .into_iter()
    .collect();

    let defs: Vec<String> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| patterns.iter().any(|p| l.contains(p.as_str())))
        .map(|(i, l)| format!("line {}: {}", i + 1, l.trim()))
        .collect();

    if defs.is_empty() {
        Ok(format!(
            "No definition found for '{}' in '{}'.",
            symbol,
            path.display()
        ))
    } else {
        Ok(format!(
            "Definitions of '{}' in '{}':\n{}",
            symbol,
            path.display(),
            defs.join("\n")
        ))
    }
}

// ── references ────────────────────────────────────────────────────────────────

fn find_references(path: &Path, line: u32, character: u32) -> Result<String> {
    if !path.exists() {
        let e = anyhow!("File not found: '{}'", path.display());
        tracing::warn!(
            tool  = "lsp_query",
            path  = %path.display(),
            error = %e,
            "lsp_tools: file not found for references query"
        );
        return Err(e);
    }

    let content = fs::read_to_string(path).map_err(|e| {
        let err = anyhow!("Failed to read '{}': {}", path.display(), e);
        tracing::warn!(
            tool  = "lsp_query",
            path  = %path.display(),
            error = %err,
            "lsp_tools: failed to read file for references query"
        );
        err
    })?;
    let lines: Vec<&str> = content.lines().collect();

    let symbol = extract_symbol(&lines, line, character)?;
    if symbol.is_empty() {
        return Ok("No symbol found at the given position.".to_string());
    }

    let refs: Vec<String> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(symbol.as_str()))
        .map(|(i, l)| format!("line {}: {}", i + 1, l.trim()))
        .collect();

    if refs.is_empty() {
        Ok(format!(
            "No references to '{}' found in '{}'.",
            symbol,
            path.display()
        ))
    } else {
        Ok(format!(
            "References to '{}' in '{}' ({} occurrence(s)):\n{}",
            symbol,
            path.display(),
            refs.len(),
            refs.join("\n")
        ))
    }
}

// ── symbol extraction ─────────────────────────────────────────────────────────

fn extract_symbol(lines: &[&str], line: u32, character: u32) -> Result<String> {
    let line_idx = line as usize;
    if line_idx >= lines.len() {
        let e = anyhow!(
            "Line {} is out of range ({} total lines).",
            line + 1,
            lines.len()
        );
        tracing::warn!(
            tool      = "lsp_query",
            requested = line + 1,
            total     = lines.len(),
            error     = %e,
            "lsp_tools: symbol extraction — line out of range"
        );
        return Err(e);
    }
    let target_line = lines[line_idx];
    let char_idx = (character as usize).min(target_line.len());

    let is_ident = |c: char| c.is_alphanumeric() || c == '_';

    let before: String = target_line[..char_idx]
        .chars()
        .rev()
        .take_while(|&c| is_ident(c))
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    let after: String = target_line[char_idx..]
        .chars()
        .take_while(|&c| is_ident(c))
        .collect();

    Ok(format!("{}{}", before, after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::security::SecurityPolicy;
    use tempfile::TempDir;

    fn make_security(dir: &TempDir) -> SecurityPolicy {
        SecurityPolicy::with_working_directory(dir.path().to_path_buf())
    }

    #[test]
    fn hover_returns_context_window() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("test.rs");
        std::fs::write(&path, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(lsp_query(path.to_str().unwrap(), 0, 3, "hover", &security));
        assert!(r.is_ok(), "{:?}", r);
        assert!(r.unwrap().contains("fn main"));
    }

    #[test]
    fn definition_finds_fn() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("src.rs");
        std::fs::write(&path, "fn hello() {}\nfn main() { hello(); }\n").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(lsp_query(
            path.to_str().unwrap(),
            1,  // line 1 (0-based) = "fn main() { hello(); }"
            15, // character inside "hello"
            "definition",
            &security,
        ));
        assert!(r.is_ok(), "{:?}", r);
        let out = r.unwrap();
        assert!(out.contains("fn hello") || out.contains("fn main"), "{out}");
    }

    #[test]
    fn unknown_query_type_returns_error() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("f.rs");
        std::fs::write(&path, "fn f() {}").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(lsp_query(
            path.to_str().unwrap(),
            0,
            0,
            "unknown_type",
            &security,
        ));
        assert!(r.is_err());
    }

    #[test]
    fn missing_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(lsp_query("nonexistent.rs", 0, 0, "hover", &security));
        assert!(r.is_err());
    }
}
