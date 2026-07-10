//! Integration tests for the ACP protocol layer — Task 111.3 regression baseline.
//!
//! Exercises the full initialize → session/new → session/load flow via
//! in-memory duplex pipes WITHOUT a real xAI API key.
//!
//!   cargo test --test acp_protocol

use grok_cli::acp::GrokAcpAgent;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

// ── helpers ───────────────────────────────────────────────────────────────────

fn test_config() -> grok_cli::config::Config {
    let mut cfg = grok_cli::config::Config::default();
    cfg.api_key = None;
    cfg
}

async fn send<W: AsyncWriteExt + Unpin>(w: &mut W, msg: &Value) {
    let mut bytes = serde_json::to_vec(msg).unwrap();
    bytes.push(b'\n');
    w.write_all(&bytes).await.unwrap();
    w.flush().await.unwrap();
}

async fn recv<R: tokio::io::AsyncRead + Unpin>(r: &mut BufReader<R>) -> Value {
    let mut line = String::new();
    let n = tokio::time::timeout(std::time::Duration::from_secs(5), r.read_line(&mut line))
        .await
        .expect("recv timeout (5 s)")
        .expect("recv I/O error");
    assert!(n > 0, "recv: EOF before response");
    serde_json::from_str(line.trim()).expect("recv: invalid JSON")
}

/// Read messages until one with the given numeric `id` arrives;
/// silently skip `session/update` notifications that have no `id`.
async fn recv_id<R: tokio::io::AsyncRead + Unpin>(r: &mut BufReader<R>, id: u64) -> Value {
    loop {
        let msg = recv(r).await;
        if msg.get("id").is_none() {
            continue;
        } // notification
        if msg["id"] == id {
            return msg;
        } // our response
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// initialize → session/new: checks protocol version, capabilities, sessionId,
/// and the `available_commands_update` notification that follows session/new.
#[tokio::test]
async fn test_initialize_and_session_new() {
    let agent = GrokAcpAgent::new(test_config(), None).await.unwrap();

    let (server_half, client_half) = tokio::io::duplex(64 * 1024);
    let (sr, sw) = tokio::io::split(server_half);
    let (cr, mut cw) = tokio::io::split(client_half);
    let mut cr = BufReader::new(cr);

    tokio::spawn(async move {
        grok_cli::cli::commands::acp::run_acp_session_for_test(sr, sw, agent)
            .await
            .ok();
    });

    // initialize
    // protocolVersion must be a JSON number (u16); the agent-client-protocol
    // crate rejects string values during deserialization.
    send(
        &mut cw,
        &json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":1,"clientInfo":{"name":"test"}}
        }),
    )
    .await;
    let r = recv_id(&mut cr, 1).await;
    assert!(r["error"].is_null(), "initialize error: {r}");
    assert!(
        !r["result"]["protocolVersion"].is_null(),
        "no protocolVersion: {r}"
    );
    assert!(
        r["result"]["agentCapabilities"].is_object(),
        "no agentCapabilities: {r}"
    );

    // session/new
    // Note: the crate's NewSessionRequest requires a `cwd` field.
    // Real clients (Zed, Gemini) always supply CWD; test uses ".".
    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":".","mcpServers":[]}}),
    )
    .await;
    let r = recv_id(&mut cr, 2).await;
    assert!(r["error"].is_null(), "session/new error: {r}");
    let sid = r["result"]["sessionId"]
        .as_str()
        .expect("missing sessionId");
    assert!(!sid.is_empty());

    // The server MUST send an available_commands_update notification next.
    let notif = recv(&mut cr).await;
    assert_eq!(
        notif["method"].as_str(),
        Some("session/update"),
        "expected session/update notification, got: {notif}"
    );
    assert_eq!(
        notif["params"]["update"]["sessionUpdate"].as_str(),
        Some("available_commands_update"),
        "unexpected update kind: {notif}"
    );
}

