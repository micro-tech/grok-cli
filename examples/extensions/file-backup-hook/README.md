# File Backup Hook Extension

## Overview

The **File Backup Hook** automatically creates backups of files before the AI modifies them. This provides a safety net, allowing you to easily restore files if the AI makes unwanted changes.

## Features

- ✅ **Automatic Backups** - Backs up files before `write_file`, `edit_file`, and `create_file` operations
- ✅ **Timestamped Backups** - Each backup includes a timestamp for easy identification
- ✅ **Automatic Cleanup** - Keeps only the N most recent backups per file (configurable)
- ✅ **Backup Manifest** - Maintains a log of all backups for easy tracking
- ✅ **Non-Intrusive** - Runs silently in the background, doesn't interrupt workflow
- ✅ **Cross-Platform** - Works on Windows (Git Bash/WSL), macOS, and Linux

## Installation

### Step 1: Copy Extension

```bash
# Copy the extension to your Grok extensions directory
cp -r examples/extensions/file-backup-hook ~/.grok/extensions/
```

### Step 2: Enable Extensions

Edit your Grok CLI config file:

**Windows**: `%APPDATA%\grok-cli\config.toml`  
**macOS/Linux**: `~/.config/grok-cli/config.toml`

Add or update the extensions section:

```toml
[experimental.extensions]
enabled = true
extension_dir = "~/.grok/extensions"
enabled_extensions = []  # Empty list = all extensions enabled
allow_config_extensions = true
```

### Step 3: Make Script Executable (Unix-like systems)

```bash
chmod +x ~/.grok/extensions/file-backup-hook/scripts/backup.sh
```

### Step 4: Restart Grok CLI

```bash
grok interactive
```

You should see log messages indicating the extension is loaded.

## Configuration

Edit `extension.json` to customize behavior:

```json
{
  "config": {
    "enabled": true,
    "backup_dir": "~/.grok/backups",
    "tools_to_backup": ["write_file", "edit_file", "create_file"],
    "max_backups_per_file": 10,
    "compress_old_backups": false,
    "verbose_logging": true
  }
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | Boolean | `true` | Enable/disable the backup hook |
| `backup_dir` | String | `~/.grok/backups` | Directory where backups are stored |
| `tools_to_backup` | Array | `["write_file", "edit_file", "create_file"]` | Which tools trigger backups |
| `max_backups_per_file` | Number | `10` | Maximum backups to keep per file |
| `compress_old_backups` | Boolean | `false` | Compress old backups (future feature) |
| `verbose_logging` | Boolean | `true` | Show backup messages in logs |

## Usage

Once installed, the extension works automatically. Every time the AI modifies a file, a backup is created.

### Example Session

```bash
grok interactive

> Please update src/main.rs to add error handling

[Backup Hook] ✓ Backed up: src/main.rs → main.rs.20250219_143022.bak
[AI proceeds to modify the file]
```

### Viewing Backups

```bash
# List all backups
ls -lah ~/.grok/backups/

# Output:
# main.rs.20250219_143022.bak
# main.rs.20250219_143545.bak
# config.toml.20250219_144001.bak
```

### Restoring from Backup

```bash
# Find the backup you want to restore
ls ~/.grok/backups/ | grep main.rs

# Restore the backup
cp ~/.grok/backups/main.rs.20250219_143022.bak src/main.rs

# Or compare first
diff src/main.rs ~/.grok/backups/main.rs.20250219_143022.bak
```

### Viewing Backup Manifest

The extension maintains a log of all backups:

```bash
# View the manifest
cat ~/.grok/backups/backup_manifest.log

# Output format: timestamp|tool_name|original_path|backup_name
# 2025-02-19T14:30:22-05:00|write_file|/home/user/project/src/main.rs|main.rs.20250219_143022.bak
# 2025-02-19T14:35:45-05:00|edit_file|/home/user/project/src/main.rs|main.rs.20250219_143545.bak
```

## How It Works

1. **Hook Triggers**: When AI calls `write_file`, `edit_file`, or `create_file`
2. **Before Execution**: The `before_tool` hook runs
3. **File Check**: Script checks if the target file exists
4. **Backup Creation**: If exists, creates a timestamped copy in backup directory
5. **Cleanup**: Removes old backups if exceeding `max_backups_per_file`
6. **Logging**: Records backup in manifest file
7. **Continues**: Tool execution proceeds normally

## Advanced Usage

### Custom Backup Directory

Set environment variable:

```bash
export GROK_BACKUP_DIR="/path/to/custom/backups"
grok interactive
```

### Disable Verbose Logging

```bash
export GROK_VERBOSE_LOGGING=false
grok interactive
```

### Backup Rotation Script

Create a cron job to archive old backups:

```bash
# Archive backups older than 30 days
find ~/.grok/backups -name "*.bak" -mtime +30 -exec gzip {} \;

