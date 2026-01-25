//! Chat session logging module
//!
//! Provides comprehensive logging of chat sessions including:
//! - User prompts and assistant responses
//! - Session metadata and timestamps
//! - Multiple output formats (JSON, text)
//! - Automatic file rotation and management
//! - Robust error handling for network/disk issues

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Configuration for chat logging
#[derive(Debug, Clone)]
pub struct ChatLoggerConfig {
    /// Enable chat logging
    pub enabled: bool,
    /// Directory to store chat logs
    pub log_dir: PathBuf,
    /// Enable JSON format
    pub json_format: bool,
    /// Enable human-readable text format
    pub text_format: bool,
    /// Maximum log file size in MB before rotation
    pub max_file_size_mb: u64,
    /// Number of rotated files to keep
    pub rotation_count: usize,
    /// Include system messages in logs
    pub include_system: bool,
}

impl Default for ChatLoggerConfig {
    fn default() -> Self {
        let log_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".grok")
            .join("logs")
            .join("chat_sessions");

        Self {
            enabled: true,
            log_dir,
            json_format: true,
            text_format: true,
            max_file_size_mb: 10,
            rotation_count: 5,
            include_system: true,
        }
    }
}

/// Represents a single chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Timestamp of the message
    pub timestamp: DateTime<Utc>,
    /// Role: "user", "assistant", or "system"
    pub role: String,
    /// Message content
    pub content: String,
    /// Optional metadata (model used, tokens, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            role: role.into(),
            content: content.into(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Represents a complete chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// Unique session identifier
    pub session_id: String,
    /// Session start time
    pub start_time: DateTime<Utc>,
    /// Session end time (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    /// All messages in the session
    pub messages: Vec<ChatMessage>,
    /// Session metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ChatSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            start_time: Utc::now(),
            end_time: None,
            messages: Vec::new(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn end_session(&mut self) {
        self.end_time = Some(Utc::now());
    }
}

/// Chat logger instance
pub struct ChatLogger {
    config: ChatLoggerConfig,
    current_session: Mutex<Option<ChatSession>>,
}

impl ChatLogger {
    /// Create a new chat logger with the given configuration
    pub fn new(config: ChatLoggerConfig) -> Result<Self> {
        // Ensure log directory exists
        if config.enabled {
            fs::create_dir_all(&config.log_dir)
                .with_context(|| format!("Failed to create log directory: {:?}", config.log_dir))?;
            info!("Chat logger initialized: {:?}", config.log_dir);
        }

        Ok(Self {
            config,
            current_session: Mutex::new(None),
        })
    }

    /// Start a new chat session
    pub fn start_session(&self, session_id: impl Into<String>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let session_id = session_id.into();
        let mut current = self.current_session.lock().unwrap();

        // End previous session if exists
        if let Some(prev_session) = current.take() {
            warn!(
                "Starting new session {} while session {} was active. Ending previous session.",
                session_id, prev_session.session_id
            );
            drop(current); // Release lock before saving
            self.save_session(&prev_session)?;
            current = self.current_session.lock().unwrap();
        }

        let session = ChatSession::new(&session_id);
        info!("Started chat session: {}", session_id);
        *current = Some(session);

        Ok(())
    }

    /// Log a user prompt
    pub fn log_user_message(&self, content: impl Into<String>) -> Result<()> {
        self.log_message("user", content, None)
    }

    /// Log an assistant response
    pub fn log_assistant_message(&self, content: impl Into<String>) -> Result<()> {
        self.log_message("assistant", content, None)
    }

    /// Log a system message
    pub fn log_system_message(&self, content: impl Into<String>) -> Result<()> {
        if !self.config.include_system {
            return Ok(());
        }
        self.log_message("system", content, None)
    }

    /// Log a message with optional metadata
    pub fn log_message(
        &self,
        role: impl Into<String>,
        content: impl Into<String>,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut current = self.current_session.lock().unwrap();
        if let Some(session) = current.as_mut() {
            let mut message = ChatMessage::new(role, content);
            if let Some(meta) = metadata {
                message = message.with_metadata(meta);
            }
            debug!(
                "Logging message in session {}: {}",
                session.session_id, message.role
            );
            session.add_message(message);
        } else {
            warn!("Attempted to log message without an active session");
        }

        Ok(())
    }

    /// End the current session and save it
    pub fn end_session(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut current = self.current_session.lock().unwrap();
        if let Some(mut session) = current.take() {
            session.end_session();
            let session_id = session.session_id.clone();
            drop(current); // Release lock before saving
            self.save_session(&session)?;
            info!("Ended chat session: {}", session_id);
        }

        Ok(())
    }

    /// Save a session to disk
    fn save_session(&self, session: &ChatSession) -> Result<()> {
        let base_path = self.config.log_dir.join(&session.session_id);

        // Save JSON format
        if self.config.json_format {
            let json_path = base_path.with_extension("json");
            self.save_json(session, &json_path)
                .with_context(|| format!("Failed to save JSON log: {:?}", json_path))?;
        }

        // Save text format
        if self.config.text_format {
            let text_path = base_path.with_extension("txt");
            self.save_text(session, &text_path)
                .with_context(|| format!("Failed to save text log: {:?}", text_path))?;
        }

        // Check and rotate if needed
        self.rotate_logs_if_needed()?;

        Ok(())
    }

    /// Save session as JSON
    fn save_json(&self, session: &ChatSession, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(session)
            .with_context(|| "Failed to serialize session to JSON")?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("Failed to open file: {:?}", path))?;

        file.write_all(json.as_bytes())
            .with_context(|| format!("Failed to write JSON to file: {:?}", path))?;

        debug!("Saved JSON log: {:?}", path);
        Ok(())
    }

    /// Save session as human-readable text
    fn save_text(&self, session: &ChatSession, path: &Path) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("Failed to open file: {:?}", path))?;

        // Write header
        let separator = "=".repeat(80);
        writeln!(file, "{}", separator)?;
        writeln!(file, "GROK CLI CHAT SESSION LOG")?;
        writeln!(file, "{}", separator)?;
        writeln!(file, "Session ID: {}", session.session_id)?;
        writeln!(
            file,
            "Start Time: {}",
            session.start_time.format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        if let Some(end_time) = session.end_time {
            writeln!(
                file,
                "End Time:   {}",
                end_time.format("%Y-%m-%d %H:%M:%S UTC")
            )?;
            let duration = end_time.signed_duration_since(session.start_time);
            writeln!(file, "Duration:   {} seconds", duration.num_seconds())?;
        }
        writeln!(file, "Messages:   {}", session.messages.len())?;
        let separator = "=".repeat(80);
        writeln!(file, "{}", separator)?;
        writeln!(file)?;

        // Write messages
        for (i, msg) in session.messages.iter().enumerate() {
            writeln!(
                file,
                "[{}] {} - {}",
                i + 1,
                msg.role.to_uppercase(),
                msg.timestamp.format("%H:%M:%S")
            )?;
            let line_sep = "-".repeat(80);
            writeln!(file, "{}", line_sep)?;
            writeln!(file, "{}", msg.content)?;

            if let Some(metadata) = &msg.metadata {
                writeln!(
                    file,
                    "\nMetadata: {}",
                    serde_json::to_string_pretty(metadata).unwrap_or_default()
                )?;
            }

            writeln!(file)?;
        }

        // Write footer
        let separator = "=".repeat(80);
        writeln!(file, "{}", separator)?;
        writeln!(file, "END OF SESSION")?;
        writeln!(file, "{}", separator)?;

        debug!("Saved text log: {:?}", path);
        Ok(())
    }

    /// Rotate logs if they exceed the size limit
    fn rotate_logs_if_needed(&self) -> Result<()> {
        let max_bytes = self.config.max_file_size_mb * 1024 * 1024;

        // Get all log files
        let entries = match fs::read_dir(&self.config.log_dir) {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read log directory for rotation: {}", e);
                return Ok(()); // Don't fail on rotation issues
            }
        };

        let mut total_size: u64 = 0;
        let mut files: Vec<(PathBuf, u64)> = Vec::new();

        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata()
                && metadata.is_file()
            {
                let size = metadata.len();
                total_size += size;
                files.push((entry.path(), size));
            }
        }

        // If total size exceeds limit, remove oldest files
        if total_size > max_bytes {
            // Sort by modification time (oldest first)
            files.sort_by_key(|(path, _)| {
                fs::metadata(path)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });

            // Calculate how many to remove
            let files_to_keep = files.len().saturating_sub(self.config.rotation_count);

            for (path, _) in files.iter().take(files_to_keep) {
                if let Err(e) = fs::remove_file(path) {
                    warn!("Failed to remove old log file {:?}: {}", path, e);
                } else {
                    debug!("Rotated old log file: {:?}", path);
                }
            }
        }

        Ok(())
    }

    /// List all saved sessions
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = Vec::new();

        let entries = fs::read_dir(&self.config.log_dir)
            .with_context(|| format!("Failed to read log directory: {:?}", self.config.log_dir))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                sessions.push(stem.to_string());
            }
        }

        sessions.sort();
        sessions.reverse(); // Most recent first

        Ok(sessions)
    }

    /// Load a session from disk
    pub fn load_session(&self, session_id: &str) -> Result<ChatSession> {
        let json_path = self.config.log_dir.join(session_id).with_extension("json");

        let content = fs::read_to_string(&json_path)
            .with_context(|| format!("Failed to read session file: {:?}", json_path))?;

        let session: ChatSession = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse session JSON: {:?}", json_path))?;

        Ok(session)
    }
}

