//! Self-update execution logic for grok-cli.
//!
//! This module provides the mechanical parts of updating:
//! - Downloading the release asset
//! - Verifying the download
//! - Safely replacing the current binary (with platform-specific care)
//!
//! The decision making, user messaging, and high-level flow live in
//! the `self-updater` skill and the `grok update` command.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;

use crate::utils::version::{GitHubRelease, ReleaseAsset, current_platform_asset_pattern};

/// Returns the path to the currently running executable.
pub fn current_exe_path() -> Result<PathBuf> {
    env::current_exe().context("Failed to determine current executable path")
}

/// Detect if we are running from a development / cargo environment.
pub fn is_running_from_source() -> bool {
    // Heuristic: if the exe is inside target/debug or target/release, or we see "cargo"
    let exe = current_exe_path().unwrap_or_default();
    let path_str = exe.to_string_lossy().to_lowercase();
    path_str.contains("target/debug") ||
    path_str.contains("target/release") ||
    env::var("CARGO").is_ok()
}

/// Download a release asset to a temporary file and return the path.
pub async fn download_asset(asset: &ReleaseAsset) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .user_agent("grok-cli-self-updater/1.0")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to create HTTP client for download")?;

    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .context("Failed to start download")?;

    if !response.status().is_success() {
        return Err(anyhow!("Download failed with status: {}", response.status()));
    }

    let total_size = asset.size;
    let mut downloaded: u64 = 0;

    let temp_dir = std::env::temp_dir();
    // Use a safe name
    let file_name = format!("grok-cli-update-{}", asset.name);
    let temp_path = temp_dir.join(&file_name);

    let mut file = fs::File::create(&temp_path)
        .with_context(|| format!("Failed to create temp file at {}", temp_path.display()))?;

    let mut stream = response.bytes_stream();

    use futures_util::StreamExt; // for next()

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error while downloading chunk")?;
        std::io::Write::write_all(&mut file, &chunk)?;
        downloaded += chunk.len() as u64;

        // Simple progress hint (can be improved with indicatif later)
        if total_size > 0 && downloaded % (1024 * 512) == 0 {
            let pct = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            eprintln!("  Download progress: {}% ({}/{} bytes)", pct, downloaded, total_size);
        }
    }

    file.sync_all()?;
    drop(file);

    // Final size check
    let metadata = fs::metadata(&temp_path)?;
    if metadata.len() != asset.size && asset.size > 0 {
        // Some releases don't report accurate size; only warn
        tracing::warn!("Downloaded size {} differs from declared {}", metadata.len(), asset.size);
    }

    Ok(temp_path)
}

/// Basic verification (size + future: checksum).
pub fn verify_download(downloaded_path: &Path, expected_size: u64) -> Result<()> {
    let meta = fs::metadata(downloaded_path)
        .with_context(|| format!("Cannot stat downloaded file {}", downloaded_path.display()))?;

    if expected_size > 0 && meta.len() != expected_size {
        return Err(anyhow!(
            "Size mismatch: downloaded {} bytes, expected {}",
            meta.len(),
            expected_size
        ));
    }

    // TODO: support sha256 if we publish checksums in future releases
    Ok(())
}

/// Perform the actual binary replacement.
/// This is the dangerous part — called only after explicit user confirmation.
pub fn replace_binary(downloaded_path: &Path, target_path: &Path) -> Result<()> {
    if !downloaded_path.exists() {
        return Err(anyhow!("Downloaded binary no longer exists"));
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(downloaded_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(downloaded_path, perms)?;
    }

    // On Windows the current exe is usually locked.
    // We do a best-effort rename. If it fails, leave the new file next to it with instructions.
    let backup_path = target_path.with_extension("old");

    // Try to remove old backup if present
    let _ = fs::remove_file(&backup_path);

    // Best effort: move current out of the way
    if target_path.exists() {
        match fs::rename(target_path, &backup_path) {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Could not rename old binary: {e}. Will try direct overwrite.");
            }
        }
    }

    // Move the new binary into place
    fs::rename(downloaded_path, target_path)
        .with_context(|| format!(
            "Failed to replace binary at {}. The new binary is at: {}",
            target_path.display(),
            downloaded_path.display()
        ))?;

    // On Unix we can try to remove the backup
    #[cfg(unix)]
    {
        let _ = fs::remove_file(&backup_path);
    }

    Ok(())
}

/// High-level update flow (download + verify + replace).
/// Returns the path that was written.
pub async fn perform_self_update(
    asset: &ReleaseAsset,
    target_path: Option<PathBuf>,   // None = use current_exe
) -> Result<PathBuf> {
    let target = match target_path {
        Some(p) => p,
        None => current_exe_path()?,
    };

    if is_running_from_source() {
        return Err(anyhow!(
            "You appear to be running from source (cargo). \
             Please run `cargo install --force --git https://github.com/micro-tech/grok-cli` or rebuild instead of using the binary updater."
        ));
    }

    println!("Downloading {} ...", asset.name);
    let downloaded = download_asset(asset).await?;

    println!("Verifying download...");
    verify_download(&downloaded, asset.size)?;

    println!("Replacing current binary at {} ...", target.display());

    replace_binary(&downloaded, &target)?;

    println!("✓ Binary successfully replaced.");
    Ok(target)
}

/// Convenience: given a full GitHubRelease, pick the best asset and update.
pub async fn update_to_release(release: &GitHubRelease) -> Result<PathBuf> {
    let asset = crate::utils::version::find_best_asset_for_current_platform(release)
        .ok_or_else(|| anyhow!("No suitable binary found for your platform ({}-{}) in this release.",
            std::env::consts::OS, std::env::consts::ARCH))?;

    println!("Selected asset: {} ({} bytes)", asset.name, asset.size);

    perform_self_update(asset, None).await
}
