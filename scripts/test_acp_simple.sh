#!/bin/bash
# Simple ACP test - sends all messages in one stream

echo "=== Testing ACP stdio mode ==="
echo ""

# Set log level
export RUST_LOG=grok_cli=info

# Create a test script that will feed all messages at once
# and keep the connection open
{
    # 1. Initialize
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1","clientInfo":{"name":"test-client","version":"1.0.0"}}}'

    # 2. Create new session
    echo '{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}'

    # Small delay to let it process
    sleep 1

    # 3. Send prompt - NOTE: We need to extract the session ID from step 2 response
    # For this simple test, we'll use a placeholder and manually check the output
    # In real usage, Zed would parse the session ID from the session/new response

    # We need to get the session ID dynamically, so let's use a different approach
} | ./target/release/grok.exe acp stdio 2>&1 | tee /tmp/acp_test_output.txt &

GROK_PID=$!
sleep 3

# Extract session ID from output
SESSION_ID=$(grep -o '"sessionId":"[^"]*"' /tmp/acp_test_output.txt | head -1 | cut -d'"' -f4)

if [ -z "$SESSION_ID" ]; then
    echo "❌ Could not get session ID from initial setup"
    cat /tmp/acp_test_output.txt
    kill $GROK_PID 2>/dev/null
    exit 1
fi

echo "✓ Got session ID: $SESSION_ID"
echo ""

# Now run the full test with actual session ID
echo "=== Running full ACP conversation ==="
echo ""

(
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1","clientInfo":{"name":"test-client","version":"1.0.0"}}}'
    echo '{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}'

    # Wait a moment for session creation
    sleep 2

    # Send the prompt with the correct session ID
    # We'll extract it from the previous response
    cat <<EOF | while read line; do
        if echo "$line" | grep -q "sessionId"; then
            SESS_ID=\$(echo "\$line" | grep -o '"sessionId":"[^"]*"' | cut -d'"' -f4)
            echo "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"session/prompt\",\"params\":{\"sessionId\":\"\$SESS_ID\",\"prompt\":[{\"type\":\"text\",\"text\":\"Say exactly: Hello World!\"}]}}"
        fi
        echo "\$line" >&2
    done
EOF

    # Keep connection alive for response
    sleep 10
) | ./target/release/grok.exe acp stdio 2>&1

echo ""
echo "=== Test Complete ==="
