#!/bin/bash
# Test Tool Loop Debug Script
# This script helps reproduce and debug tool loop issues in Grok CLI

echo "=== Grok CLI - Tool Loop Debug Test ==="
echo ""

# Set a low limit for testing
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=10

# Create a test file to read
TEST_FILE="test_loop_debug.txt"
echo "This is a simple test file." > "$TEST_FILE"
echo "It has a few lines." >> "$TEST_FILE"
echo "Reading this should take only 1-2 tool calls." >> "$TEST_FILE"

echo "Created test file: $TEST_FILE"
echo ""

# Clear previous debug log
if [ -f "acp_debug.log" ]; then
    echo "Backing up previous debug log..."
    mv acp_debug.log "acp_debug.log.backup.$(date +%s)"
fi

echo "=== Test 1: Simple File Read ==="
echo "This test should complete in 1-2 iterations."
echo ""
echo "Running: echo 'Read the file test_loop_debug.txt and tell me what it says. Then stop.' | cargo run --bin grok -- acp"
echo ""

# Run the test with a simple prompt
echo "Read the file test_loop_debug.txt and tell me what it says. Then stop." | cargo run --bin grok -- acp

EXIT_CODE=$?

echo ""
echo "=== Test Results ==="
echo "Exit code: $EXIT_CODE"

if [ $EXIT_CODE -ne 0 ]; then
    echo "❌ Test FAILED with exit code $EXIT_CODE"
else
    echo "✅ Test PASSED"
fi

echo ""
echo "=== Analyzing Debug Log ==="

if [ -f "acp_debug.log" ]; then
    echo ""
    echo "Tool loop iterations:"
    grep "Tool loop iteration" acp_debug.log | tail -20

    echo ""
    echo "Tool calls made:"
    grep "Tool [0-9]" acp_debug.log | tail -20

    echo ""
    echo "Finish reasons:"
    grep "Finish reason" acp_debug.log | tail -20

    echo ""
    echo "Errors (if any):"
    grep -i "error\|❌" acp_debug.log | tail -10

    echo ""
    echo "Full log saved to: acp_debug.log"
else
    echo "⚠️  No debug log found!"
fi

echo ""
echo "=== Cleanup ==="
if [ -f "$TEST_FILE" ]; then
    rm "$TEST_FILE"
    echo "Removed test file: $TEST_FILE"
fi

echo ""
echo "=== Next Steps ==="
echo "1. Review the debug log above"
echo "2. If the test used more than 3-4 iterations, there's a loop issue"
echo "3. Check what tool was called repeatedly"
echo "4. Run the PowerShell analyzer: ./analyze_tool_loops.ps1"
echo ""
