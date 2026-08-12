//! Self-updating Skills / Hooks / Optimization Catalog
//!
//! Generates a single, readable Markdown file (`.grok/SKILLS_HOOKS_OPTIMIZATION.md`)
//! that is injected into the model's context.
//!
//! The file uses clear section markers so both the model and humans can easily
//! reference what each part is for:
//!
//! - SKILLS CATALOG
//! - HOOKS
//! - OPTIMIZATION HEURISTICS

use crate::skills::registry::SkillRegistry;
use crate::skills::{get_default_skills_dir, list_skills};
use anyhow::Result;
use std::fs;
use std::path::Path;

/// The canonical filename for the generated catalog.
pub const CATALOG_FILENAME: &str = "SKILLS_HOOKS_OPTIMIZATION.md";

/// Generate the complete catalog as a string.
/// This is the source of truth for what the model sees.
pub fn generate_context_catalog() -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str("# Grok-CLI Skills, Hooks & Optimization Catalog\n\n");
    output.push_str("**Auto-generated and self-updating.**\n");
    output.push_str("This file is regenerated when skills are created or via `grok skills generate-catalog`.\n");
    output.push_str("The model should treat each clearly marked section as authoritative.\n\n");
    output.push_str("---\n\n");

    // ============================================
    // SECTION 1: SKILLS CATALOG
    // ============================================
    output.push_str("<!-- SKILLS CATALOG START -->\n");
    output.push_str("## 1. SKILLS CATALOG\n\n");
    output.push_str("**Purpose:** Live list of skills with everything the model needs for correct arbitration, activation, and `execute_skill` decisions.\n\n");
    output.push_str("Skills appear in descending arbitration priority (highest influence first).\n\n");

    let skills_dir = get_default_skills_dir();

    if let Some(dir) = &skills_dir {
        if let Ok(registry) = SkillRegistry::load(dir) {
            if registry.is_empty() {
                output.push_str("_No skills currently installed._\n\n");
            } else {
                for entry in registry.entries() {
                    output.push_str(&format!("### {}\n", entry.name()));
                    output.push_str(&format!("**Description:** {}\n", entry.description()));
                    output.push_str(&format!("**Arbitration Score:** {} (higher = appears earlier / stronger influence)\n", entry.arbitration_score()));

                    if let Some(manifest) = &entry.manifest {
                        if !manifest.tags.is_empty() {
                            output.push_str(&format!("**Tags:** {}\n", manifest.tags.join(", ")));
                        }
                        if !manifest.dependencies.is_empty() {
                            output.push_str(&format!("**Depends on:** {}\n", manifest.dependencies.join(", ")));
                        }
                    }

                    // Auto-activation hints (critical for the model)
                    if let Some(ref aa) = entry.skill.config.auto_activate {
                        if aa.enabled {
                            output.push_str("**Auto-activation hints:**\n");
                            if !aa.keywords.is_empty() {
                                output.push_str(&format!("- Keywords: {}\n", aa.keywords.join(", ")));
                            }
                            if !aa.patterns.is_empty() {
                                output.push_str(&format!("- Regex patterns: {}\n", aa.patterns.join(", ")));
                            }
                            if !aa.file_extensions.is_empty() {
                                output.push_str(&format!("- File extensions: {}\n", aa.file_extensions.join(", ")));
                            }
                            output.push_str(&format!("- Minimum confidence: {}\n", aa.min_confidence));
                        }
                    }

                    output.push_str("**When to prefer / activate this skill:**\n");
                    output.push_str(&format!("- Match the description and triggers above.\n"));
                    output.push_str(&format!("- Call with `execute_skill \"{}\" \"user input here\"` or activate via `/activate {}`.\n\n", entry.name(), entry.name()));
                }
            }
        } else {
            output.push_str("_Could not load skill registry._\n\n");
        }
    } else {
        output.push_str("_No skills directory configured._\n\n");
    }

    output.push_str("<!-- SKILLS CATALOG END -->\n\n");
    output.push_str("---\n\n");

    // ============================================
    // SECTION 2: HOOKS
    // ============================================
    output.push_str("<!-- HOOKS SECTION START -->\n");
    output.push_str("## 2. HOOKS\n\n");
    output.push_str("**Purpose:** Automatic logic that runs around tool calls. The model should **rely on hooks** instead of re-implementing the same behavior.\n\n");

    output.push_str("### Hook Types\n\n");
    output.push_str("**before_tool**\n");
    output.push_str("- Executes before any tool call.\n");
    output.push_str("- Can block the call or add context.\n");
    output.push_str("- Common uses: security policy, logging, auto-approval, input sanitization.\n\n");

    output.push_str("**after_tool**\n");
    output.push_str("- Executes after tool completion.\n");
    output.push_str("- Receives tool name + result.\n");
    output.push_str("- Common uses: post-processing, auditing, triggering side effects.\n\n");

    output.push_str("### Guidance for the Model\n");
    output.push_str("- Trust that `before_tool` / `after_tool` will run when `tools.enable_hooks = true`.\n");
    output.push_str("- Do not duplicate hook logic (e.g. don't manually add safety checks a hook already performs).\n");
    output.push_str("- If a tool is blocked by a hook, the response will usually explain why.\n");
    output.push_str("- Prefer relying on existing hooks over writing new custom code in skills.\n\n");

    output.push_str("<!-- HOOKS SECTION END -->\n\n");
    output.push_str("---\n\n");

    // ============================================
    // SECTION 3: OPTIMIZATION HEURISTICS
    // ============================================
    output.push_str("<!-- OPTIMIZATION HEURISTICS START -->\n");
    output.push_str("## 3. OPTIMIZATION HEURISTICS\n\n");
    output.push_str("**Purpose:** Concrete patterns the model should prefer to reduce latency, allocations, token usage, and tool-loop iterations.\n\n");

    output.push_str("### Skill & Tool Usage\n");
    output.push_str("- Prefer `execute_skill \"name\"` over re-implementing the same logic with many primitive tool calls.\n");
    output.push_str("- Activate relevant skills early (see Skills Catalog) instead of doing general reasoning.\n");
    output.push_str("- Use ranked skill context (higher arbitration scores first) rather than dumping everything.\n\n");

    output.push_str("### History & Message Construction (cheap clones)\n");
    output.push_str("- Use `Arc<str>` for message content (already done in ConversationItem).\n");
    output.push_str("- Avoid full history clones on every turn — prefer bounded windows + `std::mem::take`.\n");
    output.push_str("- Keep static prompt fragments and tool schemas cached.\n\n");

    output.push_str("### Prompt Building\n");
    output.push_str("- Use `String::with_capacity` + `push_str` instead of repeated `format!` where hot.\n");
    output.push_str("- Prefer single owned values for system messages when possible.\n\n");

    output.push_str("### General Performance Rules\n");
    output.push_str("- When `GROK_PERF=1` is set, pay attention to the reported per-turn timings.\n");
    output.push_str("- In ACP/sub-agent paths, minimize time spent holding locks.\n");
    output.push_str("- For interactive suggestions and UI, use `LazyLock` / statics (already applied to many suggestion lists).\n\n");

    output.push_str("**Primary Goal:** Fewer tool-loop iterations + lower per-turn memory allocations.\n\n");

    output.push_str("<!-- OPTIMIZATION HEURISTICS END -->\n\n");

    output.push_str("---\n");
    output.push_str("_Generated by the self-updating catalog system. Regenerate after creating or modifying skills._\n");

    Ok(output)
}

