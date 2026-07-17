//! Plugin Sandbox for Custom Tools
//!
//! Dynamic compilation and loading of custom Rust tools using libloading.

// libloading temporarily disabled — dynamic loading is a placeholder feature.
// When re-enabled, add `libloading` to Cargo.toml and uncomment the import.
// use libloading::{Library, Symbol};

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// A dynamically loaded custom tool.
pub struct CustomTool {
    pub name: String,
    pub description: String,
    pub library_path: PathBuf,
}

/// Sandbox for loading custom tools.
/// NOTE: Dynamic loading via libloading is currently disabled (feature placeholder).
pub struct PluginSandbox {
    // loaded_libraries disabled until libloading crate is added
    registered_tools: Mutex<Vec<CustomTool>>,
}

impl PluginSandbox {
    pub fn new() -> Self {
        Self {
            registered_tools: Mutex::new(Vec::new()),
        }
    }

    /// Load all custom tools from tools/custom/.
    pub fn load_custom_tools(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dir = Path::new("tools/custom");
        if !dir.exists() {
            tracing::warn!("Custom tools directory does not exist");
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension() == Some(std::ffi::OsStr::new("rs")) {
                if let Err(e) = self.compile_and_load(&path) {
                    tracing::error!("Failed to load {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Compile a Rust source file to a dylib and immediately load it.
    fn compile_and_load(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Compiling custom tool: {}", path.display());

        let file_stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let lib_name = if cfg!(target_os = "windows") {
            format!("{}.dll", file_stem)
        } else if cfg!(target_os = "macos") {
            format!("lib{}.dylib", file_stem)
        } else {
            format!("lib{}.so", file_stem)
        };

        let out_dir = Path::new("target/debug");
        std::fs::create_dir_all(out_dir)?;
        let out_path = out_dir.join(&lib_name);

        let output = Command::new("rustc")
            .args([
                "--edition=2021",
                "--crate-type",
                "cdylib",
                "-o",
                out_path.to_str().unwrap(),
            ])
            .arg(path)
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Compilation failed: {}", err);
            return Err(format!("Compilation failed: {}", err).into());
        }

        tracing::info!("Compiled → {}", out_path.display());

        // Even though dynamic loading is currently disabled (libloading placeholder),
        // we still record the compiled tool metadata. This makes the
        // `registered_tools` field actively written to and usable.
        {
            let mut tools = self.registered_tools.lock().unwrap();
            tools.push(CustomTool {
                name: file_stem.clone(),
                description: format!("Custom tool compiled from {}", path.display()),
                library_path: out_path.clone(),
            });
        }

        // Dynamic loading disabled (libloading not present).
        tracing::warn!(
            "Dynamic tool loading is currently disabled. Compiled library at {} was not loaded (metadata recorded).",
            out_path.display()
        );
        Ok(())
    }

    /// Stub: dynamic loading requires the `libloading` crate (currently disabled).
    #[allow(dead_code)]
    fn load_library(
        &self,
        _lib_path: &Path,
        _base_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Basic schema validation (unchanged).
    pub fn validate_schema(schema: &str) -> bool {
        schema.contains("\"name\"") && schema.trim_start().starts_with('{')
    }
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// C-compatible struct that a custom tool must export via `grok_tool_init`.
#[repr(C)]
pub struct CustomToolEntry {
    pub name: *const std::ffi::c_char,
    pub description: *const std::ffi::c_char,
}

/// Helper macro that custom tools can use to export themselves.
#[macro_export]
macro_rules! export_grok_tool {
    ($name:expr, $desc:expr, $func:ident) => {
        #[no_mangle]
        pub extern "C" fn grok_tool_init() -> *mut $crate::tools::sandbox::CustomToolEntry {
            use std::ffi::CString;
            use std::ptr;

            static mut ENTRY: Option<$crate::tools::sandbox::CustomToolEntry> = None;

            unsafe {
                if ENTRY.is_none() {
                    let name = CString::new($name).unwrap();
                    let desc = CString::new($desc).unwrap();
                    ENTRY = Some($crate::tools::sandbox::CustomToolEntry {
                        name: name.into_raw(),
                        description: desc.into_raw(),
                    });
                }
                ENTRY.as_mut().unwrap() as *mut _
            }
        }
    };
}
