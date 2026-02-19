#!/bin/bash
# Intercept Setup Script
# Detects when AI wants to create a project and triggers the setup script

set -e

TOOL_NAME="${1:-}"
TOOL_ARGS="${2:-}"
HOOK_TYPE="${3:-}"

# Only intercept before tool execution
if [ "$HOOK_TYPE" != "before" ]; then
    exit 0
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETUP_SCRIPT="$SCRIPT_DIR/setup_project.sh"

# Function to extract project name from various tool arguments
extract_project_name() {
    local args="$1"

    # Try to extract from common field names
    PROJECT_NAME=$(echo "$args" | jq -r '.project_name // .name // .project // empty' 2>/dev/null)

    # If not found in JSON, try to extract from path or content
    if [ -z "$PROJECT_NAME" ] || [ "$PROJECT_NAME" = "null" ]; then
        # Check if there's a path that looks like a project name
        PROJECT_NAME=$(echo "$args" | jq -r '.path // .directory // empty' 2>/dev/null | grep -oE '[a-z][a-z0-9_-]+$' || echo "")
    fi

    echo "$PROJECT_NAME"
}

# Function to check if content contains project setup keywords
check_for_setup_intent() {
    local args="$1"

    # Extract content/command/message from args
    CONTENT=$(echo "$args" | jq -r '.content // .command // .message // .text // empty' 2>/dev/null)

    if [ -z "$CONTENT" ]; then
        return 1
    fi

    # Check for setup keywords
    if echo "$CONTENT" | grep -qiE '(create|setup|init|new|make).*(rust|cargo)?.*(project|workspace)'; then
        return 0
    fi

    # Check for explicit project setup patterns
    if echo "$CONTENT" | grep -qiE 'rust.*project|cargo.*new|setup.*project|init.*project'; then
        return 0
    fi

    return 1
}

# Only intercept specific tools that might indicate project creation
RELEVANT_TOOLS=("run_shell_command" "create_directory" "write_file")
IS_RELEVANT=false

for tool in "${RELEVANT_TOOLS[@]}"; do
    if [ "$TOOL_NAME" = "$tool" ]; then
        IS_RELEVANT=true
        break
    fi
done

if [ "$IS_RELEVANT" = false ]; then
    exit 0
fi

# Check if this looks like a project setup request
if check_for_setup_intent "$TOOL_ARGS"; then
    PROJECT_NAME=$(extract_project_name "$TOOL_ARGS")

    if [ -n "$PROJECT_NAME" ] && [ "$PROJECT_NAME" != "null" ]; then
        echo "[Project Setup Hook] Detected project setup request for: $PROJECT_NAME" >&2
        echo "[Project Setup Hook] You can set this up properly with:" >&2
        echo "[Project Setup Hook]   $SETUP_SCRIPT $PROJECT_NAME" >&2
        echo "" >&2
        echo "[Project Setup Hook] Or ask me: 'Set up a Rust project called $PROJECT_NAME'" >&2
        echo "" >&2
    fi
fi

# Always allow the original tool to execute
exit 0
