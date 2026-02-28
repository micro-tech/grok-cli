pub mod auto_activate;
pub mod config;
pub mod manager;
pub mod security;

// Re-export common types
pub use auto_activate::{AutoActivationEngine, SkillMatch};
pub use config::{AutoActivateConfig, Skill, SkillConfig};
pub use manager::{
    find_skill, get_default_skills_dir, get_skills_context, list_skills, load_skill,
};
pub use security::{SkillSecurityValidator, ValidationLevel, generate_security_report};
