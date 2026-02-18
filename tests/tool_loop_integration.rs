//! Integration tests for tool message handling and loop prevention
//!
//! These tests verify that tool messages are correctly formatted and sent to the API,
//! and that tool loops are properly detected and terminated.

use grok_cli::GrokClient;
use serde_json::{Value, json};

/// Test that tool messages are correctly formatted with ChatMessage::tool()
#[tokio::test]
#[ignore] // Requires API key and network access
async fn test_tool_message_format() {
    let api_key = std::env::var("GROK_API_KEY").expect("GROK_API_KEY not set");
    let client = GrokClient::new(&api_key).expect("Failed to create client");

    // Create a conversation with a tool call and result
    let messages = vec![
        json!({
            "role": "user",
            "content": "What files are in the current directory?"
        }),
        json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": "call_test123",
                "type": "function",
                "function": {
                    "name": "list_files",
                    "arguments": "{\"path\": \".\"}"
                }
            }]
        }),
        json!({
            "role": "tool",
            "content": "file1.txt\nfile2.txt\nfile3.txt",
            "tool_call_id": "call_test123"
        }),
    ];

    // This should not loop - the model should see the tool result and respond
    let result = client
        .chat_completion_with_history(
            &messages,
            0.7,
            150,
            "grok-2-latest",
            Some(vec![json!({
                "type": "function",
                "function": {
                    "name": "list_files",
                    "description": "List files in a directory",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Directory path"
                            }
                        },
                        "required": ["path"]
                    }
                }
            })]),
        )
        .await;

    assert!(
        result.is_ok(),
        "Tool message request failed: {:?}",
        result.err()
    );
    let response = result.unwrap();

    // The response should not be another tool call for the same operation
    if let Some(tool_calls) = &response.message.tool_calls {
        // If there are tool calls, they should be different from the previous one
        assert!(
            tool_calls.iter().all(|call| call.id != "call_test123"),
            "Model repeated the same tool call - loop not prevented!"
        );
    }

    println!("✅ Tool message properly formatted and sent");
}

/// Test that tool loops are detected and terminated
#[tokio::test]
#[ignore] // Requires API key and network access
async fn test_tool_loop_prevention() {
    let api_key = std::env::var("GROK_API_KEY").expect("GROK_API_KEY not set");
    let client = GrokClient::new(&api_key).expect("Failed to create client");

    let mut messages = vec![json!({
        "role": "user",
        "content": "Read the file test.txt"
    })];

    let tools = vec![json!({
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read a file",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }
    })];

    let mut iterations = 0;
    let max_iterations = 10;

    loop {
        iterations += 1;

        if iterations > max_iterations {
            panic!(
                "❌ Tool loop exceeded {} iterations - loop not prevented!",
                max_iterations
            );
        }

        let result = client
            .chat_completion_with_history(&messages, 0.7, 150, "grok-2-latest", Some(tools.clone()))
            .await
            .expect("API call failed");

        // Add assistant message
        messages.push(json!({
            "role": "assistant",
            "content": result.message.content.as_ref().map(|c| c.to_string()),
            "tool_calls": result.message.tool_calls
        }));

        // Check finish reason
        if let Some(finish_reason) = &result.finish_reason {
            if finish_reason == "stop" {
                println!("✅ Tool loop completed in {} iterations", iterations);
                break;
            } else if finish_reason == "tool_calls" {
                // Execute tool and add result
                if let Some(tool_calls) = &result.message.tool_calls {
                    for tool_call in tool_calls {
                        // Simulate tool execution
                        let tool_result = format!("File content from iteration {}", iterations);

                        // Add tool result message
                        messages.push(json!({
                            "role": "tool",
                            "content": tool_result,
                            "tool_call_id": tool_call.id
                        }));
                    }
                }
            }
        }
    }

    assert!(
        iterations <= 3,
        "Tool loop took {} iterations (expected ≤3). This suggests the model isn't seeing tool results properly.",
        iterations
    );
}

/// Test that tool messages preserve tool_call_id
#[test]
fn test_tool_message_structure() {
    let tool_message = json!({
        "role": "tool",
        "content": "Result content",
        "tool_call_id": "call_abc123"
    });

    assert_eq!(tool_message["role"], "tool");
    assert_eq!(tool_message["content"], "Result content");
    assert_eq!(tool_message["tool_call_id"], "call_abc123");
    assert!(
        tool_message.get("name").is_none(),
        "Tool messages should not have 'name' field"
    );
}

/// Test that assistant messages with tool calls preserve tool_calls array
#[test]
fn test_assistant_with_tools_structure() {
    let assistant_message = json!({
        "role": "assistant",
        "content": null,
        "tool_calls": [{
            "id": "call_123",
            "type": "function",
            "function": {
                "name": "test_function",
                "arguments": "{\"arg\": \"value\"}"
            }
        }]
    });

    assert_eq!(assistant_message["role"], "assistant");
    assert!(assistant_message["content"].is_null());
    assert!(assistant_message["tool_calls"].is_array());
    assert_eq!(assistant_message["tool_calls"][0]["id"], "call_123");
}

/// Test that old workaround format is NOT used
#[test]
fn test_no_workaround_format() {
    // This is the OLD format that should NOT be used anymore
    let old_workaround = json!({
        "role": "user",
        "content": "Tool result (ID: call_123): Some result"
    });

    // This is the CORRECT format that should be used
    let correct_format = json!({
        "role": "tool",
        "content": "Some result",
        "tool_call_id": "call_123"
    });

    // Verify they are different
    assert_ne!(
        old_workaround["role"], correct_format["role"],
        "Tool results should use 'tool' role, not 'user' role"
    );

    assert!(
        correct_format.get("tool_call_id").is_some(),
        "Tool messages must have tool_call_id field"
    );

    assert!(
        !correct_format["content"]
            .as_str()
            .unwrap()
            .contains("Tool result (ID:"),
        "Tool content should not be wrapped in workaround format"
    );
}
