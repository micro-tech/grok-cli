pub mod client;
pub mod config;
pub mod protocol;

pub use client::McpClient;
pub use config::McpConfig;

/// The MCP protocol version we speak and advertise.
///
/// Phase 0 (pre-2026): We still default to the widely-deployed 2024-11-05
/// for maximum compatibility, but we are preparing for the stateless
/// 2026-07-28 protocol (SEP-2575 + SEP-2567).
///
/// When `use_legacy_handshake: false` is set on a server config, we will
/// embed client info + protocol version inside `_meta` on every request
/// instead of doing a one-time `initialize` handshake.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Client name we advertise (used in both legacy handshake and _meta).
pub const MCP_CLIENT_NAME: &str = "grok-cli";

/// Upcoming target. Once the 2026-07-28 spec is released we can flip defaults.
pub const MCP_PROTOCOL_VERSION_2026: &str = "2026-07-28";
