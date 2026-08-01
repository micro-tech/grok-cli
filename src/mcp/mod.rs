pub mod client;
pub mod config;
pub mod protocol;

pub use client::McpClient;
pub use config::McpConfig;

/// The MCP protocol version we speak and advertise.
///
/// **Default is now stateless** (2026-07-28 style per SEP-2575 + SEP-2567).
/// Client info + protocol version are sent on **every** request via `_meta`.
/// No `initialize` handshake is performed unless `use_legacy_handshake: true`
/// is explicitly set for ancient servers.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Client name we advertise (used in both legacy handshake and _meta).
pub const MCP_CLIENT_NAME: &str = "grok-cli";

/// Upcoming target. Once the 2026-07-28 spec is released we can flip defaults.
pub const MCP_PROTOCOL_VERSION_2026: &str = "2026-07-28";
