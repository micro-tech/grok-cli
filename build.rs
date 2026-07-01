// build.rs
// Automatically rebuilds Markmap HTML files from .mmd sources
// by delegating to scripts/build-markmaps.ps1 on Windows.
// This runs during `cargo build` / `cargo check`.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

fn main() {
    let markmap_dir = Path::new(".doc/markmap");
    if !markmap_dir.exists() {
        return;
    }

    // Always tell cargo to watch the markmap directory
    println!("cargo:rerun-if-changed=.doc/markmap");

    // Check if any .mmd is newer than its corresponding .html
    let mut needs_rebuild = false;
    if let Ok(entries) = fs::read_dir(markmap_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("mmd") {
                continue;
            }
            let html_path = path.with_extension("html");
            let mmd_time = fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let html_time = fs::metadata(&html_path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            if mmd_time > html_time {
                needs_rebuild = true;
                break;
            }
        }
    }

    if !needs_rebuild {
        return;
    }

    println!("cargo:warning=Markmap files are out of date — rebuilding diagrams...");

    // On Windows we delegate to the PowerShell helper script
    // (it handles npx / markmap-cli correctly)
    #[cfg(windows)]
    {
        let script = Path::new("scripts/build-markmaps.ps1");
        if script.exists() {
            let status = Command::new("powershell.exe")
                .args([
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    script.to_str().unwrap(),
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("cargo:warning=✓ Markmap diagrams rebuilt successfully");
                }
                Ok(s) => {
                    println!("cargo:warning=⚠ Markmap rebuild script exited with code {:?}", s.code());
                }
                Err(e) => {
                    println!("cargo:warning=⚠ Failed to run build-markmaps.ps1: {}", e);
                    println!("cargo:warning=   You can run it manually: .\\scripts\\build-markmaps.ps1");
                }
            }
        } else {
            println!("cargo:warning=⚠ scripts/build-markmaps.ps1 not found — skipping auto-rebuild");
        }
    }

    #[cfg(not(windows))]
    {
        println!("cargo:warning=Markmap auto-rebuild is only implemented for Windows.");
        println!("cargo:warning=Please run the equivalent script manually on this platform.");
    }
}