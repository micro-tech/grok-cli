fn main() {
    // Inject a build timestamp so the installer can print it at runtime.
    // This makes it easy to confirm a fresh binary is actually being used.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    // Re-run whenever the installer source or either embedded config changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/bin/installer.rs");
    println!("cargo:rerun-if-changed=config.toml");
    println!("cargo:rerun-if-changed=system_manifest.md");

    // === Markmap Documentation Build Integration ===
    // Detect when .mmd files are newer than their .html counterparts
    // and trigger the markmap build script.
    println!("cargo:rerun-if-changed=.doc/markmap");

    // Check if any .mmd file is newer than its corresponding .html
    let markmap_dir = std::path::Path::new(".doc/markmap");
    let html_dir = std::path::Path::new(".doc/markmap/html");

    if markmap_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(markmap_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "mmd" {
                        let mmd_path = entry.path();
                        let html_name =
                            mmd_path.file_stem().unwrap().to_string_lossy().to_string() + ".html";
                        let html_path = html_dir.join(html_name);

                        let mmd_modified =
                            std::fs::metadata(&mmd_path).and_then(|m| m.modified()).ok();

                        let html_modified = std::fs::metadata(&html_path)
                            .and_then(|m| m.modified())
                            .ok();

                        if let (Some(mmd_time), Some(html_time)) = (mmd_modified, html_modified) {
                            if mmd_time > html_time {
                                println!("cargo:warning=.mmd file newer than .html — attempting to rebuild markmaps...");
                                let _ = std::process::Command::new("powershell")
                                    .args(["-ExecutionPolicy", "Bypass", "-File", "scripts/build-markmaps.ps1"])
                                    .status();
                            }
                        } else if mmd_modified.is_some() && html_modified.is_none() {
                            // HTML doesn't exist yet
                            println!("cargo:warning=Missing HTML for .mmd file — attempting to build markmaps...");
                            let _ = std::process::Command::new("powershell")
                                .args(["-ExecutionPolicy", "Bypass", "-File", "scripts/build-markmaps.ps1"])
                                .status();
                        }
                    }
                }
            }
        }
    }
}
