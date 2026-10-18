# MCP (Model Context Protocol) Support

**Status**: ✅ Fully implemented and operational (v0.3+)

Grok CLI has first-class support for the [Model Context Protocol (MCP)](https://spec.modelcontextprotocol.io/). This allows the agent to connect to external tool servers (GitHub, filesystem, databases, web search, etc.) using a standardized JSON-RPC interface.

## Two Operating Modes

### 1. Connected Mode (Preferred)
When an ACP client (such as Zed) sends an `mcpServers` array during `session/new`, Grok CLI automatically connects to those servers using the persistent `McpClient`.

Use the `mcp_call_connected` path internally — no per-call spawning.

### 2. Stateless / Legacy Mode
Direct invocation via raw command string (e.g. for CLI or testing).

- Uses `_meta` protocol hints for caching (2026+ style)
- 30-second hard timeout + Starlink-aware retries
- Security denylist validation before spawning

## Built-in MCP Tools

| Tool                  | Description                                      | When to Use                     |
|-----------------------|--------------------------------------------------|---------------------------------|
| `mcp_call`            | Legacy/raw command invocation                    | Direct CLI or one-off calls     |
| `mcp_call_connected`  | Call a pre-connected server (recommended)        | Normal agent operation via ACP  |

Both tools are registered in the tool system and visible to the model.

## GitHub MCP Server (Built-in)

A reference MCP server is included:

```bash
cargo build --bin github_mcp
```

Binary: `target/release/github_mcp` (or `.exe` on Windows)

### Example Configuration (`.grok/config.toml`)

```toml
[mcp]
enabled = true

[mcp.servers.github]
command = "github_mcp"           # or full path to the binary
args = []
env = {}

# Optional: another server
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "."]
env = {}
```

### Using the GitHub Server

The server exposes tools such as:
- `search_repos`
- `get_repo`
- `list_issues`
- `create_issue`
- etc. (see `src/bin/github_mcp.rs` for current list)

## Testing the Stateless Path

A minimal echo server is provided for testing:

```bash
cargo build --bin mcp_test_echo
```

Used in integration tests (`stateless_mcp_echo_works_with_real_process`).

You can also test manually:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}' | target/debug/mcp_test_echo
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | target/debug/mcp_test_echo
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"message":"hello"}}}' | target/debug/mcp_test_echo
```

## Usage from the Agent

The model can call MCP tools directly when appropriate:

```
Use the github MCP server to find repositories about "rust cli tools".
```

The arbitration system and skills catalog know about MCP capabilities.

## Security

- All shell commands for MCP servers are validated against the `SecurityPolicy` denylist.
- Dangerous patterns (pipes to shell, reverse shells, etc.) are rejected.
- Connected servers from trusted ACP clients are still subject to tool-level safety hooks.

## Resilience Features

- Hard 30s timeout per MCP call
- Exponential backoff retries (Starlink satellite friendly)
- Proper error surfacing instead of hanging

## Configuration

MCP settings live under the `[mcp]` section in:
- Project: `.grok/config.toml`
- System: `~/.grok-cli/config.toml`

See `config.example.toml` for full template.

## Related Documentation

- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) — Command overview
- [SECURITY.md](SECURITY.md) — Security model
- [HOOKS_AND_EXTENSIONS.md](HOOKS_AND_EXTENSIONS.md)
- [ai-generated-summaries/MCP_QUICKSTART.md](ai-generated-summaries/MCP_QUICKSTART.md) (older reference)

## Development

To add a new MCP server binary:

1. Create `src/bin/my_mcp_server.rs`
2. Add to `Cargo.toml`:
   ```toml
   [[bin]]
   name = "my_mcp_server"
   path = "src/bin/my_mcp_server.rs"
   ```
3. Implement the JSON-RPC loop (see `mcp_test_echo.rs` for a minimal example)
4. Document the tools in its `tools/list` response

---

**MCP is now a first-class, production-ready integration layer in Grok CLI.**