/// Write the catalog to the user's default `.grok/` location.
pub fn write_catalog_to_default_location() -> Result<std::path::PathBuf> {
    let content = generate_context_catalog()?;

    let dir = get_default_skills_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine skills directory"))?;

    let catalog_path = dir.parent()
        .unwrap_or(&dir)
        .join(CATALOG_FILENAME);

    fs::write(&catalog_path, &content)?;
    Ok(catalog_path)
}

/// Write the catalog to an explicit path (tests, project-local, etc.).
pub fn write_catalog_to_path(path: &Path) -> Result<()> {
    let content = generate_context_catalog()?;
    fs::write(path, content)?;
    Ok(())
}

/// Load the catalog content if the file exists.
/// Returns `None` if the file has not been generated yet.
pub fn load_catalog_content() -> Option<String> {
    let dir = get_default_skills_dir()?;
    let catalog_path = dir.parent().unwrap_or(&dir).join(CATALOG_FILENAME);

    if catalog_path.exists() {
        fs::read_to_string(&catalog_path).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_generation_produces_all_three_sections() {
        let catalog = generate_context_catalog().unwrap();

        assert!(catalog.contains("<!-- SKILLS CATALOG START -->"));
        assert!(catalog.contains("<!-- SKILLS CATALOG END -->"));
        assert!(catalog.contains("## 1. SKILLS CATALOG"));

        assert!(catalog.contains("<!-- HOOKS SECTION START -->"));
        assert!(catalog.contains("## 2. HOOKS"));

        assert!(catalog.contains("<!-- OPTIMIZATION HEURISTICS START -->"));
        assert!(catalog.contains("## 3. OPTIMIZATION HEURISTICS"));
    }

    #[test]
    fn catalog_contains_readable_purpose_markers() {
        let catalog = generate_context_catalog().unwrap();
        assert!(catalog.contains("**Purpose:** Live list of skills"));
        assert!(catalog.contains("**Purpose:** Automatic logic that runs around tool calls"));
        assert!(catalog.contains("**Purpose:** Concrete patterns the model should prefer"));
    }
}
