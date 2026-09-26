//! HOH Semantic Code Understanding (Task 361.15)

use serde::{Deserialize, Serialize};

use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleProfile {
    pub path: String,
    pub public_items: Vec<String>,
    pub dependencies: Vec<String>,   // other crate:: modules used
    pub line_count: usize,
    pub complexity_estimate: f32,    // 0.0–1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseIndex {
    pub modules: Vec<ModuleProfile>,
    pub call_edges: Vec<(String, String)>,  // (from_module, to_module)
}

/// Build a lightweight semantic index of the codebase.
pub fn build_index(project_root: &Path) -> CodebaseIndex {
    let src = project_root.join("src");
    let mut modules = Vec::new();
    let mut call_edges = Vec::new();

    for entry in walkdir::WalkDir::new(&src).into_iter().flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
        let Ok(content) = std::fs::read_to_string(entry.path()) else { continue };

        let rel_path = entry.path()
            .strip_prefix(project_root).unwrap_or(entry.path())
            .to_string_lossy().to_string();

        let line_count = content.lines().count();
        let public_items: Vec<String> = content.lines()
            .filter(|l| l.starts_with("pub ") || l.starts_with("    pub "))
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.trim_end_matches('(').to_string()))
            .take(20).collect();

        let deps: Vec<String> = content.lines()
            .filter(|l| l.contains("use crate::"))
            .filter_map(|l| {
                let after = l.split("use crate::").nth(1)?;
                Some(after.split(|c: char| !c.is_alphanumeric() && c != '_' && c != ':').next()?.to_string())
            })
            .collect::<std::collections::HashSet<_>>().into_iter().collect();

        // Approximate complexity: nested braces + match arms
        let complexity_estimate = {
            let depth = content.chars().filter(|&c| c == '{').count();
            let matches = content.matches("match ").count();
            ((depth + matches * 2) as f32 / 200.0).min(1.0)
        };

        for dep in &deps {
            call_edges.push((rel_path.clone(), dep.clone()));
        }

        modules.push(ModuleProfile { path: rel_path, public_items, dependencies: deps, line_count, complexity_estimate });
    }

    CodebaseIndex { modules, call_edges }
}

/// Answer a simple question about the codebase from the index.
pub fn query(index: &CodebaseIndex, question: &str) -> String {
    let q = question.to_lowercase();

    if q.contains("largest") || q.contains("biggest") {
        let largest = index.modules.iter().max_by_key(|m| m.line_count);
        return largest.map(|m| format!("Largest file: {} ({} lines)", m.path, m.line_count))
            .unwrap_or_else(|| "No modules indexed".to_string());
    }
    if q.contains("most complex") {
        let most = index.modules.iter()
            .max_by(|a, b| a.complexity_estimate.partial_cmp(&b.complexity_estimate).unwrap_or(std::cmp::Ordering::Equal));
        return most.map(|m| format!("Most complex: {} (score={:.2})", m.path, m.complexity_estimate))
            .unwrap_or_else(|| "No modules indexed".to_string());
    }
    if q.contains("how many") && q.contains("module") {
        return format!("{} modules indexed", index.modules.len());
    }

    // Default: find modules whose path contains any query word
    let hits: Vec<&ModuleProfile> = index.modules.iter()
        .filter(|m| q.split_whitespace().any(|w| m.path.to_lowercase().contains(w)))
        .take(3).collect();

    if hits.is_empty() {
        format!("No module matching '{}' found in {} indexed modules", question, index.modules.len())
    } else {
        hits.iter().map(|m| format!("{}: {} lines, {} public items", m.path, m.line_count, m.public_items.len())).collect::<Vec<_>>().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_index_finds_rs_files() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let index = build_index(&root);
        assert!(!index.modules.is_empty());
    }

    #[test]
    fn test_query_module_count() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let index = build_index(&root);
        let answer = query(&index, "how many modules");
        assert!(answer.contains("modules indexed"));
    }
}
