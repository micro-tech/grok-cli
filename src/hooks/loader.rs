//! Extension loading and management
//!
//! This module provides functionality to discover, load, and initialize
//! extensions from configuration or extension directories.

use super::{Extension, ExtensionManager, Hook, HookManager, ToolContext};
use crate::config::ExtensionsConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Extension metadata from manifest file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Extension name
    pub name: String,

    /// Extension version
    pub version: String,

    /// Extension description
    pub description: Option<String>,

    /// Author information
    pub author: Option<String>,

    /// Extension type (hook, tool, etc.)
    pub extension_type: ExtensionType,

    /// Hooks configuration
    #[serde(default)]
    pub hooks: Vec<HookConfig>,

    /// Extension dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Enabled by default
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Type of extension
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    /// Hook-based extension
    Hook,
    /// Tool provider extension
    Tool,
    /// Combined hook and tool extension
    Combined,
}

/// Hook configuration from manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Hook name/identifier
    pub name: String,

    /// Hook type
    pub hook_type: HookType,

    /// Optional script or command to execute
    pub script: Option<String>,

    /// Optional configuration
    pub config: Option<serde_json::Value>,
}

/// Type of hook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Executes before tool invocation
    BeforeTool,
    /// Executes after tool invocation
    AfterTool,
    /// Both before and after
    Both,
}

/// Loaded extension with its manifest
pub struct LoadedExtension {
    pub manifest: ExtensionManifest,
    pub path: PathBuf,
}

/// Extension loader for discovering and loading extensions
pub struct ExtensionLoader {
    config: ExtensionsConfig,
    loaded_extensions: Vec<LoadedExtension>,
}

impl ExtensionLoader {
    /// Create a new extension loader with the given configuration
    pub fn new(config: ExtensionsConfig) -> Self {
        Self {
            config,
            loaded_extensions: Vec::new(),
        }
    }

