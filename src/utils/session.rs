use crate::display::interactive::InteractiveSession;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Get the sessions directory path
fn get_sessions_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home_dir.join(".grok").join("sessions"))
}

/// Save a session to disk
pub fn save_session(session: &InteractiveSession, name: &str) -> Result<PathBuf> {
    let sessions_dir = get_sessions_dir()?;
    if !sessions_dir.exists() {
        fs::create_dir_all(&sessions_dir)?;
    }

    let file_path = sessions_dir.join(format!("{}.json", name));
    let json = serde_json::to_string_pretty(session)?;

    fs::write(&file_path, json)?;
    Ok(file_path)
}

/// Load a session from disk
pub fn load_session(name: &str) -> Result<InteractiveSession> {
    let sessions_dir = get_sessions_dir()?;
    let file_path = sessions_dir.join(format!("{}.json", name));

    if !file_path.exists() {
        return Err(anyhow!("Session '{}' not found", name));
    }

    let json = fs::read_to_string(&file_path)?;
    let session: InteractiveSession = serde_json::from_str(&json)?;
    Ok(session)
}

/// List all saved sessions
pub fn list_sessions() -> Result<Vec<String>> {
    let sessions_dir = get_sessions_dir()?;
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(sessions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                sessions.push(stem.to_string());
            }
    }

    sessions.sort();
    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_session() {
        // We can't easily mock dirs::home_dir without more complex dependency injection or env var tricks
        // So we will verify serialization logic separately if needed, but for now this module
        // relies on file system integration.
        // A proper test would mock the session dir.
    }
}
