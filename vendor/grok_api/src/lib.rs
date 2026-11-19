//! # Grok API Client
//!
//! A Rust client library for interacting with the Grok AI API (xAI).
//!
//! This library provides a simple and robust interface to the Grok AI models,
//! with built-in retry logic, error handling, and support for challenging
//! network conditions (like Starlink satellite connections).
//!
//! ## Features
//!
//! - **Simple API**: Easy-to-use async interface
//! - **Robust Error Handling**: Comprehensive error types with detailed messages
//! - **Automatic Retries**: Built-in retry logic for transient network failures
//! - **Starlink Optimized**: Special handling for satellite network drops
//! - **Rate Limiting**: Client-side rate limit tracking (optional)
//! - **Tool/Function Calling**: Support for function calling and tools
//!
//! ## Quick Start
//!
//! ```no_run
//! use grok_api::{GrokClient, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create a client with your API key
//!     let client = GrokClient::new("your-api-key")?;
//!
//!     // Send a simple chat message
//!     let response = client
//!         .chat("What is Rust?", None)
//!         .await?;
//!
//!     println!("Response: {}", response);
//!     Ok(())
//! }
//! ```
//!
//! ## Advanced Usage
//!
//! ```no_run
//! use grok_api::{GrokClient, ChatMessage, Result};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = GrokClient::builder()
//!         .api_key("your-api-key")
//!         .timeout_secs(60)
//!         .max_retries(5)
//!         .build()?;
//!
//!     // Build a conversation
//!     let messages = vec![
//!         ChatMessage::system("You are a helpful Rust programming assistant."),
//!         ChatMessage::user("How do I create a Vec in Rust?"),
//!     ];
//!
//!     let response = client
//!         .chat_with_history(&messages)
//!         .temperature(0.7)
//!         .max_tokens(1000)
//!         .model("grok-4.7")
//!         .send()
//!         .await?;
//!
//!     println!("Response: {}", response.content().unwrap_or(""));
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod models;
mod retry;
mod types;
pub mod voice;

pub use client::{GrokClient, GrokClientBuilder};
pub use error::{Error, Result};
pub use models::{
    ChatMessage, ChatRequest, ChatResponse, Choice, ContentPart, FunctionCall, ImageUrl, Message,
    MessageContent, Model, ToolCall, Usage, VideoUrl,
};
pub use retry::RetryConfig;
pub use types::{ReasoningDepth, Role, StopReason};
pub use voice::{
    AudioCodec, AudioFormat, ReasoningEffort, TurnDetection, TurnDetectionKind, VoiceEvent,
    VoiceSession, VoiceSessionBuilder, VoiceUsage,
};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::client::{GrokClient, GrokClientBuilder};
    pub use crate::error::{Error, Result};
    pub use crate::models::{ChatMessage, ChatResponse};
    pub use crate::types::{ReasoningDepth, Role};
}

// Version information
/// The version of this crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The default API base URL for Grok (xAI)
pub const DEFAULT_API_BASE_URL: &str = "https://api.x.ai";

/// The default model to use if none is specified
pub const DEFAULT_MODEL: &str = "grok-4.7";

/// Default timeout in seconds
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default maximum number of retries
pub const DEFAULT_MAX_RETRIES: u32 = 3;
