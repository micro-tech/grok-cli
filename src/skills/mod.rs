pub mod config;
pub mod manager;

// Re-export common types
pub use config::{Skill, SkillConfig};
pub use manager::{
    find_skill, get_default_skills_dir, get_skills_context, list_skills, load_skill,
};