/// Global chat logger instance
static GLOBAL_LOGGER: Mutex<Option<ChatLogger>> = Mutex::new(None);

/// Initialize the global chat logger
pub fn init(config: ChatLoggerConfig) -> Result<()> {
    let logger = ChatLogger::new(config)?;
    let mut global = GLOBAL_LOGGER.lock().unwrap();
    *global = Some(logger);
    Ok(())
}

/// Get a reference to the global logger (if initialized)
pub fn get_logger() -> Option<ChatLogger> {
    let global = GLOBAL_LOGGER.lock().unwrap();
    global.as_ref().map(|logger| ChatLogger {
        config: logger.config.clone(),
        current_session: Mutex::new(None),
    })
}

/// Start a new session using the global logger
pub fn start_session(session_id: impl Into<String>) -> Result<()> {
    let global = GLOBAL_LOGGER.lock().unwrap();
    if let Some(logger) = global.as_ref() {
        logger.start_session(session_id)?;
    }
    Ok(())
}

/// Log a user message using the global logger
pub fn log_user(content: impl Into<String>) -> Result<()> {
    let global = GLOBAL_LOGGER.lock().unwrap();
    if let Some(logger) = global.as_ref() {
        logger.log_user_message(content)?;
    }
    Ok(())
}

/// Log an assistant message using the global logger
pub fn log_assistant(content: impl Into<String>) -> Result<()> {
    let global = GLOBAL_LOGGER.lock().unwrap();
    if let Some(logger) = global.as_ref() {
        logger.log_assistant_message(content)?;
    }
    Ok(())
}

