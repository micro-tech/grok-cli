//! Version handling and self-update primitives for grok-cli.
//!
//! Provides:
//! - Current version from build
//! - Semantic version comparison (lightweight, no external crate)
//! - GitHub release metadata fetching
//! - Platform asset selection
//!
//! Used by the `self-updater` skill and the `grok update` command.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::env;

/// Returns the current version of grok-cli (from Cargo.toml at build time).
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Lightweight semantic version comparison.
/// Returns:
/// - `Ordering::Greater` if `a > b` (a is newer)
/// - `Ordering::Less`    if `a < b`
/// - `Ordering::Equal`   if equal or unparsable (safe default)
///
/// Supports forms like "0.2.8", "0.2.8-prerelease", "v0.2.8"
use std::cmp::Ordering;

pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let a_clean = a.trim_start_matches('v').split('-').next().unwrap_or(a);
    let b_clean = b.trim_start_matches('v').split('-').next().unwrap_or(b);

    let a_parts: Vec<u32> = a_clean
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    let b_parts: Vec<u32> = b_clean
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    if a_parts.is_empty() || b_parts.is_empty() {
        // Cannot parse reliably — treat as equal to avoid accidental downgrades
        return Ordering::Equal;
    }

    // Pad shorter one with zeros
    let max_len = a_parts.len().max(b_parts.len());
    let a_padded: Vec<u32> = a_parts.iter().chain(std::iter::repeat(&0)).take(max_len).copied().collect();
    let b_padded: Vec<u32> = b_parts.iter().chain(std::iter::repeat(&0)).take(max_len).copied().collect();

    a_padded.cmp(&b_padded)
}

/// GitHub release asset.
#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Minimal GitHub release response.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub assets: Vec<ReleaseAsset>,
    pub html_url: String,
}

/// Fetch the latest release metadata from GitHub (public API, no auth required for public repos).
pub async fn fetch_latest_release() -> Result<GitHubRelease> {
    let url = "https://api.github.com/repos/micro-tech/grok-cli/releases/latest";

    let client = reqwest::Client::builder()
        .user_agent("grok-cli-self-updater/1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("Failed to build HTTP client for update check")?;

    let resp = client
        .get(url)
        .send()
        .await
        .context("Failed to fetch latest release from GitHub")?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "GitHub API returned status {} — rate limit or repo issue?",
            resp.status()
        ));
    }

    let release: GitHubRelease = resp
        .json()
        .await
        .context("Failed to parse GitHub release JSON")?;

    Ok(release)
}

/// Detect the current platform and return a best-guess asset name suffix / pattern.
///
/// Current supported patterns (adjust as release artifacts evolve):
/// - Windows: grok-cli-*-windows-*.exe or grok-cli.exe
/// - Linux:   grok-cli-*-linux-* or grok-cli
/// - macOS:   grok-cli-*-apple-* or grok-cli
pub fn current_platform_asset_pattern() -> (&'static str, &'static str) {
    match (env::consts::OS, env::consts::ARCH) {
        ("windows", "x86_64") => ("windows", "x86_64"),
        ("windows", "x86") => ("windows", "x86"),
        ("linux", "x86_64") => ("linux", "x86_64"),
        ("linux", "aarch64") => ("linux", "aarch64"),
        ("macos", "x86_64") => ("apple", "x86_64"),
        ("macos", "aarch64") => ("apple", "aarch64"),
        _ => (env::consts::OS, env::consts::ARCH),
    }
}

/// Try to find the best matching asset for the current platform from a release.
pub fn find_best_asset_for_current_platform(release: &GitHubRelease) -> Option<&ReleaseAsset> {
    let (os_hint, arch_hint) = current_platform_asset_pattern();

    // Prefer exact platform matches
    let candidates: Vec<_> = release
        .assets
        .iter()
        .filter(|a| {
            let name = a.name.to_lowercase();
            // Common patterns we may see in future releases
            (name.contains(os_hint) || name.contains("windows") || name.contains("linux") || name.contains("darwin") || name.contains("apple"))
                && (name.contains(arch_hint) || name.contains("x86_64") || name.contains("amd64") || name.contains("aarch64"))
                && (name.contains("grok") || name.ends_with(".exe") || !name.contains('.'))
        })
        .collect();

    if !candidates.is_empty() {
        // Prefer .exe on Windows, otherwise the first good match
        if env::consts::OS == "windows" {
            if let Some(exe) = candidates.iter().find(|a| a.name.ends_with(".exe")) {
                return Some(exe);
            }
        }
        return candidates.first().copied();
    }

    // Fallback: look for anything that looks like a binary for this OS
    release.assets.iter().find(|a| {
        let n = a.name.to_lowercase();
        match env::consts::OS {
            "windows" => n.ends_with(".exe") || n.contains("windows"),
            _ => !n.contains(".zip") && !n.contains(".tar") && !n.contains("sha") && !n.contains("sig"),
        }
    })
}

/// Simple version check result.
#[derive(Debug, Clone)]
pub struct UpdateCheckResult {
    pub current: String,
    pub latest: String,
    pub is_newer_available: bool,
    pub release: GitHubRelease,
}

pub async fn check_for_update() -> Result<UpdateCheckResult> {
    let current = current_version().to_string();
    let release = fetch_latest_release().await?;
    let latest = release.tag_name.trim_start_matches('v').to_string();

    let is_newer = compare_versions(&latest, &current) == Ordering::Greater;

    Ok(UpdateCheckResult {
        current,
        latest,
        is_newer_available: is_newer,
        release,
    })
}
