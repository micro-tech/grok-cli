//! HOH Architecture Stability Monitor (Task 361.13)

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityReport {
    pub module_count: usize,
    pub avg_file_size_lines: f32,
    pub large_files: Vec<(String, usize)>,
    pub high_coupling_modules: Vec<String>,
    pub stability_score: f32,   // 0.0 (chaotic) to 1.0 (stable)
    pub warnings: Vec<String>,
}

/// Scan the src/ directory and produce a stability report.
pub fn analyze_stability(project_root: &Path) -> StabilityReport {
    let src = project_root.join("src");
    let mut files: Vec<(String, usize)> = Vec::new();

    if let Ok(walker) = walkdir::WalkDir::new(&src).into_iter().collect::<Result<Vec<_>,_>>() {
        for entry in walker {
            if entry.file_type().is_file() && entry.path().extension().and_then(|e| e.to_str()) == Some("rs") {
                let path = entry.path().to_string_lossy().to_string();
                let lines = std::fs::read_to_string(entry.path())
                    .map(|c| c.lines().count()).unwrap_or(0);
                files.push((path, lines));
            }
        }
    }

    let module_count = files.len();
    let avg_lines = if files.is_empty() { 0.0 }
        else { files.iter().map(|(_, l)| *l as f32).sum::<f32>() / files.len() as f32 };

    // Large file threshold: 500 lines
    let mut large_files: Vec<(String, usize)> = files.iter()
        .filter(|(_, l)| *l > 500)
        .map(|(p, l)| {
            let rel = p.trim_start_matches(project_root.to_str().unwrap_or("")).to_string();
            (rel, *l)
        })
        .collect();
    large_files.sort_by(|a, b| b.1.cmp(&a.1));
    large_files.truncate(10);

    // High coupling: modules with many `use crate::` imports (rough proxy)
    let high_coupling: Vec<String> = files.iter()
        .filter_map(|(path, _)| {
            std::fs::read_to_string(path).ok().and_then(|content| {
                let use_count = content.lines().filter(|l| l.contains("use crate::")).count();
                if use_count > 15 {
                    Some(path.trim_start_matches(project_root.to_str().unwrap_or("")).to_string())
                } else { None }
            })
        })
        .collect();

    let mut warnings = Vec::new();
    if !large_files.is_empty() { warnings.push(format!("{} files exceed 500 lines", large_files.len())); }
    if !high_coupling.is_empty() { warnings.push(format!("{} modules have high coupling", high_coupling.len())); }

    // Stability score: penalize large files and coupling
    let size_penalty = (large_files.len() as f32 * 0.05).min(0.5);
    let coupling_penalty = (high_coupling.len() as f32 * 0.03).min(0.3);
    let stability_score = (1.0 - size_penalty - coupling_penalty).max(0.0);

    StabilityReport {
        module_count, avg_file_size_lines: avg_lines,
        large_files, high_coupling_modules: high_coupling,
        stability_score, warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stability_report_on_project() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let report = analyze_stability(&root);
        assert!(report.module_count > 0);
        assert!(report.stability_score >= 0.0 && report.stability_score <= 1.0);
    }
}
