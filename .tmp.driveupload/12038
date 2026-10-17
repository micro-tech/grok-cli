use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

pub fn get_profile_path() -> Option<PathBuf> {
    Some(crate::config::grok_config_dir().join("bayes_profile.json"))
}

/// Load persisted Bayesian priors.
/// Returns `None` for "no file yet" (normal first-run case).
/// On corrupt JSON or read error we log a warning and fall back to defaults
/// (Task 457: robust error handling, Starlink-safe, no crash).
pub fn load_profile() -> Option<HashMap<String, f32>> {
    let path = get_profile_path()?;
    if !path.exists() {
        debug!("No Bayesian profile on disk yet — using defaults");
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(priors) => Some(priors),
            Err(e) => {
                warn!(
                    "Corrupt Bayesian profile at {:?} ({}). Falling back to defaults. \
                     Delete the file to start fresh.",
                    path, e
                );
                None
            }
        },
        Err(e) => {
            warn!("Failed to read Bayesian profile {:?}: {}. Using defaults.", path, e);
            None
        }
    }
}

/// Save the current priors.
/// Uses a simple write (future improvement: atomic rename for extra safety).
/// Errors are logged at the call site; we never panic on persistence failure.
pub fn save_profile(priors: &HashMap<String, f32>) -> Result<()> {
    if let Some(path) = get_profile_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(priors)?;
        fs::write(&path, content)?;
        debug!("Bayesian profile saved to {:?}", path);
    }
    Ok(())
}
