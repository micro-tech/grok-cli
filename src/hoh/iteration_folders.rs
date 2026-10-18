//! Iteration Folder Structure (Task 297.10)

use std::path::PathBuf;

pub fn iteration_dir(base: &PathBuf, id: u64) -> PathBuf {
    base.join("iterations").join(format!("{:04}", id))
}

pub fn ensure_folders(base: &PathBuf, id: u64) -> std::io::Result<()> {
    let dir = iteration_dir(base, id);
    std::fs::create_dir_all(dir.join("patches"))?;
    std::fs::create_dir_all(dir.join("logs"))?;
    std::fs::create_dir_all(dir.join("evals"))?;
    Ok(())
}