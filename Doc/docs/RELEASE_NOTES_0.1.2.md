# Grok CLI v0.1.2 Release Notes

**Release Date**: 2026-01-13  
**Author**: john mcconnell (john.microtech@gmail.com)  
**Repository**: https://github.com/microtech/grok-cli

## 🎉 What's New

### 📜 Chat Session Logging (Major Feature)

The headline feature of v0.1.2 is **comprehensive chat session logging**! Every conversation with Grok is now automatically saved and searchable.

#### Key Features:
- ✅ **Automatic Logging**: All ACP (Zed) conversations saved automatically
- ✅ **Dual Format Output**: JSON (machine-readable) + TXT (human-readable)
- ✅ **Full History Management**: List, view, search, and clear past sessions
- ✅ **Session Tracking**: Each chat gets a unique ID with timestamps
- ✅ **Smart Search**: Find conversations by content with highlighting
- ✅ **Auto-Rotation**: Configurable log size limits with automatic cleanup

#### New Commands

```bash
grok history list              # Browse all your chat sessions
grok history view <session-id> # View complete conversation
grok history search "query"    # Search through all chats
grok history clear --confirm   # Clear chat history
```

#### Configuration

```bash
# Environment variables for chat logging
GROK_CHAT_LOGGING_ENABLED=true
GROK_CHAT_LOG_DIR=~/.grok/logs/chat_sessions
GROK_CHAT_LOG_MAX_SIZE_MB=10
GROK_CHAT_LOG_ROTATION_COUNT=5
GROK_CHAT_LOG_INCLUDE_SYSTEM=true
```

### Key Features

- **Automatic Logging**: All ACP/Zed conversations saved automatically
- **Dual Format**: JSON (machine-readable) + TXT (human-readable) 
- **Full-Text Search**: Search through all your conversations
- **Beautiful Output**: Color-coded, formatted terminal display
- **Privacy First**: 100% local storage, no cloud uploads
- **Network Resilient**: Handles Starlink drops gracefully

### Example Usage

```bash
# List all your chat sessions
grok history list

# View a specific conversation
grok history view <session-id>

# Search through all your chats
grok history search "authentication"

# Clear old logs
grok history clear --confirm
```

## 🎯 Quick Start

### Using Chat Logging

Chat logging is **enabled by default** and works automatically when you use Grok through Zed editor (ACP mode).

1. **Use Grok in Zed** or run `grok acp stdio`
2. **Have conversations** - they're automatically saved!
3. **Review history**:
   ```bash
   grok history list
   grok history view <session-id>
   grok history search "your query"
   ```

### Configuration (Optional)

Add to your `.env` file:
```bash
# Enable/disable logging (default: true)
GROK_CHAT_LOGGING_ENABLED=true

# Custom log directory
GROK_CHAT_LOG_DIR=~/.grok/logs/chat_sessions

# Rotation settings
GROK_CHAT_LOG_MAX_SIZE_MB=10
GROK_CHAT_LOG_ROTATION_COUNT=5
```

## 📦 What's Included

### Chat Logging System
- **564 lines** of robust logging code
- **421 lines** of history management commands
- **415+ lines** of comprehensive documentation
- **Zero new dependencies** - uses existing crates

### Commands Added
- `grok history list` - Browse all saved sessions
- `grok history view <session-id>` - View full conversation
- `grok history search "query"` - Search through chats
- `grok history clear --confirm` - Delete all logs

### Configuration Added
- `GROK_CHAT_LOGGING_ENABLED` - Enable/disable logging
- `GROK_CHAT_LOG_DIR` - Custom log directory
- `GROK_CHAT_LOG_MAX_SIZE_MB` - Max file size
- `GROK_CHAT_LOG_ROTATION_COUNT` - Number of files to keep
- `GROK_CHAT_LOG_INCLUDE_SYSTEM` - Include system messages

### Files Modified
- `Cargo.toml` - Version bump to 0.1.2
- `src/main.rs` - Added chat logger initialization
- `src/utils/mod.rs` - Added chat_logger module
- `src/cli/commands/mod.rs` - Added history module
- `src/cli/commands/acp.rs` - Integrated logging into ACP
- `src/cli/app.rs` - Added History command
- `src/lib.rs` - Added HistoryAction enum

### New Files
- `src/utils/chat_logger.rs` (564 lines) - Core logging implementation
- `src/cli/commands/history.rs` (421 lines) - History commands
- `docs/CHAT_LOGGING.md` (415 lines) - Complete documentation
- `docs/CHAT_LOGGING_IMPLEMENTATION.md` (451 lines) - Technical details
- `docs/CHAT_LOGGING_SUMMARY.md` (292 lines) - Quick reference

## 🎉 Highlights

### 🆕 Chat Session Logging
Every conversation you have with Grok through Zed editor (ACP mode) is now automatically saved! Review, search, and analyze your chat history anytime.

### 🔍 Powerful Search
Find that helpful explanation from last week with full-text search across all your conversations.

### 📊 Beautiful Output
Color-coded, well-formatted terminal output makes browsing history a pleasure.

### 🔒 Privacy First
All logs stored locally on your machine. Nothing sent to the cloud. You have complete control.

## 🆕 New Commands

```bash
# View all your chat sessions
grok history list

# Read a full conversation
grok history view <session-id>

# Search through all your chats
grok history search "authentication"

# Clear all history
grok history clear --confirm
```

## 📁 Where Logs Are Saved

**Default Locations:**
- **Windows**: `C:\Users\<username>\.grok\logs\chat_sessions\`
- **Linux/macOS**: `~/.grok/logs/chat_sessions/`

Each session creates two files:
- `<session-id>.json` - Machine-readable with full metadata
- `<session-id>.txt` - Beautiful human-readable transcript

## 🎨 Example Usage

### View Your Chat History
```bash
# List all sessions
grok history list

# Output:
# ================================================================================
#   CHAT SESSIONS (3 total)
# ================================================================================
# 
#   1. 550e8400-e29b-41d4-a716-446655440000 Completed
#      Started: 2026-01-13 10:30:00 UTC | 12 messages
#      Duration: 930 seconds
#      Preview: How do I implement authentication in Rust?
```

### View a Conversation
```bash
grok history view 550e8400-e29b-41d4-a716-446655440000
```

Shows the complete conversation with beautiful formatting:
```
================================================================================
GROK CLI CHAT SESSION LOG
================================================================================
Session ID: 550e8400-e29b-41d4-a716-446655440000
Start Time: 2026-01-13 10:30:00 UTC
End Time:   2026-01-13 10:45:30 UTC
Duration:   930 seconds
Messages:   12
================================================================================

[1] USER (10:30:05)
--------------------------------------------------------------------------------
How do I implement authentication in Rust?

[2] ASSISTANT (10:30:15)
--------------------------------------------------------------------------------
Here's a comprehensive guide to implementing authentication in Rust...
```

Perfect! **Version 0.1.2 is now released!** 🎉

## Summary of Version Update

✅ **Updated files:**
- `Cargo.toml` - Version bumped from 0.1.1 → 0.1.2
- `CHANGELOG.md` - Updated release date to 2026-01-13
- Created `RELEASE_NOTES_0.1.2.md` - Comprehensive release notes

✅ **Version verified:**
```
grok-cli 0.1.2
```

### 🎉 Version 0.1.2 is now officially released!

**Key Features:**
- ✅ Chat session logging with automatic saving
- ✅ History management commands (list, view, search, clear)
- ✅ Dual format output (JSON + TXT)
- ✅ Network-resilient for Starlink users
- ✅ Comprehensive documentation

**Ready for distribution!** 🚀