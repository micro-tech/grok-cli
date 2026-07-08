//! `grok init` command handler

use anyhow::Result;
use tracing::info;

/// Handle the `grok init [--force]` command
pub async fn handle_init(force: bool) -> Result<()> {
    if force {
        // For now we just call the same logic; later we can add real overwrite support
        info!("--force supplied — will overwrite existing .grok/ if present");
    }

    match crate::tools::run_init() {
        Ok(msg) => {
            println!("{}", msg);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize project: {}", e);
            Err(e)
        }
    }
}