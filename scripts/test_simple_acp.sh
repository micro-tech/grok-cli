#!/bin/bash
# Simple ACP test

echo "Starting ACP server with debug logging..."

export RUST_LOG=debug,grok_cli::acp=trace

# Start the server
./target/release/grok.exe acp --stdio &
SERVER_PID=$!

echo "Server PID: $SERVER_PID"
sleep 2

# Send a simple JSON-RPC message
echo '{"jsonrpc":"2.0","id":1,"method":"agent/initialize","params":{}}' 

# Wait a bit
sleep 2

# Kill server
kill $SERVER_PID 2>/dev/null

echo "Test complete"
