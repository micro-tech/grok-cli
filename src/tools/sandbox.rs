//! Plugin Sandbox for Custom Tools
//!
//! Dynamic compilation and loading of custom Rust tools using libloading.

// libloading temporarily disabled — dynamic loading is a placeholder feature.
// When re-enabled, add `libloading` to Cargo.toml and uncomment the import.
// use libloading::{Library, Symbol};

use std::collections::HashMap;
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
pub struct PluginSandbox {
    loaded_libraries: Mutex<HashMap<String, Library>>,
    registered_tools: Mutex<Vec<CustomTool>>,
}

impl PluginSandbox {
    pub fn new() -> Self {
        Self {
            loaded_libraries: Mutex::new(HashMap::new()),
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
        self.load_library(&out_path, &file_stem)
    }

    /// dlopen the compiled library and register any exported tools.
    fn load_library(
        &self,
        lib_path: &Path,
        base_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let lib = Library::new(lib_path)?;

            // Try to find the conventional entry point: `grok_tool_init`
            if let Ok(init_fn) = lib.get::<Symbol<fn() -> *mut CustomToolEntry>>(b"grok_tool_init")
            {
                let entry = &*init_fn();
                let tool = CustomTool {
                    name: entry.name.to_string(),
                    description: entry.description.to_string(),
                    library_path: lib_path.to_path_buf(),
                };

                // Register with the global registry
                crate::tools::registry::register_dynamic_tool(
                    &tool.name,
                    &tool.description,
                    lib_path,
                );

                let mut tools = self.registered_tools.lock().unwrap();
                tools.push(tool);

                tracing::info!("Registered dynamic tool: {}", entry.name);
            } else {
                tracing::warn!(
                    "Library {} has no grok_tool_init symbol — skipping registration",
                    base_name
                );
            }

            // Keep the library alive for the lifetime of the process
            let mut libs = self.loaded_libraries.lock().unwrap();
            libs.insert(base_name.to_string(), lib);
        }

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
