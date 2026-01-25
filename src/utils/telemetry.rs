use chrono::Utc;
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

struct TelemetryState {
    enabled: bool,
    log_file: Option<PathBuf>,
}

static TELEMETRY_STATE: Mutex<TelemetryState> = Mutex::new(TelemetryState {
    enabled: false,
    log_file: None,
});

pub fn init(enabled: bool, log_path: Option<PathBuf>) {
    let mut state = TELEMETRY_STATE.lock().unwrap();
    state.enabled = enabled;
    state.log_file = log_path;
}

pub fn track_event(event: &str, properties: Value) {
    let state = TELEMETRY_STATE.lock().unwrap();
    if !state.enabled {
        return;
    }

    let timestamp = Utc::now().to_rfc3339();
    let log_entry = json!({
        "timestamp": timestamp,
        "event": event,
        "properties": properties
    });

    if let Some(path) = &state.log_file
        && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{}", log_entry);
        }
}
