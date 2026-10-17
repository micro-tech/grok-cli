//! HOH Timeline Log (Task 297.11)

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub struct TimelineLog {
    path: PathBuf,
}

impl TimelineLog {
    pub fn new(base: PathBuf) -> Self {
        Self { path: base.join("hoh_timeline.log") }
    }

    pub fn append(&self, message: &str) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(f, "[{}] {}", chrono::Utc::now().to_rfc3339(), message);
        }
    }
}