# Or move to archive directory
find ~/.grok/backups -name "*.bak" -mtime +30 -exec mv {} ~/.grok/archives/ \;
```

### Search Backups by Date

```bash
# Find backups from today
find ~/.grok/backups -name "*.$(date +%Y%m%d)*.bak"

# Find backups from specific file
find ~/.grok/backups -name "main.rs.*.bak"
```

### Restore Helper Script

Create a helper script `restore-backup.sh`:

```bash
#!/bin/bash
# Usage: ./restore-backup.sh main.rs

FILE_PREFIX="$1"
BACKUP_DIR="$HOME/.grok/backups"

echo "Available backups for $FILE_PREFIX:"
ls -1t "$BACKUP_DIR/${FILE_PREFIX}."*.bak | nl

read -p "Select backup number to restore (or 0 to cancel): " NUM

if [ "$NUM" -eq 0 ]; then
    echo "Cancelled"
    exit 0
fi

BACKUP=$(ls -1t "$BACKUP_DIR/${FILE_PREFIX}."*.bak | sed -n "${NUM}p")

if [ -z "$BACKUP" ]; then
    echo "Invalid selection"
    exit 1
fi

read -p "Restore $BACKUP? (y/n): " CONFIRM

if [ "$CONFIRM" = "y" ]; then
    cp "$BACKUP" "$FILE_PREFIX"
    echo "✓ Restored: $FILE_PREFIX"
fi
```

## Troubleshooting

### Extension Not Loading

**Check logs:**
```bash
tail -f ~/.grok/logs/grok.log | grep -i extension
```

**Verify extension directory:**
```bash
ls -la ~/.grok/extensions/file-backup-hook/
```

**Validate JSON:**
```bash
jq . ~/.grok/extensions/file-backup-hook/extension.json
```

### Script Not Executing

**Check permissions:**
```bash
ls -l ~/.grok/extensions/file-backup-hook/scripts/backup.sh
# Should show: -rwxr-xr-x (executable)
```

**Test script manually:**
```bash
cd ~/.grok/extensions/file-backup-hook
./scripts/backup.sh "write_file" '{"path":"test.txt"}' "before"
```

**Check for jq:**
```bash
which jq
# If not found: sudo apt install jq  (Linux)
# Or: brew install jq  (macOS)
```

### Backups Not Created

**Enable verbose logging:**
```bash
export GROK_VERBOSE_LOGGING=true
grok interactive
```

**Check backup directory permissions:**
```bash
ls -ld ~/.grok/backups/
mkdir -p ~/.grok/backups/
```

**Verify file exists before modification:**
The hook only backs up *existing* files. New files won't have backups.

## Security Considerations

- ✅ Backups are stored locally, not sent anywhere
- ✅ No network access required
- ✅ Script uses `set -e` for fail-fast behavior
- ✅ File paths are validated before operations
- ⚠️ Backups are **not encrypted** - consider encrypting backup directory if needed
- ⚠️ Backup directory grows over time - monitor disk space

## Performance Impact

- **Minimal** - File copy operation is fast for most files
- **Async** - Doesn't block AI operations
- **Local** - No network latency
- **Typical overhead**: <100ms per file

For very large files (>100MB), you may notice slight delays.

## Limitations

- Only backs up files that already exist (not newly created files)
- Requires Unix-like shell (Bash) - Windows users need Git Bash or WSL
- Requires `jq` for JSON parsing
- Backups are local only (not synced to cloud)
- No versioning system integration (doesn't use Git)

## Future Enhancements

- [ ] Compress old backups automatically
- [ ] Cloud sync support (Dropbox, S3, etc.)
- [ ] Git integration (auto-commit backups)
- [ ] Web UI for browsing/restoring backups
- [ ] Differential backups (only store changes)
- [ ] Email notifications on backups
- [ ] Windows native PowerShell script
- [ ] Python version for better cross-platform support

## Related Extensions

- **git-auto-commit** - Automatically commit AI changes to Git
- **logging-hook** - Log all AI operations
- **security-validator** - Validate operations before execution

## Contributing

Found a bug or have a feature request? 

1. Check existing issues
2. Create a new issue with details
3. Submit a pull request

## License

MIT License - See [LICENSE](../../../LICENSE) file

## Author

john mcconnell <john.microtech@gmail.com>

## Changelog

### v1.0.0 (2025-02-19)
- Initial release
- Basic backup functionality
- Automatic cleanup
- Backup manifest logging
- Cross-platform support (with Bash)

---

**Questions or issues?** Open an issue on GitHub or contact the author.