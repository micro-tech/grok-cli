#!/usr/bin/env python3
"""
Test script for ACP (Agent Client Protocol) stdio communication
This script properly handles the JSON-RPC protocol and session management
"""

import json
import subprocess
import sys
import time
from typing import Optional


def send_message(proc, message: dict) -> None:
    """Send a JSON-RPC message to the process"""
    msg_str = json.dumps(message)
    print(f">>> Sending: {msg_str}", file=sys.stderr)
    proc.stdin.write(msg_str + "\n")
    proc.stdin.flush()


def read_response(proc, timeout: float = 5.0) -> Optional[dict]:
    """Read a JSON-RPC response from the process"""
    proc.stdout.flush()
    line = proc.stdout.readline()
    if not line:
        return None

    line = line.strip()
    if not line:
        return None

    print(f"<<< Received: {line}", file=sys.stderr)

    try:
        return json.loads(line)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON: {e}", file=sys.stderr)
        print(f"Raw line: {line}", file=sys.stderr)
        return None


def main():
    print("=" * 60)
    print("Testing ACP Protocol via stdio")
    print("=" * 60)
    print()

    # Start grok in ACP stdio mode
    proc = subprocess.Popen(
        ["./target/release/grok.exe", "acp", "stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    try:
        # Give it a moment to start
        time.sleep(0.5)

        # Step 1: Initialize
        print("Step 1: Initialize")
        print("-" * 60)
        init_msg = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "1",
                "clientInfo": {"name": "test-client", "version": "1.0.0"},
            },
        }
        send_message(proc, init_msg)
        init_response = read_response(proc)

        if not init_response:
            print("❌ No initialize response")
            return 1

        if "error" in init_response:
            print(f"❌ Initialize error: {init_response['error']}")
            return 1

        print(f"✓ Initialize successful")
        print()

        # Step 2: Create new session
        print("Step 2: Create Session")
        print("-" * 60)
        new_session_msg = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "session/new",
            "params": {},
        }
        send_message(proc, new_session_msg)
        session_response = read_response(proc)

        if not session_response:
            print("❌ No session/new response")
            return 1

        if "error" in session_response:
            print(f"❌ Session creation error: {session_response['error']}")
            return 1

        session_id = session_response.get("result", {}).get("sessionId")
        if not session_id:
            print(f"❌ No session ID in response: {session_response}")
            return 1

        print(f"✓ Session created: {session_id}")
        print()

        # Step 3: Send prompt
        print("Step 3: Send Prompt")
        print("-" * 60)
        prompt_msg = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "session/prompt",
            "params": {
                "sessionId": session_id,
                "prompt": [
                    {
                        "type": "text",
                        "text": "Hello! Please respond with exactly: Hello World!",
                    }
                ],
            },
        }
        send_message(proc, prompt_msg)

        # Step 4: Read all responses (notifications + final response)
        print()
        print("Step 4: Reading Responses")
        print("-" * 60)

        notifications = []
        final_response = None
        timeout_counter = 0
        max_timeout = 30  # 30 seconds max

        while timeout_counter < max_timeout:
            response = read_response(proc, timeout=1.0)

            if response:
                # Check if it's a notification (no id) or a response (has id)
                if "id" not in response:
                    # Notification
                    notifications.append(response)
                    print(f"  📢 Notification: {response.get('method', 'unknown')}")

                    # Check for session/update with content
                    if response.get("method") == "session/update":
                        params = response.get("params", {})
                        update = params.get("update", {})
                        if update.get("sessionUpdate") == "agent_message_chunk":
                            content = update.get("content", {})
                            if content.get("type") == "text":
                                text = content.get("text", "")
                                print(f"     💬 Content: {text}")
                elif response.get("id") == 3:
                    # Final response to our prompt
                    final_response = response
                    print(f"  ✓ Final response received")
                    break

                timeout_counter = 0
            else:
                timeout_counter += 1
                time.sleep(0.1)

        print()
        print("=" * 60)
        print("Results")
        print("=" * 60)

        # Analyze results
        print(f"Notifications received: {len(notifications)}")

        # Check for session/update notifications
        update_notifications = [
            n for n in notifications if n.get("method") == "session/update"
        ]
        print(f"session/update notifications: {len(update_notifications)}")

        # Extract content
        content_chunks = []
        for notif in update_notifications:
            params = notif.get("params", {})
            update = params.get("update", {})
            if update.get("sessionUpdate") == "agent_message_chunk":
                content = update.get("content", {})
                if content.get("type") == "text":
                    text = content.get("text", "")
                    content_chunks.append(text)

        if content_chunks:
            print(f"✓ Content found: {len(content_chunks)} chunk(s)")
            print()
            print("Combined content:")
            print("-" * 60)
            for i, chunk in enumerate(content_chunks, 1):
                print(f"Chunk {i}: {chunk}")
            print("-" * 60)
        else:
            print("❌ No content found in notifications")

        # Check final response
        if final_response:
            if "error" in final_response:
                print(f"❌ Error in final response: {final_response['error']}")
                return 1
            else:
                result = final_response.get("result", {})
                stop_reason = result.get("stopReason")
                print(f"✓ Stop reason: {stop_reason}")
        else:
            print("❌ No final response received")
            return 1

        print()
        if content_chunks and final_response and "error" not in final_response:
            print("✅ Test PASSED - ACP protocol working correctly!")
            return 0
        else:
            print("❌ Test FAILED - Missing content or errors")
            return 1

    except KeyboardInterrupt:
        print("\nTest interrupted")
        return 1
    except Exception as e:
        print(f"❌ Exception: {e}")
        import traceback

        traceback.print_exc()
        return 1
    finally:
        # Clean shutdown
        try:
            proc.stdin.close()
            proc.wait(timeout=2)
        except:
            proc.kill()


if __name__ == "__main__":
    sys.exit(main())