/// Log a system message using the global logger
pub fn log_system(content: impl Into<String>) -> Result<()> {
    let global = GLOBAL_LOGGER.lock().unwrap();
    if let Some(logger) = global.as_ref() {
        logger.log_system_message(content)?;
    }
    Ok(())
}

/// End the current session using the global logger
pub fn end_session() -> Result<()> {
    let global = GLOBAL_LOGGER.lock().unwrap();
    if let Some(logger) = global.as_ref() {
        logger.end_session()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_chat_session_creation() {
        let session = ChatSession::new("test-session-123");
        assert_eq!(session.session_id, "test-session-123");
        assert!(session.messages.is_empty());
        assert!(session.end_time.is_none());
    }

    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage::new("user", "Hello, world!");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, world!");
        assert!(msg.metadata.is_none());
    }

    #[test]
    fn test_session_add_message() {
        let mut session = ChatSession::new("test-session");
        session.add_message(ChatMessage::new("user", "Hello"));
        session.add_message(ChatMessage::new("assistant", "Hi there!"));
        assert_eq!(session.messages.len(), 2);
    }

    #[test]
    fn test_logger_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config = ChatLoggerConfig {
            enabled: true,
            log_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let logger = ChatLogger::new(config).unwrap();
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_session_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let config = ChatLoggerConfig {
            enabled: true,
            log_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let logger = ChatLogger::new(config).unwrap();

        logger.start_session("test-lifecycle").unwrap();
        logger.log_user_message("Test message").unwrap();
        logger.log_assistant_message("Test response").unwrap();
        logger.end_session().unwrap();

        // Check that files were created
        assert!(temp_dir.path().join("test-lifecycle.json").exists());
        assert!(temp_dir.path().join("test-lifecycle.txt").exists());
    }
}
