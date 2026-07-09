//! `grok init` command handler

use anyhow::Result;
use tracing::info;

/// Handle the `grok init [--force]` command.
///
/// Delegates to [`crate::tools::run_init`] which copies the global Grok config
/// into the current project's `.grok/` directory.
pub async fn handle_init(force: bool) -> Result<()> {
    if force {
        info!("--force supplied — existing .grok/ files will be overwritten");
    }

    match crate::tools::run_init(force) {
        Ok(msg) => {
            println!("{}", msg);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌  Failed to initialize project: {}", e);
            Err(e)
        }
    }
}
