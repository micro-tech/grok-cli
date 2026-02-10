# Grok CLI Tools Documentation

## Overview

Grok CLI provides a comprehensive set of tools that enable the AI to interact with your local development environment. These tools are automatically available when using the ACP (Agent Client Protocol) server mode or when Grok needs to perform file operations during interactive sessions.

## Accessing Tools

### In Interactive Mode
Use the `/tools` command to list all available tools:
```
/tools
```

### In ACP Server Mode
Tools are automatically available when running:
```bash
grok acp stdio
```

## Available Tools

### File Operations

#### `read_file`
**Description:** Read the content of a file

**Signature:** `read_file(path: string)`

**Parameters:**
- `path` (string, required): The path to the file to read

**Example Usage:**
```
Grok will use: read_file("src/main.rs")
```

**Returns:** The file content as a string

**Security:** Respects security policy and trusted directories

---

#### `write_file`
**Description:** Write content to a file

**Signature:** `write_file(path: string, content: string)`

**Parameters:**
- `path` (string, required): The path to the file to write
- `content` (string, required): The content to write to the file

**Example Usage:**
```
Grok will use: write_file("src/utils/helper.rs", "pub fn example() { }")
```

**Returns:** Success message or error

**Security:** Respects security policy and trusted directories

---

#### `replace`
**Description:** Replace text in a file with surgical precision

**Signature:** `replace(path: string, old_string: string, new_string: string, expected_replacements?: integer)`

**Parameters:**
- `path` (string, required): The path to the file to modify
- `old_string` (string, required): The exact string to be replaced
- `new_string` (string, required): The new string to replace with
- `expected_replacements` (integer, optional): Expected number of replacements for validation

**Example Usage:**
```
Grok will use: replace("src/config.rs", "default_port = 8080", "default_port = 3000")
```

**Returns:** Success message with replacement count or error

**Security:** Respects security policy and trusted directories

**Notes:**
- Exact string matching - whitespace matters!
- Use `expected_replacements` to catch unexpected multiple replacements
- Safer than regex-based replacements for most use cases

---

### File Search & Discovery

#### `list_directory`
**Description:** List files and directories in a specified path

**Signature:** `list_directory(path: string)`

**Parameters:**
- `path` (string, required): The directory path to list

**Example Usage:**
```
Grok will use: list_directory("src/")
```

**Returns:** JSON array of files and directories with metadata

**Security:** Respects security policy and trusted directories

---

#### `glob_search`
**Description:** Find files matching a glob pattern

**Signature:** `glob_search(pattern: string)`

**Parameters:**
- `pattern` (string, required): The glob pattern to match (e.g., `**/*.rs`, `src/**/*.toml`)

**Example Usage:**
```
Grok will use: glob_search("**/*.rs")
```

**Returns:** List of matching file paths

**Security:** Respects security policy and trusted directories

**Pattern Examples:**
- `**/*.rs` - All Rust files recursively
- `src/**/*.toml` - All TOML files under src/
- `*.json` - JSON files in current directory
- `test_*.rs` - Test files starting with "test_"

---

#### `search_file_content`
**Description:** Search for text patterns in files using regular expressions

**Signature:** `search_file_content(path: string, pattern: string)`

**Parameters:**
- `path` (string, required): The file or directory to search in
- `pattern` (string, required): The regex pattern to search for

**Example Usage:**
```
Grok will use: search_file_content("src/", "fn main")
```

**Returns:** List of matches with file paths, line numbers, and matched text

**Security:** Respects security policy and trusted directories

**Pattern Examples:**
- `fn main` - Find main functions
- `TODO|FIXME` - Find TODO or FIXME comments
- `struct \w+` - Find struct definitions
- `pub async fn` - Find public async functions

---

### Execution & Web

#### `run_shell_command`
**Description:** Execute a shell command in the system

**Signature:** `run_shell_command(command: string)`

**Parameters:**
- `command` (string, required): The shell command to execute

**Example Usage:**
```
Grok will use: run_shell_command("cargo test")
```

**Returns:** Command output (stdout and stderr combined)

**Security:** 
- Respects security policy
- Use with caution - can execute arbitrary commands
- Best for build tools, tests, and development commands

**Common Use Cases:**
- `cargo build` - Build Rust projects
- `cargo test` - Run tests
- `git status` - Check git status
- `npm install` - Install Node packages
- `rustfmt src/**/*.rs` - Format code

---

#### `web_search`
**Description:** Search the web using DuckDuckGo

**Signature:** `web_search(query: string)`

**Parameters:**
- `query` (string, required): The search query

**Example Usage:**
```
Grok will use: web_search("Rust async programming best practices")
```

**Returns:** Search results with titles, URLs, and snippets

**Requirements:**
- None (DuckDuckGo does not require an API key)

---

#### `web_fetch`
**Description:** Fetch content from a URL

**Signature:** `web_fetch(url: string)`

**Parameters:**
- `url` (string, required): The URL to fetch

**Example Usage:**
```
Grok will use: web_fetch("https://docs.rs/tokio")
```

**Returns:** The content of the fetched page

**Notes:**
- Respects network timeouts
- Handles SSL/TLS connections
- Includes retry logic for Starlink/satellite connections

---

### Memory

#### `save_memory`
**Description:** Save a fact to long-term memory for future sessions

**Signature:** `save_memory(fact: string)`

**Parameters:**
- `fact` (string, required): The fact to remember

