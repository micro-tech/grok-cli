use anyhow::Result;
use grok_cli::acp::protocol::{OutcomeDetail, PermissionOutcome, SessionId};
use grok_cli::acp::{GrokAcpAgent, PermissionBridge};
use grok_cli::config::Config;
use mockito::Server;
use serde_json::{Value, json};
use serial_test::serial;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Helper to create a standard mock response
fn mock_resp(content: Option<&str>, tool_calls: Option<Value>, finish_reason: &str) -> String {
    json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 123456789,
        "model": "grok-2-latest",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
                "tool_calls": tool_calls,
            },
            "finish_reason": finish_reason
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 10,
            "total_tokens": 20
        }
    })
    .to_string()
}

#[tokio::test]
#[serial]
async fn test_permission_proceed_once() -> Result<()> {
    let mut server = mockito::Server::new_async().await;
    unsafe {
        std::env::set_var("GROK_API_BASE_URL", server.url());
    }

    let mut config = Config::default();
    config.api_key = Some("test-key".to_string());
    config.acp.require_permission = true;
    config.acp.permission_timeout_secs = 2;

    let agent = GrokAcpAgent::new(config, None).await.unwrap();
    let session_id = SessionId::new("test-session-once");
    agent.initialize_session(session_id.clone(), None).await?;

    let tool_calls = json!([{
        "id": "call_1",
        "type": "function",
        "function": {
            "name": "run_shell_command",
            "arguments": "{\"command\": \"ls\"}"
        }
    }]);

    let _m1 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(None, Some(tool_calls), "tool_calls"))
        .expect(1)
        .create_async()
        .await;

    let _m2 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(
            Some("Here is the directory listing."),
            None,
            "stop",
        ))
        .expect(1)
        .create_async()
        .await;

    let (bridge, mut rx) = PermissionBridge::new();
    let bridge_arc = Arc::new(bridge);

    tokio::spawn(async move {
        if let Some((_req_id, _params, tx)) = rx.recv().await {
            let _ = tx.send(PermissionOutcome::proceed_once());
        }
    });

    let response = agent
        .handle_chat_completion(&session_id, "list files", None, None, Some(bridge_arc))
        .await?;

    assert_eq!(response, "Here is the directory listing.");
    assert!(
        !agent
            .is_always_allowed(&session_id, "run_shell_command")
            .await
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_permission_cancel() -> Result<()> {
    let mut server = mockito::Server::new_async().await;
    unsafe {
        std::env::set_var("GROK_API_BASE_URL", server.url());
    }

    let mut config = Config::default();
    config.api_key = Some("test-key".to_string());
    config.acp.require_permission = true;
    config.acp.permission_timeout_secs = 2;

    let agent = GrokAcpAgent::new(config, None).await.unwrap();
    let session_id = SessionId::new("test-session-cancel");
    agent.initialize_session(session_id.clone(), None).await?;

    let tool_calls = json!([{
        "id": "call_cancel",
        "type": "function",
        "function": {
            "name": "run_shell_command",
            "arguments": "{\"command\": \"rm -rf /\"}"
        }
    }]);

    let _m1 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(None, Some(tool_calls), "tool_calls"))
        .expect(1)
        .create_async()
        .await;

    let _m2 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(
            Some("I understand you rejected the command. I won't run it."),
            None,
            "stop",
        ))
        .expect(1)
        .create_async()
        .await;

    let (bridge, mut rx) = PermissionBridge::new();
    let bridge_arc = Arc::new(bridge);

    tokio::spawn(async move {
        if let Some((_req_id, _params, tx)) = rx.recv().await {
            let _ = tx.send(PermissionOutcome::cancel());
        }
    });

    let response = agent
        .handle_chat_completion(
            &session_id,
            "delete everything",
            None,
            None,
            Some(bridge_arc),
        )
        .await?;

    assert!(response.contains("rejected"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_permission_always_allow_persists() -> Result<()> {
    let mut server = mockito::Server::new_async().await;
    unsafe {
        std::env::set_var("GROK_API_BASE_URL", server.url());
    }

    let mut config = Config::default();
    config.api_key = Some("test-key".to_string());
    config.acp.require_permission = true;
    config.acp.permission_timeout_secs = 2;

    let agent = GrokAcpAgent::new(config, None).await.unwrap();
    let session_id = SessionId::new("test-session-always");
    agent.initialize_session(session_id.clone(), None).await?;

    let tool_calls = json!([{
        "id": "call_always",
        "type": "function",
        "function": {
            "name": "list_directory",
            "arguments": "{\"path\": \".\"}"
        }
    }]);

    let _m1 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(None, Some(tool_calls), "tool_calls"))
        .expect(2)
        .create_async()
        .await;

    let _m2 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(Some("Done."), None, "stop"))
        .expect(2)
        .create_async()
        .await;

    let (bridge, mut rx) = PermissionBridge::new();
    let bridge_arc = Arc::new(bridge);

    let bridge_clone = bridge_arc.clone();
    tokio::spawn(async move {
        if let Some((_req_id, _params, tx)) = rx.recv().await {
            let _ = tx.send(PermissionOutcome {
                outcome: OutcomeDetail::Selected {
                    option_id: "proceed_always".to_string(),
                },
            });
        }
    });

    agent
        .handle_chat_completion(&session_id, "list files", None, None, Some(bridge_arc))
        .await?;
    assert!(agent.is_always_allowed(&session_id, "list_directory").await);

    agent
        .handle_chat_completion(
            &session_id,
            "list files again",
            None,
            None,
            Some(bridge_clone),
        )
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_permission_timeout() -> Result<()> {
    let mut server = mockito::Server::new_async().await;
    unsafe {
        std::env::set_var("GROK_API_BASE_URL", server.url());
    }

    let mut config = Config::default();
    config.api_key = Some("test-key".to_string());
    config.acp.require_permission = true;
    config.acp.permission_timeout_secs = 1;

    let agent = GrokAcpAgent::new(config, None).await.unwrap();
    let session_id = SessionId::new("test-session-timeout");
    agent.initialize_session(session_id.clone(), None).await?;

    let tool_calls = json!([{
        "id": "call_timeout",
        "type": "function",
        "function": {
            "name": "run_shell_command",
            "arguments": "{\"command\": \"ls\"}"
        }
    }]);

    let _m1 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(None, Some(tool_calls), "tool_calls"))
        .expect(1)
        .create_async()
        .await;

    let (bridge, _rx) = PermissionBridge::new();
    let bridge_arc = Arc::new(bridge);

    let result = agent
        .handle_chat_completion(&session_id, "list files", None, None, Some(bridge_arc))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Timed out"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_permission_gate_disabled() -> Result<()> {
    let mut server = mockito::Server::new_async().await;
    unsafe {
        std::env::set_var("GROK_API_BASE_URL", server.url());
    }

    let mut config = Config::default();
    config.api_key = Some("test-key".to_string());
    config.acp.require_permission = false;

    let agent = GrokAcpAgent::new(config, None).await.unwrap();
    let session_id = SessionId::new("test-session-disabled");
    agent.initialize_session(session_id.clone(), None).await?;

    let tool_calls = json!([{
        "id": "call_no_perm",
        "type": "function",
        "function": {
            "name": "run_shell_command",
            "arguments": "{\"command\": \"ls\"}"
        }
    }]);

    let _m1 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(None, Some(tool_calls), "tool_calls"))
        .expect(1)
        .create_async()
        .await;

    let _m2 = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_body(mock_resp(Some("Done."), None, "stop"))
        .expect(1)
        .create_async()
        .await;

    let response = agent
        .handle_chat_completion(&session_id, "list files", None, None, None)
        .await?;

    assert_eq!(response, "Done.");
    Ok(())
}