/// session/list must return a `sessions` array (even if empty).
#[tokio::test]
async fn test_session_list() {
    let agent = GrokAcpAgent::new(test_config(), None).await.unwrap();

    let (sh, ch) = tokio::io::duplex(16 * 1024);
    let (sr, sw) = tokio::io::split(sh);
    let (cr, mut cw) = tokio::io::split(ch);
    let mut cr = BufReader::new(cr);

    tokio::spawn(async move {
        grok_cli::cli::commands::acp::run_acp_session_for_test(sr, sw, agent)
            .await
            .ok();
    });

    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":1,"method":"session/list","params":{}}),
    )
    .await;
    let r = recv_id(&mut cr, 1).await;
    assert!(r["error"].is_null(), "session/list error: {r}");
    assert!(r["result"]["sessions"].is_array(), "no sessions array: {r}");
}

/// session/new + session/load round-trip.
#[tokio::test]
async fn test_session_load() {
    let agent = GrokAcpAgent::new(test_config(), None).await.unwrap();

    let (sh, ch) = tokio::io::duplex(64 * 1024);
    let (sr, sw) = tokio::io::split(sh);
    let (cr, mut cw) = tokio::io::split(ch);
    let mut cr = BufReader::new(cr);

    tokio::spawn(async move {
        grok_cli::cli::commands::acp::run_acp_session_for_test(sr, sw, agent)
            .await
            .ok();
    });

    // initialize + session/new
    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}}),
    )
    .await;
    let _ = recv_id(&mut cr, 1).await;

    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":".","mcpServers":[]}}),
    )
    .await;
    let r = recv_id(&mut cr, 2).await;
    let sid = r["result"]["sessionId"].as_str().unwrap().to_string();
    let _ = recv(&mut cr).await; // commands update

    // session/load
    // crate's LoadSessionRequest requires both cwd and mcpServers fields
    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":3,"method":"session/load","params":{"sessionId": sid, "cwd":".", "mcpServers":[]}}),
    )
    .await;
    let r = recv_id(&mut cr, 3).await;
    assert!(r["error"].is_null(), "session/load error: {r}");
    // result must be null or an array of history chunks
    // The spec allows null, array, or object for session/load result.
    // The crate's LoadSessionResponse serialises as {} when there is no history.
    assert!(
        r["result"].is_null() || r["result"].is_array() || r["result"].is_object(),
        "unexpected session/load result: {r}"
    );
}

/// session/fork creates a new session derived from the original.
///
/// Ignored: session/fork is a non-standard extension method. The crate's
/// Agent::builder() only routes standard ClientRequest variants. Custom methods
/// like session/fork need typed wrappers using #[derive(JsonRpcRequest)] —
/// tracked as a task 111.3 follow-up item.
#[tokio::test]
#[ignore]
async fn test_session_fork() {
    let agent = GrokAcpAgent::new(test_config(), None).await.unwrap();

    let (sh, ch) = tokio::io::duplex(64 * 1024);
    let (sr, sw) = tokio::io::split(sh);
    let (cr, mut cw) = tokio::io::split(ch);
    let mut cr = BufReader::new(cr);

    tokio::spawn(async move {
        grok_cli::cli::commands::acp::run_acp_session_for_test(sr, sw, agent)
            .await
            .ok();
    });

    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}}),
    )
    .await;
    let _ = recv_id(&mut cr, 1).await;
    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":".","mcpServers":[]}}),
    )
    .await;
    let r = recv_id(&mut cr, 2).await;
    let sid = r["result"]["sessionId"].as_str().unwrap().to_string();
    let _ = recv(&mut cr).await; // commands update

    send(
        &mut cw,
        &json!({"jsonrpc":"2.0","id":3,"method":"session/fork","params":{"sessionId": sid}}),
    )
    .await;
    let r = recv_id(&mut cr, 3).await;
    assert!(r["error"].is_null(), "session/fork error: {r}");
    let new_sid = r["result"]["newSessionId"]
        .as_str()
        .expect("missing newSessionId");
    assert!(!new_sid.is_empty());
    assert_ne!(new_sid, sid, "fork should produce a new session id");
}
