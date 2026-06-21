use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn get_profile_path() -> Option<PathBuf> {
    Some(crate::config::grok_config_dir().join("bayes_profile.json"))
}

pub fn load_profile() -> Option<HashMap<String, f32>> {
    let path = get_profile_path()?;
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_profile(priors: &HashMap<String, f32>) -> Result<()> {
    if let Some(path) = get_profile_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(priors)?;
        fs::write(path, content)?;
    }
    Ok(())
}
