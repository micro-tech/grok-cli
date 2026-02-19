#!/bin/bash
# File Backup Hook Script
# Automatically backs up files before AI modifies them

set -e

# Parse arguments
TOOL_NAME="${1:-}"
TOOL_ARGS="${2:-}"
HOOK_TYPE="${3:-}"

# Exit if not a before hook
if [ "$HOOK_TYPE" != "before" ]; then
    exit 0
fi

# Configuration (can be overridden by extension config)
BACKUP_DIR="${GROK_BACKUP_DIR:-${HOME}/.grok/backups}"
MAX_BACKUPS="${GROK_MAX_BACKUPS:-10}"
VERBOSE="${GROK_VERBOSE_LOGGING:-true}"

# Tools that should trigger backups
BACKUP_TOOLS=("write_file" "edit_file" "create_file")

# Check if this tool should trigger a backup
should_backup=false
for tool in "${BACKUP_TOOLS[@]}"; do
    if [ "$TOOL_NAME" = "$tool" ]; then
        should_backup=true
        break
    fi
done

if [ "$should_backup" = false ]; then
    exit 0
fi

# Extract file path from tool arguments
# Try different common field names
FILE_PATH=$(echo "$TOOL_ARGS" | jq -r '.path // .file_path // .filepath // empty' 2>/dev/null)

if [ -z "$FILE_PATH" ] || [ "$FILE_PATH" = "null" ]; then
    if [ "$VERBOSE" = "true" ]; then
        echo "[Backup Hook] No file path found in arguments" >&2
    fi
    exit 0
fi

# Resolve to absolute path
if [ "${FILE_PATH:0:1}" != "/" ] && [ "${FILE_PATH:1:1}" != ":" ]; then
    FILE_PATH="$(pwd)/$FILE_PATH"
fi

# Check if file exists (only backup existing files)
if [ ! -f "$FILE_PATH" ]; then
    if [ "$VERBOSE" = "true" ]; then
        echo "[Backup Hook] File doesn't exist yet, skipping backup: $FILE_PATH" >&2
    fi
    exit 0
fi

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

# Generate backup filename with timestamp
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BASENAME=$(basename "$FILE_PATH")
BACKUP_NAME="${BASENAME}.${TIMESTAMP}.bak"
BACKUP_PATH="${BACKUP_DIR}/${BACKUP_NAME}"

# Create backup
cp "$FILE_PATH" "$BACKUP_PATH"

if [ "$VERBOSE" = "true" ]; then
    echo "[Backup Hook] ✓ Backed up: $FILE_PATH → $BACKUP_NAME" >&2
fi

# Clean up old backups (keep only MAX_BACKUPS most recent)
# Find all backups for this file and remove oldest if exceeding limit
BACKUP_PATTERN="${BASENAME}.*.bak"
BACKUP_COUNT=$(find "$BACKUP_DIR" -name "$BACKUP_PATTERN" 2>/dev/null | wc -l)

if [ "$BACKUP_COUNT" -gt "$MAX_BACKUPS" ]; then
    # Remove oldest backups
    TO_REMOVE=$((BACKUP_COUNT - MAX_BACKUPS))
    find "$BACKUP_DIR" -name "$BACKUP_PATTERN" -type f -printf '%T+ %p\n' 2>/dev/null | \
        sort | \
        head -n "$TO_REMOVE" | \
        cut -d' ' -f2- | \
        xargs -r rm -f

    if [ "$VERBOSE" = "true" ]; then
        echo "[Backup Hook] Cleaned up $TO_REMOVE old backup(s)" >&2
    fi
fi

# Create a manifest file tracking all backups
MANIFEST_FILE="${BACKUP_DIR}/backup_manifest.log"
echo "$(date -Iseconds)|$TOOL_NAME|$FILE_PATH|$BACKUP_NAME" >> "$MANIFEST_FILE"

exit 0
