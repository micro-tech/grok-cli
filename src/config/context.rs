//! Context and file filtering configuration.
//!
//! Extracted from the monolithic `config/mod.rs` as part of ARCH-1.

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub import_format: String,
    #[serde(default)]
    pub discovery_max_dirs: u32,
    #[serde(default)]
    pub include_directories: Vec<String>,
    #[serde(default)]
    pub load_memory_from_include_directories: bool,
    #[serde(default)]
    pub file_filtering: FileFilteringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilteringConfig {
    #[serde(default = "default_true")]
    pub respect_git_ignore: bool,
    #[serde(default = "default_true")]
    pub respect_grok_ignore: bool,
    #[serde(default = "default_true")]
    pub enable_recursive_file_search: bool,
    #[serde(default)]
    pub disable_fuzzy_search: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            file_name: String::new(),
            import_format: String::new(),
            discovery_max_dirs: 200,
            include_directories: Vec::new(),
            load_memory_from_include_directories: false,
            file_filtering: FileFilteringConfig::default(),
        }
    }
}

impl Default for FileFilteringConfig {
    fn default() -> Self {
        Self {
            respect_git_ignore: true,
            respect_grok_ignore: true,
            enable_recursive_file_search: true,
            disable_fuzzy_search: false,
        }
    }
}
