pub mod client;
pub mod config;
pub mod protocol;

pub use client::McpClient;
pub use config::McpConfig;

/// The MCP protocol version we speak and advertise.
///
/// **Default (and strongly preferred) is the 2026-07-28 stateless model**.
/// Client info + protocol version are sent on **every** request via `_meta`.
/// No `initialize` handshake is performed unless `use_legacy_handshake: true`
/// is explicitly set for very old servers.
///
/// See tasks 239/240 for the migration.
pub const MCP_PROTOCOL_VERSION: &str = "2026-07-28";

/// Client name we advertise (used in both legacy handshake and _meta).
pub const MCP_CLIENT_NAME: &str = "grok-cli";

/// Legacy protocol version kept for servers that still require the old handshake.
pub const MCP_PROTOCOL_VERSION_LEGACY: &str = "2024-11-05";
