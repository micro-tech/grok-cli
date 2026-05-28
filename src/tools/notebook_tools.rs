//! Jupyter notebook editing tool.
//!
//! Reads, modifies, and writes `.ipynb` files (Jupyter Notebook JSON format
//! v4).  Creates a minimal notebook scaffold when the target file does not
//! yet exist.

use crate::acp::security::SecurityPolicy;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::fs;

/// Edit or append a cell in a Jupyter notebook.
///
/// * If `cell_index` is within the existing cells array the cell at that
///   index is **replaced**.
/// * If `cell_index >= cells.len()` a **new cell is appended** regardless
///   of the exact index value.
/// * If the notebook does not exist a minimal v4 scaffold is created first.
///
/// `cell_type` must be `"code"` or `"markdown"` (case-insensitive).
pub fn notebook_edit(
    path: &str,
    cell_index: usize,
    source: &str,
    cell_type: &str,
    security: &SecurityPolicy,
) -> Result<String> {
    let resolved = security.resolve_path(path).map_err(|e| {
        tracing::warn!(error = %e, "notebook_tools::notebook_edit: failed to resolve path");
        anyhow!("Failed to resolve path '{}': {}", path, e)
    })?;

    if !security.is_path_trusted(&resolved) {
        tracing::warn!(
            path = %resolved.display(),
            "notebook_tools::notebook_edit: access denied — path not in trusted directory"
        );
        return Err(anyhow!(
            "Access denied: '{}' is not in a trusted directory",
            resolved.display()
        ));
    }

    let cell_type_lower = cell_type.to_lowercase();
    if cell_type_lower != "code" && cell_type_lower != "markdown" {
        tracing::warn!(
            cell_type = cell_type,
            "notebook_tools::notebook_edit: invalid cell_type"
        );
        return Err(anyhow!(
            "Invalid cell_type '{}': must be 'code' or 'markdown'",
            cell_type
        ));
    }

    // Guard: cell source must not be blank.
    if source.trim().is_empty() {
        tracing::warn!("notebook_tools::notebook_edit: cell source is empty");
        return Err(anyhow::anyhow!(
            "notebook_edit: cell source must not be empty"
        ));
    }

    // Load existing notebook or create a minimal scaffold
    let notebook_content = if resolved.exists() {
        fs::read_to_string(&resolved).map_err(|e| {
            tracing::warn!(
                error = %e,
                path = %resolved.display(),
                "notebook_tools::notebook_edit: failed to read notebook"
            );
            anyhow!("Failed to read notebook '{}': {}", resolved.display(), e)
        })?
    } else {
        // Minimal Jupyter v4 scaffold
        serde_json::to_string_pretty(&json!({
            "nbformat": 4,
            "nbformat_minor": 5,
            "cells": [],
            "metadata": {
                "kernelspec": {
                    "display_name": "Python 3",
                    "language": "python",
                    "name": "python3"
                },
                "language_info": {
                    "name": "python",
                    "version": "3.0.0"
                }
            }
        }))?
    };

    let mut notebook: Value = serde_json::from_str(&notebook_content).map_err(|e| {
        tracing::warn!(
            error = %e,
            path = %resolved.display(),
            "notebook_tools::notebook_edit: invalid notebook JSON"
        );
        anyhow!("Invalid notebook JSON in '{}': {}", resolved.display(), e)
    })?;

    let cells = notebook["cells"].as_array_mut().ok_or_else(|| {
        tracing::warn!(
            path = %resolved.display(),
            "notebook_tools::notebook_edit: notebook has no 'cells' array"
        );
        anyhow!("Notebook '{}' has no 'cells' array", resolved.display())
    })?;

    // Convert source string to Jupyter source-lines format:
    // every line except the last ends with "\n".
    let source_lines: Vec<Value> = {
        let line_count = source.lines().count();
        source
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i + 1 < line_count {
                    json!(format!("{}\n", line))
                } else {
                    json!(line)
                }
            })
            .collect()
    };

    let new_cell: Value = if cell_type_lower == "markdown" {
        json!({
            "cell_type": "markdown",
            "source":    source_lines,
            "metadata":  {}
        })
    } else {
        json!({
            "cell_type":       "code",
            "source":          source_lines,
            "metadata":        {},
            "outputs":         [],
            "execution_count": null
        })
    };

    let action = if cell_index < cells.len() {
        cells[cell_index] = new_cell;
        format!("Updated cell {} in '{}'.", cell_index, resolved.display())
    } else {
        cells.push(new_cell);
        let new_idx = cells.len() - 1;
        format!(
            "Appended new {} cell at index {} in '{}'.",
            cell_type_lower,
            new_idx,
            resolved.display()
        )
    };

    // Ensure the parent directory exists.
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            tracing::warn!(
                error = %e,
                "notebook_tools::notebook_edit: failed to create parent directory"
            );
            anyhow!("Failed to create parent directory: {}", e)
        })?;
    }

    // Atomic write: serialise → tmp file → rename into place so that a
    // Starlink drop mid-write cannot leave a half-written notebook on disk.
    let json_str = serde_json::to_string_pretty(&notebook).map_err(|e| {
        tracing::warn!(
            error = %e,
            "notebook_tools::notebook_edit: failed to serialise notebook"
        );
        anyhow!("Failed to serialise notebook: {}", e)
    })?;

    let tmp_path = resolved.with_extension("ipynb.tmp");

    fs::write(&tmp_path, &json_str).map_err(|e| {
        tracing::warn!(
            error = %e,
            tmp = %tmp_path.display(),
            "notebook_tools::notebook_edit: failed to write tmp file"
        );
        anyhow::anyhow!("notebook_edit: failed to write tmp: {}", e)
    })?;

    fs::rename(&tmp_path, &resolved).map_err(|e| {
        tracing::warn!(
            error = %e,
            tmp = %tmp_path.display(),
            dest = %resolved.display(),
            "notebook_tools::notebook_edit: failed to rename tmp to notebook"
        );
        anyhow::anyhow!("notebook_edit: failed to rename tmp → notebook: {}", e)
    })?;

    Ok(action)
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
    fn creates_new_notebook_with_code_cell() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("test.ipynb");

        let result = notebook_edit(
            path.to_str().unwrap(),
            0,
            "print('hello')",
            "code",
            &security,
        );
        assert!(result.is_ok(), "{:?}", result);
        assert!(path.exists());

        let content: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["cells"].as_array().unwrap().len(), 1);
        assert_eq!(content["cells"][0]["cell_type"], "code");
    }

    #[test]
    fn appends_cell_when_index_out_of_range() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("nb.ipynb");

        notebook_edit(path.to_str().unwrap(), 0, "first", "code", &security).unwrap();
        notebook_edit(path.to_str().unwrap(), 99, "second", "markdown", &security).unwrap();

        let content: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["cells"].as_array().unwrap().len(), 2);
        assert_eq!(content["cells"][1]["cell_type"], "markdown");
    }

    #[test]
    fn replaces_existing_cell() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("nb2.ipynb");

        notebook_edit(path.to_str().unwrap(), 0, "old", "code", &security).unwrap();
        notebook_edit(path.to_str().unwrap(), 0, "new", "code", &security).unwrap();

        let content: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let src = &content["cells"][0]["source"];
        assert_eq!(src[0], "new");
    }

    #[test]
    fn rejects_invalid_cell_type() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("nb3.ipynb");
        let r = notebook_edit(path.to_str().unwrap(), 0, "x", "raw", &security);
        assert!(r.is_err());
    }

    #[test]
    fn rejects_empty_source() {
        let dir = TempDir::new().unwrap();
        let security = make_security(&dir);
        let path = dir.path().join("nb4.ipynb");
        let r = notebook_edit(path.to_str().unwrap(), 0, "   ", "code", &security);
        assert!(r.is_err(), "blank source must return Err");
        assert!(
            r.unwrap_err().to_string().contains("must not be empty"),
            "error message must mention 'must not be empty'"
        );
    }
}