**Example Usage:**
```
Grok will use: save_memory("User prefers tabs over spaces for indentation")
```

**Returns:** Confirmation message

**Use Cases:**
- User preferences
- Project-specific conventions
- Important context for future sessions

---

## Tool Execution Flow

When Grok needs to use a tool, the following happens:

1. **Request Analysis** - Grok analyzes your request and determines which tools are needed
2. **Tool Call** - Grok makes a function call with the appropriate parameters
3. **Security Check** - The security manager validates the operation
4. **Execution** - The tool is executed with the provided parameters
5. **Result Integration** - The result is integrated into Grok's response
6. **Iteration** - If needed, Grok may call additional tools (up to 10 iterations)

## Security Model

### Trusted Directories
- By default, the current working directory is trusted
- Additional directories can be added via configuration
- File operations are restricted to trusted directories

### Security Policy
The security manager enforces:
- Path validation (no directory traversal attacks)
- File operation restrictions
- Command execution policies
- Rate limiting on network operations

### Configuration
Configure security settings in your config file:

```json
{
  "acp": {
    "enabled": true,
    "dev_mode": false,
    "trusted_directories": [
      "/path/to/your/project"
    ]
  }
}
```

## Best Practices

### For File Operations
1. **Read before write** - Always read a file before modifying it
2. **Use `replace` for precision** - Prefer `replace` over full file rewrites
3. **Validate paths** - Ensure paths are correct before operations
4. **Check results** - Always verify tool results before proceeding

### For Search Operations
1. **Start broad, then narrow** - Use `glob_search` to find files, then `search_file_content` to find specific content
2. **Test patterns** - Test regex patterns before using in production
3. **Use appropriate tools** - `glob_search` for file names, `search_file_content` for file content

### For Shell Commands
1. **Be specific** - Use exact commands, avoid complex shell scripts
2. **Check dependencies** - Ensure required tools are installed
3. **Handle errors** - Expect and handle command failures
4. **Use timeouts** - Long-running commands may timeout

### For Web Operations
1. **Handle failures** - Network operations can fail, especially on Starlink
2. **Cache results** - Avoid repeated fetches of the same data
3. **Respect rate limits** - Don't abuse web APIs

## Troubleshooting

### "Permission denied" errors
- Check that the file/directory is in a trusted directory
- Verify file permissions on your system
- Add the directory to `trusted_directories` in config

### "File not found" errors
- Verify the path is correct (use `list_directory` to check)
- Ensure you're using the correct path separator for your OS
- Check that the file exists using `glob_search`

### "Tool execution timeout"
- Increase timeout in configuration
- For long-running commands, consider background execution
- Check network connectivity for web operations

### "Max tool loop iterations reached"
- The tool execution loop has a limit of 10 iterations
- This prevents infinite loops
- Break complex tasks into smaller steps

## Examples

### Example 1: Refactor a function
```
You: "Can you refactor the parse_config function in src/config.rs to use serde?"

Grok will:
1. Use read_file("src/config.rs") to read the current implementation
2. Analyze the code
3. Use replace(...) to update the function
4. Use run_shell_command("cargo check") to verify it compiles
```

### Example 2: Find and fix TODOs
```
You: "Find all TODO comments in the project and show me what needs to be done"

Grok will:
1. Use glob_search("**/*.rs") to find all Rust files
2. Use search_file_content(".", "TODO|FIXME") to find all TODOs
3. Analyze and summarize the results
```

### Example 3: Add a new feature
```
You: "Add a new logger module to the project"

Grok will:
1. Use list_directory("src/") to understand the project structure
2. Use write_file("src/logger.rs") to create the new module
3. Use read_file("src/lib.rs") to read the lib file
4. Use replace(...) to add the module declaration
5. Use run_shell_command("cargo test") to verify everything works
```

## Integration with Zed Editor

When using Grok with Zed via ACP:

1. **Start the ACP server:**
   ```bash
   grok acp stdio
   ```

2. **In Zed, tools are automatically available** - No manual invocation needed

3. **Grok will use tools contextually** - Based on your requests

4. **Security is enforced** - Only trusted directories are accessible

## Configuration Reference

### Environment Variables
```env
# Required for API access
GROK_API_KEY=your_grok_api_key_here
```

### Config File (`~/.config/grok-cli/config.json`)
```json
{
  "api_key": "your_api_key",
  "default_model": "grok-3",
  "timeout_secs": 30,
  "max_retries": 3,
  "acp": {
    "enabled": true,
    "dev_mode": false,
    "default_port": null,
    "trusted_directories": [
      "/home/user/projects"
    ]
  }
}
```

## Tool Development

Want to add your own tools? See the implementation in:
- `src/acp/tools.rs` - Tool implementations
- `src/acp/mod.rs` - Tool registration and execution

Each tool needs:
1. **Implementation function** - The actual tool logic
2. **Tool definition** - JSON schema for parameters
3. **Registration** - Add to `get_tool_definitions()`
4. **Handler** - Add to the match statement in `handle_chat_completion()`

## Notes

- Tools are automatically invoked by Grok when needed
- Tool execution is logged for debugging
- All file operations are relative to the current working directory
- Network operations include retry logic for unreliable connections (Starlink support)
- Tool results are integrated into the conversation context

## Support

For issues or questions about tools:
1. Check the [GitHub Issues](https://github.com/microtech/grok-cli/issues)
2. Review the [security documentation](SECURITY.md)
3. Check tool logs in verbose mode: `grok -v acp stdio`
