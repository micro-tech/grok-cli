# Chat Logging in Grok CLI

## Overview

Grok CLI provides robust chat logging functionality to save and manage your conversations with the AI. This feature allows you to review past interactions, search for specific content, and replay sessions for analysis or documentation purposes.

## Enabling Chat Logging

Chat logging is enabled by default in Grok CLI. Logs are saved to `~/.grok/logs/chat_sessions/` in both JSON and human-readable text formats. You can configure logging behavior using environment variables or configuration files.

### Configuration Options

- **Enable/Disable Logging**: Set `GROK_CHAT_LOGGING_ENABLED=true` to enable logging or `false` to disable it.
- **Log Directory**: Customize the log directory with `GROK_CHAT_LOG_DIR=/path/to/logs`.
- **Max Log Size**: Set the maximum size before rotation with `GROK_CHAT_LOG_MAX_SIZE_MB=10`.
- **Rotation Count**: Define how many rotated logs to keep with `GROK_CHAT_LOG_ROTATION_COUNT=5`.
- **Include System Messages**: Include system prompts in logs with `GROK_CHAT_LOG_INCLUDE_SYSTEM=true`.

## Managing Chat History

Grok CLI provides commands to manage your chat history:

- **List Sessions**: Use `grok history list` to see all saved chat sessions.
- **View Session**: Use `grok history view <session-id>` to view a specific session.
- **Search History**: Use `grok history search "query"` to search through all conversations.
- **Clear History**: Use `grok history clear --confirm` to delete all chat history.

## Log Formats

- **JSON Format**: Saved as `<session-id>.json`, this format includes full metadata and is machine-readable.
- **Text Format**: Saved as `<session-id>.txt`, this format provides a human-readable transcript of the conversation.

## Use Cases

- **Documentation**: Save important AI responses for project documentation.
- **Analysis**: Review past interactions to improve prompt crafting or understand AI behavior.
- **Sharing**: Export conversations to share with team members or for troubleshooting.

For more information on configuration, see the [CONFIGURATION.md](../CONFIGURATION.md) guide.