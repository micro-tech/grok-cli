#!/bin/bash
# Test script for ACP stdio communication

echo "Testing ACP stdio mode..."
echo ""

# Set environment to show debug logs
export RUST_LOG=grok_cli=info

# Create a temporary file for the ACP conversation
TEMP_INPUT=$(mktemp)
TEMP_OUTPUT=$(mktemp)

# Cleanup on exit
trap "rm -f $TEMP_INPUT $TEMP_OUTPUT" EXIT

# Step 1: Initialize
echo "Step 1: Sending initialize request..."
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1","clientInfo":{"name":"test-client","version":"1.0.0"}}}' | ./target/release/grok.exe acp stdio > $TEMP_OUTPUT 2>&1 &
GROK_PID=$!
sleep 2

# Get the session ID from the response
SESSION_ID=$(grep -o '"sessionId":"[^"]*"' $TEMP_OUTPUT | head -1 | cut -d'"' -f4)

if [ -z "$SESSION_ID" ]; then
    echo "Failed to get session ID, creating
 full test..."

    # Write all ACP protocol messages in one go
    cat > $TEMP_INPUT <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1","clientInfo":{"name":"test-client","version":"1.0.0"}}}
{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}
EOF

    echo ""
    echo "=== Sending initialization messages ==="
    cat $TEMP_INPUT
    echo ""

    # Get session ID from session/new
    cat $TEMP_INPUT | ./target/release/grok.exe acp stdio 2>&1 | tee $TEMP_OUTPUT &
    GROK_PID=$!
    sleep 3

    # Extract session ID from output
    SESSION_ID=$(grep -o '"sessionId":"[^"]*"' $TEMP_OUTPUT | head -1 | cut -d'"' -f4)

    if [ -z "$SESSION_ID" ]; then
        echo "❌ Could not extract session ID from response"
        echo ""
        echo "=== Full Output ==="
        cat $TEMP_OUTPUT
        exit 1
    fi

    echo "✓ Got session ID: $SESSION_ID"
fi

# Now create a complete test with the actual session ID
cat > $TEMP_INPUT <<EOF
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1","clientInfo":{"name":"test-client","version":"1.0.0"}}}
{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}
{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"$SESSION_ID","prompt":[{"type":"text","text":"Hello, please respond with exactly: Hello World!"}]}}
EOF

echo ""
echo "=== Running complete ACP test ==="
echo "Using session ID: $SESSION_ID"
echo ""
echo "=== Input messages ==="
cat $TEMP_INPUT
echo ""
echo "=== Response from grok ==="
echo ""

# Run grok in ACP stdio mode with timeout
timeout 15s bash -c "cat $TEMP_INPUT | ./target/release/grok.exe acp stdio 2>&1" | tee $TEMP_OUTPUT

echo ""
echo "=== Analysis ==="
echo ""

# Check for initialize response
if grep -q '"id":1' $TEMP_OUTPUT && grep -q '"result"' $TEMP_OUTPUT; then
    echo "✓ Initialize response received"
else
    echo "❌ No initialize response found"
fi

# Check for session/new response
if grep -q '"id":2' $TEMP_OUTPUT && grep -q '"sessionId"' $TEMP_OUTPUT; then
    echo "✓ Session created successfully"
else
    echo "❌ No session creation response found"
fi

# Check for session/update notifications
if grep -q 'session/update' $TEMP_OUTPUT; then
    echo "✓ session/update notification found"

    # Count how many chunks
    CHUNK_COUNT=$(grep -c 'session/update' $TEMP_OUTPUT)
    echo "  Found $CHUNK_COUNT update notification(s)"
else
    echo "❌ No session/update notifications found"
fi

# Check for agent_message_chunk
if grep -q 'agent_message_chunk' $TEMP_OUTPUT; then
    echo "✓ agent_message_chunk found"
else
    echo "❌ No agent_message_chunk found"
fi

# Check for content in the update
if grep -q '"text"' $TEMP_OUTPUT && grep -q 'Hello' $TEMP_OUTPUT; then
    echo "✓ Text content found in response"

    # Extract and display the content
    echo ""
    echo "=== Extracted Content ==="
    grep -o '"text":"[^"]*"' $TEMP_OUTPUT | cut -d'"' -f4
else
    echo "❌ No text content found in response"
fi

# Check for stopReason
if grep -q '"stopReason"' $TEMP_OUTPUT; then
    echo "✓ stopReason found in final response"
else
    echo "❌ No stopReason found"
fi

# Check for errors
if grep -q '"error"' $TEMP_OUTPUT; then
    echo ""
    echo "⚠️  Errors detected:"
    grep '"error"' $TEMP_OUTPUT
fi

echo ""
echo "=== Test Complete ==="
