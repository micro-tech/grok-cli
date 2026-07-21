pub mod client;
pub mod config;
pub mod protocol;

pub use client::McpClient;
pub use config::McpConfig;

/// The MCP protocol version we speak and advertise.
/// We target the stable 2024-11-05 release (the first widely-deployed
/// version after the initial 0.1.0 draft).
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
