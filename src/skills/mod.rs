pub mod auto_activate;
pub mod catalog;
pub mod config;
pub mod manager;
pub mod registry;
pub mod rules;
pub mod security;

// Re-export common types
pub use auto_activate::{AutoActivationEngine, SkillMatch};
pub use catalog::{
    CATALOG_FILENAME, generate_context_catalog, load_catalog_content,
    write_catalog_to_default_location, write_catalog_to_path,
};
pub use config::{AutoActivateConfig, Skill, SkillConfig};
pub use manager::{
    find_skill, get_default_skills_dir, get_global_rules_dir, get_project_rules_dir,
    get_skills_context, list_skills, load_skill,
};
pub use registry::{
    SkillEntry, SkillManifest, SkillRegistry, default_manifest_template, load_manifest,
};
pub use rules::{
    RuleFile, RuleSource, format_rules_for_prompt, load_all_rules, load_and_format_rules,
};
pub use security::{SkillSecurityValidator, ValidationLevel, generate_security_report};
