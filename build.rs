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
    //
    // We only declare that Cargo should re-run this build script
    // when anything inside .doc/markmap changes.
    //
    // We do **NOT** auto-invoke the markmap generator here anymore.
    // This was causing the repeated "Missing HTML for .mmd file" warnings
    // on every `cargo check` / `cargo build`.
    //
    // To (re)generate the mindmaps, run manually:
    //     .\scripts\build-markmaps.ps1
    //
    // (The script prefers a globally installed `markmap` / `markmap-cli`,
    //  otherwise falls back to `npx`.)
    println!("cargo:rerun-if-changed=.doc/markmap");

    // Intentionally do nothing else here — no cargo:warning, no cargo:info,
    // no auto-execution.  This keeps `cargo check` completely clean.
}
