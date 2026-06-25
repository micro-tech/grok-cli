//! Plugin Sandbox for Custom Tools
//!
//! Dynamic compilation and loading of custom Rust tools.

use std::path::Path;

use libloading::{Library, Symbol};

/// Sandbox for loading custom tools.
pub struct PluginSandbox {
    loaded_libraries: Vec<Library>,
}

impl PluginSandbox {
    pub fn new() -> Self {
        Self {
            loaded_libraries: Vec::new(),
        }
    }

    /// Load custom tools from tools/custom/.
    pub fn load_custom_tools(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = Path::new("tools/custom");
        if !dir.exists() {
            tracing::warn!("Custom tools directory does not exist");
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension() == Some(std::ffi::OsStr::new("rs")) {
                self.compile_and_load(&path)?;
            }
        }

        Ok(())
    }

    /// Compile a Rust file to a dylib and load it.
    fn compile_and_load(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Compiling custom tool: {}", path.display());

        // Determine output filename (platform-specific)
        let file_stem = path.file_stem().unwrap().to_string_lossy();
        #[cfg(target_os = "windows")]
        let dylib_name = format!("{}.dll", file_stem);
        #[cfg(target_os = "macos")]
        let dylib_name = format!("lib{}.dylib", file_stem);
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let dylib_name = format!("lib{}.so", file_stem);

        let output_path = std::env::temp_dir().join(&dylib_name);

        // Compile with rustc
        let output = std::process::Command::new("rustc")
            .args(["--crate-type", "cdylib", "-o"])
            .arg(&output_path)
            .arg(path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Compilation failed: {}", stderr);
            return Err(format!("Compilation failed: {}", stderr).into());
        }

        // Load the compiled dylib
        // SAFETY: We assume the dylib follows our expected interface.
        let lib = unsafe { Library::new(&output_path)? };
        self.loaded_libraries.push(lib);

        tracing::info!("Successfully loaded custom tool from {}", path.display());
        Ok(())
    }

    /// Validate a tool schema JSON string.
    pub fn validate_schema(schema: &str) -> bool {
        // Basic validation: must be valid JSON and contain a "name" field
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(schema) {
            value.get("name").is_some()
        } else {
            false
        }
    }
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self::new()
    }
}