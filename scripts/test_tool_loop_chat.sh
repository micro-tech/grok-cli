#!/bin/bash
# Test Tool Loop with Chat Command
# This script tests tool calling with the regular chat command to verify
# that tool results are properly handled and loops are prevented.

set -e  # Exit on error

echo "=== Grok CLI - Tool Loop Test (Chat Command) ==="
echo ""

# Check for API key
if [ -z "$GROK_API_KEY" ]; then
    echo "❌ Error: GROK_API_KEY environment variable not set"
    echo "Please set your API key: export GROK_API_KEY='your-key-here'"
    exit 1
fi

# Set a reasonable limit for testing
export GROK_ACP_MAX_TOOL_LOOP_ITERATIONS=5

# Create test files
TEST_DIR="./test_tool_loop"
mkdir -p "$TEST_DIR"

echo "Creating test files..."
echo "This is test file 1." > "$TEST_DIR/file1.txt"
echo "This is test file 2." > "$TEST_DIR/file2.txt"
echo "This is test file 3." > "$TEST_DIR/file3.txt"

echo "Test files created in $TEST_DIR/"
echo ""

# Build the project first
echo "=== Building Project ==="
cargo build --bin grok --release
if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi
echo "✅ Build successful"
echo ""

# Test 1: Simple tool usage (should complete quickly)
echo "=== Test 1: Simple File Read (Expected: 1-2 API calls) ==="
echo ""

START_TIME=$(date +%s)

# Run with a specific prompt that should trigger tool use
echo "Read the file $TEST_DIR/file1.txt and tell me what it says in one sentence." | \
    timeout 60 ./target/release/grok chat \
    --temperature 0.3 \
    --max-tokens 150 \
    --model grok-2-latest 2>&1 | tee test_output.log

EXIT_CODE=${PIPESTATUS[0]}
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
echo "=== Test 1 Results ==="
echo "Duration: ${DURATION}s"
echo "Exit code: $EXIT_CODE"

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Test 1 PASSED"
elif [ $EXIT_CODE -eq 124 ]; then
    echo "❌ Test 1 TIMEOUT (60s) - Possible infinite loop!"
    echo "⚠️  Check if tool results are being sent back to the model"
else
    echo "⚠️  Test 1 exited with code $EXIT_CODE"
fi

echo ""
echo "=== Response Analysis ==="
if [ -f "test_output.log" ]; then
    echo "Checking for tool execution indicators..."

    if grep -q "executing" test_output.log; then
        echo "✅ Tool execution detected"
    else
        echo "ℹ️  No tool execution detected (may have used cached knowledge)"
    fi

    if grep -q "error\|Error\|ERROR" test_output.log; then
        echo "⚠️  Errors detected in output:"
        grep -i "error" test_output.log
    fi
else
    echo "⚠️  No output log found"
fi

# Test 2: Multiple file operation (should still be quick)
echo ""
echo "=== Test 2: Multiple Files (Expected: 2-4 API calls) ==="
echo ""

START_TIME=$(date +%s)

echo "List all .txt files in $TEST_DIR and tell me how many there are." | \
    timeout 60 ./target/release/grok chat \
    --temperature 0.3 \
    --max-tokens 200 \
    --model grok-2-latest 2>&1 | tee test_output2.log

EXIT_CODE=${PIPESTATUS[0]}
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
echo "=== Test 2 Results ==="
echo "Duration: ${DURATION}s"
echo "Exit code: $EXIT_CODE"

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Test 2 PASSED"
elif [ $EXIT_CODE -eq 124 ]; then
    echo "❌ Test 2 TIMEOUT (60s) - Possible infinite loop!"
else
    echo "⚠️  Test 2 exited with code $EXIT_CODE"
fi

# Cleanup
echo ""
echo "=== Cleanup ==="
rm -rf "$TEST_DIR"
rm -f test_output.log test_output2.log
echo "Removed test files and logs"

echo ""
echo "=== Summary ==="
echo "If both tests completed within reasonable time (<30s each),"
echo "then tool message handling is working correctly!"
echo ""
echo "If tests timed out, check:"
echo "  1. Tool results are using ChatMessage::tool() in src/grok_client_ext.rs"
echo "  2. Messages have proper 'role: tool' and 'tool_call_id' fields"
echo "  3. No infinite loop in tool execution logic"
echo ""
echo "=== Detailed Testing ==="
echo "For detailed ACP testing, use: cargo test --test tool_loop_integration -- --ignored"
echo "(Requires GROK_API_KEY to be set)"
echo ""