    /// Discover all available extensions from the extension directory
    pub fn discover_extensions(&mut self) -> Result<Vec<ExtensionManifest>> {
        if !self.config.enabled {
            debug!("Extension system is disabled");
            return Ok(Vec::new());
        }

        let extension_dir = match &self.config.extension_dir {
            Some(dir) => dir.clone(),
            None => self.get_default_extension_dir()?,
        };

        if !extension_dir.exists() {
            info!(
                "Extension directory does not exist: {}",
                extension_dir.display()
            );
            return Ok(Vec::new());
        }

        let mut manifests = Vec::new();

        for entry in fs::read_dir(&extension_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                match self.load_extension_manifest(&path) {
                    Ok(manifest) => {
                        info!("Discovered extension: {}", manifest.name);
                        self.loaded_extensions.push(LoadedExtension {
                            manifest: manifest.clone(),
                            path: path.clone(),
                        });
                        manifests.push(manifest);
                    }
                    Err(e) => {
                        warn!("Failed to load extension from {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Load extension manifest from a directory
    fn load_extension_manifest(&self, extension_path: &Path) -> Result<ExtensionManifest> {
        let manifest_path = extension_path.join("extension.json");

        if !manifest_path.exists() {
            return Err(anyhow!(
                "Extension manifest not found at {}",
                manifest_path.display()
            ));
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest: ExtensionManifest = serde_json::from_str(&content)?;

        Ok(manifest)
    }

    /// Get the default extension directory
    fn get_default_extension_dir(&self) -> Result<PathBuf> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home_dir.join(".grok").join("extensions"))
    }

    /// Load and register all enabled extensions
    pub fn load_extensions(&mut self, extension_manager: &mut ExtensionManager) -> Result<()> {
        if !self.config.enabled {
            debug!("Extension system is disabled, skipping extension loading");
            return Ok(());
        }

        // Discover extensions if not already done
        if self.loaded_extensions.is_empty() {
            self.discover_extensions()?;
        }

        // Filter to only enabled extensions
        let enabled_extensions: Vec<_> = self
            .loaded_extensions
            .iter()
            .filter(|ext| {
                ext.manifest.enabled
                    && (self.config.enabled_extensions.is_empty()
                        || self.config.enabled_extensions.contains(&ext.manifest.name))
            })
            .collect();

        for loaded_ext in enabled_extensions {
            match self.instantiate_extension(loaded_ext) {
                Ok(extension) => {
                    info!("Loading extension: {}", loaded_ext.manifest.name);
                    extension_manager.register(extension);
                }
                Err(e) => {
                    warn!(
                        "Failed to instantiate extension {}: {}",
                        loaded_ext.manifest.name, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Instantiate an extension from its loaded metadata
    fn instantiate_extension(&self, loaded_ext: &LoadedExtension) -> Result<Box<dyn Extension>> {
        // For now, we create a simple hook-based extension
        // In a full implementation, this could load dynamic libraries or scripts
        Ok(Box::new(ConfigBasedExtension {
            manifest: loaded_ext.manifest.clone(),
        }))
    }

    /// Get list of loaded extension manifests
    pub fn get_loaded_extensions(&self) -> Vec<&ExtensionManifest> {
        self.loaded_extensions.iter().map(|e| &e.manifest).collect()
    }
}

/// A simple extension implementation based on configuration
struct ConfigBasedExtension {
    manifest: ExtensionManifest,
}

impl Extension for ConfigBasedExtension {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn register_hooks(&self, hook_manager: &mut HookManager) -> Result<()> {
        for hook_config in &self.manifest.hooks {
            let hook = create_hook_from_config(&self.manifest.name, hook_config)?;
            hook_manager.register(hook);
        }
        Ok(())
    }
}

/// Create a hook instance from configuration
fn create_hook_from_config(
    extension_name: &str,
    hook_config: &HookConfig,
) -> Result<Box<dyn Hook>> {
    Ok(Box::new(ConfigBasedHook {
        extension_name: extension_name.to_string(),
        config: hook_config.clone(),
    }))
}

/// A hook implementation based on configuration
struct ConfigBasedHook {
    extension_name: String,
    config: HookConfig,
}

impl Hook for ConfigBasedHook {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn before_tool(&self, context: &ToolContext) -> Result<bool> {
        if self.config.hook_type == HookType::BeforeTool || self.config.hook_type == HookType::Both
        {
            debug!(
                "Extension '{}' hook '{}' executing before tool '{}'",
                self.extension_name, self.config.name, context.tool_name
            );

            // In a full implementation, this could execute a script or custom logic
            // For now, we just log and continue
        }
        Ok(true)
    }

    fn after_tool(&self, context: &ToolContext, result: &str) -> Result<()> {
        if self.config.hook_type == HookType::AfterTool || self.config.hook_type == HookType::Both {
            debug!(
                "Extension '{}' hook '{}' executing after tool '{}' (result length: {})",
                self.extension_name,
                self.config.name,
                context.tool_name,
                result.len()
            );

            // In a full implementation, this could execute a script or custom logic
            // For now, we just log
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_load_extension_manifest() {
        let temp_dir = tempdir().unwrap();
        let ext_dir = temp_dir.path().join("test-extension");
        fs::create_dir(&ext_dir).unwrap();

        let manifest = ExtensionManifest {
            name: "test-extension".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test extension".to_string()),
            author: Some("Test Author".to_string()),
            extension_type: ExtensionType::Hook,
            hooks: vec![HookConfig {
                name: "test-hook".to_string(),
                hook_type: HookType::BeforeTool,
                script: None,
                config: None,
            }],
            dependencies: vec![],
            enabled: true,
        };

        let manifest_path = ext_dir.join("extension.json");
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        fs::write(&manifest_path, json).unwrap();

        let config = ExtensionsConfig {
            enabled: true,
            extension_dir: Some(temp_dir.path().to_path_buf()),
            enabled_extensions: vec![],
            allow_config_extensions: true,
        };

        let loader = ExtensionLoader::new(config);
        let loaded_manifest = loader.load_extension_manifest(&ext_dir).unwrap();

        assert_eq!(loaded_manifest.name, "test-extension");
        assert_eq!(loaded_manifest.version, "1.0.0");
        assert_eq!(loaded_manifest.hooks.len(), 1);
    }

    #[test]
    fn test_discover_extensions() {
        let temp_dir = tempdir().unwrap();

        // Create two test extensions
        for i in 1..=2 {
            let ext_dir = temp_dir.path().join(format!("extension-{}", i));
            fs::create_dir(&ext_dir).unwrap();

            let manifest = ExtensionManifest {
                name: format!("extension-{}", i),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                extension_type: ExtensionType::Hook,
                hooks: vec![],
                dependencies: vec![],
                enabled: true,
            };

            let manifest_path = ext_dir.join("extension.json");
            let json = serde_json::to_string_pretty(&manifest).unwrap();
            fs::write(&manifest_path, json).unwrap();
        }

        let config = ExtensionsConfig {
            enabled: true,
            extension_dir: Some(temp_dir.path().to_path_buf()),
            enabled_extensions: vec![],
            allow_config_extensions: true,
        };

        let mut loader = ExtensionLoader::new(config);
        let manifests = loader.discover_extensions().unwrap();

        assert_eq!(manifests.len(), 2);
    }

    #[test]
    fn test_extension_disabled() {
        let config = ExtensionsConfig {
            enabled: false,
            extension_dir: None,
            enabled_extensions: vec![],
            allow_config_extensions: false,
        };

        let mut loader = ExtensionLoader::new(config);
        let manifests = loader.discover_extensions().unwrap();

        assert_eq!(manifests.len(), 0);
    }
}
